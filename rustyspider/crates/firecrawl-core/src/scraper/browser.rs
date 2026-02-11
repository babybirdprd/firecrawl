use super::{Action, DocumentFormat, Scraper, ScrapeOptions, ScrapeResult, WaitFor};
use async_trait::async_trait;
use chromiumoxide::{Browser, BrowserConfig, Page};
use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use chromiumoxide::cdp::browser_protocol::network::SetBlockedUrLsParams;
use futures::StreamExt;
use std::sync::Arc;
use tokio::task::JoinHandle;
use std::time::Duration;
use tracing::debug;

/// A guard that ensures the page is closed when dropped.
struct PageGuard {
    page: Page,
}

impl PageGuard {
    fn new(page: Page) -> Self {
        Self { page }
    }
}

impl Drop for PageGuard {
    fn drop(&mut self) {
        let page = self.page.clone();
        tokio::spawn(async move {
            if let Err(e) = page.close().await {
                debug!("Failed to close page in Drop: {}", e);
            }
        });
    }
}

impl std::ops::Deref for PageGuard {
    type Target = Page;

    fn deref(&self) -> &Self::Target {
        &self.page
    }
}

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
                    debug!("Browser handler error: {:?}", h);
                    break;
                }
            }
        });

        Ok(Self {
            browser: Arc::new(browser),
            _handle: Arc::new(handle),
        })
    }

    async fn wait_for_selector(
        &self,
        page: &Page,
        selector: &str,
        timeout_ms: u64,
    ) -> anyhow::Result<()> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let mut delay = Duration::from_millis(50);

        while start.elapsed() < timeout {
            if page.find_element(selector).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(delay).await;
            delay = (delay * 2).min(Duration::from_millis(500));
        }

        Err(anyhow::anyhow!("Timeout waiting for selector: {}", selector))
    }

    async fn execute_action(
        &self,
        page: &Page,
        action: &Action,
        timeout_ms: u64,
    ) -> anyhow::Result<()> {
        match action {
            Action::Wait {
                milliseconds,
                selector,
            } => {
                if let Some(ms) = milliseconds {
                    tokio::time::sleep(Duration::from_millis(*ms)).await;
                }
                if let Some(sel) = selector {
                    self.wait_for_selector(page, sel, timeout_ms).await?;
                }
            }
            Action::Click { selector } => {
                page.find_element(selector).await?.click().await?;
            }
            Action::Screenshot => {
                // Handled at the end of scrape if needed, or we could take one here
            }
            Action::WriteText { selector, text } => {
                page.find_element(selector).await?.type_str(text).await?;
            }
            Action::Press { key } => {
                page.find_element("body").await?.press_key(key).await?;
            }
            Action::Scroll { direction, amount } => {
                let dist = amount.unwrap_or(500) as i64;
                let (x, y) = match direction.as_str() {
                    "up" => (0, -dist),
                    "down" => (0, dist),
                    _ => (0, 0),
                };
                page.evaluate(format!("window.scrollBy({}, {})", x, y))
                    .await?;
            }
            Action::Hover { selector } => {
                page.find_element(selector).await?.hover().await?;
            }
        }
        Ok(())
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
                let timeout = options.timeout.unwrap_or(30000);
                self.wait_for_selector(page, selector, timeout).await?;
            }
            Some(WaitFor::Time(ms)) => {
                tokio::time::sleep(Duration::from_millis(*ms)).await;
            }
            None => {}
        }

        // Execute actions
        let timeout_ms = options.timeout.unwrap_or(30000);
        for action in &options.actions {
            self.execute_action(page, action, timeout_ms).await?;
        }

        let content = page.content().await?;
        let mut screenshot = None;

        if options.formats.contains(&DocumentFormat::Screenshot) {
            screenshot = Some(
                page.screenshot(chromiumoxide::page::ScreenshotParams::builder().build())
                    .await?,
            );
        }

        Ok(ScrapeResult {
            url: options.url.clone(),
            raw_html: Some(content),
            screenshot,
            status_code: None, // chromiumoxide doesn't easily give status code on page.goto?
            ..Default::default()
        })
    }
}

#[async_trait]
impl Scraper for BrowserScraper {
    async fn scrape(&self, options: ScrapeOptions) -> anyhow::Result<ScrapeResult> {
        let page = self.browser.new_page("about:blank").await?;
        let page_guard = PageGuard::new(page);

        let timeout_duration = Duration::from_millis(options.timeout.unwrap_or(30000));

        // wrap in timeout
        let result = tokio::time::timeout(timeout_duration, self.scrape_page(&page_guard, &options)).await;

        match result {
            Ok(res) => res, // inner result
            Err(_) => Err(anyhow::anyhow!("Scrape timed out")),
        }
        // PageGuard will automatically close the page when dropped
    }
}
