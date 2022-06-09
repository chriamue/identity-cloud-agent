use super::{receive, sign_and_encrypt};
use crate::connection::Connection;
use crate::ping::{PingRequest, PingResponse};
use crate::test_rocket;
use base58::FromBase58;
use didcomm_rs::Message;
use identity::account::Account;
use identity::account::AutoSave;
use identity::account::IdentitySetup;
use identity::account::MethodContent;
use identity::account::Result;
use identity::account_storage::MemStore;
use identity::prelude::KeyPair;
use identity::prelude::*;
use rocket::http::{ContentType, Status};
use rocket::local::blocking::Client;
use serde_json::{from_value, json, Value};

#[test]
fn test_send_ping() {
    let client = Client::tracked(test_rocket()).expect("valid rocket instance");
    let response = client.get("/connections").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let response = response.into_json::<Value>().unwrap();
    let connections = response.as_array().unwrap();
    assert_eq!(connections.len(), 0);

    let response = client.post("/out-of-band/create-invitation").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let invitation: Value = response.into_json::<Value>().unwrap();
    let invitation: String = serde_json::to_string(&invitation).unwrap();

    let response = client
        .post("/out-of-band/receive-invitation")
        .header(ContentType::JSON)
        .body(invitation)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/connections").dispatch();
    assert_eq!(response.status(), Status::Ok);
    let response = response.into_json::<Value>().unwrap();
    let _connections: Vec<Connection> = from_value(response).unwrap();

    let body: Value = json!( {
        "response_requested": true
    });

    let ping_request: PingRequest = PingRequest {
        type_: "https://didcomm.org/trust-ping/2.0/ping".to_string(),
        id: "foo".to_string(),
        from: "bar".to_string(),
        body,
    };
    let ping_request: String = serde_json::to_string(&ping_request).unwrap();

    let response = client
        .post("/")
        .header(ContentType::JSON)
        .body(ping_request)
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
    let response = response.into_json::<PingResponse>().unwrap();
    assert_eq!(response.thid, "foo".to_string());
}

#[tokio::test]
async fn test_message_encryption() -> Result<(), Box<dyn std::error::Error>> {
    let seed = "CLKmgQ7NbRw3MpGu47TiSjQknGf2oBPnW9nFygzBkh9h";
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

    let did_from = "did:iota:HcFFrR72GJq2hXuwbz2UwE7wkDE2VRkX2NwHeSVroeUH".to_string();
    let did_to = "did:iota:HcFFrR72GJq2hXuwbz2UwE7wkDE2VRkX2NwHeSVroeUH".to_string();

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
    let seed = "CLKmgQ7NbRw3MpGu47TiSjQknGf2oBPnW9nFygzBkh9h";
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
    let did_to = "did:iota:HcFFrR72GJq2hXuwbz2UwE7wkDE2VRkX2NwHeSVroeUH".to_string();

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
