use crate::connection::Connections;
use crate::wallet::Wallet;
use didcomm_mediator::message::{add_return_route_all_header, sign_and_encrypt};
use didcomm_protocols::{CredentialAttribute, CredentialPreview, IssueCredentialResponseBuilder};
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

fn example_credential_preview() -> CredentialPreview {
    CredentialPreview {
        type_: "https://didcomm.org/issue-credential/2.1/credential-preview".to_string(),
        attributes: vec![CredentialAttribute::new(
            "favourite_drink".to_string(),
            "martini".to_string(),
        )],
    }
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

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CreateProposalRequest {
    #[schemars(example = "example_connection_id")]
    pub connection_id: String,
    pub comment: String,
    #[schemars(example = "example_credential_preview")]
    pub credential_preview: CredentialPreview,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CreateOfferRequest {
    #[schemars(example = "example_connection_id")]
    pub connection_id: String,
    pub comment: String,
    #[schemars(example = "example_credential_preview")]
    pub credential_preview: CredentialPreview,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SendRequest {
    #[schemars(example = "example_connection_id")]
    pub connection_id: String,
    pub comment: String,
    #[schemars(example = "example_credential_preview")]
    pub credential_preview: CredentialPreview,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Issuance {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "signedCredential")]
    pub signed_credential: Credential,
}

/// # Send holder a credential offer, independent of any proposal
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

/// # Send issuer a credential proposal
#[openapi(tag = "issue-credential v2.1")]
#[post("/issue-credential-2.1/send-proposal", data = "<request>")]
pub async fn post_send_proposal_2(
    wallet: &State<Arc<Mutex<Wallet>>>,
    connections: &State<Connections>,
    request: Json<CreateProposalRequest>,
) -> Result<Json<Value>, Status> {
    let wallet = wallet.try_lock().unwrap();
    let did_from = wallet.did_iota().unwrap();
    let keypair = wallet.keypair();
    drop(wallet);

    let request = request.into_inner();

    let (did_to, endpoint) = {
        let connections = connections.connections.lock().await;
        let connection = connections.get(&request.connection_id).unwrap().clone();
        (connection.did.to_string(), connection.endpoint)
    };

    let mut offer = IssueCredentialResponseBuilder::new()
        .goal_code("issue-vc".to_string())
        .comment(request.comment)
        .credential_preview(request.credential_preview)
        .build_propose_credential()
        .unwrap();
    offer = add_return_route_all_header(offer);
    let message = sign_and_encrypt(&offer, &did_from, &did_to, &keypair)
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let _res = client
        .post(endpoint.to_string())
        .json(&message)
        .send()
        .await
        .unwrap();
    /*
    let json: Value = res.json().await.unwrap();
    let body_str = serde_json::to_string(&json).unwrap();

    let _received = match receive(&body_str, Some(&keypair.private_key_bytes()), None, None).await {
        Ok(received) => received,
        Err(_) => return Err(Status::BadRequest),
    };
    */
    Ok(Json(json!(offer)))
}

/// # Send holder a credential offer, independent of any proposal
#[openapi(tag = "issue-credential v2.1")]
#[post("/issue-credential-2.1/send-offer", data = "<request>")]
pub async fn post_send_offer_2(
    wallet: &State<Arc<Mutex<Wallet>>>,
    connections: &State<Connections>,
    request: Json<CreateOfferRequest>,
) -> Result<Json<Value>, Status> {
    let wallet = wallet.try_lock().unwrap();
    let did_from = wallet.did_iota().unwrap();
    let keypair = wallet.keypair();
    drop(wallet);

    let request = request.into_inner();

    let (did_to, endpoint) = {
        let connections = connections.connections.lock().await;
        let connection = connections.get(&request.connection_id).unwrap().clone();
        (connection.did.to_string(), connection.endpoint)
    };

    let mut offer = IssueCredentialResponseBuilder::new()
        .goal_code("issue-vc".to_string())
        .comment(request.comment)
        .credential_preview(request.credential_preview)
        .build_offer_credential()
        .unwrap();
    offer = add_return_route_all_header(offer);
    let message = sign_and_encrypt(&offer, &did_from, &did_to, &keypair)
        .await
        .unwrap();

    let client = reqwest::Client::new();
    match client
        .post(endpoint.to_string())
        .json(&message)
        .send()
        .await
    {
        Ok(_) => Ok(Json(json!(offer))),
        Err(err) => {
            println!("{:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

/// # Send holder a credential
#[openapi(tag = "issue-credential v2.1")]
#[post("/issue-credential-2.1/send", data = "<request>")]
pub async fn post_send_2(
    wallet: &State<Arc<Mutex<Wallet>>>,
    connections: &State<Connections>,
    request: Json<SendRequest>,
) -> Result<Json<Value>, Status> {
    let wallet = wallet.try_lock().unwrap();
    let iota_did: IotaDID = IotaDID::from_str(&wallet.did_iota().unwrap()).unwrap();
    let did = iota_did.clone();

    let request = request.into_inner();

    let (did_to, endpoint) = {
        let connections = connections.connections.lock().await;
        let connection = connections.get(&request.connection_id).unwrap().clone();
        (connection.did.to_string(), connection.endpoint)
    };

    let subject_key: KeyPair = KeyPair::new(KeyType::Ed25519).unwrap();
    let subject_did: IotaDID = IotaDID::new(subject_key.public().as_ref()).unwrap();

    let subject: Subject =
        Subject::from_json_value(json!({"id": Url::parse(subject_did.as_str()).unwrap(), "attributes": request.credential_preview.attributes}))
            .unwrap();

    let mut credential: Credential = CredentialBuilder::default()
        .id(Url::parse("https://example.edu/credentials/3732").unwrap())
        .issuer(Url::parse(did.as_str()).unwrap())
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
    let did_from = wallet.did_iota().unwrap();
    let keypair = wallet.keypair();
    drop(wallet);

    let attachment = serde_json::to_value(&credential).unwrap();
    let mut issue = IssueCredentialResponseBuilder::new()
        .goal_code("issue-vc".to_string())
        .comment(request.comment)
        .credential_preview(request.credential_preview)
        .attachment(attachment)
        .build_issue_credential()
        .unwrap();
    issue = add_return_route_all_header(issue);
    let message = sign_and_encrypt(&issue, &did_from, &did_to, &keypair)
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let res = client
        .post(endpoint.to_string())
        .json(&message)
        .send()
        .await
        .unwrap();
    match res.json::<Value>().await.is_ok() {
        true => Ok(Json(json!(issue))),
        false => Err(Status::InternalServerError),
    }
}
