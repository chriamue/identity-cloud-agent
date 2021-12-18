use identity::did::resolution;
use identity::did::resolution::InputMetadata;
use identity::iota::ClientMap;
use rocket::{get, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use std::thread;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct EndpointResponse {
    pub endpoint: String,
}

#[openapi(tag = "ledger")]
#[get("/ledger/did-endpoint?<did>")]
pub fn get_did_endpoint(did: String) -> Json<EndpointResponse> {
    let client: ClientMap = ClientMap::new();
    let input: InputMetadata = Default::default();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let output = thread::spawn(move || {
        let out = runtime.block_on(resolution::resolve(did.as_str(), input, &client));
        out
    })
    .join()
    .expect("Thread panicked")
    .unwrap();

    let document = output.document.unwrap();
    let services = document.service();
    let service = services.get(0).unwrap();
    Json(EndpointResponse {
        endpoint: service.service_endpoint().to_string(),
    })
}
