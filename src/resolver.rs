use crate::wallet::Wallet;
use identity::iota::ExplorerUrl;
use identity::iota::IotaDID;
use rocket::State;
use rocket::{get, serde::json::Json};
use rocket_okapi::openapi;

#[openapi(tag = "resolver")]
#[get("/resolver/resolve/<did>")]
pub fn get_resolve(did: String) -> Json<String> {
    let iota_did: IotaDID = IotaDID::try_from(did).unwrap();
    let explorer: &ExplorerUrl = ExplorerUrl::mainnet();
    Json(explorer.resolver_url(&iota_did).unwrap().to_string())
}
