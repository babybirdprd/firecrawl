use firecrawl_core::DocumentConverter;

#[test]
fn test_html_to_markdown_basic() {
    let converter = DocumentConverter::new();
    let html = "<html><head><title>Test Page</title></head><body><p>Hello world</p></body></html>";
    let markdown = converter.convert_html_to_markdown(html).expect("Conversion failed");

    // The renderer adds the title from metadata if present
    assert!(markdown.contains("# Test Page"));
    assert!(markdown.contains("Hello world"));
}

#[test]
fn test_html_to_markdown_headings() {
    let converter = DocumentConverter::new();
    let html = "<h1>Heading 1</h1><h2>Heading 2</h2>";
    let markdown = converter.convert_html_to_markdown(html).expect("Conversion failed");

    assert!(markdown.contains("# Heading 1"));
    assert!(markdown.contains("## Heading 2"));
}

#[test]
fn test_html_to_markdown_list() {
    let converter = DocumentConverter::new();
    let html = "<ul><li>Item 1</li><li>Item 2</li></ul>";
    let markdown = converter.convert_html_to_markdown(html).expect("Conversion failed");

    assert!(markdown.contains("- Item 1"));
    assert!(markdown.contains("- Item 2"));
}

#[test]
fn test_html_to_markdown_formatting() {
    let converter = DocumentConverter::new();
    let html = "<p><strong>Bold</strong> and <em>Italic</em> and <a href='https://example.com'>Link</a></p>";
    let markdown = converter.convert_html_to_markdown(html).expect("Conversion failed");

    assert!(markdown.contains("**Bold**"));
    assert!(markdown.contains("*Italic*"));
    assert!(markdown.contains("[Link](https://example.com)"));
}

#[test]
fn test_html_to_markdown_table() {
    let converter = DocumentConverter::new();
    let html = "<table><thead><tr><th>Head 1</th><th>Head 2</th></tr></thead><tbody><tr><td>Cell 1</td><td>Cell 2</td></tr></tbody><tfoot><tr><td>Foot 1</td><td>Foot 2</td></tr></tfoot></table>";
    let markdown = converter.convert_html_to_markdown(html).expect("Conversion failed");

    assert!(markdown.contains("| Head 1 | Head 2 |"));
    assert!(markdown.contains("| --- | --- |"));
    assert!(markdown.contains("| Cell 1 | Cell 2 |"));
    assert!(markdown.contains("| Foot 1 | Foot 2 |"));
}

#[test]
fn test_html_to_markdown_image() {
    let converter = DocumentConverter::new();
    let html = "<img src='image.png' alt='An image'>";
    let markdown = converter.convert_html_to_markdown(html).expect("Conversion failed");

    assert!(markdown.contains("![An image](image.png)"));
}

#[test]
fn test_html_to_markdown_nested_list() {
    let converter = DocumentConverter::new();
    let html = "
    <ul>
        <li>Item 1
            <ul>
                <li>SubItem A</li>
            </ul>
        </li>
        <li>Item 2</li>
    </ul>";
    let markdown = converter.convert_html_to_markdown(html).expect("Conversion failed");

    // Check for indentation
    assert!(markdown.contains("- Item 1"));
    assert!(markdown.contains("  - SubItem A"));
    assert!(markdown.contains("- Item 2"));
}
