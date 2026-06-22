use std::sync::Arc;

use anyhow::Result;
use tokio::time::Instant;
use tracing::{debug, error, info};

use crate::{
    config, consts,
    models::{inbound::ReviewRequest, outbound::EscalationMessage},
    state,
};

pub struct NatsServer {
    nats_client: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
    state: Arc<state::AppState>,
}

impl NatsServer {
    pub async fn new(config: &config::Config, state: Arc<state::AppState>) -> Result<Self> {
        tracing::info!("Connecting to NATS at {}...", config.nats_url);
        let nats_client = async_nats::connect(config.nats_url.clone())
            .await
            .expect("Failed to connect to NATS");
        let jetstream = async_nats::jetstream::new(nats_client.clone());
        Ok(Self {
            nats_client,
            jetstream,
            state,
        })
    }
    pub async fn handle_inbound_reviews(&self) -> Result<()> {
        use futures_util::StreamExt;
        let js = self.jetstream.clone();
        let inbound_stream = js
            .get_or_create_stream(async_nats::jetstream::stream::Config {
                name: "inbound-reviews".to_string(),
                subjects: vec![consts::REVIEW_REQUEST_SUBJECT.to_string()],
                ..Default::default()
            })
            .await?;
        let consumer = inbound_stream
            .get_or_create_consumer(
                "review-gating",
                async_nats::jetstream::consumer::pull::Config {
                    durable_name: Some("review-gating".to_string()),
                    ..Default::default()
                },
            )
            .await?;

        info!(
            "Successfully subscribed to '{}'",
            consts::REVIEW_REQUEST_SUBJECT
        );

        let mut messages = consumer.messages().await?;
        let mut js_context = self.jetstream.clone();
        let state = self.state.clone();
        tokio::spawn(async move {
            while let Some(Ok(message)) = messages.next().await {
                Self::handle_inbound_review_message(&mut js_context, message, &state).await;
            }
        });
        Ok(())
    }
    async fn handle_inbound_review_message(
        js_context: &mut async_nats::jetstream::Context,
        message: async_nats::jetstream::Message,
        state: &Arc<state::AppState>,
    ) {
        let start_time = Instant::now();

        let req: ReviewRequest = match serde_json::from_reader(message.payload.as_ref()) {
            Ok(req) => req,
            Err(err) => {
                tracing::error!(error = %err, "Failed to deserialize ReviewRequest payload");
                return;
            }
        };

        info!(
            session_id = %req.payload.session_id,
            business_id = %req.payload.business_id,
            "Received inbound review request"
        );

        let analysis = match state
            .analyzer()
            .analyze_sentiment(&req.payload.message.text)
            .await
        {
            Ok(analysis) => analysis,
            Err(err) => {
                tracing::error!(error = %err, "Failed to analyze sentiment");
                return;
            }
        };

        info!(
            sentiment = %analysis.sentiment,
            issues = ?&analysis.issues,
            "Completed sentiment analysis"
        );

        let reply_payload = match serde_json::to_vec(&EscalationMessage::from_analysis(
            analysis,
            req.payload.session_id.clone(),
            req.payload.business_id.clone(),
        )) {
            Ok(payload) => payload,
            Err(err) => {
                tracing::error!(error = %err, "Failed to serialize EscalationMessage");
                return;
            }
        };

        if let Err(err) = js_context
            .publish(
                format!("review.v1.escalate.{}", req.payload.business_id),
                reply_payload.into(),
            )
            .await
        {
            error!(
                business_id = %req.payload.business_id,
                error = %err,
                "Failed to publish reply"
            );
        } else {
            debug!(
                business_id = %req.payload.business_id,
                elapsed_ms = %start_time.elapsed().as_millis(),
                "Successfully published reply"
            );
        }
        if let Err(err) = message.ack().await {
            error!(
                business_id = %req.payload.business_id,
                error = %err,
                "Failed to acknowledge message"
            );
        }
    }
}
