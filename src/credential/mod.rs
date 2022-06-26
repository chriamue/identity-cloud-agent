use identity_iota::core::FromJson;
use identity_iota::core::Url;
use identity_iota::credential::Credential;
use identity_iota::credential::Subject;
use identity_iota::did::DID;
use identity_iota::iota_core::IotaDID;
use identity_iota::prelude::KeyPair;
use identity_iota::prelude::*;
use rocket::State;
use rocket::{get, serde::json::Json};
use rocket_okapi::okapi::schemars::{self, JsonSchema};
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod events;
pub mod issue;

pub use events::IssueCredentialEvent;
pub use events::IssueCredentialEvents;

#[derive(Default)]
pub struct Credentials {
    pub credentials: Arc<Mutex<HashMap<String, Credential>>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CredentialsResponse {
    #[schemars(example = "example_credentials")]
    result: Vec<Value>,
}

pub fn example_credential() -> Value {
    let subject_key: KeyPair = KeyPair::new(KeyType::Ed25519).unwrap();
    let subject_did: IotaDID = IotaDID::new(subject_key.public().as_ref()).unwrap();

    let subject: Subject = Subject::from_json_value(json!({
      "id": subject_did.as_str(),
      "degree": {
        "type": "BachelorDegree",
        "name": "Bachelor of Science and Arts"
      }
    }))
    .unwrap();

    let credential: Credential = Credential::builder(Default::default())
        .issuer(Url::parse("did://example.com").unwrap())
        .type_("UniversityDegreeCredential")
        .subject(subject)
        .build()
        .unwrap();
    serde_json::to_value(&credential).unwrap()
}

pub fn example_credentials() -> Vec<Value> {
    vec![example_credential()]
}

#[openapi(tag = "credentials")]
#[get("/credentials")]
pub async fn get_all_credentials(credentials: &State<Credentials>) -> Json<CredentialsResponse> {
    let lock = credentials.credentials.lock().await;
    let result: Vec<Value> = lock
        .values()
        .cloned()
        .map(|c| serde_json::to_value(&c).unwrap())
        .collect();
    Json(CredentialsResponse { result })
}

#[cfg(test)]
mod tests {
    use crate::test_rocket;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use serde_json::Value;

    #[tokio::test]
    async fn test_credentials() {
        let client = Client::tracked(test_rocket().await)
            .await
            .expect("valid rocket instance");
        let response = client.get("/credentials").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        let credentials = response.get("result").unwrap().as_array().unwrap();
        assert_eq!(credentials.len(), 0);
    }
}
