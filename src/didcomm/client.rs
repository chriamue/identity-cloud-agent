use super::DidComm;
use reqwest::RequestBuilder;
use serde_json::Value;

pub struct Client {}

impl Client {
    pub fn new() -> Self {
        Client {}
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DidComm for Client {
    fn request(&self, endpoint: &str, body: &Value) -> RequestBuilder {
        let client = reqwest::Client::new();
        client.post(format!("{}/", endpoint)).json(body)
    }

    async fn post(
        &self,
        endpoint: &str,
        body: &Value,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.request(endpoint, body).send().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_topic() {
        let client = Client::new();
        let post = client.request("http://example.com", &json!({}));
        assert_eq!(post.build().unwrap().url().as_str(), "http://example.com/");
    }
}
