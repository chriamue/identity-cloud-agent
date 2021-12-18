use identity::account::Account;
use identity::account::AccountStorage;
use identity::account::AutoSave;
use identity::account::IdentitySetup;
use identity::account::Result;
use identity::core::Url;
use identity::did::resolution;
use identity::did::resolution::InputMetadata;
use identity::iota::ClientMap;
use identity::iota::IotaDID;
use rocket::response::status::NotFound;
use rocket::State;
use rocket::{get, post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str;
use std::sync::Arc;
use std::thread;
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
        println!("{:?}", iota_did);
        let account: Account = match Account::builder()
            .autosave(AutoSave::Every)
            .storage(AccountStorage::Stronghold(
                stronghold_path.clone(),
                Some(password.to_string()),
                None,
            ))
            .autopublish(true)
            .load_identity(iota_did)
            .await
        {
            Ok(account) => account,
            Err(err) => {
                println!("{:?}", err);
                let mut account = Account::builder()
                    .autosave(AutoSave::Every)
                    .storage(AccountStorage::Stronghold(
                        stronghold_path,
                        Some(password),
                        None,
                    ))
                    .autopublish(true)
                    .create_identity(IdentitySetup::default())
                    .await
                    .unwrap();

                account
                    .update_identity()
                    .create_method()
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
pub fn get_did_endpoint(did: String) -> Json<String> {
    let client: ClientMap = ClientMap::new();
    let input: InputMetadata = Default::default();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let output = thread::spawn(move || {
        let out = runtime.block_on(resolution::resolve(did.as_str(), input, &client));
        out
    })
    .join()
    .expect("Thread panicked")
    .unwrap();

    let document = output.document.unwrap();
    let services = document.service();
    let service = services.get(0).unwrap();
    let endpoint = service.service_endpoint().to_string();
    let endpoint = endpoint.replace("\"", "");
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
        .endpoint(Url::parse(post_data.endpoint.to_string()).unwrap())
        .apply()
        .await
        .unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::rocket;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    #[test]
    fn test_public_did() {
        let client = Client::tracked(rocket()).expect("valid rocket instance");

        let response = client.get("/wallet/did/public").dispatch();
        assert_eq!(response.status(), Status::Ok);
    }
}
