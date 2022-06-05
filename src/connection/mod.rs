use crate::config::Config;
use crate::didcomm::DidComm;
use crate::wallet::get_did_endpoint;
use crate::wallet::Wallet;
use identity::iota_core::IotaDID;
use rocket::http::Status;
use rocket::State;
use rocket::{delete, get, post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod invitation;
use invitation::{build_issue_vc_invitation, Invitation};

#[derive(Default, Debug, PartialEq, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Connection {
    pub id: String,
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

#[openapi(tag = "out-of-band")]
#[post("/out-of-band/create-invitation")]
pub async fn post_create_invitation(wallet: &State<Wallet>) -> Json<Invitation> {
    let lock = wallet.account.lock().await;
    let did: &IotaDID = lock.did();
    let endpoint = get_did_endpoint(did.to_string()).await.as_str().to_string();
    let invitation: Invitation = build_issue_vc_invitation(endpoint);
    Json(invitation)
}

#[openapi(tag = "out-of-band")]
#[post(
    "/out-of-band/receive-invitation",
    format = "application/json",
    data = "<invitation>"
)]
pub async fn post_receive_invitation(
    connections: &State<Connections>,
    invitation: Json<Invitation>,
) -> Json<Invitation> {
    let invitation: Invitation = invitation.into_inner();
    let id = invitation.id.to_string();
    let endpoint: String = invitation.attachments[0].data["json"]["service"]["serviceEndpoint"]
        .as_str()
        .unwrap()
        .to_string();

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
    let connection = Connection { id, endpoint };
    let mut lock = connections.connections.lock().await;
    lock.insert(connection.id.to_string(), connection);

    Json(invitation)
}

#[openapi(tag = "connection")]
#[get("/connections")]
pub async fn get_all_connections(connections: &State<Connections>) -> Json<Vec<Connection>> {
    let lock = connections.connections.lock().await;
    let connections = lock.values().cloned().collect();
    Json(connections)
}

#[openapi(tag = "connection")]
#[get("/connections/<conn_id>")]
pub async fn get_connection(connections: &State<Connections>, conn_id: String) -> Json<Connection> {
    let lock = connections.connections.lock().await;
    let connection = lock.get(&conn_id).unwrap().clone();
    Json(connection)
}

#[openapi(tag = "connection")]
#[get("/connections/<conn_id>/endpoints")]
pub async fn get_connection_endpoints(
    config: &State<Config>,
    connections: &State<Connections>,
    conn_id: String,
) -> Json<ConnectionEndpoints> {
    let lock = connections.connections.lock().await;
    let endpoint = config.endpoint.to_string();
    let connection = lock.get(&conn_id).unwrap().clone();
    let their_endpoint = connection.endpoint;
    Json(ConnectionEndpoints {
        my_endpoint: endpoint,
        their_endpoint,
    })
}

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
    use crate::test_rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;
    use serde_json::Value;

    #[test]
    fn test_connections() {
        let client = Client::tracked(test_rocket()).expect("valid rocket instance");
        let response = client.get("/connections").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 0);

        let response = client.post("/out-of-band/create-invitation").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let invitation: Value = response.into_json::<Value>().unwrap();
        let invitation: String = serde_json::to_string(&invitation).unwrap();

        let response = client
            .post("/out-of-band/receive-invitation")
            .header(ContentType::JSON)
            .body(invitation)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/connections").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 1);
    }

    #[test]
    fn test_termination() {
        let client = Client::tracked(test_rocket()).expect("valid rocket instance");
        let response = client.get("/connections").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 0);

        let response = client.post("/out-of-band/create-invitation").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let invitation: Value = response.into_json::<Value>().unwrap();
        let invitation: String = serde_json::to_string(&invitation).unwrap();

        let response = client
            .post("/out-of-band/receive-invitation")
            .header(ContentType::JSON)
            .body(invitation)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/connections").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 1);

        let connection: Connection = serde_json::from_value(connections[0].clone()).unwrap();
        let response = client
            .delete(format!("/connections/{}", connection.id))
            .dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/connections").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().unwrap();
        let connections = response.as_array().unwrap();
        assert_eq!(connections.len(), 0);
    }
}
