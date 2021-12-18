use crate::config::Config;
use crate::wallet::get_did_endpoint;
use crate::wallet::Wallet;
use identity::iota::IotaDID;
use rocket::http::Status;
use rocket::State;
use rocket::{delete, get, post, serde::json::Json};
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

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ConnectionEndpoints {
    pub my_endpoint: String,
    pub their_endpoint: String,
}

#[openapi(tag = "out-of-band")]
#[post("/out-of-band/create-invitation")]
pub async fn post_create_invitation(wallet: &State<Wallet>) -> Json<Invitation> {
    let lock = wallet.account.lock().await;
    let did: &IotaDID = lock.did();
    let endpoint = get_did_endpoint(did.to_string()).as_str().to_string();
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

#[openapi(tag = "connection")]
#[get("/connections/<conn_id>")]
pub async fn get_connection(connections: &State<Connections>, conn_id: String) -> Json<Connection> {
    let lock = connections.connections.lock().await;
    let connection = lock.get(&conn_id).unwrap().clone();
    Json(connection)
}

#[openapi(tag = "connection")]
#[get("/connections/<conn_id>/endpoints")]
pub async fn get_connection_endpoints(
    config: &State<Config>,
    connections: &State<Connections>,
    conn_id: String,
) -> Json<ConnectionEndpoints> {
    let lock = connections.connections.lock().await;
    let endpoint = config.endpoint.to_string();
    let connection = lock.get(&conn_id).unwrap().clone();
    let their_endpoint = connection.endpoint;
    Json(ConnectionEndpoints {
        my_endpoint: endpoint,
        their_endpoint,
    })
}

#[openapi(tag = "connection")]
#[delete("/connections/<conn_id>")]
pub async fn delete_connection(connections: &State<Connections>, conn_id: String) -> Status {
    let mut lock = connections.connections.lock().await;
    lock.remove(&conn_id).unwrap();
    Status::Ok
}
