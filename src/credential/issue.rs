use crate::connection::Connections;
use crate::wallet::Wallet;
use identity::core::FromJson;
use identity::core::Url;
use identity::credential::Credential;
use identity::credential::CredentialBuilder;
use identity::credential::Subject;
use identity::crypto::SignatureOptions;
use identity::did::resolution;
use identity::did::resolution::InputMetadata;
use identity::iota::ClientMap;
use identity::iota::IotaDID;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::thread;

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
    wallet: &State<Wallet>,
    connections: &State<Connections>,
    issue_request: Json<IssueRequest>,
) -> Json<Value> {
    let lock = wallet.account.lock().await;
    let iota_did: &IotaDID = lock.did();
    let did = iota_did.clone();

    let client: ClientMap = ClientMap::new();
    let input: InputMetadata = Default::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let output = thread::spawn(move || {
        runtime.block_on(resolution::resolve(did.to_string(), input, &client))
    })
    .join()
    .expect("Thread panicked")
    .unwrap();

    let document = output.document.unwrap();

    let issue_request = issue_request.into_inner();
    let conn_id = issue_request.connection_id;

    let lock = connections.connections.lock().await;
    let connection = lock.get(&conn_id).unwrap().clone();

    let subject: Subject = Subject::from_json_value(issue_request.attributes).unwrap();
    std::mem::drop(lock);
    let mut credential: Credential = CredentialBuilder::default()
        .id(Url::parse("https://example.edu/credentials/3732").unwrap())
        .issuer(Url::parse(document.id().to_string().as_str()).unwrap())
        .type_(issue_request.type_)
        .subject(subject)
        .build()
        .unwrap();

    let account = wallet.account.lock().await;
    account
        .sign("key-1", &mut credential, SignatureOptions::default())
        .await
        .unwrap();

    let issue = Issuance {
        type_: "iota/issuance/0.1/issuance".to_string(),
        signed_credential: credential.clone(),
    };
    info!("issue: {:?}", issue);

    let client = reqwest::Client::new();
    let _res = client
        .post(connection.endpoint.to_string())
        .json(&issue)
        .send()
        .await
        .unwrap();
    Json(json!(credential))
}
