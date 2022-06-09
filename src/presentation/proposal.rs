use crate::credential::Credentials;
use crate::wallet::Wallet;
use identity::core::Url;
use identity::credential::Presentation;
use identity::credential::PresentationBuilder;
use identity::did::DID;
use identity::iota_core::IotaDID;
use identity::prelude::KeyPair;
use identity::prelude::*;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    wallet: &State<Wallet>,
    credentials: &State<Credentials>,
    proof_request: Json<ProofRequest>,
) -> Json<Value> {
    let account = wallet.account.lock().await;
    let iota_did: &IotaDID = account.did();
    let did = iota_did.clone();
    std::mem::drop(account);

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
    use rocket::local::blocking::Client;

    #[test]
    fn test_send_proposal() {
        let client = Client::tracked(test_rocket()).expect("valid rocket instance");

        let proof_request = ProofRequest {
            connection_id: "foo".to_string(),
            credential_id: "bar".to_string(),
        };
        let invitation: String = serde_json::to_string(&proof_request).unwrap();

        let response = client
            .post("/present-proof/send-proposal")
            .header(ContentType::JSON)
            .body(invitation)
            .dispatch();
        assert_eq!(response.status(), Status::InternalServerError);
    }
}