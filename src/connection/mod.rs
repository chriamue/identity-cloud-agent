use crate::didcomm::DidComm;
use crate::wallet::get_did_endpoint;
use crate::Config;
use crate::Wallet;
use didcomm_mediator::protocols::didexchange::DidExchangeResponseBuilder;
use didcomm_mediator::protocols::invitation::InvitationBuilder;
use didcomm_mediator::service::Service;
use didcomm_rs::Message;
use identity_iota::client::ExplorerUrl;
use identity_iota::iota_core::IotaDID;
use rocket::http::Status;
use rocket::State;
use rocket::{delete, get, post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod events;
pub mod invitation;

pub use events::ConnectionEvent;
pub use events::ConnectionEvents;

#[derive(Default, Debug, PartialEq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Connection {
    pub id: String,
    pub did: String,
    pub endpoint: String,
}

#[derive(Default)]
pub struct Connections {
    pub connections: Arc<Mutex<HashMap<String, Connection>>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ConnectionEndpoints {
    pub my_endpoint: String,
    pub their_endpoint: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Termination {
    pub typ: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    pub body: Value,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TerminationResponse {
    pub typ: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    pub body: Value,
}

/// # Create a new connection invitation.
#[openapi(tag = "out-of-band")]
#[post("/out-of-band/create-invitation")]
pub async fn post_create_invitation(wallet: &State<Arc<Mutex<Wallet>>>) -> Json<Value> {
    let wallet = wallet.try_lock().unwrap();
    let did: IotaDID = IotaDID::from_str(&wallet.did_iota().unwrap()).unwrap();
    let endpoint = get_did_endpoint(did.to_string()).await.as_str().to_string();

    let explorer: &ExplorerUrl = ExplorerUrl::mainnet();
    let did_doc = explorer.resolver_url(&did).unwrap();

    let did_exchange = DidExchangeResponseBuilder::new()
        .did_doc(serde_json::to_value(&did_doc).unwrap())
        .did(did.to_string())
        .build_request()
        .unwrap();

    let services: Vec<Service> = vec![Service::new(did.to_string(), endpoint).await.unwrap()];
    let invitation = InvitationBuilder::new()
        .goal("to create a relationship".to_string())
        .goal_code("aries.rel.build".to_string())
        .services(services)
        .attachments(vec![did_exchange])
        .build()
        .unwrap();

    let response = serde_json::from_str(&invitation.as_raw_json().unwrap()).unwrap();
    Json(response)
}

/// # Receive a new connection invitation.
#[openapi(tag = "out-of-band")]
#[post(
    "/out-of-band/receive-invitation",
    format = "application/json",
    data = "<invitation>"
)]
pub async fn post_receive_invitation(
    connections: &State<Connections>,
    connection_events: &State<Arc<Mutex<ConnectionEvents>>>,
    invitation: Json<Value>,
) -> Json<Value> {
    let invitation = invitation.into_inner();
    let message: Message = serde_json::from_value(invitation.clone()).unwrap();
    let id = message.get_didcomm_header().id.to_string();
    let (_, services) = message
        .get_application_params()
        .find(|(key, _)| *key == "services")
        .unwrap();
    let services: Vec<Service> = serde_json::from_str(services).unwrap();
    let services = services
        .iter()
        .filter(|service| service.id.starts_with("did:iota"))
        .cloned()
        .collect::<Vec<Service>>();

    let endpoint: String = services.first().unwrap().service_endpoint.to_string();
    let did: String = services.first().unwrap().id.replace("#didcomm", "");

    let client = reqwest::Client::new();
    match client
        .post(endpoint.to_string())
        .json(&invitation)
        .send()
        .await
    {
        Ok(_) => (),
        Err(err) => error!("{:?}", err),
    };
    let connection = Connection { id, endpoint, did };
    let connection_id = connection.id.to_string();
    let mut lock = connections.connections.lock().await;
    lock.insert(connection_id.to_string(), connection);
    connection_events
        .try_lock()
        .unwrap()
        .send(ConnectionEvent::Created(connection_id))
        .await;
    Json(invitation)
}

/// # Query agent-to-agent connections
#[openapi(tag = "connection")]
#[get("/connections")]
pub async fn get_all_connections(connections: &State<Connections>) -> Json<Vec<Connection>> {
    let lock = connections.connections.lock().await;
    let connections = lock.values().cloned().collect();
    Json(connections)
}

/// # Fetch a single connection record
#[openapi(tag = "connection")]
#[get("/connections/<conn_id>")]
pub async fn get_connection(connections: &State<Connections>, conn_id: String) -> Json<Connection> {
    let lock = connections.connections.lock().await;
    let connection = lock.get(&conn_id).unwrap().clone();
    Json(connection)
}

/// # Fetch connection remote endpoint
#[openapi(tag = "connection")]
#[get("/connections/<conn_id>/endpoints")]
pub async fn get_connection_endpoints(
    config: &State<Config>,
    connections: &State<Connections>,
    conn_id: String,
) -> Json<ConnectionEndpoints> {
    let lock = connections.connections.lock().await;
    let endpoint = config.ext_service.to_string();
    let connection = lock.get(&conn_id).unwrap().clone();
    let their_endpoint = connection.endpoint;
    Json(ConnectionEndpoints {
        my_endpoint: endpoint,
        their_endpoint,
    })
}

/// # Remove an existing connection record
#[openapi(tag = "connection")]
#[delete("/connections/<conn_id>")]
pub async fn delete_connection(
    didcomm: &State<Box<dyn DidComm>>,
    connections: &State<Connections>,
    conn_id: String,
) -> Status {
    let lock = connections.connections.lock().await;
    let connection = lock.get(&conn_id).unwrap().clone();
    std::mem::drop(lock);
    let endpoint = connection.endpoint.to_string();
    let termination: Termination = Termination {
        typ: "application/didcomm-plain+json".to_string(),
        type_: "iota/termination/0.1/termination".to_string(),
        id: connection.id.clone(),
        body: Value::default(),
    };
    match didcomm.post(&endpoint, &json!(termination)).await {
        Ok(_) => (),
        Err(err) => error!("{:?}", err),
    };
    let mut lock = connections.connections.lock().await;
    lock.remove(&conn_id).unwrap();
    Status::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webhook;
    use crate::{test_rocket, test_rocket_with_webhook_client};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;

    #[tokio::test]
    async fn test_connections() {
        let webhook_client = Box::new(webhook::test_client::TestClient::new(Default::default()))
            as Box<dyn webhook::Webhook>;
        let webhook_client = Arc::new(Mutex::new(webhook_client));
        let client = Client::tracked(test_rocket_with_webhook_client(webhook_client.clone()).await)
            .await
            .expect("valid rocket instance");
        let response = client.get("/connections").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 0);

        let response = client
            .post("/out-of-band/create-invitation")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let invitation: Value = response.into_json::<Value>().await.unwrap();
        let invitation: String = serde_json::to_string(&invitation).unwrap();

        let response = client
            .post("/out-of-band/receive-invitation")
            .header(ContentType::JSON)
            .body(invitation)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/connections").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 1);
        assert_eq!(webhook::test_client::last_response(&webhook_client).unwrap(), Value::default());
    }

    #[tokio::test]
    async fn test_termination() {
        let client = Client::tracked(test_rocket().await)
            .await
            .expect("valid rocket instance");
        let response = client.get("/connections").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 0);

        let response = client
            .post("/out-of-band/create-invitation")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let invitation: Value = response.into_json::<Value>().await.unwrap();
        let invitation: String = serde_json::to_string(&invitation).unwrap();

        let response = client
            .post("/out-of-band/receive-invitation")
            .header(ContentType::JSON)
            .body(invitation)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/connections").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 1);

        let connection: Connection = serde_json::from_value(connections[0].clone()).unwrap();
        let response = client
            .delete(format!("/connections/{}", connection.id))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/connections").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().await.unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 0);
    }
}
