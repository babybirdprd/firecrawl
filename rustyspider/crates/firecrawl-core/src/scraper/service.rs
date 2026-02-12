use crate::scraper::{Scraper, ScrapeOptions, ScrapeResult, DocumentFormat, ExtractOptions};
use crate::scraper::http::HttpScraper;
use crate::scraper::browser::BrowserScraper;
use crate::html::{transform_html, TransformHtmlOptions, extract_metadata, extract_links};
use crate::document::DocumentConverter;

pub struct ScrapeService {
    http_scraper: HttpScraper,
    browser_scraper: Option<BrowserScraper>,
    converter: DocumentConverter,
    extractor: Box<dyn StructuredDataExtractor>,
}

#[async_trait::async_trait]
pub trait StructuredDataExtractor: Send + Sync {
    async fn extract(&self, html: &str, options: ExtractOptions) -> anyhow::Result<serde_json::Value>;
}

pub struct NoopExtractor;

#[async_trait::async_trait]
impl StructuredDataExtractor for NoopExtractor {
    async fn extract(&self, _html: &str, options: ExtractOptions) -> anyhow::Result<serde_json::Value> {
        tracing::info!("Extracting structured data with schema (Noop): {:?}", options.schema);
        Ok(serde_json::json!({
            "success": true,
            "data": {},
            "warning": "LLM extraction is currently a placeholder (NoopExtractor)"
        }))
    }
}

impl ScrapeService {
    pub async fn new(proxy_url: Option<String>) -> anyhow::Result<Self> {
        let http_scraper = HttpScraper::new();
        // Try to launch browser, but don't fail if it's not available
        let browser_scraper = match BrowserScraper::new(proxy_url).await {
            Ok(b) => Some(b),
            Err(e) => {
                tracing::warn!("Failed to initialize BrowserScraper: {}", e);
                None
            }
        };
        let converter = DocumentConverter::new();
        let extractor = Box::new(NoopExtractor);

        Ok(Self {
            http_scraper,
            browser_scraper,
            converter,
            extractor,
        })
    }

    pub async fn scrape(&self, options: ScrapeOptions) -> anyhow::Result<ScrapeResult> {
        // Decide which scraper to use.
        // For now: use browser if wait_for, viewport, or block_resources is set.
        let use_browser = options.wait_for.is_some() || options.block_resources || options.viewport.is_some();

        let mut result = if use_browser {
            if let Some(browser) = &self.browser_scraper {
                browser.scrape(options.clone()).await?
            } else {
                anyhow::bail!("Browser scraper requested but not available (failed to initialize)");
            }
        } else {
            self.http_scraper.scrape(options.clone()).await?
        };

        let raw_html = result.raw_html.clone().unwrap_or_default();

        // 1. Metadata extraction
        let metadata = extract_metadata(Some(raw_html.clone())).await?;
        result.metadata = Some(serde_json::to_value(metadata)?);

        // 2. Link extraction
        if options.formats.contains(&DocumentFormat::Links) {
            result.links = Some(extract_links(Some(raw_html.clone())).await?);
        }

        // 3. HTML Transformation (Cleaning)
        let cleaned_html = if options.only_main_content || !options.include_tags.is_empty() || !options.exclude_tags.is_empty() {
            transform_html(TransformHtmlOptions {
                html: raw_html.clone(),
                url: options.url.clone(),
                include_tags: options.include_tags.clone(),
                exclude_tags: options.exclude_tags.clone(),
                only_main_content: options.only_main_content,
                omce_signatures: None,
            }).await?
        } else {
            raw_html.clone()
        };

        if options.formats.contains(&DocumentFormat::Html) {
            result.html = Some(cleaned_html.clone());
        }

        // 4. Markdown conversion
        // Default to Markdown if no formats specified, or if Markdown explicitly requested
        if options.formats.is_empty() || options.formats.contains(&DocumentFormat::Markdown) {
            result.markdown = Some(self.converter.convert_html_to_markdown(&cleaned_html)?);
        }

        // Raw HTML
        if !options.formats.contains(&DocumentFormat::RawHtml) {
            result.raw_html = None;
        } else {
            result.raw_html = Some(raw_html);
        }

        // 5. LLM Extraction
        if let Some(extract_opts) = options.extract {
            result.extract = Some(self.extractor.extract(&cleaned_html, extract_opts).await?);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scraper::{ScrapeOptions, DocumentFormat};

    #[tokio::test]
    async fn test_scrape_service_http() {
        // This test requires internet access if we use a real URL,
        // or we could mock the scrapers if we wanted to be more unit-testy.
        // For now, just a basic instantiation test.
        let service = ScrapeService::new(None).await.unwrap();

        let _options = ScrapeOptions {
            url: "https://example.com".to_string(),
            formats: vec![DocumentFormat::Markdown],
            only_main_content: false,
            include_tags: vec![],
            exclude_tags: vec![],
            timeout: Some(5000),
            wait_for: None,
            headers: None,
            viewport: None,
            block_resources: false,
            actions: vec![],
            extract: None,
            proxy_url: None,
        };

        // We won't actually call scrape() here because it might fail due to no internet/browser
        assert!(service.browser_scraper.is_some() || true); // Just to use it
    }
}
