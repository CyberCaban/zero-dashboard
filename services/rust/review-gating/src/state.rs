use anyhow::Result;

use crate::config;

pub struct AppState {
    analyzer: crate::analyzer::Analyzer,
}

impl AppState {
    pub async fn new(config: &config::Config) -> Result<Self> {
        let analyzer =
            crate::analyzer::Analyzer::new(config.groq_api_key.clone(), config.model.clone());
        Ok(Self { analyzer })
    }
    pub fn analyzer(&self) -> &crate::analyzer::Analyzer {
        &self.analyzer
    }
}
