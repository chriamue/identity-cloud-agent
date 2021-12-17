use crate::wallet::get_did_endpoint;
use crate::wallet::Wallet;
use identity::iota::IotaDID;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;

mod invitation;
use invitation::{build_issue_vc_invitation, Invitation};

#[openapi(tag = "out-of-band")]
#[post("/out-of-band/create-invitation")]
pub async fn post_create_invitation(wallet: &State<Wallet>) -> Json<Invitation> {
    let lock = wallet.identity.lock().await;
    let did: &IotaDID = lock.try_did().unwrap();
    let endpoint = get_did_endpoint(did.to_string()).to_string();
    let invitation: Invitation = build_issue_vc_invitation(endpoint);
    Json(invitation)
}
