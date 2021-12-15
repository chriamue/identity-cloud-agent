use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
pub struct Config {
    pub ext_hostname: String,
    pub stronghold_path: String,
    pub password: String,
    pub endpoint: String,
}
