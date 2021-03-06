use identity_iota::client::ExplorerUrl;
use identity_iota::iota_core::IotaDID;
use rocket::{get, serde::json::Json};
use rocket_okapi::openapi;

/// # did resolver interface
#[openapi(tag = "resolver")]
#[get("/resolver/resolve/<did>")]
pub fn get_resolve(did: String) -> Json<String> {
    let iota_did: IotaDID = IotaDID::try_from(did).unwrap();
    let explorer: &ExplorerUrl = ExplorerUrl::mainnet();
    Json(explorer.resolver_url(&iota_did).unwrap().to_string())
}
