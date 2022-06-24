use identity_iota::credential::Credential;
use rocket::State;
use rocket::{get, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod issue;

#[derive(Default)]
pub struct Credentials {
    pub credentials: Arc<Mutex<HashMap<String, Credential>>>,
}

#[openapi(tag = "credentials")]
#[get("/credentials")]
pub async fn get_all_credentials(credentials: &State<Credentials>) -> Json<Value> {
    let lock = credentials.credentials.lock().await;
    let credentials: Vec<Credential> = lock.values().cloned().collect();
    let credentials = serde_json::to_value(credentials).unwrap();
    Json(credentials)
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
        let credentials = response.as_array().unwrap();
        assert_eq!(credentials.len(), 0);
    }
}
