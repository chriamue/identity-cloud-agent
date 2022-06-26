use crate::wallet::Wallet;
use identity_iota::client::ResolvedIotaDocument;
use identity_iota::credential::Credential;
use identity_iota::did::verifiable::VerifierOptions;
use identity_iota::prelude::*;
use rocket::http::Status;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars::{self, JsonSchema};
use rocket_okapi::openapi;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Serialize, JsonSchema)]
pub struct VerifyResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    valid: bool,
}

/// # Verify a JSON-LD structure
#[openapi(tag = "jsonld")]
#[post("/jsonld/verify", data = "<request>")]
pub async fn post_verify(
    wallet: &State<Arc<Mutex<Wallet>>>,
    request: Json<Value>,
) -> Result<Json<VerifyResponse>, Status> {
    let response = match serde_json::from_value::<Credential>(request.into_inner()) {
        Ok(credential) => {
            let wallet = wallet.try_lock().unwrap();
            let account = wallet.account.as_ref().unwrap();
            let resolved: ResolvedIotaDocument = account.resolve_identity().await.unwrap();
            let valid = resolved
                .document
                .verify_data(&credential, &VerifierOptions::default())
                .is_ok();
            VerifyResponse { error: None, valid }
        }
        Err(err) => VerifyResponse {
            error: Some(format!("{}", err)),
            valid: false,
        },
    };
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::issue::example_credential_preview;
    use crate::credential::issue::prepare_issue_credential_request;
    use crate::credential::issue::SendRequest;
    use crate::test_rocket;
    use crate::Config;
    use crate::Wallet;
    use base64::decode;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};

    #[tokio::test]
    async fn test_verify_jsonld() {
        let rocket = test_rocket().await;
        let figment = rocket.figment();
        let config: Config = figment.extract().expect("config");
        let wallet = Wallet::new_from_config(&config).await.unwrap();
        let request = SendRequest {
            connection_id: "".to_string(),
            comment: "".to_string(),
            credential_preview: example_credential_preview(),
        };
        let did_to = wallet.did_iota().unwrap();
        let (message, _value) =
            prepare_issue_credential_request(&wallet, did_to.to_string(), request)
                .await
                .unwrap();
        let attachment = message.get_attachments().next().unwrap();
        let credential: Credential =
            serde_json::from_slice(&decode(&attachment.data.base64.as_ref().unwrap()).unwrap())
                .unwrap();

        let client = Client::tracked(test_rocket().await)
            .await
            .expect("valid rocket instance");

        let response = client
            .post("/jsonld/verify")
            .json(&credential)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let response = dbg!(response.into_json::<Value>().await.unwrap());
        let valid = response.get("valid").unwrap().as_bool().unwrap();
        assert_eq!(valid, true);
    }

    #[tokio::test]
    async fn test_verify_invalid_jsonld() {
        let client = Client::tracked(test_rocket().await)
            .await
            .expect("valid rocket instance");

        let response = client
            .post("/jsonld/verify")
            .json(&json!({"credential": "no credential".to_string()}))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let response = dbg!(response.into_json::<Value>().await.unwrap());
        let valid = response.get("valid").unwrap().as_bool().unwrap();
        assert_eq!(valid, false);
    }
}
