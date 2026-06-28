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

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate()).expect("register SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}

#[cfg(windows)]
async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("listen Ctrl+C");
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

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());
    let consumer_handle = nats_server.handle_inbound_reviews(shutdown_rx).await?;

    shutdown_signal().await;
    tracing::info!("Shutdown signal received, stopping consumer...");

    let _ = shutdown_tx.send(());
    if tokio::time::timeout(Duration::from_secs(30), consumer_handle)
        .await
        .is_err()
    {
        tracing::warn!("Consumer did not stop in 30s, forcing shutdown");
    }

    tracing::info!("Shutdown complete");
    Ok(())
}
