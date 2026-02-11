use super::{Scraper, ScrapeOptions, ScrapeResult};
use async_trait::async_trait;
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use std::sync::Arc;
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct BrowserScraper {
    browser: Arc<Browser>,
    _handle: Arc<JoinHandle<()>>,
}

impl BrowserScraper {
    pub async fn new() -> anyhow::Result<Self> {
        let (browser, mut handler) = Browser::launch(
            BrowserConfig::builder()
                .build()
                .map_err(|e| anyhow::anyhow!(e))?,
        )
        .await?;

        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            browser: Arc::new(browser),
            _handle: Arc::new(handle),
        })
    }
}

#[async_trait]
impl Scraper for BrowserScraper {
    async fn scrape(&self, options: ScrapeOptions) -> anyhow::Result<ScrapeResult> {
        let page = self.browser.new_page("about:blank").await?;

        page.goto(&options.url).await?;

        if let Some(selector) = &options.wait_for_selector {
            for _ in 0..5 {
                if page.find_element(selector).await.is_ok() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        let content = page.content().await?;

        page.close().await?;

        Ok(ScrapeResult {
            url: options.url,
            content,
            status_code: None,
            metadata: None,
        })
    }
}
