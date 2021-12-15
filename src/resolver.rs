use identity::iota::IotaDID;
use rocket::{get, serde::json::Json};
use rocket_okapi::openapi;

#[openapi(tag = "resolver")]
#[get("/resolver/resolve/<did>")]
pub fn get_resolve(did: String) -> Json<String> {
    let iota_did: IotaDID = IotaDID::try_from(did).unwrap();
    let network = iota_did.network().unwrap();
    let explorer = network.explorer_url().unwrap();
    Json(explorer.to_string())
}
