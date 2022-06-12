pub use didcomm_mediator::wallet::Wallet;
use identity::account::Result;
use identity::iota::ResolvedIotaDocument;
use identity::iota::Resolver;
use identity::iota_core::IotaDID;
use rocket::response::status::NotFound;
use rocket::State;
use rocket::{get, post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use std::str;
use std::str::FromStr;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Did {
    id: String,
    key_type: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DidEndpoint {
    did: String,
    endpoint: String,
}

#[openapi(tag = "wallet")]
#[get("/wallet/did")]
pub async fn get_all_dids(wallet: &State<Wallet>) -> Json<Vec<Did>> {
    let did: IotaDID = IotaDID::from_str(&wallet.did_iota().unwrap()).unwrap();
    let key_type = "Ed25519VerificationKey2018".to_string();
    Json(vec![Did {
        id: did.to_string(),
        key_type,
    }])
}

#[openapi(tag = "wallet")]
#[get("/wallet/did/public")]
pub async fn get_public_did(wallet: &State<Wallet>) -> Json<Did> {
    let did: IotaDID = IotaDID::from_str(&wallet.did_iota().unwrap()).unwrap();
    let key_type = "Ed25519VerificationKey2018".to_string();
    Json(Did {
        id: did.to_string(),
        key_type,
    })
}

#[openapi(tag = "wallet")]
#[get("/wallet/get-did-endpoint?<did>")]
pub async fn get_did_endpoint(did: String) -> Json<String> {
    let did = IotaDID::from_str(&did).unwrap();
    let resolver: Resolver = Resolver::new().await.unwrap();
    let resolved_did_document: ResolvedIotaDocument = resolver.resolve(&did).await.unwrap();

    let document = resolved_did_document.document;
    let services = document.service();
    let service = services.get(0).unwrap();
    let endpoint = service.service_endpoint().to_string();
    let endpoint = endpoint.replace('\"', "");
    Json(endpoint)
}

#[openapi(tag = "wallet")]
#[post("/wallet/set-did-endpoint", data = "<_post_data>")]
pub async fn post_did_endpoint(
    _wallet: &State<Wallet>,
    _post_data: Json<DidEndpoint>,
) -> Result<(), NotFound<String>> {
    /*wallet.account.as_ref().unwrap()
        .update_identity()
        .create_service()
        .fragment("endpoint")
        .type_("Endpoint")
        .endpoint(Url::parse(&post_data.endpoint).unwrap())
        .apply()
        .await
        .unwrap();
    */
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::test_rocket;
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use serde_json::Value;

    #[test]
    fn test_public_did() {
        let client = Client::tracked(test_rocket()).expect("valid rocket instance");
        let response = client.get("/wallet/did/public").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().unwrap();
        assert!(response.get("id").is_some());
    }
}
