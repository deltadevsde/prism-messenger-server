use std::path::Path;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use prism_telemetry::config::TelemetryConfig;

#[derive(Debug, Clone, Deserialize)]
pub struct WebserverSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrismSettings {
    pub host: String,
    pub port: u16,
    #[serde(rename = "signing_key")]
    pub signing_key_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApnsSettings {
    pub team_id: String,
    pub key_id: String,
    pub bundle_id: String,
    #[serde(rename = "private_key")]
    pub private_key_path: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum CoreDatabaseSettings {
    InMemory,
    Sqlite { path: String },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum EphemeralDatabaseSettings {
    InMemory,
    Redis { host: String, port: u16 },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum AssetsDatabaseSettings {
    InMemory,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    pub core: CoreDatabaseSettings,
    pub ephemeral: EphemeralDatabaseSettings,
    pub assets: AssetsDatabaseSettings,
}

// TODO: Defaults for these settings?
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub development: bool,
    pub webserver: WebserverSettings,
    pub prism: PrismSettings,
    pub apns: ApnsSettings,
    pub database: DatabaseSettings,
    pub telemetry: Option<TelemetryConfig>,
}

impl Settings {
    pub fn load() -> Result<Settings, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name("settings"))
            .add_source(Environment::with_prefix("PRISM_MSG").separator("__"))
            .build()?;

        settings.try_deserialize()
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Settings, ConfigError> {
        let settings = Config::builder()
            .add_source(File::from(path.as_ref()))
            .add_source(Environment::with_prefix("PRISM_MSG").separator("__"))
            .build()?;

        settings.try_deserialize()
    }
}
