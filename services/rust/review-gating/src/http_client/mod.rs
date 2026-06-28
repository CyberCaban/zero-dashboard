use std::sync::Arc;

use anyhow::{Result, bail};
use reqwest::Response;

use crate::{
    http_client::circuit_breaker::CircuitBreaker,
    retry::{self, backoff::ExponentialBackoff},
};

pub mod circuit_breaker;
pub mod config;

#[derive(Debug, thiserror::Error)]
pub enum HttpClientError {
    #[error("Circuit breaker is open for operation {0}")]
    CircuitOpen(String),

    #[error("Timeout exceeded")]
    Timeout,

    #[error("Failed to clone request builder")]
    CloneRequestBuilderError,

    #[error("Retry attempts exhausted")]
    RetryExhausted,

    #[error("HTTP request failed with status: {0}")]
    ResponseError(u16),

    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),
}

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
            bail!(HttpClientError::CircuitOpen(operation_name.to_owned()));
        }
        let backoff = self.backoff.clone();
        let bearer_token = self.bearer_token.clone();

        let result = retry::retry_with_backoff(
            async || {
                let mut builder = request_builder
                    .try_clone()
                    .ok_or_else(|| HttpClientError::CloneRequestBuilderError)?;
                if let Some(token) = &bearer_token {
                    builder = builder.bearer_auth(token);
                }
                let response = match builder.send().await {
                    Ok(resp) => resp,
                    Err(e) => {
                        if e.is_timeout() {
                            return Err(HttpClientError::Timeout);
                        } else {
                            return Err(HttpClientError::RequestError(e));
                        }
                    }
                };
                if response.status().is_success() {
                    Ok(response)
                } else {
                    Err(HttpClientError::ResponseError(response.status().as_u16()))
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
            Err(_) => {
                self.circuit_breaker.record_failure().await;
                bail!(HttpClientError::RetryExhausted)
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
