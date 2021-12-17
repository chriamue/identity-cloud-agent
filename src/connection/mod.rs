use crate::wallet::get_did_endpoint;
use crate::wallet::Wallet;
use identity::iota::IotaDID;
use reqwest;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;

pub mod invitation;
use invitation::{build_issue_vc_invitation, Invitation};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default, Debug, PartialEq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Connection {
    pub id: String,
    pub endpoint: String,
}

#[derive(Default)]
pub struct Connections {
    pub connections: Arc<Mutex<HashMap<String, Connection>>>,
}

#[openapi(tag = "out-of-band")]
#[post("/out-of-band/create-invitation")]
pub async fn post_create_invitation(wallet: &State<Wallet>) -> Json<Invitation> {
    let lock = wallet.identity.lock().await;
    let did: &IotaDID = lock.try_did().unwrap();
    let endpoint = get_did_endpoint(did.to_string()).to_string();
    let invitation: Invitation = build_issue_vc_invitation(endpoint);
    Json(invitation)
}

#[openapi(tag = "out-of-band")]
#[post(
    "/out-of-band/receive-invitation",
    format = "application/json",
    data = "<invitation>"
)]
pub async fn post_receive_invitation(
    connections: &State<Connections>,
    invitation: Json<Invitation>,
) -> Json<Invitation> {
    let invitation: Invitation = invitation.into_inner();
    let id = invitation.id.to_string();
    let endpoint: String = invitation.attachments[0].data["json"]["service"]["serviceEndpoint"]
        .as_str()
        .unwrap()
        .to_string();

    let client = reqwest::Client::new();
    client
        .post(endpoint.to_string())
        .json(&invitation)
        .send()
        .await
        .unwrap();
    let connection = Connection { id, endpoint };
    let mut lock = connections.connections.lock().await;
    lock.insert(connection.id.to_string(), connection);

    Json(invitation)
}

#[openapi(tag = "connection")]
#[get("/connections")]
pub async fn get_all_connections(connections: &State<Connections>) -> Json<Vec<Connection>> {
    let lock = connections.connections.lock().await;
    let connections = lock.values().cloned().collect();
    Json(connections)
}
