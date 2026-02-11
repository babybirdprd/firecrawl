use anyhow::anyhow;
pub mod model;
pub mod providers;
pub mod renderers;

pub use providers::factory::DocumentType;

use crate::document::model::Document;
use crate::document::providers::factory::ProviderFactory;
use crate::document::renderers::html::HtmlRenderer;
use crate::document::renderers::markdown::MarkdownRenderer;

pub struct DocumentConverter {
  factory: ProviderFactory,
  html_renderer: HtmlRenderer,
  markdown_renderer: MarkdownRenderer,
}

impl Default for DocumentConverter {
  fn default() -> Self {
    Self::new()
  }
}

impl DocumentConverter {
  pub fn new() -> Self {
    Self {
      factory: ProviderFactory::new(),
      html_renderer: HtmlRenderer::new(),
      markdown_renderer: MarkdownRenderer::new(),
    }
  }

  pub fn convert_buffer_to_html(
    &self,
    data: &[u8],
    doc_type: DocumentType,
  ) -> anyhow::Result<String> {
    let provider = self.factory.get_provider(doc_type);

    let document: Document = provider
      .parse_buffer(data)
      .map_err(|e| anyhow!(format!("Provider error: {e}")))?;

    let html = self.html_renderer.render(&document);
    Ok(html)
  }

  pub fn convert_buffer_to_markdown(
    &self,
    data: &[u8],
    doc_type: DocumentType,
  ) -> anyhow::Result<String> {
    let provider = self.factory.get_provider(doc_type);

    let document: Document = provider
        .parse_buffer(data)
        .map_err(|e| anyhow!(format!("Provider error: {e}")))?;

    let markdown = self.markdown_renderer.render(&document);
    Ok(markdown)
  }

  pub fn convert_html_to_markdown(&self, html: &str) -> anyhow::Result<String> {
      self.convert_buffer_to_markdown(html.as_bytes(), DocumentType::Html)
  }
}
