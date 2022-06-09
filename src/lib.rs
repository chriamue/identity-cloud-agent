#[macro_use]
extern crate rocket;
use rocket::{Build, Rocket};

use identity::iota::ExplorerUrl;
use identity::iota_core::IotaDID;
use rocket::get;
use rocket::response::Redirect;
use rocket_okapi::{openapi, openapi_get_routes, swagger_ui::*};
use std::thread;

pub mod config;
pub mod connection;
pub mod credential;
pub mod didcomm;
pub mod keyutils;
pub mod ledger;
pub mod message;
pub mod ping;
pub mod presentation;
pub mod resolver;
pub mod schema;
pub mod server;
mod tests;
pub mod topic;
pub mod wallet;
pub mod webhook;

pub use config::Config;
use connection::Connections;
use credential::Credentials;
pub use didcomm::DidComm;
use schema::Schemas;
use wallet::Wallet;
pub use webhook::Webhook;

#[openapi(skip)]
#[get("/")]
fn index() -> Redirect {
    Redirect::to("/swagger-ui")
}

async fn log_wallet(wallet: &Wallet) {
    let lock = wallet.account.lock().await;
    let iota_did: &IotaDID = lock.did();
    info!("Local Document from {} = {:#?}", iota_did, lock.document());
    let explorer: &ExplorerUrl = ExplorerUrl::mainnet();
    println!(
        "Explore the DID Document = {}",
        explorer.resolver_url(iota_did).unwrap()
    );
}

pub fn rocket(
    rocket: Rocket<Build>,
    config: Config,
    webhook: Box<dyn webhook::Webhook>,
    didcomm: Box<dyn didcomm::DidComm>,
) -> Rocket<Build> {
    let connections: Connections = Connections::default();
    let credentials: Credentials = Credentials::default();
    let schemas: Schemas = Schemas::default();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let stronghold_path = config.stronghold_path.to_string();
    let password = config.password.to_string();
    let endpoint = config.endpoint.to_string();
    let did = config.did.to_string();

    let wallet = thread::spawn(move || {
        let wallet = runtime.block_on(Wallet::load(
            stronghold_path.into(),
            password.to_string(),
            endpoint.to_string(),
            did.to_string(),
        ));
        runtime.block_on(log_wallet(&wallet));
        wallet
    })
    .join()
    .expect("Thread panicked");

    rocket
        .mount(
            "/",
            openapi_get_routes![
                index,
                connection::post_create_invitation,
                connection::post_receive_invitation,
                connection::get_all_connections,
                connection::get_connection,
                connection::delete_connection,
                connection::get_connection_endpoints,
                credential::issue::post_send_offer,
                credential::get_all_credentials,
                didcomm::post_endpoint,
                ledger::get_did_endpoint,
                message::post_send_message,
                ping::post_send_ping,
                presentation::proposal::post_send_proposal,
                resolver::get_resolve,
                schema::post_schemas,
                schema::get_all_schemas,
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
        .manage(credentials)
        .manage(schemas)
        .manage(webhook)
        .manage(didcomm)
}

#[cfg(test)]
pub fn test_rocket() -> Rocket<Build> {
    let rocket = rocket::build();
    let figment = rocket.figment();
    let config: Config = figment.extract().expect("config");

    let webhook = Box::new(webhook::test_client::TestClient::new(
        config.webhook_url.to_string(),
    )) as Box<dyn webhook::Webhook>;
    let didcomm = Box::new(didcomm::test_client::TestClient::new()) as Box<dyn didcomm::DidComm>;
    self::rocket(rocket, config, webhook, didcomm)
}