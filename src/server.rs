use rocket::{get, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct LiveResponse {
    pub alive: bool,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ReadyResponse {
    pub ready: bool,
}

#[openapi(tag = "server")]
#[get("/server/live")]
pub fn get_live() -> Json<LiveResponse> {
    Json(LiveResponse { alive: true })
}

#[openapi(tag = "server")]
#[get("/server/ready")]
pub fn get_ready() -> Json<ReadyResponse> {
    Json(ReadyResponse { ready: true })
}
