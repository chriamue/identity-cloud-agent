use identity::account::AccountStorage;
use identity::account::AutoSave;
use identity::account::IdentityCreate;
use identity::account::Result;
use identity::account::{Account, IdentityState};
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
use std::sync::Arc;
use std::thread;
use tokio::sync::Mutex;

pub struct Wallet {
    pub account: Arc<Mutex<identity::account::Account>>,
    pub identity: Arc<Mutex<identity::account::IdentityState>>,
}

impl Wallet {
    pub async fn load(stronghold_path: PathBuf, password: String, endpoint: String) -> Self {
        let account: Account = Account::builder()
            .autosave(AutoSave::Every)
            .storage(AccountStorage::Stronghold(stronghold_path, Some(password)))
            .autopublish(true)
            .build()
            .await
            .unwrap();

        let identity: IdentityState = account
            .create_identity(IdentityCreate::default())
            .await
            .unwrap();

        let iota_did: &IotaDID = identity.try_did().unwrap();

        account
            .update_identity(&iota_did)
            .create_service()
            .fragment("endpoint")
            .type_("Endpoint")
            .endpoint(Url::parse(endpoint).unwrap())
            .apply()
            .await
            .unwrap();

        Wallet {
            account: Arc::new(Mutex::new(account)),
            identity: Arc::new(Mutex::new(identity)),
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
    let lock = wallet.identity.lock().await;
    let did: &IotaDID = lock.try_did().unwrap();
    let key_type = "Ed25519VerificationKey2018".to_string();
    Json(vec![Did {
        id: did.to_string(),
        key_type,
    }])
}

#[openapi(tag = "wallet")]
#[get("/wallet/did/public")]
pub async fn get_public_did(wallet: &State<Wallet>) -> Json<Did> {
    let lock = wallet.identity.lock().await;
    let did: &IotaDID = lock.try_did().unwrap();
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
    Json(service.service_endpoint().to_string())
}

#[openapi(tag = "wallet")]
#[post("/wallet/set-did-endpoint", data = "<post_data>")]
pub async fn post_did_endpoint(
    wallet: &State<Wallet>,
    post_data: Json<DidEndpoint>,
) -> Result<(), NotFound<String>> {
    let identity = wallet.identity.lock().await;

    let iota_did: &IotaDID = identity.try_did().unwrap();

    let account = wallet.account.lock().await;
    account
        .update_identity(&iota_did)
        .create_service()
        .fragment("endpoint")
        .type_("Endpoint")
        .endpoint(Url::parse(post_data.endpoint.to_string()).unwrap())
        .apply()
        .await
        .unwrap();

    Ok(())
}
