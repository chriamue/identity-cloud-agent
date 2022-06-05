use identity::iota::ResolvedIotaDocument;
use identity::iota::Resolver;
use identity::iota_core::IotaDID;
use rocket::{get, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct EndpointResponse {
    pub endpoint: String,
}

#[openapi(tag = "ledger")]
#[get("/ledger/did-endpoint?<did>")]
pub async fn get_did_endpoint(did: String) -> Json<EndpointResponse> {
    let did = IotaDID::from_str(&did).unwrap();

    let resolver: Resolver = Resolver::new().await.unwrap();
    let resolved_did_document: ResolvedIotaDocument = resolver.resolve(&did).await.unwrap();

    let document = resolved_did_document.document;
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
