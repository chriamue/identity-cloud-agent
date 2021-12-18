use serde_json::Value;

pub struct Webhook {
    pub url: String,
}

impl Webhook {
    pub fn new(url: String) -> Self {
        Webhook { url }
    }

    pub async fn send(&self, topic: &str, payload: Value) {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/topic{}", self.url, topic))
            .json(&payload)
            .send()
            .await
            .unwrap();
    }
}
