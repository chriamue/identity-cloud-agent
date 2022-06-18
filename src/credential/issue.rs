use crate::connection::Connections;
use crate::wallet::Wallet;
use identity_iota::core::FromJson;
use identity_iota::core::Url;
use identity_iota::credential::Credential;
use identity_iota::credential::CredentialBuilder;
use identity_iota::credential::Subject;
use identity_iota::crypto::ProofOptions;
use identity_iota::did::DID;
use identity_iota::iota_core::IotaDID;
use identity_iota::prelude::KeyPair;
use identity_iota::prelude::*;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars::{self, JsonSchema};
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

fn example_type() -> &'static str {
    "UniversityDegreeCredential"
}

fn example_connection_id() -> &'static str {
    "2fecc993-b92c-4152-8c81-35adde124382"
}

fn example_attributes() -> Value {
    json!({
        "degree": {
            "type": "BachelorDegree",
            "name": "Bachelor of Science and Arts"
          }
    })
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct IssueRequest {
    #[serde(rename = "type")]
    #[schemars(example = "example_type")]
    pub type_: String,
    #[schemars(example = "example_connection_id")]
    pub connection_id: String,
    #[schemars(example = "example_attributes")]
    pub attributes: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Issuance {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "signedCredential")]
    pub signed_credential: Credential,
}

#[openapi(tag = "issue-credential")]
#[post("/issue-credential/send-offer", data = "<issue_request>")]
pub async fn post_send_offer(
    wallet: &State<Arc<Mutex<Wallet>>>,
    connections: &State<Connections>,
    issue_request: Json<IssueRequest>,
) -> Json<Value> {
    let wallet = wallet.try_lock().unwrap();
    let iota_did: IotaDID = IotaDID::from_str(&wallet.did_iota().unwrap()).unwrap();
    let did = iota_did.clone();

    let issue_request = issue_request.into_inner();
    let conn_id = issue_request.connection_id;

    let connections = connections.connections.lock().await;
    let connection = connections.get(&conn_id).unwrap().clone();
    std::mem::drop(connections);

    let subject_key: KeyPair = KeyPair::new(KeyType::Ed25519).unwrap();
    let subject_did: IotaDID = IotaDID::new(subject_key.public().as_ref()).unwrap();

    let mut subject: Subject = Subject::from_json_value(issue_request.attributes).unwrap();
    subject.id = Some(Url::parse(subject_did.as_str()).unwrap());

    let mut credential: Credential = CredentialBuilder::default()
        .id(Url::parse("https://example.edu/credentials/3732").unwrap())
        .issuer(Url::parse(did.as_str()).unwrap())
        .type_(issue_request.type_)
        .subject(subject)
        .build()
        .unwrap();

    wallet
        .account
        .as_ref()
        .unwrap()
        .sign("sign-0", &mut credential, ProofOptions::default())
        .await
        .unwrap();

    let issue = Issuance {
        type_: "iota/issuance/0.1/issuance".to_string(),
        signed_credential: credential.clone(),
    };

    let client = reqwest::Client::new();
    let _res = client
        .post(connection.endpoint.to_string())
        .json(&issue)
        .send()
        .await
        .unwrap();
    Json(json!(credential))
}
