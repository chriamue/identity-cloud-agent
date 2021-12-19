use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde_json::Value;

pub mod client;

#[cfg(test)]
pub mod test_client;

pub use client::Client;

#[async_trait]
pub trait Webhook: Send + Sync {
    fn request(&self, topic: &str, body: &Value) -> RequestBuilder;
    async fn post(&self, topic: &str, body: &Value) -> Result<reqwest::Response, reqwest::Error>;
}
