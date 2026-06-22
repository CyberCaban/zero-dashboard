use crate::models::{SentimentAnalysisResult, outbound::{AiMessage, AiRequest}};

pub mod groq_client_gpt;

#[async_trait::async_trait]
pub trait AiClient: Send + Sync {
    async fn analyze_sentiment(&self, text: &str) -> anyhow::Result<SentimentAnalysisResult>;
}

pub fn prompt_for_sentiment_analysis(text: &str) -> String {
    format!(
        "Проанализируй следующий отзыв клиента и определи его тональность (sentiment: NEGATIVE, POSITIVE, NEUTRAL). \
        Если тональность негативная, выдели конкретные причины (что именно не понравилось, например напиши КУХНЯ в issues если не понравилась кухня). \
        Верни ответ в формате JSON (пример): {{\"sentiment\": \"POSITIVE\", \"issues\": [КУХНЯ, ОБСЛУЖИВАНИЕ], \"summary\": \"...\"}}.\n\nОтзыв: {}",
        text
    )
}
pub fn system_message_for_sentiment_analysis() -> String {
    r#"
    Ты профессиональный аналитик отзывов клиентов. 
    Твоя задача: определить общую тональность отзыва и извлечь конкретные жалобы (то, что именно не понравилось).
    Ответь СТРОГО в формате JSON без markdown-оберток (без ```json).
    Формат:
    {
        "sentiment": "positive | negative | neutral | mixed",
        "issues": ["жалоба 1", "жалоба 2"],
        "summary": "Краткое резюме отзыва в 1 предложении"
    }
    Если негатива нет, оставь массив issues пустым.
    "#
    .to_string()
}

pub fn sentiment_analysis_request(model: String, prompt: String) -> AiRequest {
    AiRequest {
        model: model.to_string(),
        messages: vec![
            AiMessage {
                role: "system".to_string(),
                content: system_message_for_sentiment_analysis(),
            },
            AiMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            },
        ],
        response_format: serde_json::from_str(
            r#"{
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
            }"#,
        )
        .unwrap(),
        temperature: 0.1,
    }
}
