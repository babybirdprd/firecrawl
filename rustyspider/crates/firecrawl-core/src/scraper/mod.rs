pub mod http;
pub mod browser;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeOptions {
    pub url: String,
    pub timeout: Option<u64>,
    pub wait_for_selector: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub url: String,
    pub content: String,
    pub status_code: Option<u16>,
    pub metadata: Option<serde_json::Value>,
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
            timeout: None,
            wait_for_selector: None,
            headers: None,
        };
        // Verify it implements Scraper trait
        let _scraper_trait: &dyn Scraper = &scraper;
    }
}
