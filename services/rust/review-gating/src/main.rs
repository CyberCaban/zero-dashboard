use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use anyhow::Result;

use crate::analyzer::groq_client_gpt::GroqClientGPT;

mod analyzer;
mod config;
mod consts;
mod logger;
mod models;
mod nats;
mod state;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    use reqwest::{StatusCode, retry};
    HTTP_CLIENT.get_or_init(|| {
        let retry_policy = retry::for_host("api.groq.com")
            .max_retries_per_request(3)
            .max_extra_load(0.2)
            .classify_fn(|rep_req| match rep_req.status() {
                Some(status) if status.is_server_error() => rep_req.retryable(),
                Some(StatusCode::TOO_MANY_REQUESTS) => rep_req.retryable(),
                _ if rep_req.error().is_some() => rep_req.retryable(),
                _ => rep_req.success(),
            });
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(5))
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .tcp_keepalive(Duration::from_secs(60))
            .retry(retry_policy)
            .build()
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
