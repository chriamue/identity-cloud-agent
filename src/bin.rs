#[macro_use]
extern crate rocket;
use identity_cloud_agent::{didcomm, webhook, Config};

#[launch]
pub fn rocket() -> _ {
    let rocket = rocket::build();
    let figment = rocket.figment();
    let config: Config = figment.extract().expect("config");

    let webhook =
        Box::new(webhook::Client::new(config.webhook_url.to_string())) as Box<dyn webhook::Webhook>;
    let didcomm = Box::new(didcomm::Client::new()) as Box<dyn didcomm::DidComm>;

    identity_cloud_agent::rocket(rocket, config, webhook, didcomm)
}
