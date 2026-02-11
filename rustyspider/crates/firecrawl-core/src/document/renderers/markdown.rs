use crate::document::model::*;

pub struct MarkdownRenderer;

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, document: &Document) -> String {
        let mut out = String::new();

        // Render title if present
        if let Some(title) = &document.metadata.title {
            if !title.trim().is_empty() {
                out.push_str(&format!("# {}\n\n", title.trim()));
            }
        }

        self.render_blocks(&document.blocks, &mut out);

        out.trim().to_string()
    }

    fn render_blocks(&self, blocks: &[Block], out: &mut String) {
        for (i, block) in blocks.iter().enumerate() {
            self.render_block(block, out);
            if i < blocks.len() - 1 {
                out.push('\n');
            }
        }
    }

    fn render_block(&self, block: &Block, out: &mut String) {
        match block {
            Block::Paragraph(p) => {
                self.render_paragraph(p, out);
                out.push('\n');
            }
            Block::List(l) => {
                self.render_list(l, 0, out);
                out.push('\n');
            }
            Block::Table(t) => {
                self.render_table(t, out);
                out.push('\n');
            }
            Block::Image(i) => {
                self.render_image(i, out);
                out.push('\n');
            }
        }
    }

    fn render_paragraph(&self, p: &Paragraph, out: &mut String) {
        match p.kind {
            ParagraphKind::Normal => {
                self.render_inlines(&p.inlines, out);
            }
            ParagraphKind::Heading(level) => {
                let hashes = "#".repeat(level as usize);
                out.push_str(&hashes);
                out.push(' ');
                self.render_inlines(&p.inlines, out);
            }
            ParagraphKind::Blockquote => {
                out.push_str("> ");
                self.render_inlines(&p.inlines, out);
            }
        }
    }

    fn render_list(&self, l: &List, indent: usize, out: &mut String) {
        for (i, item) in l.items.iter().enumerate() {
            let prefix = match l.list_type {
                ListType::Unordered => "- ".to_string(),
                ListType::Ordered => format!("{}. ", i + 1),
            };

            let indent_str = "  ".repeat(indent);
            out.push_str(&indent_str);
            out.push_str(&prefix);

            // Render first block of item inline with bullet
            if let Some(first_block) = item.blocks.first() {
                // If it's a paragraph, render its content directly
                if let Block::Paragraph(p) = first_block {
                    self.render_inlines(&p.inlines, out);
                } else {
                    // Fallback for other blocks
                    self.render_block(first_block, out);
                }
            }
            out.push('\n');

            // Render remaining blocks indented
            for block in item.blocks.iter().skip(1) {
                // Nested lists handling
                 if let Block::List(nested_list) = block {
                     self.render_list(nested_list, indent + 1, out);
                 } else {
                     // Indent other blocks
                     // This is tricky in markdown, usually just indenting works
                     let block_indent = "  ".repeat(indent + 1);
                     let mut block_out = String::new();
                     self.render_block(block, &mut block_out);
                     for line in block_out.lines() {
                         out.push_str(&block_indent);
                         out.push_str(line);
                         out.push('\n');
                     }
                 }
            }
        }
    }

    fn render_table(&self, t: &Table, out: &mut String) {
        // Simple GFM table renderer
        // Find header row
        let header_row = t.rows.iter().find(|r| matches!(r.kind, TableRowKind::Header));
        let body_rows: Vec<&TableRow> = t.rows.iter().filter(|r| matches!(r.kind, TableRowKind::Body)).collect();
        let footer_rows: Vec<&TableRow> = t.rows.iter().filter(|r| matches!(r.kind, TableRowKind::Footer)).collect();

        if let Some(header) = header_row {
            self.render_table_row(header, out);
            // Separator line
            out.push('|');
            for _ in &header.cells {
                out.push_str(" --- |");
            }
            out.push('\n');
        }

        for row in body_rows {
            self.render_table_row(row, out);
        }

        for row in footer_rows {
            self.render_table_row(row, out);
        }
    }

    fn render_table_row(&self, row: &TableRow, out: &mut String) {
        out.push('|');
        for cell in &row.cells {
            out.push(' ');
            // Flatten blocks in cell to single line string
            let mut cell_content = String::new();
            for block in &cell.blocks {
                if let Block::Paragraph(p) = block {
                    self.render_inlines(&p.inlines, &mut cell_content);
                } else {
                    // fallback
                     self.render_block(block, &mut cell_content);
                }
                cell_content.push(' ');
            }
            out.push_str(cell_content.trim());
            out.push_str(" |");
        }
        out.push('\n');
    }

    fn render_image(&self, i: &Image, out: &mut String) {
        let alt = i.alt.as_deref().unwrap_or("");
        out.push_str(&format!("![{}]({})", alt, i.src));
    }

    fn render_inlines(&self, inlines: &[Inline], out: &mut String) {
        for inline in inlines {
            self.render_inline(inline, out);
        }
    }

    fn render_inline(&self, inline: &Inline, out: &mut String) {
        match inline {
            Inline::Text(t) => out.push_str(t),
            Inline::LineBreak => out.push_str("  \n"),
            Inline::Link { href, children } => {
                out.push('[');
                self.render_inlines(children, out);
                out.push_str(&format!("]({})", href));
            }
            Inline::Strong(children) => {
                out.push_str("**");
                self.render_inlines(children, out);
                out.push_str("**");
            }
            Inline::Em(children) => {
                out.push('*');
                self.render_inlines(children, out);
                out.push('*');
            }
            Inline::Del(children) => {
                out.push_str("~~");
                self.render_inlines(children, out);
                out.push_str("~~");
            }
            Inline::Code(code) => {
                out.push('`');
                out.push_str(code);
                out.push('`');
            }
            Inline::Sup(children) => {
                out.push('^');
                self.render_inlines(children, out);
            }
            Inline::Sub(children) => {
                out.push('~');
                self.render_inlines(children, out);
            }
            Inline::FootnoteRef(id) => {
                out.push_str(&format!("[^{}]", id.0));
            }
            Inline::EndnoteRef(id) => {
                out.push_str(&format!("[^{}]", id.0));
            }
            _ => {} // Ignore others for now
        }
    }
}
