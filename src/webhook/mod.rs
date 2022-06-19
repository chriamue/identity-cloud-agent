use crate::connection::ConnectionEvents;
use crate::message::MessageEvents;
use crate::ping::PingEvents;
use async_trait::async_trait;
use reqwest::RequestBuilder;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use {futures::StreamExt, pharos::*};

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

#[derive(Debug, Clone)]
pub struct WebhookPool {
    pub webhooks: Arc<Mutex<HashMap<String, (WebhookEndpoint, Box<dyn Webhook>)>>>,
    pub connection_task: Option<Arc<JoinHandle<()>>>,
    pub ping_task: Option<Arc<JoinHandle<()>>>,
    pub message_task: Option<Arc<JoinHandle<()>>>,
}

impl Default for WebhookPool {
    fn default() -> Self {
        WebhookPool {
            webhooks: Arc::new(Mutex::new(HashMap::new())),
            connection_task: None,
            ping_task: None,
            message_task: None,
        }
    }
}

impl WebhookPool {
    pub async fn post(&self, topic: &str, body: &Value) -> Result<(), reqwest::Error> {
        Self::post_webhooks(topic, body, self.webhooks.clone()).await
    }

    pub async fn post_webhooks(
        topic: &str,
        body: &Value,
        webhooks: Arc<Mutex<HashMap<String, (WebhookEndpoint, Box<dyn Webhook>)>>>,
    ) -> Result<(), reqwest::Error> {
        let map = webhooks.try_lock().unwrap();
        for (key, value) in &*map {
            let (_webhook_endpoint, webhook) = value;
            match webhook.post(topic, body).await {
                Ok(_) => (),
                Err(err) => println!("{}: {:?}", key, err),
            }
        }
        Ok(())
    }

    pub async fn spawn_connection_events(
        &mut self,
        connection_events: Arc<Mutex<ConnectionEvents>>,
    ) {
        let mut events = {
            let mut connection_events = connection_events.try_lock().unwrap();
            connection_events
                .observe(Channel::Bounded(20).into())
                .await
                .expect("observe")
        };
        let webhooks: Arc<Mutex<HashMap<String, (WebhookEndpoint, Box<dyn Webhook>)>>> =
            self.webhooks.clone();
        let future = async move {
            while let Some(event) = events.next().await {
                match Self::post_webhooks(
                    "connections",
                    &serde_json::to_value(&event).unwrap(),
                    webhooks.clone(),
                )
                .await
                {
                    Ok(_) => (),
                    Err(err) => println!("{:?}", err),
                }
            }
            println!("end async connection future events");
        };
        let task = tokio::task::spawn(future);
        self.connection_task = Some(Arc::new(task));
    }

    pub async fn spawn_ping_events(&mut self, ping_events: Arc<Mutex<PingEvents>>) {
        let mut events = {
            let mut ping_events = ping_events.try_lock().unwrap();
            ping_events
                .observe(Channel::Unbounded.into())
                .await
                .expect("observe")
        };
        let webhooks: Arc<Mutex<HashMap<String, (WebhookEndpoint, Box<dyn Webhook>)>>> =
            self.webhooks.clone();
        let future = async move {
            while let Some(event) = events.next().await {
                match Self::post_webhooks(
                    "ping",
                    &serde_json::to_value(&event).unwrap(),
                    webhooks.clone(),
                )
                .await
                {
                    Ok(_) => (),
                    Err(err) => println!("{:?}", err),
                }
            }
            println!("end async ping future events");
        };
        let task = tokio::task::spawn(future);
        self.ping_task = Some(Arc::new(task));
    }

    pub async fn spawn_message_events(&mut self, message_events: Arc<Mutex<MessageEvents>>) {
        let mut events = {
            let mut message_events = message_events.try_lock().unwrap();
            message_events
                .observe(Channel::Unbounded.into())
                .await
                .expect("observe")
        };
        let webhooks: Arc<Mutex<HashMap<String, (WebhookEndpoint, Box<dyn Webhook>)>>> =
            self.webhooks.clone();
        let future = async move {
            while let Some(event) = events.next().await {
                match Self::post_webhooks(
                    "basicmessages",
                    &serde_json::to_value(&event).unwrap(),
                    webhooks.clone(),
                )
                .await
                {
                    Ok(_) => (),
                    Err(err) => println!("{:?}", err),
                }
            }
            println!("end async basicmessages future events");
        };
        let task = tokio::task::spawn(future);
        self.message_task = Some(Arc::new(task));
    }
}

/// # List registered webhooks
#[openapi(tag = "webhook")]
#[get("/webhooks")]
pub async fn get_all_webhooks(webhook_pool: &State<WebhookPool>) -> Json<Vec<WebhookEndpoint>> {
    let webhooks: Vec<WebhookEndpoint> = {
        let mut webhooks = Vec::new();
        let map = webhook_pool.webhooks.try_lock().unwrap();
        for value in (*map).values() {
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
        .post("webhook", &serde_json::to_value(&webhook_endpoint).unwrap())
        .await
        .unwrap();
    Json(webhook_endpoint)
}

#[cfg(test)]
mod tests {
    use crate::test_rocket;
    use crate::webhook::WebhookEndpoint;
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;

    #[tokio::test]
    async fn test_post_webhook() {
        let client = Client::tracked(test_rocket().await)
            .await
            .expect("valid rocket instance");

        let mut webhook = WebhookEndpoint::default();
        webhook.id = None;

        let response = client
            .post("/webhook")
            .header(ContentType::JSON)
            .json(&webhook)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<WebhookEndpoint>().await.unwrap();
        assert!(response.id.is_some());
    }
}
