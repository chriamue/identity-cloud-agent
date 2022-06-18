use async_trait::async_trait;
use reqwest::RequestBuilder;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod client;
mod endpoint;
pub use endpoint::WebhookEndpoint;

#[cfg(test)]
pub mod test_client;

pub use client::Client;

#[async_trait]
pub trait Webhook: core::fmt::Debug + Send + Sync {
    fn request(&self, topic: &str, body: &Value) -> RequestBuilder;
    async fn post(&self, topic: &str, body: &Value) -> Result<reqwest::Response, reqwest::Error>;
}

#[derive(Debug)]
pub struct WebhookPool {
    pub webhooks: Arc<Mutex<HashMap<String, (WebhookEndpoint, Box<dyn Webhook>)>>>,
}

impl Default for WebhookPool {
    fn default() -> Self {
        WebhookPool {
            webhooks: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl WebhookPool {
    pub async fn post(&self, topic: &str, body: &Value) -> Result<(), reqwest::Error> {
        let map = self.webhooks.try_lock().unwrap();
        for (key, value) in &*map {
            let (_webhook_endpoint, webhook) = value;
            match webhook.post(topic, body).await {
                Ok(_) => (),
                Err(err) => println!("{}: {:?}", key, err),
            }
        }
        Ok(())
    }
}

/// # List registered webhooks
#[openapi(tag = "webhook")]
#[get("/webhook")]
pub async fn get_all_webhooks(webhook_pool: &State<WebhookPool>) -> Json<Vec<WebhookEndpoint>> {
    let webhooks: Vec<WebhookEndpoint> = {
        let mut webhooks = Vec::new();
        let map = webhook_pool.webhooks.try_lock().unwrap();
        for (_key, value) in &*map {
            let (webhook_endpoint, _webhook) = value;
            webhooks.push(webhook_endpoint.clone());
        }
        webhooks
    };
    Json(webhooks)
}

/// # Register a new webhook
#[openapi(tag = "webhook")]
#[post("/webhook", data = "<request>")]
pub async fn post_webhook(
    webhook_pool: &State<WebhookPool>,
    request: Json<WebhookEndpoint>,
) -> Json<WebhookEndpoint> {
    let webhook_endpoint = WebhookEndpoint::new_from(&request.into_inner());
    let webhook_client =
        Box::new(Client::new(webhook_endpoint.url.to_string())) as Box<dyn Webhook>;
    webhook_pool.webhooks.try_lock().unwrap().insert(
        webhook_endpoint.id.as_ref().unwrap().to_string(),
        (webhook_endpoint.clone(), webhook_client),
    );
    webhook_pool
        .post(
            "webhook-added",
            &serde_json::to_value(&webhook_endpoint).unwrap(),
        )
        .await
        .unwrap();
    Json(webhook_endpoint)
}

#[cfg(test)]
mod tests {
    use crate::test_rocket;
    use crate::webhook::WebhookEndpoint;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;

    #[test]
    fn test_post_webhook() {
        let client = Client::tracked(test_rocket()).expect("valid rocket instance");

        let mut webhook = WebhookEndpoint::default();
        webhook.id = None;

        let response = client
            .post("/webhook")
            .header(ContentType::JSON)
            .json(&webhook)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<WebhookEndpoint>().unwrap();
        assert!(response.id.is_some());
    }
}
