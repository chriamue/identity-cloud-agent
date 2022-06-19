#[macro_use]
extern crate rocket;
pub use didcomm_mediator::config::Config;
pub use didcomm_mediator::wallet::Wallet;
use rocket::get;
use rocket::response::Redirect;
use rocket::{Build, Rocket};
use rocket_okapi::{openapi, openapi_get_routes, swagger_ui::*};
use std::sync::Arc;
use tokio::sync::Mutex;
use webhook::WebhookPool;

pub mod configext;
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
pub use configext::ConfigExt;
use connection::{ConnectionEvents, Connections};
use credential::Credentials;
pub use didcomm::DidComm;
use message::MessageEvents;
use ping::PingEvents;
use schema::Schemas;
pub use webhook::Webhook;

#[openapi(skip)]
#[get("/")]
fn index() -> Redirect {
    Redirect::to("/swagger-ui")
}

pub async fn rocket(
    rocket: Rocket<Build>,
    config: Config,
    webhook_pool: WebhookPool,
    didcomm: Box<dyn didcomm::DidComm>,
) -> Rocket<Build> {
    let connections: Connections = Connections::default();
    let credentials: Credentials = Credentials::default();
    let schemas: Schemas = Schemas::default();

    let cloned_config = config.clone();
    let wallet = Wallet::new_from_config(&cloned_config).await.unwrap();
    wallet.log();
    let wallet = Arc::new(Mutex::new(wallet));

    let connection_events: Arc<Mutex<ConnectionEvents>> =
        Arc::new(Mutex::new(ConnectionEvents::new()));
    let ping_events: Arc<Mutex<PingEvents>> = Arc::new(Mutex::new(PingEvents::new()));
    let message_events: Arc<Mutex<MessageEvents>> = Arc::new(Mutex::new(MessageEvents::new()));

    let mut webhook_pool = webhook_pool;

    webhook_pool
        .spawn_connection_events(connection_events.clone())
        .await;
    webhook_pool.spawn_ping_events(ping_events.clone()).await;
    webhook_pool
        .spawn_message_events(message_events.clone())
        .await;

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
                ledger::get_did_verkey,
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
                wallet::post_did_endpoint,
                webhook::get_all_webhooks,
                webhook::post_webhook,
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
        .manage(webhook_pool)
        .manage(didcomm)
        .manage(connection_events)
        .manage(ping_events)
        .manage(message_events)
}

#[cfg(test)]
pub async fn test_rocket() -> Rocket<Build> {
    let rocket = rocket::build();
    let figment = rocket.figment();
    let config: Config = figment.extract().expect("config");
    let didcomm = Box::new(didcomm::test_client::TestClient::new()) as Box<dyn didcomm::DidComm>;
    self::rocket(rocket, config, WebhookPool::default(), didcomm).await
}

#[cfg(test)]
pub async fn test_rocket_with_webhook_client(webhook_client: Arc<Mutex<Box<dyn webhook::Webhook>>>) -> Rocket<Build> {
    let rocket = rocket::build();
    let figment = rocket.figment();
    let config: Config = figment.extract().expect("config");
    let config_ext: ConfigExt = figment.extract().expect("config ext");

    let webhook_endpoint = crate::webhook::WebhookEndpoint {
        url: config_ext.webhook_url.as_ref().unwrap().to_string(),
        ..Default::default()
    };

    let webhook_pool = WebhookPool::default();
    webhook_pool.webhooks.try_lock().unwrap().insert(
        webhook_endpoint.id.as_ref().unwrap().to_string(),
        (webhook_endpoint, webhook_client),
    );

    let didcomm = Box::new(didcomm::test_client::TestClient::new()) as Box<dyn didcomm::DidComm>;
    self::rocket(rocket, config, WebhookPool::default(), didcomm).await
}
