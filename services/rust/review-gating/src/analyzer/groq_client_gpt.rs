use crate::{
    analyzer::{
        AiClient, AnalysisError, prompt_for_sentiment_analysis, sentiment_analysis_request,
    },
    consts::MAX_REVIEW_LENGTH,
    http_client::RetryableHttpClient,
    models::SentimentAnalysisResult,
};

use anyhow::{Context, bail};
use tracing::{error, info};

use crate::consts::SENTIMENT_ANALYSIS_URL;

pub struct GroqClientGPT {
    http_client: RetryableHttpClient,
    model: String,
}

impl GroqClientGPT {
    pub fn new(http_client: RetryableHttpClient, api_key: String, model: String) -> Self {
        Self { http_client, model }
    }
}

#[async_trait::async_trait]
impl AiClient for GroqClientGPT {
    async fn analyze_sentiment(
        &self,
        text: &str,
    ) -> anyhow::Result<crate::models::SentimentAnalysisResult> {
        if text.len() > MAX_REVIEW_LENGTH {
            bail!(AnalysisError::ReviewTextTooLong);
        }

        let prompt = prompt_for_sentiment_analysis(text);

        let request_body = sentiment_analysis_request(self.model.clone(), prompt);

        info!("Sending request to AI for sentiment analysis...");
        let response = self
            .http_client
            .post_json(SENTIMENT_ANALYSIS_URL, &request_body, "analyze_sentiment")
            .await?;

        let raw_json: serde_json::Value = match response.json().await {
            Ok(raw_json) => raw_json,
            Err(e) => {
                if e.is_decode() {
                    bail!(AnalysisError::JsonError(e.to_string()));
                } else {
                    bail!(AnalysisError::HttpError(e))
                }
            }
        };

        let response = raw_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or(AnalysisError::ExtractionError)?;

        let analysis_result =  match serde_json::from_str::<SentimentAnalysisResult>(response) {
            Ok(analysis_result) => analysis_result,
            Err(e) => {
                error!("Raw AI response content: {}", response);
                bail!(AnalysisError::FormatError(e.to_string()));
            }
        };

        Ok(analysis_result)
    }
}
