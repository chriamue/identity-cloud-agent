use crate::connection::Connections;
use crate::Wallet;
use did_key::KeyMaterial;
use didcomm_mediator::message::{add_return_route_all_header, receive, sign_and_encrypt};
use didcomm_mediator::protocols::trustping::TrustPingResponseBuilder;
use rocket::http::Status;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
) -> Result<Json<Value>, Status> {
    let lock = connections.connections.lock().await;
    let connection = lock.get(&conn_id).unwrap().clone();

    let mut message = TrustPingResponseBuilder::new().build_ping().unwrap();
    message = add_return_route_all_header(message);
    let did_from = wallet.did_iota().unwrap();
    let did_to = connection.did;
    let keypair = wallet.keypair();
    let ping = sign_and_encrypt(&message, &did_from, &did_to, &keypair)
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let res = client
        .post(connection.endpoint.to_string())
        .json(&ping)
        .send()
        .await
        .unwrap();
    let json: Value = res.json().await.unwrap();
    let body_str = serde_json::to_string(&json).unwrap();

    let received = match receive(
        &body_str,
        Some(&wallet.keypair().private_key_bytes()),
        None,
        None,
    )
    .await
    {
        Ok(received) => received,
        Err(_) => return Err(Status::BadRequest),
    };
    let received: Value = serde_json::to_value(&received).unwrap();
    Ok(Json(received))
}

#[cfg(test)]
mod tests {
    use crate::connection::Connection;
    use crate::test_rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;
    use serde_json::{from_value, Value};

    #[test]
    fn test_send_ping() {
        let client = Client::tracked(test_rocket()).expect("valid rocket instance");
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
