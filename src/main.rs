#[macro_use]
extern crate tracing;

use cloudflare_access_webhook_redirect::config::Config;
use cloudflare_access_webhook_redirect::server::{Server, WebHookData};
use cloudflare_access_webhook_redirect::Result;
use reqwest_middleware::ClientBuilder;
use reqwest_tracing::{SpanBackendWithUrl, TracingMiddleware};
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

    let server = Server::new(config.server().host().to_string(), config.server().port());

    let client = ClientBuilder::new(reqwest::Client::new())
        .with(TracingMiddleware::<SpanBackendWithUrl>::new())
        .build();

    let web_hook_data = WebHookData::new(
        client,
        config.webhook().target().clone(),
        vec!["api/webhook".to_string(), "test/a/.*".to_string()],
        config.cloudflare().client_id().clone(),
        config.cloudflare().client_secret().clone(),
    )?;
    server.run_until_stopped(web_hook_data).await?;

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
