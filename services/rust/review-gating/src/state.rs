use anyhow::Result;

use crate::{analyzer::AiClient, config};

pub struct AppState {
    analyzer: Box<dyn AiClient>,
}

impl AppState {
    pub async fn new(config: &config::Config, analyzer: Box<dyn AiClient>) -> Result<Self> {
        Ok(Self { analyzer })
    }
    pub fn analyzer(&self) -> &Box<dyn AiClient> {
        &self.analyzer
    }
}
