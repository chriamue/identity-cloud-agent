use crate::connection::ConnectionEvents;
use crate::credential::IssueCredentialEvents;
use crate::message::MessageEvents;
use crate::ping::PingEvents;
use async_trait::async_trait;
use reqwest::RequestBuilder;
use rocket::http::Status;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::openapi;
use serde_json::Value;
use std::any::Any;
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

pub type WebhookHashMap =
    Arc<Mutex<HashMap<String, (WebhookEndpoint, Arc<Mutex<Box<dyn Webhook>>>)>>>;

#[async_trait]
pub trait Webhook: core::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn request(&self, topic: &str, body: &Value) -> RequestBuilder;
    async fn post(
        &mut self,
        topic: &str,
        body: &Value,
    ) -> Result<reqwest::Response, reqwest::Error>;
}

#[derive(Debug, Clone)]
pub struct WebhookPool {
    pub webhooks: WebhookHashMap,
    pub connection_task: Option<Arc<JoinHandle<()>>>,
    pub issue_credential_task: Option<Arc<JoinHandle<()>>>,
    pub ping_task: Option<Arc<JoinHandle<()>>>,
    pub message_task: Option<Arc<JoinHandle<()>>>,
}

impl Default for WebhookPool {
    fn default() -> Self {
        WebhookPool {
            webhooks: Arc::new(Mutex::new(HashMap::new())),
            connection_task: None,
            issue_credential_task: None,
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
        webhooks: WebhookHashMap,
    ) -> Result<(), reqwest::Error> {
        let map = webhooks.try_lock().unwrap();
        for (key, value) in &*map {
            let (_webhook_endpoint, webhook) = value;
            match webhook.try_lock().unwrap().post(topic, body).await {
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
        let webhooks: WebhookHashMap = self.webhooks.clone();
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

    pub async fn spawn_issue_credential_events(
        &mut self,
        issue_credential_events: Arc<Mutex<IssueCredentialEvents>>,
    ) {
        let mut events = {
            let mut issue_credential_events = issue_credential_events.try_lock().unwrap();
            issue_credential_events
                .observe(Channel::Bounded(20).into())
                .await
                .expect("observe")
        };
        let webhooks: WebhookHashMap = self.webhooks.clone();
        let future = async move {
            while let Some(event) = events.next().await {
                match Self::post_webhooks(
                    "issue_credential_v2_0",
                    &serde_json::to_value(&event).unwrap(),
                    webhooks.clone(),
                )
                .await
                {
                    Ok(_) => (),
                    Err(err) => println!("{:?}", err),
                }
            }
            println!("end async issue credential future events");
        };
        let task = tokio::task::spawn(future);
        self.issue_credential_task = Some(Arc::new(task));
    }

    pub async fn spawn_ping_events(&mut self, ping_events: Arc<Mutex<PingEvents>>) {
        let mut events = {
            let mut ping_events = ping_events.try_lock().unwrap();
            ping_events
                .observe(Channel::Unbounded.into())
                .await
                .expect("observe")
        };
        let webhooks: WebhookHashMap = self.webhooks.clone();
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
        let webhooks: WebhookHashMap = self.webhooks.clone();
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
        (
            webhook_endpoint.clone(),
            Arc::new(Mutex::new(webhook_client)),
        ),
    );
    webhook_pool
        .post("webhook", &serde_json::to_value(&webhook_endpoint).unwrap())
        .await
        .unwrap();
    Json(webhook_endpoint)
}

/// # Delete a registered webhook
#[openapi(tag = "webhook")]
#[delete("/webhook/<webhook_id>")]
pub async fn delete_webhook(webhook_pool: &State<WebhookPool>, webhook_id: String) -> Status {
    webhook_pool
        .webhooks
        .try_lock()
        .unwrap()
        .remove(&webhook_id)
        .unwrap();
    Status::Ok
}

#[cfg(test)]
mod tests {
    use crate::test_rocket;
    use crate::webhook::WebhookEndpoint;
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;

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

    #[tokio::test]
    async fn test_delete_webhook() {
        let client = Client::tracked(test_rocket().await)
            .await
            .expect("valid rocket instance");

        let mut webhook = WebhookEndpoint::default();
        webhook.id = None;

        let response = client.get("/webhooks").json(&webhook).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        assert_eq!(response.as_array().unwrap().len(), 0);

        let response = client
            .post("/webhook")
            .header(ContentType::JSON)
            .json(&WebhookEndpoint::default())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/webhooks").json(&webhook).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        assert_eq!(response.as_array().unwrap().len(), 1);
        let webhook_id = response[0].get("id").unwrap().as_str().unwrap();

        let response = client
            .delete(format!("/webhook/{}", webhook_id))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/webhooks").json(&webhook).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        assert_eq!(response.as_array().unwrap().len(), 0);
    }
}
