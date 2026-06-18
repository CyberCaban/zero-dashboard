use anyhow::Result;
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::time::Instant;

#[derive(Debug, Deserialize)]
struct Metadata {
    traceparent: String,
    timestamp: i64,
}

#[derive(Debug, Deserialize)]
struct Customer {
    platform_user_id: String,
    phone: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct Message {
    message_id: String,
    text: String,
    raw_rating: i32,
}

#[derive(Debug, Deserialize)]
struct RequestPayload {
    session_id: String,
    business_id: String,
    customer: Customer,
    message: Message,
}

#[derive(Debug, Deserialize)]
struct ReviewRequest {
    metadata: Metadata,
    payload: RequestPayload,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_level(true).init();

    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());

    tracing::info!("Connecting to NATS at {}...", nats_url);
    let client = async_nats::connect(nats_url).await?;
    let subject = "reviews.v1.inbound.>";
    let js = async_nats::jetstream::new(client);
    let inbound_stream = js
        .get_or_create_stream(async_nats::jetstream::stream::Config {
            name: "inbound-reviews".to_string(),
            subjects: vec![subject.to_string()],
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
    tracing::info!("Successfully subscribed to '{}'", subject);

    let mut messages = consumer.messages().await?;
    while let Some(Ok(message)) = messages.next().await {
        let start_time = Instant::now();

        let req: ReviewRequest = match serde_json::from_slice(&message.payload) {
            Ok(req) => req,
            Err(err) => {
                tracing::error!(error = %err, "Failed to deserialize ReviewRequest payload");
                continue;
            }
        };

        tracing::info!(
            session_id = %req.payload.session_id,
            business_id = %req.payload.business_id,
            "Received inbound review request"
        );

        println!("Received review request: {:#?}", req);

        let reply_payload = r#"{"status":"success"}"#;
        if let Err(err) = js
            .publish(
                format!("review.v1.escalate.{}", req.payload.business_id),
                reply_payload.into(),
            )
            .await
        {
            tracing::error!(
                business_id = %req.payload.business_id,
                error = %err,
                "Failed to publish reply"
            );
        } else {
            tracing::debug!(
                business_id = %req.payload.business_id,
                elapsed_ms = %start_time.elapsed().as_millis(),
                "Successfully published reply"
            );
        }
        if let Err(err) = message.ack().await {
            tracing::error!(
                business_id = %req.payload.business_id,
                error = %err,
                "Failed to acknowledge message"
            );
        }
    }
    Ok(())
}
