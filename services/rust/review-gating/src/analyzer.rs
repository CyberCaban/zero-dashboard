use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info};

use crate::{consts::SENTIMENT_ANALYSIS_URL, models::SentimentAnalysisResult};

#[derive(Debug, Serialize)]
struct AiRequest {
    model: String,
    messages: Vec<AiMessage>,
    response_format: Value,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct AiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize, Debug)]
struct AiResponse {
    choices: Vec<AiChoice>,
}

#[derive(Deserialize, Debug)]
struct AiChoice {
    message: AiMessageResponse,
}

#[derive(Deserialize, Debug)]
struct AiMessageResponse {
    content: String,
}

pub struct Analyzer {
    client: Client,
    api_key: String,
    model: String,
}

impl Analyzer {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }
    pub async fn analyze_sentiment(&self, text: &str) -> Result<SentimentAnalysisResult> {
        let prompt = format!(
            "Проанализируй следующий отзыв клиента и определи его тональность (sentiment: NEGATIVE, POSITIVE, NEUTRAL). \
            Если тональность негативная, выдели конкретные причины (что именно не понравилось, например напиши КУХНЯ в issues если не понравилась кухня). \
            Верни ответ в формате JSON (пример): {{\"sentiment\": \"POSITIVE\", \"issues\": [КУХНЯ, ОБСЛУЖИВАНИЕ], \"summary\": \"...\"}}.\n\nОтзыв: {}",
            text
        );

        let request_body = AiRequest {
            model: self.model.clone(),
            messages: vec![AiMessage {
                role: "system".to_string(),
                content: r#"
                    Ты профессиональный аналитик отзывов клиентов. 
                    Твоя задача: определить общую тональность отзыва и извлечь конкретные жалобы (то, что именно не понравилось).
                    Ответь СТРОГО в формате JSON без markdown-оберток (без ```json).
                    Формат:
                    {
                        "sentiment": "positive | negative | neutral | mixed",
                        "negative_points": ["жалоба 1", "жалоба 2"],
                        "summary": "Краткое резюме отзыва в 1 предложении"
                    }
                    Если негатива нет, оставь массив negative_points пустым.
                "#.to_string()
            }, AiMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            response_format: serde_json::from_str(r#"{
                "type": "json_schema",
                "json_schema": {
                "name": "review_analysis",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                    "sentiment": {
                        "type": "string",
                        "enum": ["positive", "negative", "neutral"]
                    },
                    "issues": {
                        "type": "array",
                        "items": {
                        "type": "string"
                        }
                    },
                    "summary": {
                        "type": "string"
                    }
                    },
                    "required": ["sentiment", "issues", "summary"],
                    "additionalProperties": false
                }
                }
            }"#).unwrap(),
            temperature: 0.1,
        };
        println!("{}", serde_json::to_string_pretty(&request_body)?);

        info!("Sending request to AI for sentiment analysis...");
        let response = self
            .client
            .post(SENTIMENT_ANALYSIS_URL)
            .bearer_auth(self.api_key.clone())
            .json(&request_body)
            .send()
            .await
            .context("Failed to get response from AI")?;

        let Ok(raw_json): Result<serde_json::Value, _> = response.json().await else {
            bail!("Failed to read AI response json");
        };

        println!("Raw AI response: {:#?}", raw_json);
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
