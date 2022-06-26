use crate::connection::Connections;
use crate::wallet::Wallet;
use didcomm_mediator::message::{add_return_route_all_header, sign_and_encrypt};
use didcomm_protocols::{CredentialAttribute, CredentialPreview, IssueCredentialResponseBuilder};
use didcomm_rs::Message;
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
use std::sync::Arc;
use tokio::sync::Mutex;

pub fn example_type() -> &'static str {
    "UniversityDegreeCredential"
}

pub fn example_connection_id() -> &'static str {
    "2fecc993-b92c-4152-8c81-35adde124382"
}

pub fn example_attributes() -> Value {
    json!({
        "degree": {
            "type": "BachelorDegree",
            "name": "Bachelor of Science and Arts"
          }
    })
}

pub fn example_credential_preview() -> CredentialPreview {
    CredentialPreview {
        type_: "https://didcomm.org/issue-credential/2.1/credential-preview".to_string(),
        attributes: vec![CredentialAttribute::new(
            "favourite_drink".to_string(),
            "martini".to_string(),
        )],
    }
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

pub async fn prepare_proposal_request(
    wallet: &Wallet,
    did_to: String,
    request: CreateProposalRequest,
) -> Result<(Message, Value), Box<dyn std::error::Error>> {
    let mut proposal = IssueCredentialResponseBuilder::new()
        .goal_code("issue-vc".to_string())
        .comment(request.comment)
        .credential_preview(request.credential_preview)
        .build_propose_credential()
        .unwrap();
    proposal = add_return_route_all_header(proposal);
    let did_from = wallet.did_iota().unwrap();
    let keypair = wallet.keypair();
    let message = sign_and_encrypt(&proposal, &did_from, &did_to, &keypair)
        .await
        .unwrap();
    Ok((proposal, message))
}

/// # Send issuer a credential proposal
#[openapi(tag = "issue-credential v2.1")]
#[post("/issue-credential-2.1/send-proposal", data = "<request>")]
pub async fn post_send_proposal_2(
    wallet: &State<Arc<Mutex<Wallet>>>,
    connections: &State<Connections>,
    request: Json<CreateProposalRequest>,
) -> Result<Json<Value>, Status> {
    let (did_to, endpoint) = {
        let connections = connections.connections.lock().await;
        let connection = connections.get(&request.connection_id).unwrap().clone();
        (connection.did.to_string(), connection.endpoint)
    };
    let request = request.into_inner();

    let (offer, message) = prepare_proposal_request(&wallet.try_lock().unwrap(), did_to, request)
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
        true => Ok(Json(json!(offer))),
        false => Err(Status::InternalServerError),
    }
}

pub async fn prepare_offer_request(
    wallet: &Wallet,
    did_to: String,
    request: CreateOfferRequest,
) -> Result<(Message, Value), Box<dyn std::error::Error>> {
    let mut offer = IssueCredentialResponseBuilder::new()
        .goal_code("issue-vc".to_string())
        .comment(request.comment)
        .credential_preview(request.credential_preview)
        .build_offer_credential()
        .unwrap();
    offer = add_return_route_all_header(offer);
    let did_from = wallet.did_iota().unwrap();
    let keypair = wallet.keypair();
    let message = sign_and_encrypt(&offer, &did_from, &did_to, &keypair)
        .await
        .unwrap();
    Ok((offer, message))
}

