use crate::connection::Connections;
use crate::wallet::Wallet;
use didcomm_mediator::message::sign_and_encrypt;
use didcomm_mediator::protocols::basicmessage::BasicMessageBuilder;
use rocket::http::Status;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use {futures::SinkExt, pharos::*};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MessageEvent {
    Received(String, String),
}

pub struct MessageEvents {
    pharos: Pharos<MessageEvent>,
}

impl Default for MessageEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageEvents {
    pub fn new() -> Self {
        Self {
            pharos: Pharos::default(),
        }
    }
    pub async fn send(&mut self, event: MessageEvent) {
        self.pharos.send(event).await.expect("notify observers");
    }
}

impl Observable<MessageEvent> for MessageEvents {
    type Error = PharErr;

    fn observe(
        &mut self,
        options: ObserveConfig<MessageEvent>,
    ) -> Observe<'_, MessageEvent, Self::Error> {
        self.pharos.observe(options)
    }
}

/// # Send a basic message to a connection
#[openapi(tag = "basicmessage")]
#[post("/connections/<conn_id>/send-message", data = "<payload>")]
pub async fn post_send_message(
    wallet: &State<Arc<Mutex<Wallet>>>,
    connections: &State<Connections>,
    conn_id: String,
    payload: Json<Value>,
) -> Status {
    let wallet = wallet.try_lock().unwrap();
    let (did_to, endpoint) = {
        let connections = connections.connections.lock().await;
        let connection = connections.get(&conn_id).unwrap().clone();
        (connection.did.to_string(), connection.endpoint)
    };

    let payload = serde_json::to_string(&payload.into_inner()).unwrap();
    let message = BasicMessageBuilder::new().message(payload).build().unwrap();

    let did_from = wallet.did_iota().unwrap();
    let keypair = wallet.keypair();
    drop(wallet);
    let message_request = sign_and_encrypt(&message, &did_from, &did_to, &keypair)
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let _res = client
        .post(endpoint.to_string())
        .json(&message_request)
        .send()
        .await
        .unwrap();
    Status::Ok
}

#[cfg(test)]
mod tests {
    use crate::connection::Connection;
    use crate::test_rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;
    use serde_json::{from_value, Value};

    #[ignore = "SIGABRT"]
    #[test]
    fn test_send_basicmessage() {
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
            .post(format!("/connections/{}/send-message", connection_id))
            .dispatch();
        assert_ne!(response.status(), Status::InternalServerError);
    }
}
