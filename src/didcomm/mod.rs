use crate::connection::{invitation::Invitation, Connections, Termination, TerminationResponse};
use crate::credential::{issue::Issuance, Credentials};
use crate::message::MessageRequest;
use crate::ping::{PingRequest, PingResponse};
use crate::webhook::Webhook;
use async_trait::async_trait;
use didcomm_rs::Jwe;
use didcomm_rs::{
    crypto::{CryptoAlgorithm, SignatureAlgorithm},
    Message,
};
use identity::did::MethodScope;
use identity::iota::ResolvedIotaDocument;
use identity::iota::Resolver;
use identity::iota_core::{IotaDID, IotaVerificationMethod};
use identity::prelude::{KeyPair, KeyType};
use reqwest::RequestBuilder;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::{json, Value};
use std::str::FromStr;
use uuid::Uuid;

pub mod client;
#[cfg(test)]
pub mod test_client;
#[cfg(test)]
pub mod tests;

pub use client::Client;

#[async_trait]
pub trait DidComm: Send + Sync {
    fn request(&self, endpoint: &str, body: &Value) -> RequestBuilder;
    async fn post(&self, endpoint: &str, body: &Value)
        -> Result<reqwest::Response, reqwest::Error>;
}

#[openapi(tag = "didcomm")]
#[post("/", format = "application/json", data = "<data>")]
pub async fn post_endpoint(
    webhook: &State<Box<dyn Webhook>>,
    connections: &State<Connections>,
    credentials: &State<Credentials>,
    data: Json<Value>,
) -> Json<Value> {
    match data["type"].as_str().unwrap() {
        "https://didcomm.org/out-of-band/2.0/invitation" => {
            let invitation: Invitation = serde_json::from_value(data.into_inner()).unwrap();
            info!("invitation = {:?}", invitation.id);
            Json(json!({}))
        }
        "https://didcomm.org/trust-ping/2.0/ping" => {
            let ping_request: PingRequest = serde_json::from_value(data.into_inner()).unwrap();
            let ping_response: PingResponse = PingResponse {
                type_: "https://didcomm.org/trust-ping/2.0/ping-response".to_string(),
                id: Uuid::new_v4().to_string(),
                thid: ping_request.id,
            };
            Json(json!(ping_response))
        }
        "iota/post/0.1/post" => {
            let message_request: MessageRequest =
                serde_json::from_value(data.into_inner()).unwrap();
            info!("message: {:?}", message_request.payload);
            webhook
                .post("message", &message_request.payload)
                .await
                .unwrap();
            Json(json!({}))
        }
        "iota/termination/0.1/termination" => {
            let termination: Termination = serde_json::from_value(data.into_inner()).unwrap();
            let mut lock = connections.connections.lock().await;
            lock.remove(&termination.id).unwrap();
            std::mem::drop(lock);
            let termination: TerminationResponse = TerminationResponse {
                typ: "application/didcomm-plain+json".to_string(),
                type_: "iota/termination/0.1/termination-response".to_string(),
                id: termination.id,
                body: Value::default(),
            };
            Json(json!(termination))
        }
        "iota/issuance/0.1/issuance" => {
            let issuance: Issuance = serde_json::from_value(data.into_inner()).unwrap();
            let credential = issuance.signed_credential;
            info!("issuance: {:?}", credential);
            let mut lock = credentials.credentials.lock().await;

            lock.insert(credential.id.clone().unwrap().to_string(), credential);
            Json(json!({}))
        }
        _ => Json(json!({})),
    }
}

pub async fn sign_and_encrypt(
    message: &Message,
    did_from: &String,
    did_to: &String,
    key: &KeyPair,
) -> Result<Value, Box<dyn std::error::Error>> {
    let sign_key = KeyPair::new(KeyType::Ed25519).unwrap();

    let resolver: Resolver = Resolver::new().await?;

    let recipient_did = IotaDID::from_str(did_to)?;
    let recipient_document: ResolvedIotaDocument = resolver.resolve(&recipient_did).await?;
    let recipient_method: &IotaVerificationMethod = recipient_document
        .document
        .resolve_method("kex-0", Some(MethodScope::VerificationMethod))
        .unwrap();
    let recipient_key: Vec<u8> = recipient_method.data().try_decode()?;

    let response = message
        .clone()
        .from(did_from)
        .to(&[did_to])
        .as_jwe(&CryptoAlgorithm::XC20P, Some(recipient_key.to_vec()))
        .kid(&hex::encode(sign_key.public().as_ref()));

    let ready_to_send = response
        .seal_signed(
            key.private().as_ref(),
            Some(vec![Some(recipient_key)]),
            SignatureAlgorithm::EdDsa,
            &[sign_key.private().as_ref(), sign_key.public().as_ref()].concat(),
        )
        .unwrap();
    Ok(serde_json::from_str(&ready_to_send).unwrap())
}

pub async fn receive(
    message: &String,
    encryption_recipient_private_key: &[u8],
    encryption_sender_public_key: Option<Vec<u8>>,
) -> Result<Message, didcomm_rs::Error> {
    let sender_public_key = match &encryption_sender_public_key {
        Some(value) => value.to_vec(),
        None => {
            let jwe: Jwe = serde_json::from_str(message)?;
            let skid = &jwe
                .get_skid()
                .ok_or_else(|| didcomm_rs::Error::Generic("skid missing".to_string()))
                .unwrap();

            let resolver: Resolver = Resolver::new().await.unwrap();
            let sender_did = IotaDID::from_str(skid).unwrap();
            let sender_document = match resolver.resolve(&sender_did).await {
                Ok(did) => Ok(did),
                Err(_) => Err(didcomm_rs::Error::DidResolveFailed),
            }?;
            let sender_method = sender_document
                .document
                .resolve_method("kex-0", Some(MethodScope::VerificationMethod))
                .ok_or(didcomm_rs::Error::DidResolveFailed);
            sender_method?.data().try_decode().unwrap()
        }
    };

    Message::receive(
        message,
        Some(encryption_recipient_private_key),
        Some(sender_public_key),
        None,
    )
}
