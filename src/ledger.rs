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

#[cfg(test)]
mod tests {
    use crate::test_rocket;
    use crate::Config;
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use rocket::State;

    #[test]
    fn test_get_endpoint() {
        let rocket = test_rocket();
        let config: &State<Config> = State::get(&rocket).expect("managed `ConfigState`");
        let did = config.did.to_string();
        let client = Client::tracked(rocket).expect("valid rocket instance");
        let response = client
            .get(format!("/ledger/did-endpoint?did={}", did))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
    }
}
