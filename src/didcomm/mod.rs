use crate::connection::invitation::Invitation;
use crate::message::MessageRequest;
use crate::ping::{PingRequest, PingResponse};
use crate::topic::webhook::Webhook;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::{json, Value};
use uuid::Uuid;

#[openapi(tag = "didcomm")]
#[post("/", format = "application/json", data = "<data>")]
pub async fn post_endpoint(webhook: &State<Webhook>, data: Json<Value>) -> Json<Value> {
    match data["type"].as_str().unwrap() {
        "https://didcomm.org/out-of-band/2.0/invitation" => {
            let invitation: Invitation = serde_json::from_value(data.into_inner()).unwrap();
            println!("invitation = {:?}", invitation.id);
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
            println!("message: {:?}", message_request.payload);
            webhook.send("/message", message_request.payload).await;
            Json(json!({}))
        }
        _ => Json(json!({})),
    }
}
