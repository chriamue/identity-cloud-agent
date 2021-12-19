use crate::connection::Connections;
use crate::wallet::Wallet;
use identity::iota::IotaDID;

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
    let lock = wallet.account.lock().await;
    let did: &IotaDID = lock.did();

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

#[cfg(test)]
mod tests {
    use crate::connection::Connection;
    use crate::rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;
    use serde_json::{from_value, Value};

    #[test]
    fn test_send_ping() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");
        let response = client.get("/connections").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 0);

        let response = client.post("/out-of-band/create-invitation").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let invitation: Value = response.into_json::<Value>().unwrap();
        let invitation: String = serde_json::to_string(&invitation).unwrap();

        let response = client
            .post("/out-of-band/receive-invitation")
            .header(ContentType::JSON)
            .body(invitation)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/connections").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().unwrap();
        let connections: Vec<Connection> = from_value(response).unwrap();

        let connection_id = connections[0].id.to_string();

        let response = client
            .post(format!("/connections/{}/send-ping", connection_id))
            .dispatch();
        assert_eq!(response.status(), Status::InternalServerError);
    }
}
