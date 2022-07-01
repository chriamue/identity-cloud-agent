use crate::connection::Connections;
use crate::credential::Credentials;
use crate::wallet::Wallet;
use didcomm_mediator::message::{add_return_route_all_header, sign_and_encrypt};
use didcomm_protocols::PresentProofResponseBuilder;
use identity_iota::core::Url;
use identity_iota::credential::Presentation;
use identity_iota::credential::PresentationBuilder;
use identity_iota::did::DID;
use identity_iota::iota_core::IotaDID;
use identity_iota::prelude::KeyPair;
use identity_iota::prelude::*;
use rocket::http::Status;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars::{self, JsonSchema};
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod events;
pub mod proposal;

pub use events::{PresentProofEvent, PresentProofEvents};

fn example_connection_id() -> &'static str {
    "2fecc993-b92c-4152-8c81-35adde124382"
}

fn example_credential_id() -> &'static str {
    "https://example.edu/credentials/3732"
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ProofRequest {
    #[schemars(example = "example_connection_id")]
    pub connection_id: String,
    #[schemars(example = "example_credential_id")]
    pub credential_id: String,
}

#[openapi(tag = "present-proof")]
#[post("/present-proof/send-presentation", data = "<request>")]
pub async fn post_send_presentation(
    wallet: &State<Arc<Mutex<Wallet>>>,
    credentials: &State<Credentials>,
    connections: &State<Connections>,
    request: Json<ProofRequest>,
) -> Result<Json<Value>, Status> {
    let (did_to, endpoint) = {
        let connections = connections.connections.lock().await;
        let connection = connections.get(&request.connection_id).unwrap().clone();
        (connection.did.to_string(), connection.endpoint)
    };
    let wallet = wallet.try_lock().unwrap();
    let iota_did: IotaDID = IotaDID::from_str(&wallet.did_iota().unwrap()).unwrap();
    let did = iota_did.clone();
    let did_from = wallet.did_iota().unwrap();
    let keypair = wallet.keypair();
    drop(wallet);

    let request = request.into_inner();

    let presentation_key: KeyPair = KeyPair::new(KeyType::Ed25519).unwrap();
    let presentation_did: IotaDID = IotaDID::new(presentation_key.public().as_ref()).unwrap();

    let credentials = credentials.credentials.lock().await;
    let credential = credentials.get(&request.credential_id).unwrap();

    let presentation: Presentation = PresentationBuilder::default()
        .id(Url::parse(presentation_did.as_str()).unwrap())
        .holder(Url::parse(did.as_str()).unwrap())
        .credential(credential.clone())
        .build()
        .unwrap();

    let attachment = serde_json::to_value(&presentation).unwrap();
    let mut proof = PresentProofResponseBuilder::new()
        .goal_code("present-proof".to_string())
        .attachment(attachment)
        .build_presentation()
        .unwrap();
    proof = add_return_route_all_header(proof);
    let message = sign_and_encrypt(&proof, &did_from, &did_to, &keypair)
        .await
        .unwrap();

    let client = reqwest::Client::new();
    match client
        .post(endpoint.to_string())
        .json(&message)
        .send()
        .await
    {
        Ok(_) => Ok(Json(json!(presentation))),
        Err(err) => {
            println!("{:?}", err);
            Err(Status::InternalServerError)
        }
    }
}
