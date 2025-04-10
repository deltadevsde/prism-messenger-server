use config::{Config, ConfigError};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct WebserverSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrismSettings {
    pub host: String,
    pub port: u16,
    pub signing_key_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApnsSettings {
    pub team_id: String,
    pub key_id: String,
    pub bundle_id: String,
    pub private_key_path: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum DatabaseSettings {
    Sqlite { path: String },
}

// TODO: Defaults for these settings?
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub development: bool,
    pub webserver: WebserverSettings,
    pub prism: PrismSettings,
    pub apns: ApnsSettings,
    pub database: DatabaseSettings,
}

impl Settings {
    pub fn load() -> Result<Settings, ConfigError> {
        let settings = Config::builder()
            .add_source(config::File::with_name("settings"))
            .add_source(config::Environment::with_prefix("PRISM_MSG_"))
            .build()?;

        settings.try_deserialize()
    }
}
