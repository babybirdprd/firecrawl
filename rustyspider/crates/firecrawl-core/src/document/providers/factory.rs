use super::doc::DocProvider;
use super::docx::DocxProvider;
use super::odt::OdtProvider;
use super::rtf::RtfProvider;
use super::DocumentProvider;
use super::xlsx::XlsxProvider;
use super::html::HtmlProvider;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentType {
  Doc,
  Docx,
  Rtf,
  Odt,
  Xlsx,
  Html,
}

pub struct ProviderFactory {
  doc_provider: DocProvider,
  docx_provider: DocxProvider,
  rtf_provider: RtfProvider,
  odt_provider: OdtProvider,
  xlsx_provider: XlsxProvider,
  html_provider: HtmlProvider,
}

impl ProviderFactory {
  pub fn new() -> Self {
    Self {
      doc_provider: DocProvider::new(),
      docx_provider: DocxProvider::new(),
      rtf_provider: RtfProvider::new(),
      odt_provider: OdtProvider::new(),
      xlsx_provider: XlsxProvider::new(),
      html_provider: HtmlProvider::new(),
    }
  }

  pub fn get_provider(&self, doc_type: DocumentType) -> &dyn DocumentProvider {
    match doc_type {
      DocumentType::Doc => &self.doc_provider,
      DocumentType::Docx => &self.docx_provider,
      DocumentType::Rtf => &self.rtf_provider,
      DocumentType::Odt => &self.odt_provider,
      DocumentType::Xlsx => &self.xlsx_provider,
      DocumentType::Html => &self.html_provider,
    }
  }
}
