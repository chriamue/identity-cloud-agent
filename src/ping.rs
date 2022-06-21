use crate::connection::Connections;
use crate::Wallet;
use did_key::KeyMaterial;
use didcomm_mediator::message::{add_return_route_all_header, receive, sign_and_encrypt};
use didcomm_protocols::TrustPingResponseBuilder;
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
pub enum PingEvent {
    Received(String),
}

pub struct PingEvents {
    pharos: Pharos<PingEvent>,
}

impl Default for PingEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl PingEvents {
    pub fn new() -> Self {
        Self {
            pharos: Pharos::default(),
        }
    }
    pub async fn send(&mut self, event: PingEvent) {
        self.pharos.send(event).await.expect("notify observers");
    }
}

impl Observable<PingEvent> for PingEvents {
    type Error = PharErr;

    fn observe(
        &mut self,
        options: ObserveConfig<PingEvent>,
    ) -> Observe<'_, PingEvent, Self::Error> {
        self.pharos.observe(options)
    }
}

/// # Send a trustping to a connection
#[openapi(tag = "trustping")]
#[post("/connections/<conn_id>/send-ping")]
pub async fn post_send_ping(
    wallet: &State<Arc<Mutex<Wallet>>>,
    connections: &State<Connections>,
    conn_id: String,
) -> Result<Json<Value>, Status> {
    let wallet = wallet.try_lock().unwrap();

    let (did_to, endpoint) = {
        let connections = connections.connections.lock().await;
        let connection = connections.get(&conn_id).unwrap().clone();
        (connection.did.to_string(), connection.endpoint)
    };
    let mut message = TrustPingResponseBuilder::new().build_ping().unwrap();
    message = add_return_route_all_header(message);
    let did_from = wallet.did_iota().unwrap();
    let keypair = wallet.keypair();
    drop(wallet);
    let ping = sign_and_encrypt(&message, &did_from, &did_to, &keypair)
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let res = client
        .post(endpoint.to_string())
        .json(&ping)
        .send()
        .await
        .unwrap();
    let json: Value = res.json().await.unwrap();
    let body_str = serde_json::to_string(&json).unwrap();

    let received = match receive(&body_str, Some(&keypair.private_key_bytes()), None, None).await {
        Ok(received) => received,
        Err(_) => return Err(Status::BadRequest),
    };
    let received: Value = serde_json::to_value(&received).unwrap();
    Ok(Json(received))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::test_rocket;
    use futures::StreamExt;
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::{from_value, Value};

    #[tokio::test]
    async fn test_send_ping() {
        let client = Client::tracked(test_rocket().await)
            .await
            .expect("valid rocket instance");
        let response = client.get("/connections").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 0);

        let response = client
            .post("/out-of-band/create-invitation")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let invitation: Value = response.into_json::<Value>().await.unwrap();
        let invitation: String = serde_json::to_string(&invitation).unwrap();

        let response = client
            .post("/out-of-band/receive-invitation")
            .header(ContentType::JSON)
            .body(invitation)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/connections").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        let connections: Vec<Connection> = from_value(response).unwrap();

        let connection_id = connections.first().unwrap().id.to_string();

        let response = client
            .post(format!("/connections/{}/send-ping", connection_id))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::InternalServerError);
    }

    #[tokio::test]
    async fn test_ping_events() {
        let mut ping_events = PingEvents::new();
        let mut events = ping_events
            .observe(Channel::Bounded(3).into())
            .await
            .expect("observe");
        ping_events
            .send(PingEvent::Received(String::default()))
            .await;
        let evt = dbg!(events.next().await.unwrap());
        drop(ping_events);
        assert_eq!(PingEvent::Received(String::default()), evt);
        assert_eq!(None, events.next().await);
    }
}
