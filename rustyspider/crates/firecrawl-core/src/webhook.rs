use serde::Serialize;
use crate::scraper::{WebhookOptions, ScrapeResult};
use reqwest::Client;

#[derive(Debug, Serialize)]
pub struct WebhookPayload {
    pub success: bool,
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    #[serde(rename = "webhookId")]
    pub webhook_id: String,
    pub data: Vec<ScrapeResult>,
    pub metadata: Option<serde_json::Value>,
}

pub struct WebhookSender {
    client: Client,
}

impl WebhookSender {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn send(
        &self,
        options: &WebhookOptions,
        event_type: &str,
        id: &str,
        data: Vec<ScrapeResult>,
    ) -> anyhow::Result<()> {
        // Check if event should be sent
        if let Some(events) = &options.events {
            // events in config are often like ["started", "page", "completed"]
            // but event_type is "crawl.started", etc.
            let sub_type = event_type.split('.').last().unwrap_or(event_type);
            if !events.contains(&sub_type.to_string()) && !events.contains(&"*".to_string()) {
                return Ok(());
            }
        }

        let payload = WebhookPayload {
            success: true,
            type_: event_type.to_string(),
            id: id.to_string(),
            webhook_id: uuid::Uuid::now_v7().to_string(),
            data,
            metadata: options.metadata.clone(),
        };

        let mut request = self.client.post(&options.url).json(&payload);

        if let Some(headers) = &options.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            tracing::warn!(
                "Webhook delivery failed for {} to {}: {}",
                event_type,
                options.url,
                response.status()
            );
        }

        Ok(())
    }
}
