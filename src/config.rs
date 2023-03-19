use crate::error::Error;
use reqwest::Url;
use secrecy::Secret;
use serde::{Deserialize, Deserializer};
use std::str::FromStr;

const DEFAULT_SERVER_HOST: &str = "127.0.0.1";
const DEFAULT_SERVER_PORT: u16 = 8080;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    server: ServerConfig,
    cloudflare: CloudFlareConfig,
    webhook: WebhookConfig,
}

#[derive(Debug, serde::Deserialize)]
pub struct CloudFlareConfig {
    client_id: Secret<String>,
    client_secret: Secret<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, serde::Deserialize)]
pub struct WebhookConfig {
    #[serde(deserialize_with = "from_string_to_url")]
    target: Url,
    secret: Secret<String>,
}

impl Config {
    pub fn get_configurations() -> crate::Result<Self> {
        config::Config::builder()
            .add_source(config::Environment::default())
            .set_default("server.host", DEFAULT_SERVER_HOST)?
            .set_default("server.port", DEFAULT_SERVER_PORT)?
            .build()
            .map_err(|e| Error::custom(format!("Can't parse config: {e}")))?
            .try_deserialize::<Config>()
            .map_err(|e| Error::custom(format!("Failed to deserialize configuration: {e}")))
    }

    pub fn server(&self) -> &ServerConfig {
        &self.server
    }

    pub fn cloudflare(&self) -> &CloudFlareConfig {
        &self.cloudflare
    }
}

impl CloudFlareConfig {
    pub fn client_id(&self) -> &Secret<String> {
        &self.client_id
    }

    pub fn client_secret(&self) -> &Secret<String> {
        &self.client_secret
    }
}

impl ServerConfig {
    pub fn host(&self) -> &String {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl WebhookConfig {
    pub fn target(&self) -> &Url {
        &self.target
    }

    pub fn secret(&self) -> &Secret<String> {
        &self.secret
    }
}

pub fn from_string_to_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    let string: String = Deserialize::deserialize(deserializer)?;
    Url::parse(&string).map_err(serde::de::Error::custom)
}
