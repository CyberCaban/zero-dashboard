use std::{sync::Arc, time::Duration};

use anyhow::Result;
use reqwest::{StatusCode, retry};

use crate::{
    http_client::{RetryableHttpClient, circuit_breaker::CircuitBreaker},
    retry::backoff::ExponentialBackoff,
};

pub struct HttpClientConfig {
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub pool_idle_timeout: Duration,
    pub pool_max_idle_per_host: usize,
    pub tcp_keepalive: Duration,
    pub backoff: ExponentialBackoff,
    pub circuit_breaker: CircuitBreaker,
    pub bearer_token: Option<String>,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(5),
            pool_idle_timeout: Duration::from_secs(30),
            pool_max_idle_per_host: 10,
            tcp_keepalive: Duration::from_secs(60),
            backoff: ExponentialBackoff::default(),
            circuit_breaker: CircuitBreaker::default(),
            bearer_token: None,
        }
    }
}

impl HttpClientConfig {
    pub fn build_client(&self) -> Result<reqwest::Client> {
        let retry_policy = retry::for_host("api.groq.com")
            .max_retries_per_request(3)
            .max_extra_load(0.2)
            .classify_fn(|rep_req| match rep_req.status() {
                Some(status) if status.is_server_error() => rep_req.retryable(),
                Some(StatusCode::TOO_MANY_REQUESTS) => rep_req.retryable(),
                _ if rep_req.error().is_some() => rep_req.retryable(),
                _ => rep_req.success(),
            });
        Ok(reqwest::Client::builder()
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout)
            .pool_idle_timeout(self.pool_idle_timeout)
            .pool_max_idle_per_host(self.pool_max_idle_per_host)
            .tcp_keepalive(self.tcp_keepalive)
            .retry(retry_policy)
            .build()?)
    }

    pub fn build_retryable_client(self) -> Result<RetryableHttpClient> {
        let inner = self.build_client()?;
        let mut client =
            RetryableHttpClient::new(inner, self.backoff.clone(), Arc::new(self.circuit_breaker));

        if let Some(token) = &self.bearer_token {
            client = client.with_bearer_token(token.clone());
        }

        Ok(client)
    }

    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }
}
