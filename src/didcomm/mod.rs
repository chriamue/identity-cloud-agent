use crate::connection::{invitation::Invitation, Connections, Termination, TerminationResponse};
use crate::credential::{issue::Issuance, Credentials};
use crate::message::MessageRequest;
use crate::ping::{PingRequest, PingResponse};
use crate::webhook::Webhook;
use async_trait::async_trait;
use reqwest::RequestBuilder;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::{json, Value};
use uuid::Uuid;
pub mod client;
#[cfg(test)]
pub mod test_client;

pub use client::Client;

#[async_trait]
pub trait DidComm: Send + Sync {
    fn request(&self, endpoint: &str, body: &Value) -> RequestBuilder;
    async fn post(&self, endpoint: &str, body: &Value)
        -> Result<reqwest::Response, reqwest::Error>;
}

#[openapi(tag = "didcomm")]
#[post("/", format = "application/json", data = "<data>")]
pub async fn post_endpoint(
    webhook: &State<Box<dyn Webhook>>,
    connections: &State<Connections>,
    credentials: &State<Credentials>,
    data: Json<Value>,
) -> Json<Value> {
    match data["type"].as_str().unwrap() {
        "https://didcomm.org/out-of-band/2.0/invitation" => {
            let invitation: Invitation = serde_json::from_value(data.into_inner()).unwrap();
            info!("invitation = {:?}", invitation.id);
            Json(json!({}))
        }
        "https://didcomm.org/trust-ping/2.0/ping" => {
            let ping_request: PingRequest = serde_json::from_value(data.into_inner()).unwrap();
            let ping_response: PingResponse = PingResponse {
                type_: "https://didcomm.org/trust-ping/2.0/ping-response".to_string(),
                id: Uuid::new_v4().to_string(),
                thid: ping_request.id,
            };
            Json(json!(ping_response))
        }
        "iota/post/0.1/post" => {
            let message_request: MessageRequest =
                serde_json::from_value(data.into_inner()).unwrap();
            info!("message: {:?}", message_request.payload);
            webhook
                .post("message", &message_request.payload)
                .await
                .unwrap();
            Json(json!({}))
        }
        "iota/termination/0.1/termination" => {
            let termination: Termination = serde_json::from_value(data.into_inner()).unwrap();
            let mut lock = connections.connections.lock().await;
            lock.remove(&termination.id).unwrap();
            std::mem::drop(lock);
            let termination: TerminationResponse = TerminationResponse {
                typ: "application/didcomm-plain+json".to_string(),
                type_: "iota/termination/0.1/termination-response".to_string(),
                id: termination.id,
                body: Value::default(),
            };
            Json(json!(termination))
        }
        "iota/issuance/0.1/issuance" => {
            let issuance: Issuance = serde_json::from_value(data.into_inner()).unwrap();
            let credential = issuance.signed_credential;
            info!("issuance: {:?}", credential);
            let mut lock = credentials.credentials.lock().await;

            lock.insert(credential.id.clone().unwrap().to_string(), credential);
            Json(json!({}))
        }
        _ => Json(json!({})),
    }
}

#[cfg(test)]
mod tests {
    use crate::connection::Connection;
    use crate::ping::{PingRequest, PingResponse};
    use crate::test_rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;
    use serde_json::{from_value, json, Value};

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
        let _connections: Vec<Connection> = from_value(response).unwrap();

        let body: Value = json!( {
            "response_requested": true
        });

        let ping_request: PingRequest = PingRequest {
            type_: "https://didcomm.org/trust-ping/2.0/ping".to_string(),
            id: "foo".to_string(),
            from: "bar".to_string(),
            body,
        };
        let ping_request: String = serde_json::to_string(&ping_request).unwrap();

        let response = client
            .post("/")
            .header(ContentType::JSON)
            .body(ping_request)
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<PingResponse>().unwrap();
        assert_eq!(response.thid, "foo".to_string());
    }
}
