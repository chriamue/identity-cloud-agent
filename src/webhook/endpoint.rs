use rocket_okapi::okapi::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

fn example_url() -> &'static str {
    "http://localhost:5000"
}

fn example_id() -> &'static str {
    "2fecc993-b92c-4152-8c81-35adde124382"
}

fn example_authorization() -> &'static str {
    "Basic YWxhZGRpbjpvcGVuc2VzYW1l"
}

fn example_x_api_key() -> &'static str {
    "abcdef12345"
}

fn example_registered_event() -> Value {
    json!(vec![RegisteredEvent::All])
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub enum RegisteredEvent {
    #[serde(rename = "ALL")]
    All,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct WebhookEndpoint {
    #[schemars(example = "example_id")]
    pub id: Option<String>,
    #[schemars(example = "example_url")]
    pub url: String,
    #[serde(rename = "Authorization")]
    #[schemars(example = "example_authorization")]
    pub authorization: Option<String>,
    #[serde(rename = "X-API-Key")]
    #[schemars(example = "example_x_api_key")]
    pub x_api_key: Option<String>,
    #[serde(rename = "registeredEvent")]
    #[schemars(example = "example_registered_event")]
    pub registered_event: Vec<RegisteredEvent>,
}

impl Default for WebhookEndpoint {
    fn default() -> Self {
        WebhookEndpoint {
            id: Some(Uuid::new_v4().to_string()),
            url: example_url().to_string(),
            authorization: Some(example_authorization().to_string()),
            x_api_key: Some(example_x_api_key().to_string()),
            registered_event: vec![RegisteredEvent::All],
        }
    }
}

impl WebhookEndpoint {
    pub fn new_from(webhook_endpoint: &WebhookEndpoint) -> WebhookEndpoint {
        WebhookEndpoint {
            id: match &webhook_endpoint.id {
                Some(id) => Some(id.to_string()),
                None => Some(Uuid::new_v4().to_string()),
            },
            url: webhook_endpoint.url.to_string(),
            authorization: webhook_endpoint.authorization.clone(),
            x_api_key: webhook_endpoint.x_api_key.clone(),
            registered_event: webhook_endpoint.registered_event.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let mut webhook_endpoint = WebhookEndpoint::default();
        webhook_endpoint.id = None;
        let new_webhook_endpoint = WebhookEndpoint::new_from(&webhook_endpoint);
        assert!(new_webhook_endpoint.id.is_some());
    }
}
