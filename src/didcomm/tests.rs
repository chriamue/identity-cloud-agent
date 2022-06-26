use super::{receive, sign_and_encrypt};
use crate::credential::IssueCredentialEvent;
use crate::wallet::tests::get_did;
use crate::webhook;
use crate::Config;
use crate::{test_rocket, test_rocket_with_webhook_client};
use base58::FromBase58;
use didcomm_mediator::message::add_return_route_all_header;
use didcomm_protocols::IssueCredentialResponseBuilder;
use didcomm_protocols::TrustPingResponseBuilder;
use didcomm_rs::Message;
use identity_iota::account::{Account, AutoSave, IdentitySetup, MethodContent, Result};
use identity_iota::account_storage::MemStore;
use identity_iota::core::{FromJson, Url};
use identity_iota::credential::{Credential, CredentialBuilder, Subject};
use identity_iota::prelude::KeyPair;
use identity_iota::prelude::*;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_receive_ping() {
    let client = Client::tracked(test_rocket().await)
        .await
        .expect("valid rocket instance");

    let did = get_did(&client).await.unwrap();

    let mut message = TrustPingResponseBuilder::new().build_ping().unwrap();
    message = add_return_route_all_header(message);
    message = message.from(&did).to(&[&did]);

    let response = client.post(format!("/")).json(&message).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

#[tokio::test]
async fn test_receive_issue_credential() {
    let webhook_client = Box::new(webhook::test_client::TestClient::new(
        "http://localhost".to_string(),
    )) as Box<dyn webhook::Webhook>;
    let webhook_client = Arc::new(Mutex::new(webhook_client));
    let client = Client::tracked(test_rocket_with_webhook_client(webhook_client.clone()).await)
        .await
        .expect("valid rocket instance");

    let did = get_did(&client).await.unwrap();

    let subject: Subject = Subject::from_json_value(
        json!({"id": Url::parse(did.as_str()).unwrap(), "attributes": "None".to_string()}),
    )
    .unwrap();

    let credential: Credential = CredentialBuilder::default()
        .id(Url::parse("https://example.edu/credentials/3732").unwrap())
        .issuer(Url::parse(did.as_str()).unwrap())
        .subject(subject)
        .build()
        .unwrap();

    let webhook_response = serde_json::to_value(IssueCredentialEvent::IssueCredentialReceived(
        did.to_string(),
        serde_json::to_value(&credential).unwrap(),
    ))
    .unwrap();

    let attachment = serde_json::to_value(&credential).unwrap();
    let mut message = IssueCredentialResponseBuilder::new()
        .goal_code("issue-vc".to_string())
        .attachment(attachment)
        .build_issue_credential()
        .unwrap();
    message = add_return_route_all_header(message);
    message = message.from(&did).to(&[&did]);

    let response = client.post(format!("/")).json(&message).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/credentials").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let response = response.into_json::<Value>().await.unwrap();
    let credentials = response.get("result").unwrap().as_array().unwrap();
    assert_eq!(credentials.len(), 1);
    assert_eq!(
        webhook::test_client::last_response(&webhook_client).unwrap(),
        webhook_response
    );
}

#[tokio::test]
async fn test_message_encryption() -> Result<(), Box<dyn std::error::Error>> {
    let rocket = test_rocket().await;
    let figment = rocket.figment();
    let config: Config = figment.extract().expect("config");
    let seed = &config.key_seed.unwrap();
    let private = seed.from_base58().unwrap();

    let keypair_ed = KeyPair::new(KeyType::Ed25519)?;
    let keypair_key_exchange = KeyPair::new(KeyType::X25519)?;

    let sender_keypair_ex = KeyPair::try_from_private_key_bytes(KeyType::X25519, &private).unwrap();
    let receiver_keypair_ex =
        KeyPair::try_from_private_key_bytes(KeyType::X25519, &private).unwrap();

    let mut account: Account = Account::builder()
        .autosave(AutoSave::Never)
        .autopublish(false)
        .storage(MemStore::new())
        .create_identity(IdentitySetup::default())
        .await?;

    account
        .update_identity()
        .create_method()
        .fragment("signing-method")
        .content(MethodContent::PrivateEd25519(
            keypair_ed.private().to_owned(),
        ))
        .apply()
        .await?;

    account
        .update_identity()
        .create_method()
        .fragment("key-exchange-method")
        .content(MethodContent::PrivateX25519(
            keypair_key_exchange.private().to_owned(),
        ))
        .apply()
        .await?;

    let did_from = config.did_iota.as_ref().unwrap().to_string();
    let did_to = config.did_iota.unwrap().to_string();

    let message = Message::new();
    let message = serde_json::to_string(
        &sign_and_encrypt(&message, &did_from, &did_to, &sender_keypair_ex)
            .await
            .unwrap(),
    )
    .unwrap();

    println!("{:?}", message);

    let received = receive(&message, &receiver_keypair_ex.private().as_ref(), None).await;
    received.unwrap();

    Ok(())
}

#[tokio::test]
async fn test_did_not_on_ledger_on_message_encryption() -> Result<(), Box<dyn std::error::Error>> {
    let rocket = test_rocket().await;
    let figment = rocket.figment();
    let config: Config = figment.extract().expect("config");
    let seed = &config.key_seed.unwrap();
    let private = seed.from_base58().unwrap();

    let keypair_ed = KeyPair::new(KeyType::Ed25519)?;
    let sender_keypair_ex = KeyPair::new(KeyType::X25519)?;

    let receiver_keypair_ex =
        KeyPair::try_from_private_key_bytes(KeyType::X25519, &private).unwrap();

    let mut account: Account = Account::builder()
        .autosave(AutoSave::Never)
        .autopublish(false)
        .storage(MemStore::new())
        .create_identity(IdentitySetup::default())
        .await?;

    account
        .update_identity()
        .create_method()
        .fragment("key-0")
        .content(MethodContent::PrivateEd25519(
            keypair_ed.private().to_owned(),
        ))
        .apply()
        .await?;

    account
        .update_identity()
        .create_method()
        .fragment("kex-0")
        .content(MethodContent::PrivateX25519(
            sender_keypair_ex.private().to_owned(),
        ))
        .apply()
        .await?;

    let did_from = account.did().to_string();
    let did_to = config.did_iota.unwrap().to_string();

    let message = Message::new();
    let message = serde_json::to_string(
        &sign_and_encrypt(&message, &did_from, &did_to, &sender_keypair_ex)
            .await
            .unwrap(),
    )
    .unwrap();

    let received = receive(&message, &receiver_keypair_ex.private().as_ref(), None).await;
    assert!(received.is_err());
    Ok(())
}