/// # Send holder a credential offer, independent of any proposal
#[openapi(tag = "issue-credential v2.1")]
#[post("/issue-credential-2.1/send-offer", data = "<request>")]
pub async fn post_send_offer_2(
    wallet: &State<Arc<Mutex<Wallet>>>,
    connections: &State<Connections>,
    request: Json<CreateOfferRequest>,
) -> Result<Json<Value>, Status> {
    let (did_to, endpoint) = {
        let connections = connections.connections.lock().await;
        let connection = connections.get(&request.connection_id).unwrap().clone();
        (connection.did.to_string(), connection.endpoint)
    };

    let request = request.into_inner();

    let (offer, message) = prepare_offer_request(&wallet.try_lock().unwrap(), did_to, request)
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

pub async fn prepare_issue_credential_request(
    wallet: &Wallet,
    did_to: String,
    request: SendRequest,
) -> Result<(Message, Value), Box<dyn std::error::Error>> {
    let subject_key: KeyPair = KeyPair::new(KeyType::Ed25519).unwrap();
    let subject_did: IotaDID = IotaDID::new(subject_key.public().as_ref()).unwrap();

    let subject: Subject =
        Subject::from_json_value(json!({"id": Url::parse(subject_did.as_str()).unwrap(), "attributes": request.credential_preview.attributes}))
            .unwrap();

    let mut credential: Credential = CredentialBuilder::default()
        .id(Url::parse("https://example.edu/credentials/3732").unwrap())
        .issuer(Url::parse(&did_to).unwrap())
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

    let attachment = serde_json::to_value(&credential).unwrap();
    let mut issue = IssueCredentialResponseBuilder::new()
        .goal_code("issue-vc".to_string())
        .comment(request.comment)
        .credential_preview(request.credential_preview)
        .attachment(attachment)
        .build_issue_credential()?;
    issue = add_return_route_all_header(issue);
    let request = sign_and_encrypt(&issue, &did_from, &did_to, &keypair).await?;
    Ok((issue, request))
}

/// # Send holder a credential
#[openapi(tag = "issue-credential v2.1")]
#[post("/issue-credential-2.1/send", data = "<request>")]
pub async fn post_send_2(
    wallet: &State<Arc<Mutex<Wallet>>>,
    connections: &State<Connections>,
    request: Json<SendRequest>,
) -> Result<Json<Value>, Status> {
    let (did_to, endpoint) = {
        let connections = connections.connections.lock().await;
        let connection = connections.get(&request.connection_id).unwrap().clone();
        (connection.did.to_string(), connection.endpoint)
    };
    let request = request.into_inner();

    let (issue, request) = {
        let wallet = wallet.try_lock().unwrap();
        prepare_issue_credential_request(&wallet, did_to, request)
            .await
            .unwrap()
    };

    let client = reqwest::Client::new();
    let res = client
        .post(endpoint.to_string())
        .json(&request)
        .send()
        .await
        .unwrap();
    match res.json::<Value>().await.is_ok() {
        true => Ok(Json(json!(issue))),
        false => Err(Status::InternalServerError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_rocket;
    use crate::Config;
    use crate::Wallet;

    #[tokio::test]
    async fn test_prepare_issue_credential_request() {
        let rocket = test_rocket().await;
        let figment = rocket.figment();
        let config: Config = figment.extract().expect("config");
        let wallet = Wallet::new_from_config(&config).await.unwrap();
        let request = SendRequest {
            connection_id: "".to_string(),
            comment: "".to_string(),
            credential_preview: example_credential_preview(),
        };
        let did_to = wallet.did_iota().unwrap();
        let (message, _value) =
            prepare_issue_credential_request(&wallet, did_to.to_string(), request)
                .await
                .unwrap();
        assert!(message.get_attachments().next().is_some());
    }

    #[tokio::test]
    async fn test_prepare_offer_request() {
        let rocket = test_rocket().await;
        let figment = rocket.figment();
        let config: Config = figment.extract().expect("config");
        let wallet = Wallet::new_from_config(&config).await.unwrap();
        let request = CreateOfferRequest {
            connection_id: "".to_string(),
            comment: "".to_string(),
            credential_preview: example_credential_preview(),
        };
        let did_to = wallet.did_iota().unwrap();
        let (message, _value) = prepare_offer_request(&wallet, did_to.to_string(), request)
            .await
            .unwrap();
        assert!(message
            .get_application_params()
            .filter(|(key, _)| *key == "credential_preview")
            .next()
            .is_some());
    }

    #[tokio::test]
    async fn test_prepare_proposal_request() {
        let rocket = test_rocket().await;
        let figment = rocket.figment();
        let config: Config = figment.extract().expect("config");
        let wallet = Wallet::new_from_config(&config).await.unwrap();
        let request = CreateProposalRequest {
            connection_id: "".to_string(),
            comment: "".to_string(),
            credential_preview: example_credential_preview(),
        };
        let did_to = wallet.did_iota().unwrap();
        let (message, _value) = prepare_proposal_request(&wallet, did_to.to_string(), request)
            .await
            .unwrap();
        assert!(message
            .get_application_params()
            .filter(|(key, _)| *key == "credential_preview")
            .next()
            .is_some());
    }
}
