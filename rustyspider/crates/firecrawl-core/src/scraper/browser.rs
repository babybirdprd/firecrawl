use super::{Scraper, ScrapeOptions, ScrapeResult, WaitFor};
use async_trait::async_trait;
use chromiumoxide::{Browser, BrowserConfig, Page};
use chromiumoxide::cdp::browser_protocol::network::SetBlockedUrLsParams;
use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use futures::StreamExt;
use std::sync::Arc;
use tokio::task::JoinHandle;
use std::time::Duration;

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

    async fn scrape_page(&self, page: &Page, options: &ScrapeOptions) -> anyhow::Result<ScrapeResult> {
        // block resources if requested
        if options.block_resources {
             let params = SetBlockedUrLsParams::builder()
                .urls(vec![
                    "*.png".to_string(), "*.jpg".to_string(), "*.jpeg".to_string(), "*.gif".to_string(), "*.svg".to_string(),
                    "*.css".to_string(), "*.woff".to_string(), "*.woff2".to_string(), "*.ico".to_string()
                ])
                .build()
                .map_err(|e| anyhow::anyhow!(e))?;
             page.execute(params).await?;
        }

        if let Some(viewport) = &options.viewport {
             let params = SetDeviceMetricsOverrideParams::builder()
                .width(viewport.width as i64)
                .height(viewport.height as i64)
                .device_scale_factor(1.0)
                .mobile(false)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?;
            page.execute(params).await?;
        }

        page.goto(&options.url).await?;

        match &options.wait_for {
            Some(WaitFor::Selector(selector)) => {
                // improved wait for selector with polling
                let start = std::time::Instant::now();
                let timeout = Duration::from_millis(options.timeout.unwrap_or(30000));
                let mut found = false;

                while start.elapsed() < timeout {
                    match page.find_element(selector).await {
                        Ok(_) => {
                            found = true;
                            break;
                        }
                        Err(_) => {
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
                if !found {
                     return Err(anyhow::anyhow!("Timeout waiting for selector: {}", selector));
                }
            }
            Some(WaitFor::Time(ms)) => {
                tokio::time::sleep(Duration::from_millis(*ms)).await;
            }
            None => {}
        }

        let content = page.content().await?;

        Ok(ScrapeResult {
            url: options.url.clone(),
            content,
            status_code: None, // chromiumoxide doesn't easily give status code on page.goto?
            metadata: None,
        })
    }
}

#[async_trait]
impl Scraper for BrowserScraper {
    async fn scrape(&self, options: ScrapeOptions) -> anyhow::Result<ScrapeResult> {
        let page = self.browser.new_page("about:blank").await?;

        let timeout_duration = Duration::from_millis(options.timeout.unwrap_or(30000));

        // wrap in timeout
        let result = tokio::time::timeout(timeout_duration, self.scrape_page(&page, &options)).await;

        let scrape_res = match result {
            Ok(res) => res, // inner result
            Err(_) => Err(anyhow::anyhow!("Scrape timed out")),
        };

        // ensure page is closed
        if let Err(e) = page.close().await {
            // log error but don't overwrite scrape error if it exists?
            // for now, just print to stderr or ignore if we want to be silent
            eprintln!("Failed to close page: {}", e);
        }

        scrape_res
    }
}
