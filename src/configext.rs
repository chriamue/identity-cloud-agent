use serde::Deserialize;
#[derive(Default, PartialEq, Deserialize)]
pub struct ConfigExt {
    pub webhook_url: Option<String>,
}
