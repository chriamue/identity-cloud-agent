use base58::ToBase58;
use identity::account::Account;
use identity::account::AutoSave;
use identity::account::IdentitySetup;
use identity::account::MethodContent;
use identity::account::Result;
use identity::account_storage::Stronghold;
use identity::core::Url;
use identity::iota::ResolvedIotaDocument;
use identity::iota::Resolver;
use identity::iota_core::IotaDID;
use identity::prelude::*;
use rocket::response::status::NotFound;
use rocket::State;
use rocket::{get, post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Wallet {
    pub account: Arc<Mutex<identity::account::Account>>,
}

impl Wallet {
    pub async fn load(
        stronghold_path: PathBuf,
        password: String,
        endpoint: String,
        did: String,
    ) -> Self {
        let iota_did: IotaDID = IotaDID::try_from(did).unwrap();
        info!("account: {:?}", iota_did);
        let account: Account = match Account::builder()
            .autosave(AutoSave::Every)
            .storage(
                Stronghold::new(&stronghold_path, password.to_string(), None)
                    .await
                    .unwrap(),
            )
            .autopublish(true)
            .load_identity(iota_did)
            .await
        {
            Ok(account) => account,
            Err(err) => {
                error!("{:?}", err);
                let mut account = Account::builder()
                    .autosave(AutoSave::Every)
                    .storage(
                        Stronghold::new(&stronghold_path, password.to_string(), None)
                            .await
                            .unwrap(),
                    )
                    .autopublish(true)
                    .create_identity(IdentitySetup::default())
                    .await
                    .unwrap();

                let x_keypair = KeyPair::new(KeyType::X25519).unwrap();

                println!(
                    "private x key: {}",
                    x_keypair.private().as_ref().to_base58()
                );

                account
                    .update_identity()
                    .create_method()
                    .content(MethodContent::PrivateX25519(x_keypair.private().clone()))
                    .fragment("kex-0")
                    .apply()
                    .await
                    .unwrap();

                let ed_keypair = KeyPair::new(KeyType::Ed25519).unwrap();

                println!(
                    "private sign key: {}",
                    ed_keypair.private().as_ref().to_base58()
                );

                account
                    .update_identity()
                    .create_method()
                    .content(MethodContent::PrivateEd25519(ed_keypair.private().clone()))
                    .fragment("key-1")
                    .apply()
                    .await
                    .unwrap();

                account
                    .update_identity()
                    .create_service()
                    .fragment("endpoint")
                    .type_("Endpoint")
                    .endpoint(Url::parse(endpoint).unwrap())
                    .apply()
                    .await
                    .unwrap();
                info!("created new identity: {:?}", account.did());
                account
            }
        };

        Wallet {
            account: Arc::new(Mutex::new(account)),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Did {
    id: String,
    key_type: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DidEndpoint {
    did: String,
    endpoint: String,
}

#[openapi(tag = "wallet")]
#[get("/wallet/did")]
pub async fn get_all_dids(wallet: &State<Wallet>) -> Json<Vec<Did>> {
    let lock = wallet.account.lock().await;
    let did: &IotaDID = lock.did();
    let key_type = "Ed25519VerificationKey2018".to_string();
    Json(vec![Did {
        id: did.to_string(),
        key_type,
    }])
}

#[openapi(tag = "wallet")]
#[get("/wallet/did/public")]
pub async fn get_public_did(wallet: &State<Wallet>) -> Json<Did> {
    let lock = wallet.account.lock().await;
    let did: &IotaDID = lock.did();
    let key_type = "Ed25519VerificationKey2018".to_string();
    Json(Did {
        id: did.to_string(),
        key_type,
    })
}

#[openapi(tag = "wallet")]
#[get("/wallet/get-did-endpoint?<did>")]
pub async fn get_did_endpoint(did: String) -> Json<String> {
    let did = IotaDID::from_str(&did).unwrap();
    let resolver: Resolver = Resolver::new().await.unwrap();
    let resolved_did_document: ResolvedIotaDocument = resolver.resolve(&did).await.unwrap();

    let document = resolved_did_document.document;
    let services = document.service();
    let service = services.get(0).unwrap();
    let endpoint = service.service_endpoint().to_string();
    let endpoint = endpoint.replace('\"', "");
    Json(endpoint)
}

#[openapi(tag = "wallet")]
#[post("/wallet/set-did-endpoint", data = "<post_data>")]
pub async fn post_did_endpoint(
    wallet: &State<Wallet>,
    post_data: Json<DidEndpoint>,
) -> Result<(), NotFound<String>> {
    let mut account = wallet.account.lock().await;
    account
        .update_identity()
        .create_service()
        .fragment("endpoint")
        .type_("Endpoint")
        .endpoint(Url::parse(&post_data.endpoint).unwrap())
        .apply()
        .await
        .unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::test_rocket;
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use serde_json::Value;

    #[test]
    fn test_public_did() {
        let client = Client::tracked(test_rocket()).expect("valid rocket instance");
        let response = client.get("/wallet/did/public").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().unwrap();
        assert!(response.get("id").is_some());
    }
}
