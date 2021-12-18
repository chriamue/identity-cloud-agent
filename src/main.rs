#[macro_use]
extern crate rocket;
use identity::iota::IotaDID;
use rocket::get;
use rocket::response::Redirect;
use rocket_okapi::{openapi, openapi_get_routes, swagger_ui::*};

mod config;
mod connection;
mod credential;
mod didcomm;
mod ledger;
mod message;
mod ping;
mod resolver;
mod server;
mod tests;
mod topic;
mod wallet;

use config::Config;
use connection::Connections;
use topic::webhook::Webhook;
use wallet::Wallet;

#[openapi(skip)]
#[get("/")]
fn index() -> Redirect {
    Redirect::to("/swagger-ui")
}

async fn print_wallet(wallet: &Wallet) {
    let lock = wallet.identity.lock().await;
    let iota_did: &IotaDID = lock.try_did().unwrap();
    println!(
        "Local Document from {} = {:#?}",
        iota_did,
        lock.to_document()
    );
    let network = iota_did.network().unwrap();
    let explorer = network.explorer_url().unwrap();
    println!("Explore the DID Document = {}", explorer.to_string());
}

#[launch]
pub fn rocket() -> _ {
    let rocket = rocket::build();
    let figment = rocket.figment();
    let config: Config = figment.extract().expect("config");

    let connections: Connections = Connections::default();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let wallet = runtime.block_on(Wallet::load(
        config.stronghold_path.to_string().into(),
        config.password.to_string(),
        config.endpoint.to_string(),
    ));

    runtime.block_on(print_wallet(&wallet));

    let webhook = Webhook::new(config.webhook_url.to_string());

    rocket
        .mount(
            "/",
            openapi_get_routes![
                index,
                credential::issue::post_send_offer,
                connection::post_create_invitation,
                connection::post_receive_invitation,
                connection::get_all_connections,
                connection::get_connection,
                connection::delete_connection,
                connection::get_connection_endpoints,
                didcomm::post_endpoint,
                ledger::get_did_endpoint,
                message::post_send_message,
                ping::post_send_ping,
                resolver::get_resolve,
                server::get_live,
                server::get_ready,
                topic::post_topic,
                topic::post_message_topic,
                wallet::get_all_dids,
                wallet::get_public_did,
                wallet::get_did_endpoint,
                wallet::post_did_endpoint
            ],
        )
        .mount(
            "/swagger-ui/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .manage(config)
        .manage(wallet)
        .manage(connections)
        .manage(webhook)
}
