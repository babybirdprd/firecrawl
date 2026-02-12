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
        let client = if let Some(proxy_url) = &options.proxy_url {
            reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(proxy_url)?)
                .build()?
        } else {
            self.client.clone()
        };

        let mut request = client.get(&options.url);

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
            raw_html: Some(content),
            status_code: Some(status.as_u16()),
            ..Default::default()
        })
    }
}
