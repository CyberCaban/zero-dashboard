use std::sync::Arc;

use anyhow::Result;

mod analyzer;
mod config;
mod consts;
mod logger;
mod models;
mod nats;
mod state;

#[tokio::main]
async fn main() -> Result<()> {
    logger::init_logger();

    let config = config::Config::from_env();

    let state = Arc::new(state::AppState::new(&config).await?);
    let nats_server = nats::NatsServer::new(&config, state).await?;

    nats_server.handle_inbound_reviews().await?;

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for shutdown signal");
    tracing::info!("Shutting down gracefully...");
    Ok(())
}
