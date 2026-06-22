use anyhow::Result;
use reqwest::Response;

use crate::retry::{self, backoff::ExponentialBackoff};

pub mod config;

#[derive(Clone)]
pub struct RetryableHttpClient {
    client: reqwest::Client,
    backoff: ExponentialBackoff,
    bearer_token: Option<String>,
}

impl RetryableHttpClient {
    pub fn new(client: reqwest::Client, backoff: ExponentialBackoff) -> Self {
        Self {
            client,
            backoff,
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
        let backoff = self.backoff.clone();
        let bearer_token = self.bearer_token.clone();

        retry::retry_with_backoff(
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
        .await
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
