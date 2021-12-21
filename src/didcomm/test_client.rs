use super::DidComm;
use http::response::Builder;
use reqwest::RequestBuilder;
use reqwest::{Response, ResponseBuilderExt};
use serde_json::Value;
use url::Url;

pub struct TestClient {
    response: Value,
}

impl TestClient {
    pub fn new() -> Self {
        TestClient {
            response: Value::default(),
        }
    }

    pub fn response(&mut self, response: Value) {
        self.response = response;
    }
}

#[async_trait]
impl DidComm for TestClient {
    fn request(&self, endpoint: &str, body: &Value) -> RequestBuilder {
        let client = reqwest::Client::new();
        client.post(format!("{}/", endpoint)).json(body)
    }
    async fn post(
        &self,
        endpoint: &str,
        _body: &Value,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = Url::parse(&format!("{}/", endpoint)).unwrap();
        let response = Builder::new()
            .status(200)
            .url(url.clone())
            .body(serde_json::to_string(&self.response).unwrap())
            .unwrap();
        let response = Response::from(response);
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_topic() {
        let response = json!({ "hello": "world!"});
        let mut client = TestClient::new();
        client.response(response);

        let post = client.request("http://example.com", &json!({}));
        assert_eq!(post.build().unwrap().url().as_str(), "http://example.com/");
    }
}
