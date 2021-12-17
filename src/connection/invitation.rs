use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Attachment {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "mime-type")]
    pub mime_type: String,
    pub data: Value,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Invitation {
    pub typ: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    pub body: Value,
    pub attachments: Vec<Attachment>,
}

pub fn build_issue_vc_invitation(endpoint: String) -> Invitation {
    let body: Value = json!({
        "goal_code": "issue-vc",
        "goal": "To issue a credential",
        "accept": [
            "didcomm/v2"
        ]
    });

    let json: Value = json!({
        "service":  {
            "serviceEndpoint": endpoint
        }
    });

    let attachment: Attachment = serde_json::from_value(json!(
        {
            "@id": "request-0",
            "mime-type": "application/json",
            "data": {
                "json": json
            }
        }
    ))
    .unwrap();

    let invitation: Invitation = Invitation {
        typ: "application/didcomm-plain+json".to_string(),
        type_: "https://didcomm.org/out-of-band/2.0/invitation".to_string(),
        id: Uuid::new_v4().to_string(),
        body: body,
        attachments: vec![attachment],
    };
    invitation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_issue_vc_invitation() {
        let endpoint = "https://example.com";
        let invitation = build_issue_vc_invitation(endpoint.to_string());
        assert_eq!(invitation.body["goal_code"].as_str().unwrap(), "issue-vc");
        assert_eq!(
            invitation.attachments[0].data["json"]["service"]["serviceEndpoint"]
                .as_str()
                .unwrap(),
            endpoint
        )
    }
}
