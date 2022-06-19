use base58::ToBase58;
use identity_iota::client::ResolvedIotaDocument;
use identity_iota::client::Resolver;
use identity_iota::iota_core::IotaDID;
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

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct VerkeyResponse {
    pub verkey: String,
}

/// # Get the verkey for a DID from the ledger.
///
/// Returns verkey in base58.
#[openapi(tag = "ledger")]
#[get("/ledger/did-verkey?<did>")]
pub async fn get_did_verkey(did: String) -> Json<VerkeyResponse> {
    let did = IotaDID::from_str(&did).unwrap();

    let resolver: Resolver = Resolver::new().await.unwrap();
    let resolved_did_document: ResolvedIotaDocument = resolver.resolve(&did).await.unwrap();

    let document = resolved_did_document.document;
    let verkey = document.default_signing_method().unwrap();
    let verkey = verkey.data().try_decode().unwrap().to_base58();
    Json(VerkeyResponse { verkey })
}

/// # Get the endpoint for a DID from the ledger.
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
    use rocket::local::asynchronous::Client;
    use rocket::State;

    #[tokio::test]
    async fn test_get_endpoint() {
        let rocket = test_rocket().await;
        let config: &State<Config> = State::get(&rocket).expect("managed `ConfigState`");
        let did = config.did_iota.as_ref().unwrap().to_string();
        let client = Client::tracked(rocket)
            .await
            .expect("valid rocket instance");
        let response = client
            .get(format!("/ledger/did-endpoint?did={}", did))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
    }

    #[tokio::test]
    async fn test_get_verkey() {
        let rocket = test_rocket().await;
        let config: &State<Config> = State::get(&rocket).expect("managed `ConfigState`");
        let did = config.did_iota.as_ref().unwrap().to_string();
        let client = Client::tracked(rocket)
            .await
            .expect("valid rocket instance");
        let response = client
            .get(format!("/ledger/did-verkey?did={}", did))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
    }
}
