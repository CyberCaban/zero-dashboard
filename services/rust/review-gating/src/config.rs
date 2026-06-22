pub struct Config {
    pub nats_url: String,
    pub groq_api_key: String,
    pub model: String,
}

impl Config {
    /// Load configuration from environment variables
    /// - `NATS_URL`: The URL of the NATS server (default: "nats://localhost:4222")
    /// - `GROQ_API_KEY`: The API key for Groq (default: "")
    /// - `MODEL`: The model to use for sentiment analysis (default: "llama-3.1-8b-instant")
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let groq_api_key = std::env::var("GROQ_API_KEY").unwrap_or_else(|_| "".to_string());
        let model = std::env::var("GROQ_MODEL").unwrap_or_else(|_| "llama-3.1-8b-instant".to_string());
        Self {
            nats_url,
            groq_api_key,
            model,
        }
    }
}
