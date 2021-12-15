use std::path::PathBuf;

use identity::account::Account;
use identity::account::AccountStorage;
use identity::account::AutoSave;
use identity::account::IdentitySetup;

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
