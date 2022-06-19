use crate::credential::Credentials;
use crate::wallet::Wallet;
use identity_iota::core::Url;
use identity_iota::credential::Presentation;
use identity_iota::credential::PresentationBuilder;
use identity_iota::did::DID;
use identity_iota::iota_core::IotaDID;
use identity_iota::prelude::KeyPair;
use identity_iota::prelude::*;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

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
#[post("/present-proof/send-proposal", data = "<proof_request>")]
pub async fn post_send_proposal(
    wallet: &State<Arc<Mutex<Wallet>>>,
    credentials: &State<Credentials>,
    proof_request: Json<ProofRequest>,
) -> Json<Value> {
    let wallet = wallet.try_lock().unwrap();
    let iota_did: IotaDID = IotaDID::from_str(&wallet.did_iota().unwrap()).unwrap();
    let did = iota_did.clone();

    let proof_request = proof_request.into_inner();

    let presentation_key: KeyPair = KeyPair::new(KeyType::Ed25519).unwrap();
    let presentation_did: IotaDID = IotaDID::new(presentation_key.public().as_ref()).unwrap();

    let credentials = credentials.credentials.lock().await;
    let credential = credentials.get(&proof_request.credential_id).unwrap();

    let presentation: Presentation = PresentationBuilder::default()
        .id(Url::parse(presentation_did.as_str()).unwrap())
        .holder(Url::parse(did.as_str()).unwrap())
        .credential(credential.clone())
        .build()
        .unwrap();

    Json(json!(presentation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;

    #[tokio::test]
    async fn test_send_proposal() {
        let client = Client::tracked(test_rocket().await)
            .await
            .expect("valid rocket instance");

        let proof_request = ProofRequest {
            connection_id: "foo".to_string(),
            credential_id: "bar".to_string(),
        };
        let invitation: String = serde_json::to_string(&proof_request).unwrap();

        let response = client
            .post("/present-proof/send-proposal")
            .header(ContentType::JSON)
            .body(invitation)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::InternalServerError);
    }
}
