use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct Attachment {
    pub typ: String,
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "mime-type")]
    pub mime_type: String,
    pub data: Value,
}

#[derive(Serialize, Deserialize)]
pub struct Invitation {
    pub typ: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    pub body: Value,
    pub attachments: Vec<Attachment>,
}
