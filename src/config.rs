use reqwest::Url;
use secrecy::Secret;
use serde::{Deserialize, Deserializer};

use crate::error::Error;

const DEFAULT_SERVER_HOST: &str = "127.0.0.1";
const DEFAULT_SERVER_PORT: u16 = 8080;

#[derive(Debug, serde::Deserialize, Getters)]
#[getset(get = "pub")]
pub struct Config {
    server: ServerConfig,
    cloudflare: CloudFlareConfig,
    webhook: WebhookConfig,
}

#[derive(Debug, serde::Deserialize, Getters)]
#[getset(get = "pub")]
pub struct CloudFlareConfig {
    client_id: Secret<String>,
    client_secret: Secret<String>,
}

#[derive(Debug, serde::Deserialize, Getters)]
#[getset(get = "pub")]
pub struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, serde::Deserialize, Getters)]
#[getset(get = "pub")]
pub struct WebhookConfig {
    #[serde(deserialize_with = "deserialize_url_from_string")]
    target_base: Url,
    paths: Vec<String>,
}

impl Config {
    pub fn get_configurations() -> crate::Result<Self> {
        config::Config::builder()
            .add_source(
                config::Environment::default()
                    .list_separator(", ")
                    .with_list_parse_key("webhook.paths")
                    .try_parsing(true),
            )
            .set_default("server.host", DEFAULT_SERVER_HOST)?
            .set_default("server.port", DEFAULT_SERVER_PORT)?
            .build()
            .map_err(|e| Error::custom(format!("Can't parse config: {e}")))?
            .try_deserialize::<Config>()
            .map_err(|e| Error::custom(format!("Failed to deserialize configuration: {e}")))
    }
}

pub fn deserialize_url_from_string<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    let string: String = Deserialize::deserialize(deserializer)?;
    Url::parse(&string).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use secrecy::ExposeSecret;

    const ENV_SERVER_HOST: &str = "SERVER.HOST";
    const ENV_SERVER_PORT: &str = "SERVER.PORT";

    const ENV_CLOUDFLARE_CLIENT_ID: &str = "CLOUDFLARE.CLIENT_ID";
    const ENV_CLOUDFLARE_CLIENT_SECRET: &str = "CLOUDFLARE.CLIENT_SECRET";

    const ENV_WEBHOOK_TARGET_BASE: &str = "WEBHOOK.TARGET_BASE";
    const ENV_WEBHOOK_PATHS: &str = "WEBHOOK.PATHS";

    const CORRECT_SERVER_HOST: &str = "0.0.0.0";
    const CORRECT_SERVER_PORT: &str = "8080";

    const CORRECT_CLOUDFLARE_CLIENT_ID: &str = "client_id";
    const CORRECT_CLOUDFLARE_CLIENT_SECRET: &str = "client_secret";

    const CORRECT_WEBHOOK_TARGET_BASE: &str = "https://example.com/";
    const CORRECT_WEBHOOK_PATHS: &str = "/test";

    #[test]
    fn test_get_configurations_minimal_correct() {
        temp_env::with_vars(
            vec![
                (ENV_CLOUDFLARE_CLIENT_ID, Some(CORRECT_CLOUDFLARE_CLIENT_ID)),
                (
                    ENV_CLOUDFLARE_CLIENT_SECRET,
                    Some(CORRECT_CLOUDFLARE_CLIENT_SECRET),
                ),
                (ENV_WEBHOOK_TARGET_BASE, Some(CORRECT_WEBHOOK_TARGET_BASE)),
                (ENV_WEBHOOK_PATHS, Some(CORRECT_WEBHOOK_PATHS)),
            ],
            || {
                let result = Config::get_configurations();
                assert!(result.is_ok());

                let config = result.unwrap();

                assert_eq!(
                    config.cloudflare().client_id().expose_secret(),
                    CORRECT_CLOUDFLARE_CLIENT_ID
                );
                assert_eq!(
                    config.cloudflare().client_secret().expose_secret(),
                    CORRECT_CLOUDFLARE_CLIENT_SECRET
                );

                assert_eq!(
                    config.webhook().target_base().as_str(),
                    CORRECT_WEBHOOK_TARGET_BASE
                );
                assert_eq!(config.webhook().paths(), &vec![CORRECT_WEBHOOK_PATHS]);
            },
        );
    }

    #[test]
    fn test_get_configurations_full_correct() {
        temp_env::with_vars(
            vec![
                (ENV_SERVER_HOST, Some(CORRECT_SERVER_HOST)),
                (ENV_SERVER_PORT, Some(CORRECT_SERVER_PORT)),
                (ENV_CLOUDFLARE_CLIENT_ID, Some(CORRECT_CLOUDFLARE_CLIENT_ID)),
                (
                    ENV_CLOUDFLARE_CLIENT_SECRET,
                    Some(CORRECT_CLOUDFLARE_CLIENT_SECRET),
                ),
                (ENV_WEBHOOK_TARGET_BASE, Some(CORRECT_WEBHOOK_TARGET_BASE)),
                (ENV_WEBHOOK_PATHS, Some(r"/test, /test2, /test\d*")),
            ],
            || {
                let result = Config::get_configurations();
                assert!(result.is_ok());

                let config = result.unwrap();

                assert_eq!(config.server().host(), CORRECT_SERVER_HOST);
                assert_eq!(config.server().port(), &8080u16);

                assert_eq!(
                    config.cloudflare().client_id().expose_secret(),
                    CORRECT_CLOUDFLARE_CLIENT_ID
                );
                assert_eq!(
                    config.cloudflare().client_secret().expose_secret(),
                    CORRECT_CLOUDFLARE_CLIENT_SECRET
                );

                assert_eq!(
                    config.webhook().target_base().as_str(),
                    CORRECT_WEBHOOK_TARGET_BASE
                );
                assert_eq!(
                    config.webhook().paths(),
                    &vec!["/test", "/test2", r"/test\d*"]
                );
            },
        );
    }
}
