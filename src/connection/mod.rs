use crate::wallet::get_did_endpoint;
use crate::wallet::Wallet;
use identity::iota::IotaDID;
use reqwest;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;

pub mod invitation;
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

#[openapi(tag = "out-of-band")]
#[post(
    "/out-of-band/receive-invitation",
    format = "application/json",
    data = "<invitation>"
)]
pub async fn post_receive_invitation(invitation: Json<Invitation>) -> Json<Invitation> {
    let invitation: Invitation = invitation.into_inner();
    let endpoint: String = invitation.attachments[0].data["json"]["service"]["serviceEndpoint"]
        .as_str()
        .unwrap()
        .to_string();

    let client = reqwest::Client::new();
    client
        .post(endpoint)
        .json(&invitation)
        .send()
        .await
        .unwrap();
    Json(invitation)
}
