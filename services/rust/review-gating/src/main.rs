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
struct Payload {
    session_id: String,
    business_id: String,
    customer: Customer,
    message: Message,
}

#[derive(Debug, Deserialize)]
struct ReviewRequest {
    metadata: Metadata,
    payload: Payload,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_level(true).init();

    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());

    tracing::info!("Connecting to NATS at {}...", nats_url);
    let client = async_nats::connect(nats_url).await?;
    let subject = "reviews.v1.inbound.>";
    let mut sub = client.subscribe(subject).await?;
    tracing::info!("Successfully subscribed to '{}'", subject);

    while let Some(message) = sub.next().await {
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

        let Some(reply_to) = message.reply else {
            tracing::debug!("No reply subject found in the message");
            continue;
        };

        let reply_payload = r#"{"status":"success"}"#;
        if let Err(err) = client.publish(reply_to.clone(), reply_payload.into()).await {
            tracing::error!(
                reply_to = %reply_to,
                error = %err,
                "Failed to publish reply"
            );
        } else {
            tracing::debug!(
                reply_to = %reply_to,
                elapsed_ms = %start_time.elapsed().as_millis(),
                "Successfully published reply"
            );
        }
    }
    Ok(())
}
