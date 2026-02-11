use firecrawl_core::DocumentConverter;

#[test]
fn test_html_to_markdown_code_block() {
    let converter = DocumentConverter::new();
    let html = "<pre><code class='language-rust'>fn main() {}</code></pre>";
    let markdown = converter.convert_html_to_markdown(html).expect("Conversion failed");

    assert!(markdown.contains("```rust"));
    assert!(markdown.contains("fn main() {}"));
    assert!(markdown.contains("```"));
}

#[test]
fn test_html_to_markdown_table_robustness() {
    let converter = DocumentConverter::new();
    // Table with varying cell counts and no header
    let html = "
    <table>
        <tbody>
            <tr><td>Cell 1</td><td>Cell 2</td><td>Cell 3</td></tr>
            <tr><td>Row 2 Cell 1</td></tr>
        </tbody>
    </table>";
    let markdown = converter.convert_html_to_markdown(html).expect("Conversion failed");

    // Should detect 3 columns
    // First row treated as header
    assert!(markdown.contains("| Cell 1 | Cell 2 | Cell 3 |"));
    assert!(markdown.contains("| --- | --- | --- |"));
    // Second row should be padded
    assert!(markdown.contains("| Row 2 Cell 1 | | |"));
}

#[test]
fn test_html_to_markdown_divider() {
    let converter = DocumentConverter::new();
    let html = "<hr>";
    let markdown = converter.convert_html_to_markdown(html).expect("Conversion failed");

    assert!(markdown.contains("---"));
}
