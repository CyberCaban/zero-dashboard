use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use anyhow::Result;

use crate::{analyzer::groq_client_gpt::GroqClientGPT, http_client::RetryableHttpClient};

mod analyzer;
mod config;
mod consts;
mod http_client;
mod logger;
mod models;
mod nats;
mod retry;
mod state;

static HTTP_CLIENT: OnceLock<RetryableHttpClient> = OnceLock::new();

fn http_client() -> &'static RetryableHttpClient {
    HTTP_CLIENT.get_or_init(|| {
        let config = config::Config::from_env();
        let http_client_config = http_client::config::HttpClientConfig::default()
            .with_bearer_token(config.groq_api_key.clone());
        http_client_config
            .build_retryable_client()
            .expect("Failed to build HTTP client")
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    logger::init_logger();

    let config = config::Config::from_env();

    let analyzer = Box::new(GroqClientGPT::new(
        http_client().clone(),
        config.groq_api_key.clone(),
        config.model.clone(),
    ));
    let state = Arc::new(state::AppState::new(&config, analyzer).await?);
    let nats_server = nats::NatsServer::new(&config, state).await?;

    nats_server.handle_inbound_reviews().await?;

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for shutdown signal");
    tracing::info!("Shutting down gracefully...");
    Ok(())
}
