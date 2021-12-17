use crate::connection::invitation::Invitation;
use rocket::http::Status;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;

#[openapi(tag = "didcomm")]
#[post("/", format = "application/json", data = "<invitation>")]
pub async fn post_endpoint(invitation: Json<Invitation>) -> Status {
    let invitation: Invitation = invitation.into_inner();

    println!("invitation = {:?}", invitation.id);
    Status::Ok
}
