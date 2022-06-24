use crate::connection::{invitation::Invitation, Connections, Termination, TerminationResponse};
use crate::credential::{issue::Issuance, Credentials};
use crate::message::{MessageEvent, MessageEvents};
use crate::ping::{PingEvent, PingEvents};
use crate::wallet::Wallet;
use async_trait::async_trait;
use base64::decode;
use did_key::KeyMaterial;
use didcomm_mediator::protocols::trustping::TrustPingResponseBuilder;
use didcomm_rs::Jwe;
use didcomm_rs::{
    crypto::{CryptoAlgorithm, SignatureAlgorithm},
    Message,
};
use identity_iota::client::ResolvedIotaDocument;
use identity_iota::client::Resolver;
use identity_iota::credential::Credential;
use identity_iota::did::MethodScope;
use identity_iota::iota_core::{IotaDID, IotaVerificationMethod};
use identity_iota::prelude::{KeyPair, KeyType};
use reqwest::RequestBuilder;
use rocket::http::Status;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

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
#[options("/")]
pub fn didcomm_options() -> Status {
    Status::Ok
}

#[openapi(tag = "didcomm")]
#[post("/", format = "any", data = "<body>")]
pub async fn post_endpoint(
    wallet: &State<Arc<Mutex<Wallet>>>,
    connections: &State<Connections>,
    credentials: &State<Credentials>,
    ping_events: &State<Arc<Mutex<PingEvents>>>,
    message_events: &State<Arc<Mutex<MessageEvents>>>,
    body: Json<Value>,
) -> Result<Json<Value>, Status> {
    let body_str = serde_json::to_string(&body.into_inner()).unwrap();
    let (my_did, private_key) = {
        let wallet = wallet.try_lock().unwrap();
        (
            wallet.did_iota().unwrap(),
            wallet.keypair().private_key_bytes(),
        )
    };
    let received: Message = match receive(&body_str, &private_key, None).await {
        Ok(received) => received,
        Err(_) => return Err(Status::BadRequest),
    };
    match received.get_didcomm_header().m_type.as_str() {
        "https://didcomm.org/out-of-band/2.0/invitation" => {
            let invitation: Invitation =
                serde_json::from_str(&received.get_body().unwrap()).unwrap();
            info!("invitation = {:?}", invitation.id);
            Ok(Json(json!({})))
        }
        "https://didcomm.org/trust-ping/2.0/ping" => {
            let did_to = received.get_didcomm_header().from.clone().unwrap();
            let response = TrustPingResponseBuilder::new()
                .message(received.clone())
                .build()
                .unwrap();
            ping_events
                .try_lock()
                .unwrap()
                .send(PingEvent::Received(
                    received
                        .get_didcomm_header()
                        .from
                        .as_ref()
                        .unwrap()
                        .to_string(),
                ))
                .await;
            let keypair =
                KeyPair::try_from_private_key_bytes(KeyType::X25519, &private_key).unwrap();
            let ping_response = sign_and_encrypt(&response, &my_did, &did_to, &keypair)
                .await
                .unwrap();
            Ok(Json(json!(ping_response)))
        }
        "https://didcomm.org/issue-credential/2.1/issue-credential" => {
            for attachment in received.get_attachments() {
                let credential = decode(attachment.data.base64.as_ref().unwrap()).unwrap();
                let credential = std::str::from_utf8(&credential).unwrap();
                let credential: Credential = serde_json::from_str(credential).unwrap();

                info!("issuance: {:?}", credential);
                let mut lock = credentials.credentials.lock().await;

                lock.insert(credential.id.clone().unwrap().to_string(), credential);
            }
            Ok(Json(json!({})))
        }
        "https://didcomm.org/basicmessage/2.0/message" => {
            let did_from = received.get_didcomm_header().from.clone().unwrap();
            let payload = received.get_body().unwrap();
            message_events
                .try_lock()
                .unwrap()
                .send(MessageEvent::Received(did_from, payload))
                .await;
            Ok(Json(json!({})))
        }
        "iota/termination/0.1/termination" => {
            let termination: Termination =
                serde_json::from_str(&received.get_body().unwrap()).unwrap();
            let mut lock = connections.connections.lock().await;
            lock.remove(&termination.id).unwrap();
            std::mem::drop(lock);
            let termination: TerminationResponse = TerminationResponse {
                typ: "application/didcomm-plain+json".to_string(),
                type_: "iota/termination/0.1/termination-response".to_string(),
                id: termination.id,
                body: Value::default(),
            };
            Ok(Json(json!(termination)))
        }
        "iota/issuance/0.1/issuance" => {
            let issuance: Issuance = serde_json::from_str(&received.get_body().unwrap()).unwrap();
            let credential = issuance.signed_credential;
            info!("issuance: {:?}", credential);
            let mut lock = credentials.credentials.lock().await;

            lock.insert(credential.id.clone().unwrap().to_string(), credential);
            Ok(Json(json!({})))
        }
        _ => Ok(Json(json!({}))),
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
        Some(value) => Some(value.to_vec()),
        None => match serde_json::from_str::<Jwe>(message) {
            Ok(jwe) => {
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
                Some(sender_method?.data().try_decode().unwrap())
            }
            Err(_) => None,
        },
    };

    Message::receive(
        message,
        Some(encryption_recipient_private_key),
        sender_public_key,
        None,
    )
}
