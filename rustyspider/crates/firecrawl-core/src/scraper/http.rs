use super::{Scraper, ScrapeOptions, ScrapeResult};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;
use std::time::Duration;

pub struct HttpScraper {
    client: reqwest::Client,
}

impl HttpScraper {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for HttpScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Scraper for HttpScraper {
    async fn scrape(&self, options: ScrapeOptions) -> anyhow::Result<ScrapeResult> {
        let mut request = self.client.get(&options.url);

        if let Some(timeout) = options.timeout {
            request = request.timeout(Duration::from_millis(timeout));
        }

        if let Some(headers) = options.headers {
            let mut header_map = HeaderMap::new();
            for (k, v) in headers {
                if let (Ok(name), Ok(val)) = (HeaderName::from_str(&k), HeaderValue::from_str(&v)) {
                    header_map.insert(name, val);
                }
            }
            request = request.headers(header_map);
        }

        let response = request.send().await?;
        let status = response.status();
        let content = response.text().await?;

        Ok(ScrapeResult {
            url: options.url,
            content,
            status_code: Some(status.as_u16()),
            metadata: None,
        })
    }
}
