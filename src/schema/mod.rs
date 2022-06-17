use identity_iota::core::FromJson;
use identity_iota::credential::Schema;
use rocket::State;
use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct Schemas {
    pub schemas: Arc<Mutex<HashMap<String, Schema>>>,
}

fn example_schema() -> &'static str {
    include_str!("../assets/degree_schema.json")
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(example = "example_schema")]
pub struct SchemaRequest(Value);

#[openapi(tag = "schema")]
#[post("/schemas", data = "<schema_request>")]
pub async fn post_schemas(
    schemas: &State<Schemas>,
    schema_request: Json<SchemaRequest>,
) -> Json<SchemaRequest> {
    let schema = schema_request.clone();
    let schema = serde_json::to_string(&schema.into_inner()).unwrap();
    let schema: Schema = Schema::from_json(&schema).unwrap();
    let mut schemas = schemas.schemas.lock().await;
    schemas.insert(schema.id.to_string(), schema);
    schema_request
}

#[openapi(tag = "schema")]
#[get("/schemas")]
pub async fn get_all_schemas(schemas: &State<Schemas>) -> Json<Value> {
    let lock = schemas.schemas.lock().await;
    let schemas: Vec<Schema> = lock.values().cloned().collect();
    let schemas = serde_json::to_value(schemas).unwrap();
    Json(schemas)
}

#[cfg(test)]
mod tests {
    use crate::test_rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;
    use serde_json::Value;

    #[test]
    fn test_schema() {
        let client = Client::tracked(test_rocket()).expect("valid rocket instance");

        let schema = include_str!("../assets/degree_schema.json");

        let response = client
            .post("/schemas")
            .header(ContentType::JSON)
            .body(schema)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/schemas").dispatch();
        assert_eq!(response.status(), Status::Ok);
        let response = response.into_json::<Value>().unwrap();
        let schemas = response.as_array().unwrap();
        assert_eq!(schemas.len(), 1);
    }
}
