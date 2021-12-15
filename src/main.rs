#[macro_use]
extern crate rocket;
use identity::iota::ExplorerUrl;
use identity::iota::IotaDID;
use rocket::get;
use rocket_okapi::{openapi, openapi_get_routes, swagger_ui::*};

mod config;
mod resolver;
mod wallet;

use config::Config;
use wallet::Wallet;

#[openapi(skip)]
#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

async fn print_wallet(wallet: &Wallet) {
    let iota_did: &IotaDID = wallet.account.did();
    println!(
        "Local Document from {} = {:#?}",
        iota_did,
        wallet.account.state().document()
    );
    let explorer: &ExplorerUrl = ExplorerUrl::mainnet();
    println!(
        "Explore the DID Document = {}",
        explorer.resolver_url(iota_did).unwrap()
    );
}

#[launch]
async fn rocket() -> _ {
    let rocket = rocket::build();
    let figment = rocket.figment();
    let config: Config = figment.extract().expect("config");

    let wallet: Wallet = Wallet::load(
        config.stronghold_path.to_string().into(),
        config.password.to_string(),
    )
    .await;
    print_wallet(&wallet).await;

    rocket
        .mount(
            "/",
            openapi_get_routes![
                index,
                wallet::get_all_dids,
                wallet::get_public_did,
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
}
