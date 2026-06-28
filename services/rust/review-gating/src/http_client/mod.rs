use std::sync::Arc;

use anyhow::{Result, bail};
use reqwest::Response;

use crate::{
    analyzer::circuit_breaker::CircuitBreaker,
    retry::{self, backoff::ExponentialBackoff},
};

pub mod config;

#[derive(Clone)]
pub struct RetryableHttpClient {
    client: reqwest::Client,
    backoff: ExponentialBackoff,
    circuit_breaker: Arc<CircuitBreaker>,
    bearer_token: Option<String>,
}

impl RetryableHttpClient {
    pub fn new(
        client: reqwest::Client,
        backoff: ExponentialBackoff,
        circuit_breaker: Arc<CircuitBreaker>,
    ) -> Self {
        Self {
            client,
            backoff,
            circuit_breaker,
            bearer_token: None,
        }
    }
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }
    pub async fn execute_with_retry(
        &self,
        request_builder: reqwest::RequestBuilder,
        operation_name: &str,
    ) -> Result<Response> {
        if !self.circuit_breaker.is_request_allowed().await {
            bail!("Circuit breaker is OPEN for operation: {}", operation_name);
        }
        let backoff = self.backoff.clone();
        let bearer_token = self.bearer_token.clone();

        let result = retry::retry_with_backoff(
            || {
                let mut builder = request_builder
                    .try_clone()
                    .expect("Failed to clone request builder");
                if let Some(token) = &bearer_token {
                    builder = builder.bearer_auth(token);
                }
                async move {
                    let response = builder.send().await?;
                    if response.status().is_success() {
                        Ok(response)
                    } else {
                        Err(anyhow::anyhow!(
                            "HTTP request failed with status {}",
                            response.status()
                        ))
                    }
                }
            },
            backoff,
            operation_name,
        )
        .await;

        match result {
            Ok(value) => {
                self.circuit_breaker.record_success().await;
                Ok(value)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                bail!(e.to_string())
            }
        }
    }
    pub async fn post_json<T: serde::Serialize>(
        &self,
        url: &str,
        json_body: &T,
        operation_name: &str,
    ) -> Result<Response> {
        let request_builder = self.client.post(url).json(json_body);
        self.execute_with_retry(request_builder, operation_name)
            .await
    }
}
