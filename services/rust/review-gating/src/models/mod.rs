use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Customer {
    pub platform_user_id: String,
    pub phone: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub message_id: String,
    pub text: String,
    pub raw_rating: i32,
}

#[derive(Deserialize, Debug)]
pub struct SentimentAnalysisResult {
    pub sentiment: String, // "positive", "negative", "neutral", "mixed"
    pub issues: Vec<String>,
    pub summary: String,
}

pub mod inbound {
    use serde::Deserialize;

    use crate::models::{Customer, Message};

    #[derive(Debug, Deserialize)]
    pub struct Metadata {
        pub traceparent: String,
        pub timestamp: i64,
    }
    #[derive(Debug, Deserialize)]
    pub struct RequestPayload {
        pub session_id: String,
        pub business_id: String,
        pub customer: Customer,
        pub message: Message,
    }
    #[derive(Debug, Deserialize)]
    pub struct ReviewRequest {
        pub metadata: Metadata,
        pub payload: RequestPayload,
    }
}

pub mod outbound {
    use serde::Serialize;
    use serde_json::Value;

    use crate::models::SentimentAnalysisResult;

    #[derive(Debug, Serialize)]
    pub struct EscalationMessage {
        pub session_id: String,
        pub business_id: String,
        pub sentiment: String,
        pub issues: Vec<String>,
        pub summary: String,
    }
    impl EscalationMessage {
        pub fn from_analysis(
            result: SentimentAnalysisResult,
            session_id: String,
            business_id: String,
        ) -> Self {
            Self {
                session_id,
                business_id,
                sentiment: result.sentiment,
                issues: result.issues,
                summary: result.summary,
            }
        }
    }

    #[derive(Debug, Serialize)]
    pub struct AiRequest {
        pub model: String,
        pub messages: Vec<AiMessage>,
        pub response_format: Value,
        pub temperature: f32,
    }

    #[derive(Debug, Serialize)]
    pub struct AiMessage {
        pub role: String,
        pub content: String,
    }
}
