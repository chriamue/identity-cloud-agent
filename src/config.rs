use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
pub struct Config {
    pub ext_hostname: String,
    pub stronghold_path: String,
    pub password: String,
    pub endpoint: String,
    pub webhook_url: String,
    pub key_seed: Option<String>,
    pub did: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::test_rocket;

    #[test]
    fn test_config() {
        let rocket = test_rocket().await;
        let figment = rocket.figment();
        let config: Config = figment.extract().expect("config");
        assert_ne!(config.stronghold_path, "".to_string());
    }
}