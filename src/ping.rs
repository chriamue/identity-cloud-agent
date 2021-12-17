use crate::connection::Connections;
use crate::wallet::Wallet;
use identity::iota::IotaDID;
use reqwest;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct PingRequest {
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    pub from: String,
    pub body: Value,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct PingResponse {
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    pub thid: String,
}

#[openapi(tag = "trustping")]
#[post("/connections/<conn_id>/send-ping")]
pub async fn post_send_ping(
    wallet: &State<Wallet>,
    connections: &State<Connections>,
    conn_id: String,
) -> Json<PingResponse> {
    let lock = wallet.identity.lock().await;
    let did: &IotaDID = lock.try_did().unwrap();

    let lock = connections.connections.lock().await;
    let connection = lock.get(&conn_id).unwrap().clone();

    let body: Value = json!( {
        "response_requested": true
    });

    let ping_request: PingRequest = PingRequest {
        type_: "https://didcomm.org/trust-ping/2.0/ping".to_string(),
        id: Uuid::new_v4().to_string(),
        from: did.to_string(),
        body,
    };

    let client = reqwest::Client::new();
    let res = client
        .post(connection.endpoint.to_string())
        .json(&ping_request)
        .send()
        .await
        .unwrap();
    let json = res.json();
    let ping_response: PingResponse = json.await.unwrap();
    Json(ping_response)
}
