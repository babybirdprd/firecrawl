use crate::document::model::*;
use crate::document::providers::DocumentProvider;
use kuchikiki::traits::TendrilSink;
use kuchikiki::{parse_html, NodeData, NodeRef};
use std::error::Error;
use std::num::NonZeroU32;

pub struct HtmlProvider;

impl HtmlProvider {
    pub fn new() -> Self {
        Self
    }

    fn parse_block(&self, node: &NodeRef) -> Option<Block> {
        match node.data() {
            NodeData::Element(data) => {
                let tag_name = data.name.local.as_ref();
                match tag_name {
                    "p" => Some(Block::Paragraph(Paragraph {
                        kind: ParagraphKind::Normal,
                        inlines: self.collect_inlines(node),
                    })),
                    "h1" => Some(Block::Paragraph(Paragraph {
                        kind: ParagraphKind::Heading(1),
                        inlines: self.collect_inlines(node),
                    })),
                    "h2" => Some(Block::Paragraph(Paragraph {
                        kind: ParagraphKind::Heading(2),
                        inlines: self.collect_inlines(node),
                    })),
                    "h3" => Some(Block::Paragraph(Paragraph {
                        kind: ParagraphKind::Heading(3),
                        inlines: self.collect_inlines(node),
                    })),
                    "h4" => Some(Block::Paragraph(Paragraph {
                        kind: ParagraphKind::Heading(4),
                        inlines: self.collect_inlines(node),
                    })),
                    "h5" => Some(Block::Paragraph(Paragraph {
                        kind: ParagraphKind::Heading(5),
                        inlines: self.collect_inlines(node),
                    })),
                    "h6" => Some(Block::Paragraph(Paragraph {
                        kind: ParagraphKind::Heading(6),
                        inlines: self.collect_inlines(node),
                    })),
                    "blockquote" => Some(Block::Paragraph(Paragraph {
                        kind: ParagraphKind::Blockquote,
                        inlines: self.collect_inlines(node),
                    })),
                    "ul" => Some(Block::List(List {
                        list_type: ListType::Unordered,
                        items: self.parse_list_items(node),
                    })),
                    "ol" => Some(Block::List(List {
                        list_type: ListType::Ordered,
                        items: self.parse_list_items(node),
                    })),
                    "table" => Some(Block::Table(self.parse_table(node))),
                    "img" => {
                        let attrs = data.attributes.borrow();
                        let src = attrs.get("src").unwrap_or("").to_string();
                        let alt = attrs.get("alt").map(|s| s.to_string());
                        Some(Block::Image(Image { src, alt }))
                    }
                    // Containers that should be recursed into are handled in parse_blocks_recursive
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn collect_inlines(&self, node: &NodeRef) -> Vec<Inline> {
        let mut inlines = Vec::new();
        for child in node.children() {
            self.parse_inlines(&child, &mut inlines);
        }
        inlines
    }

    fn parse_inlines(&self, node: &NodeRef, inlines: &mut Vec<Inline>) {
        match node.data() {
            NodeData::Text(text) => {
                let content = text.borrow();
                if !content.trim().is_empty() {
                    inlines.push(Inline::Text(content.to_string()));
                }
            }
            NodeData::Element(data) => {
                let tag_name = data.name.local.as_ref();
                match tag_name {
                    "br" => inlines.push(Inline::LineBreak),
                    "a" => {
                        let attrs = data.attributes.borrow();
                        let href = attrs.get("href").unwrap_or("").to_string();
                        let children = self.collect_inlines(node);
                        inlines.push(Inline::Link { href, children });
                    }
                    "strong" | "b" => {
                        let children = self.collect_inlines(node);
                        inlines.push(Inline::Strong(children));
                    }
                    "em" | "i" => {
                        let children = self.collect_inlines(node);
                        inlines.push(Inline::Em(children));
                    }
                    "del" | "s" | "strike" => {
                        let children = self.collect_inlines(node);
                        inlines.push(Inline::Del(children));
                    }
                    "code" => {
                        let content = node.text_contents();
                        inlines.push(Inline::Code(content));
                    }
                    "sup" => {
                        let children = self.collect_inlines(node);
                        inlines.push(Inline::Sup(children));
                    }
                    "sub" => {
                        let children = self.collect_inlines(node);
                        inlines.push(Inline::Sub(children));
                    }
                    // Handle generic containers inside inline context (like span)
                    "span" | "div" => {
                         // Treat as transparent container for inlines
                         for child in node.children() {
                             self.parse_inlines(&child, inlines);
                         }
                    }
                    _ => {
                        // Unknown tag, treat children as inlines
                        for child in node.children() {
                            self.parse_inlines(&child, inlines);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn parse_list_items(&self, node: &NodeRef) -> Vec<ListItem> {
        let mut items = Vec::new();
        for child in node.children() {
             if let NodeData::Element(data) = child.data() {
                 if data.name.local.as_ref() == "li" {
                     items.push(ListItem {
                         blocks: self.parse_blocks_recursive(&child),
                     });
                 }
             }
        }
        items
    }

    fn parse_table(&self, node: &NodeRef) -> Table {
        let mut rows = Vec::new();
        // Handle thead, tbody, tfoot
        for child in node.children() {
            if let NodeData::Element(data) = child.data() {
                let tag = data.name.local.as_ref();
                match tag {
                    "thead" => self.parse_table_section(&child, TableRowKind::Header, &mut rows),
                    "tbody" => self.parse_table_section(&child, TableRowKind::Body, &mut rows),
                    "tfoot" => self.parse_table_section(&child, TableRowKind::Footer, &mut rows),
                    "tr" => {
                        // Direct tr child of table
                        self.parse_table_row(&child, TableRowKind::Body, &mut rows);
                    }
                    _ => {}
                }
            }
        }
        Table { rows }
    }

    fn parse_table_section(&self, node: &NodeRef, kind: TableRowKind, rows: &mut Vec<TableRow>) {
        for child in node.children() {
            if let NodeData::Element(data) = child.data() {
                if data.name.local.as_ref() == "tr" {
                    self.parse_table_row(&child, kind, rows);
                }
            }
        }
    }

    fn parse_table_row(&self, node: &NodeRef, kind: TableRowKind, rows: &mut Vec<TableRow>) {
        let mut cells = Vec::new();
        for child in node.children() {
            if let NodeData::Element(data) = child.data() {
                let tag = data.name.local.as_ref();
                if tag == "td" || tag == "th" {
                    let attrs = data.attributes.borrow();
                    let colspan = attrs.get("colspan").and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                    let rowspan = attrs.get("rowspan").and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);

                    cells.push(TableCell {
                        blocks: self.parse_blocks_recursive(&child),
                        colspan: NonZeroU32::new(colspan).unwrap_or(NonZeroU32::new(1).unwrap()),
                        rowspan: NonZeroU32::new(rowspan).unwrap_or(NonZeroU32::new(1).unwrap()),
                    });
                }
            }
        }
        rows.push(TableRow { cells, kind });
    }

    // This function handles the recursive parsing of blocks, handling containers transparently
    fn parse_blocks_recursive(&self, node: &NodeRef) -> Vec<Block> {
         let mut blocks = Vec::new();
         let mut current_inlines = Vec::new();

         for child in node.children() {
             let maybe_block = self.parse_block(&child);

             // Check if it's a block element
             if let Some(block) = maybe_block {
                 if !current_inlines.is_empty() {
                     blocks.push(Block::Paragraph(Paragraph {
                         kind: ParagraphKind::Normal,
                         inlines: std::mem::take(&mut current_inlines),
                     }));
                 }
                 blocks.push(block);
             } else {
                 // Check if it's a container that we should recurse into (div, section, etc.)
                 if self.is_container(&child) {
                     if !current_inlines.is_empty() {
                         blocks.push(Block::Paragraph(Paragraph {
                             kind: ParagraphKind::Normal,
                             inlines: std::mem::take(&mut current_inlines),
                         }));
                     }
                     blocks.extend(self.parse_blocks_recursive(&child));
                 } else {
                     // Otherwise it's inline content
                     self.parse_inlines(&child, &mut current_inlines);
                 }
             }
         }

         if !current_inlines.is_empty() {
             blocks.push(Block::Paragraph(Paragraph {
                 kind: ParagraphKind::Normal,
                 inlines: current_inlines,
             }));
         }

         blocks
    }

    fn is_container(&self, node: &NodeRef) -> bool {
        if let NodeData::Element(data) = node.data() {
            matches!(
                data.name.local.as_ref(),
                "div" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside" | "body"
            )
        } else {
            false
        }
    }
}

impl DocumentProvider for HtmlProvider {
    fn parse_buffer(&self, data: &[u8]) -> Result<Document, Box<dyn Error + Send + Sync>> {
        let html = std::str::from_utf8(data)?;
        let document_node = parse_html().one(html);

        // Find body
        let body = document_node.select("body")
            .map_err(|_| "Failed to select body")?
            .next()
            .ok_or("No body found")?;

        let blocks = self.parse_blocks_recursive(body.as_node());

        // Extract title
        let title = document_node.select("title")
            .ok()
            .and_then(|mut x| x.next())
            .map(|x| x.text_contents());

        Ok(Document {
            blocks,
            metadata: DocumentMetadata {
                title,
                author: None,
                created: None,
            },
            notes: Vec::new(),
            comments: Vec::new(),
        })
    }

    fn name(&self) -> &'static str {
        "html"
    }
}
