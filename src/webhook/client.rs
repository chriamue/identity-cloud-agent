use super::Webhook;
use reqwest::RequestBuilder;
use serde_json::Value;

pub struct Client {
    url: String,
}

impl Client {
    pub fn new(url: String) -> Self {
        Client { url }
    }
}

#[async_trait]
impl Webhook for Client {
    fn request(&self, topic: &str, body: &Value) -> RequestBuilder {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/topic/{}", self.url, topic))
            .json(body)
    }

    async fn post(&self, topic: &str, body: &Value) -> Result<reqwest::Response, reqwest::Error> {
        self.request(topic, body).send().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_topic() {
        let client = Client::new("http://example.com".to_string());
        let post = client.request("foo", &json!({}));
        assert_eq!(
            post.build().unwrap().url().as_str(),
            "http://example.com/topic/foo"
        );
    }
}