use rocket::http::Status;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::Value;
pub mod webhook;

#[openapi(tag = "topic")]
#[post("/topic/<name>")]
pub async fn post_topic(name: String) -> Status {
    println!("name: {}", name);
    Status::Ok
}

#[openapi(tag = "topic")]
#[post("/topic/message", data = "<payload>")]
pub async fn post_message_topic(payload: Json<Value>) -> Status {
    println!("payload: {}", payload.into_inner());
    Status::Ok
}
