pub mod http;
pub mod browser;
pub mod service;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Wait {
        milliseconds: Option<u64>,
        selector: Option<String>,
    },
    Click {
        selector: String,
    },
    Screenshot,
    WriteText {
        selector: String,
        text: String,
    },
    Press {
        key: String,
    },
    Scroll {
        direction: String,
        amount: Option<u32>,
    },
    Hover {
        selector: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaitFor {
    Selector(String),
    Time(u64), // milliseconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOptions {
    pub schema: serde_json::Value,
    pub system_prompt: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DocumentFormat {
    Markdown,
    Html,
    RawHtml,
    Screenshot,
    Links,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScrapeOptions {
    pub url: String,
    #[serde(default)]
    pub formats: Vec<DocumentFormat>,
    #[serde(default)]
    pub only_main_content: bool,
    #[serde(default)]
    pub include_tags: Vec<String>,
    #[serde(default)]
    pub exclude_tags: Vec<String>,
    pub timeout: Option<u64>, // milliseconds
    pub wait_for: Option<WaitFor>,
    pub headers: Option<HashMap<String, String>>,
    pub viewport: Option<Viewport>,
    #[serde(default)]
    pub block_resources: bool,
    #[serde(default)]
    pub actions: Vec<Action>,
    pub extract: Option<ExtractOptions>,
    pub proxy_url: Option<String>,
    pub webhook: Option<WebhookOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookOptions {
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub metadata: Option<serde_json::Value>,
    pub events: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScrapeResult {
    pub url: String,
    pub markdown: Option<String>,
    pub html: Option<String>,
    pub raw_html: Option<String>,
    pub screenshot: Option<Vec<u8>>,
    pub links: Option<Vec<String>>,
    pub status_code: Option<u16>,
    pub metadata: Option<serde_json::Value>,
    pub extract: Option<serde_json::Value>,
    pub warning: Option<String>,
}

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn scrape(&self, options: ScrapeOptions) -> anyhow::Result<ScrapeResult>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scraper::http::HttpScraper;

    #[tokio::test]
    async fn test_http_scraper_instantiation() {
        let scraper = HttpScraper::new();
        let _options = ScrapeOptions {
            url: "https://example.com".to_string(),
            formats: vec![],
            only_main_content: false,
            include_tags: vec![],
            exclude_tags: vec![],
            timeout: None,
            wait_for: None,
            headers: None,
            viewport: None,
            block_resources: false,
            actions: vec![],
            extract: None,
            proxy_url: None,
            webhook: None,
        };
        // Verify it implements Scraper trait
        let _scraper_trait: &dyn Scraper = &scraper;
    }
}
