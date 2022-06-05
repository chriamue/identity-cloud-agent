use crate::connection::Connections;
use crate::wallet::Wallet;
use identity::iota_core::IotaDID;
use rocket::http::Status;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MessageRequest {
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    pub from: String,
    pub payload: Value,
}

#[openapi(tag = "basicmessage")]
#[post("/connections/<conn_id>/send-message", data = "<payload>")]
pub async fn post_send_message(
    wallet: &State<Wallet>,
    connections: &State<Connections>,
    conn_id: String,
    payload: Json<Value>,
) -> Status {
    let lock = wallet.account.lock().await;
    let did: &IotaDID = lock.did();

    let lock = connections.connections.lock().await;
    let connection = lock.get(&conn_id).unwrap().clone();

    let payload = payload.into_inner();

    let message_request: MessageRequest = MessageRequest {
        type_: "iota/post/0.1/post".to_string(),
        id: Uuid::new_v4().to_string(),
        from: did.to_string(),
        payload,
    };

    let client = reqwest::Client::new();
    let _res = client
        .post(connection.endpoint.to_string())
        .json(&message_request)
        .send()
        .await
        .unwrap();
    Status::Ok
}
