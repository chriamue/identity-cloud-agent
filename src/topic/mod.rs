use rocket::http::Status;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::Value;

#[openapi(tag = "topic")]
#[post("/topic/<name>")]
pub async fn post_topic(name: String) -> Status {
    info!("name: {}", name);
    Status::Ok
}

#[openapi(tag = "topic")]
#[post("/topic/message", data = "<payload>")]
pub async fn post_message_topic(payload: Json<Value>) -> Status {
    debug!("payload: {}", payload.into_inner());
    Status::Ok
}

pub enum Topic {
    ALL,
}
