use identity::account::Account;
use identity::account::AccountStorage;
use identity::account::AutoSave;
use identity::account::IdentitySetup;
use identity::iota::IotaDID;
use rocket::{get, serde::json::Json};
use rocket::{State};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::{openapi};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct Wallet {
    pub account: identity::account::Account,
}

impl Wallet {
    pub async fn load(stronghold_path: PathBuf, password: String) -> Self {
        let account: Account = Account::builder()
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

        Wallet { account }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Did {
    id: String,
    key_type: String,
}

#[openapi(tag = "Wallet")]
#[get("/wallet/did")]
pub fn get_all_dids(wallet: &State<Wallet>) -> Json<Vec<Did>> {
    let did: &IotaDID = wallet.account.did();
    let key_type = "Ed25519VerificationKey2018".to_string();
    Json(vec![Did {
        id: did.to_string(),
        key_type,
    }])
}
