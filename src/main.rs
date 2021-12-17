#[macro_use]
extern crate rocket;
use identity::iota::IotaDID;
use rocket::get;
use rocket_okapi::{openapi, openapi_get_routes, swagger_ui::*};

mod config;
mod connection;
mod didcomm;
mod ping;
mod resolver;
mod wallet;

use config::Config;
use connection::Connections;
use wallet::Wallet;

#[openapi(skip)]
#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
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
async fn rocket() -> _ {
    let rocket = rocket::build();
    let figment = rocket.figment();
    let config: Config = figment.extract().expect("config");

    let connections: Connections = Connections::default();

    let wallet: Wallet = Wallet::load(
        config.stronghold_path.to_string().into(),
        config.password.to_string(),
        config.endpoint.to_string(),
    )
    .await;
    print_wallet(&wallet).await;

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
                didcomm::post_endpoint,
                ping::post_send_ping,
                wallet::get_all_dids,
                wallet::get_public_did,
                wallet::get_did_endpoint,
                wallet::post_did_endpoint,
                resolver::get_resolve
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
}
