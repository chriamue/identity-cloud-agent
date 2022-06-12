#[macro_use]
extern crate rocket;
use base58::{FromBase58, ToBase58};
use did_key::{generate, DIDCore, KeyMaterial, X25519KeyPair};
use identity_cloud_agent::{didcomm, webhook, Config, ConfigExt};

#[launch]
pub fn rocket() -> _ {
    let rocket = rocket::build();
    let figment = rocket.figment();
    let mut config: Config = figment.extract().expect("config");
    let config_ext: ConfigExt = figment.extract().expect("config ext");

    let key = match config.key_seed.clone() {
        Some(seed) => generate::<X25519KeyPair>(Some(&seed.from_base58().unwrap())),
        None => {
            let key = generate::<X25519KeyPair>(None);
            let seed = key.private_key_bytes().to_base58();
            println!("Generated Seed: {}", seed);
            config.key_seed = Some(seed);
            key
        }
    };
    config.did_key = Some(key.get_did_document(Default::default()).id);

    let webhook = Box::new(webhook::Client::new(
        config_ext.webhook_url.as_ref().unwrap().to_string(),
    )) as Box<dyn webhook::Webhook>;
    let didcomm = Box::new(didcomm::Client::new()) as Box<dyn didcomm::DidComm>;

    identity_cloud_agent::rocket(rocket, config, webhook, didcomm)
}
