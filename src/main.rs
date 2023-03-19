#[macro_use]
extern crate tracing;

use cloudflare_access_webhook_redirect::config::Config;
use cloudflare_access_webhook_redirect::Result;
use std::env;
use std::str::FromStr;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{filter, Layer};

const ENV_LOG_LEVEL: &str = "LOG_LEVEL";

const DEFAULT_LOG_LEVEL: &str = "info";

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing()?;

    let config = Config::get_configurations()?;

    println!("{:#?}", config);

    Ok(())
}

fn setup_tracing() -> Result<()> {
    let level = env::var(ENV_LOG_LEVEL).unwrap_or_else(|_| DEFAULT_LOG_LEVEL.to_string());
    let level = tracing::Level::from_str(&level)?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter::LevelFilter::from_level(level)))
        .init();

    Ok(())
}
