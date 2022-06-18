use async_trait::async_trait;
use reqwest::RequestBuilder;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::Value;

pub mod client;
mod endpoint;
pub use endpoint::WebhookEndpoint;

#[cfg(test)]
pub mod test_client;

pub use client::Client;

#[async_trait]
pub trait Webhook: Send + Sync {
    fn request(&self, topic: &str, body: &Value) -> RequestBuilder;
    async fn post(&self, topic: &str, body: &Value) -> Result<reqwest::Response, reqwest::Error>;
}

pub struct WebhookPool {}

/// # List registered webhooks
#[openapi(tag = "webhook")]
#[get("/webhook")]
pub async fn get_all_webhooks() -> Json<Vec<WebhookEndpoint>> {
    let webhooks: Vec<WebhookEndpoint> = vec![WebhookEndpoint::default()];
    Json(webhooks)
}

/// # Register a new webhook
#[openapi(tag = "webhook")]
#[post("/webhook", data = "<request>")]
pub async fn post_webhook(request: Json<WebhookEndpoint>) -> Json<WebhookEndpoint> {
    let webhook = WebhookEndpoint::new_from(&request.into_inner());
    Json(webhook)
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
