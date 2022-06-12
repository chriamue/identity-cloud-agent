use serde::Deserialize;
#[derive(PartialEq, Deserialize)]
pub struct ConfigExt {
    pub webhook_url: Option<String>,
}

impl Default for ConfigExt {
    fn default() -> Self {
        ConfigExt { webhook_url: None }
    }
}
