use crate::{
    analyzer::{AiClient, prompt_for_sentiment_analysis, sentiment_analysis_request},
    models::SentimentAnalysisResult,
};

use anyhow::{Context, bail};
use tracing::{error, info};

use crate::consts::SENTIMENT_ANALYSIS_URL;

pub struct GroqClientGPT {
    http_client: reqwest::Client,
    api_key: String,
    model: String,
}

impl GroqClientGPT {
    pub fn new(http_client: reqwest::Client, api_key: String, model: String) -> Self {
        Self {
            http_client,
            api_key,
            model,
        }
    }
}

#[async_trait::async_trait]
impl AiClient for GroqClientGPT {
    async fn analyze_sentiment(
        &self,
        text: &str,
    ) -> anyhow::Result<crate::models::SentimentAnalysisResult> {
        let prompt = prompt_for_sentiment_analysis(text);

        let request_body = sentiment_analysis_request(self.model.clone(), prompt);

        info!("Sending request to AI for sentiment analysis...");
        let response = self
            .http_client
            .post(SENTIMENT_ANALYSIS_URL)
            .bearer_auth(self.api_key.clone())
            .json(&request_body)
            .send()
            .await
            .context("Failed to get response from AI")?;

        let Ok(raw_json): Result<serde_json::Value, _> = response.json().await else {
            bail!("Failed to read AI response json");
        };

        let response = raw_json["choices"][0]["message"]["content"]
            .as_str()
            .context("Failed to extract content from AI response")?;

        let Ok(analysis_result) = serde_json::from_str::<SentimentAnalysisResult>(response) else {
            error!("Raw AI response content: {}", response);
            bail!("Failed to parse AI response as SentimentAnalysisResult");
        };

        Ok(analysis_result)
    }
}
