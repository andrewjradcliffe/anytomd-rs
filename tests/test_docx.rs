#![cfg(not(target_arch = "wasm32"))]

mod common;

use anytomd::{ConversionOptions, convert_file};
use common::normalize;
use std::io::Cursor;

/// Content coverage test: verify key elements are present in the converted output.
///
/// Fixture: tests/fixtures/sample.docx
/// Contains: H1 "Sample Document", body paragraph, H2 "Section One",
///           paragraph with hyperlink to example.com, Korean text, emoji, H3 "Subsection".
#[test]
fn test_docx_convert_file_sample() {
    let result = convert_file("tests/fixtures/sample.docx", &ConversionOptions::default()).unwrap();

    // Title should be extracted from first H1
    assert_eq!(result.title, Some("Sample Document".to_string()));

    // Headings
    assert!(result.markdown.contains("# Sample Document"));
    assert!(result.markdown.contains("## Section One"));
    assert!(result.markdown.contains("### Subsection"));

    // Body paragraphs
    assert!(result.markdown.contains("This is a simple paragraph."));
    assert!(
        result
            .markdown
            .contains("Final paragraph with mixed content.")
    );

    // Hyperlink
    assert!(result.markdown.contains("[Example](https://example.com)"));

    // Unicode: Korean
    assert!(result.markdown.contains("한국어 테스트"));

    // Emoji
    assert!(result.markdown.contains("🚀"));
    assert!(result.markdown.contains("✨"));
    assert!(result.markdown.contains("🌍"));
}

/// Golden test: normalized comparison against expected output file.
#[test]
fn test_docx_golden_sample() {
    let result = convert_file("tests/fixtures/sample.docx", &ConversionOptions::default()).unwrap();
    let expected = include_str!("fixtures/expected/sample.docx.md");
    assert_eq!(normalize(&result.markdown), normalize(expected));
}

/// Direct convert_bytes test: verify the converter works with explicit extension.
#[test]
fn test_docx_convert_bytes_direct() {
    let data = std::fs::read("tests/fixtures/sample.docx").unwrap();
    let result = anytomd::convert_bytes(&data, "docx", &ConversionOptions::default()).unwrap();
    assert!(result.markdown.contains("# Sample Document"));
    assert!(result.markdown.contains("한국어"));
}

/// Integration test: text box extraction via mc:AlternateContent > Fallback > w:pict.
///
/// Builds a DOCX in memory with a normal paragraph, a text box (via mc:AlternateContent),
/// and another normal paragraph. Verifies text box content is extracted and document flow
/// is preserved.
#[test]
fn test_docx_textbox_integration() {
    use std::io::Write;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let buf = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(buf));
    let opts = SimpleFileOptions::default();

    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
    )
    .unwrap();

    // _rels/.rels
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
    )
    .unwrap();

    // word/document.xml — paragraph + text box via mc:AlternateContent + paragraph
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:v="urn:schemas-microsoft-com:vml"><w:body><w:p><w:r><w:t>Normal paragraph before.</w:t></w:r></w:p><mc:AlternateContent><mc:Choice Requires="wps"><w:p><w:r><w:t>DrawingML (hidden)</w:t></w:r></w:p></mc:Choice><mc:Fallback><w:p><w:r><w:pict><v:shape><v:textbox><w:txbxContent><w:p><w:r><w:t>Text box content here</w:t></w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict></w:r></w:p></mc:Fallback></mc:AlternateContent><w:p><w:r><w:t>Normal paragraph after.</w:t></w:r></w:p></w:body></w:document>"#;
    zip.start_file("word/document.xml", opts).unwrap();
    zip.write_all(document_xml.as_bytes()).unwrap();

    let cursor = zip.finish().unwrap();
    let data = cursor.into_inner();

    let result = anytomd::convert_bytes(&data, "docx", &ConversionOptions::default()).unwrap();
    assert!(
        result.markdown.contains("Normal paragraph before."),
        "markdown was: {}",
        result.markdown
    );
    assert!(
        result.markdown.contains("Text box content here"),
        "markdown was: {}",
        result.markdown
    );
    assert!(
        result.markdown.contains("Normal paragraph after."),
        "markdown was: {}",
        result.markdown
    );
    assert!(
        !result.markdown.contains("DrawingML (hidden)"),
        "Choice content should be skipped, markdown was: {}",
        result.markdown
    );
}

/// Build a DOCX fixture with text boxes for testing.
///
/// Contains:
/// - H1 "Text Box Test Document"
/// - Normal paragraph before text box
/// - mc:AlternateContent with text box in Fallback (bold + unicode + emoji)
/// - Normal paragraph after text box
/// - Direct w:pict text box (italic)
/// - Final paragraph
fn build_textbox_docx() -> Vec<u8> {
    use std::io::Write;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let buf = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(buf));
    let opts = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
    ).unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
    ).unwrap();

    let document_xml = concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#,
        r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#,
        r#" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006""#,
        r#" xmlns:v="urn:schemas-microsoft-com:vml""#,
        r#" xmlns:o="urn:schemas-microsoft-com:office:office">"#,
        r#"<w:body>"#,
        // H1
        r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Text Box Test Document</w:t></w:r></w:p>"#,
        // Normal paragraph
        r#"<w:p><w:r><w:t xml:space="preserve">This paragraph comes before the text box.</w:t></w:r></w:p>"#,
        // mc:AlternateContent with text box in Fallback
        r#"<mc:AlternateContent>"#,
        r#"<mc:Choice Requires="wps"><w:p><w:r><w:t>DrawingML content that should be hidden</w:t></w:r></w:p></mc:Choice>"#,
        r#"<mc:Fallback><w:p><w:r><w:pict>"#,
        r#"<v:shape id="tb1" style="width:200pt;height:50pt"><v:textbox><w:txbxContent>"#,
        r#"<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Important notice in text box</w:t></w:r></w:p>"#,
        r#"<w:p><w:r><w:t xml:space="preserve">Second paragraph with 한국어 and 🚀 emoji.</w:t></w:r></w:p>"#,
        r#"</w:txbxContent></v:textbox></v:shape>"#,
        r#"</w:pict></w:r></w:p></mc:Fallback>"#,
        r#"</mc:AlternateContent>"#,
        // Normal paragraph after
        r#"<w:p><w:r><w:t xml:space="preserve">This paragraph comes after the text box.</w:t></w:r></w:p>"#,
        // Direct w:pict text box (italic)
        r#"<w:p><w:r><w:pict><v:rect style="width:150pt;height:30pt"><v:textbox><w:txbxContent>"#,
        r#"<w:p><w:r><w:rPr><w:i/></w:rPr><w:t>Direct VML text box content</w:t></w:r></w:p>"#,
        r#"</w:txbxContent></v:textbox></v:rect></w:pict></w:r></w:p>"#,
        // Final paragraph
        r#"<w:p><w:r><w:t>Final paragraph of the document.</w:t></w:r></w:p>"#,
        r#"</w:body></w:document>"#,
    );

    let styles_xml = r#"<?xml version="1.0" encoding="UTF-8"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/></w:style></w:styles>"#;

    zip.start_file("word/document.xml", opts).unwrap();
    zip.write_all(document_xml.as_bytes()).unwrap();

    zip.start_file("word/styles.xml", opts).unwrap();
    zip.write_all(styles_xml.as_bytes()).unwrap();

    let cursor = zip.finish().unwrap();
    cursor.into_inner()
}

/// Integration test: text box extraction from a programmatic DOCX fixture.
///
/// Tests the full end-to-end pipeline via `convert_bytes` with a realistic DOCX
/// containing heading, normal paragraphs, mc:AlternateContent text boxes, direct
/// VML text boxes, bold/italic formatting, Unicode, and emoji.
#[test]
fn test_docx_textbox_convert_file() {
    let data = build_textbox_docx();
    let result = anytomd::convert_bytes(&data, "docx", &ConversionOptions::default()).unwrap();

    // Heading
    assert!(
        result.markdown.contains("# Text Box Test Document"),
        "markdown was: {}",
        result.markdown
    );

    // Normal paragraphs preserved
    assert!(
        result
            .markdown
            .contains("This paragraph comes before the text box."),
        "markdown was: {}",
        result.markdown
    );
    assert!(
        result
            .markdown
            .contains("This paragraph comes after the text box."),
        "markdown was: {}",
        result.markdown
    );
    assert!(
        result.markdown.contains("Final paragraph of the document."),
        "markdown was: {}",
        result.markdown
    );

    // Text box content (via mc:AlternateContent > Fallback)
    assert!(
        result.markdown.contains("**Important notice in text box**"),
        "bold text box content missing, markdown was: {}",
        result.markdown
    );
    assert!(
        result.markdown.contains("한국어"),
        "Korean text in text box missing, markdown was: {}",
        result.markdown
    );
    assert!(
        result.markdown.contains("🚀"),
        "emoji in text box missing, markdown was: {}",
        result.markdown
    );

    // Direct w:pict text box
    assert!(
        result.markdown.contains("*Direct VML text box content*"),
        "italic direct text box content missing, markdown was: {}",
        result.markdown
    );

    // Choice content must NOT appear
    assert!(
        !result
            .markdown
            .contains("DrawingML content that should be hidden"),
        "mc:Choice content should be hidden, markdown was: {}",
        result.markdown
    );

    // Title
    assert_eq!(result.title, Some("Text Box Test Document".to_string()),);
}

/// End-to-end comment extraction through the public `convert_bytes` API.
///
/// Builds a DOCX in memory with a commented range and a `word/comments.xml`,
/// then verifies the appended `# Comments` section in both markdown and plain
/// text, and that it is absent without the flag.
#[test]
fn test_docx_extract_comments_end_to_end() {
    use std::io::Write;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let buf = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(buf));
    let opts = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
    )
    .unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
    )
    .unwrap();

    // Body: a sentence with a commented range (id 1).
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t xml:space="preserve">The report is due </w:t></w:r><w:commentRangeStart w:id="1"/><w:r><w:t>next Friday</w:t></w:r><w:commentRangeEnd w:id="1"/><w:r><w:commentReference w:id="1"/></w:r><w:r><w:t>.</w:t></w:r></w:p></w:body></w:document>"#;
    zip.start_file("word/document.xml", opts).unwrap();
    zip.write_all(document_xml.as_bytes()).unwrap();

    let comments_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:comment w:id="1" w:author="Reviewer" w:date="2024-03-01T12:00:00Z"><w:p><w:r><w:t>Can we move this up a week?</w:t></w:r></w:p></w:comment></w:comments>"#;
    zip.start_file("word/comments.xml", opts).unwrap();
    zip.write_all(comments_xml.as_bytes()).unwrap();

    let data = zip.finish().unwrap().into_inner();

    // Without the flag: no Comments section.
    let plain_result =
        anytomd::convert_bytes(&data, "docx", &ConversionOptions::default()).unwrap();
    assert!(!plain_result.markdown.contains("# Comments"));
    assert!(
        plain_result
            .markdown
            .contains("The report is due next Friday.")
    );

    // With the flag: appended section in markdown and plain text.
    let options = ConversionOptions {
        extract_comments: true,
        ..Default::default()
    };
    let result = anytomd::convert_bytes(&data, "docx", &options).unwrap();
    assert!(
        result.markdown.contains("# Comments"),
        "md: {}",
        result.markdown
    );
    assert!(result.markdown.contains("## 1"));
    assert!(
        result
            .markdown
            .contains("- **author**: Reviewer (2024-03-01T12:00:00Z)")
    );
    assert!(
        result
            .markdown
            .contains("- **comment**: Can we move this up a week?")
    );
    assert!(result.markdown.contains("- **source**: next Friday"));
    // Body still present and the section is appended at the end.
    let body = result.markdown.find("The report is due").unwrap();
    let section = result.markdown.find("# Comments").unwrap();
    assert!(body < section);
    // Plain text mirrors the layout with markers stripped.
    assert!(result.plain_text.contains("Comments\n"));
    assert!(
        result
            .plain_text
            .contains("author: Reviewer (2024-03-01T12:00:00Z)")
    );
    assert!(
        result
            .plain_text
            .contains("comment: Can we move this up a week?")
    );
    assert!(result.plain_text.contains("source: next Friday"));
    assert!(!result.plain_text.contains("# Comments"));
}

/// Build a minimal DOCX in memory from a document.xml body and optional styles.
fn build_docx_from_body(body: &str, styles_xml: Option<&str>) -> Vec<u8> {
    use std::io::Write;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let buf = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(buf));
    let opts = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
    )
    .unwrap();

    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
    )
    .unwrap();

    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
    );
    zip.start_file("word/document.xml", opts).unwrap();
    zip.write_all(document_xml.as_bytes()).unwrap();

    if let Some(styles) = styles_xml {
        zip.start_file("word/styles.xml", opts).unwrap();
        zip.write_all(styles.as_bytes()).unwrap();
    }

    zip.finish().unwrap().into_inner()
}

/// End-to-end test for a merged/layout table converted via the hybrid path.
///
/// The synthetic document mirrors the structure of a layout table that uses
/// `gridSpan`/`vMerge` for sectioning rather than tabular data: a full-width,
/// heading-styled banner row; a narrow-label + wide-spanning-value row; and a
/// two-row data grid with a vertically merged label column, plus an
/// authoritative `<w:tblGrid>` declaring four columns.
///
/// The previous converter collapsed all of this to a single column; the hybrid
/// renderer must preserve every column and linearize the layout rows.
#[test]
fn test_docx_layout_table_hybrid_integration() {
    let styles = r#"<?xml version="1.0" encoding="UTF-8"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/></w:style></w:styles>"#;

    // 4-column grid. Rows: banner (span4, Heading2), label/value (1 + span3),
    // data grid header (vMerge restart label + 3 cols), data row (vMerge continue + 3 cols).
    let body = concat!(
        r#"<w:tbl>"#,
        r#"<w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid>"#,
        // Banner row, heading-styled
        r#"<w:tr><w:tc><w:tcPr><w:gridSpan w:val="4"/></w:tcPr><w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>Section Title</w:t></w:r></w:p></w:tc></w:tr>"#,
        // Label/value row
        r#"<w:tr><w:tc><w:p><w:r><w:t>Field</w:t></w:r></w:p></w:tc><w:tc><w:tcPr><w:gridSpan w:val="3"/></w:tcPr><w:p><w:r><w:t>Value</w:t></w:r></w:p></w:tc></w:tr>"#,
        // Data grid header: vMerge restart label + 3 data columns
        r#"<w:tr><w:tc><w:tcPr><w:vMerge w:val="restart"/></w:tcPr><w:p><w:r><w:t>Info</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Col A</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Col B</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Col C</w:t></w:r></w:p></w:tc></w:tr>"#,
        // Data row: vMerge continue + 3 values
        r#"<w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc><w:tc><w:p><w:r><w:t>1</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>2</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>3</w:t></w:r></w:p></w:tc></w:tr>"#,
        r#"</w:tbl>"#,
    );

    let data = build_docx_from_body(body, Some(styles));
    let result = anytomd::convert_bytes(&data, "docx", &ConversionOptions::default()).unwrap();

    // Banner row → heading.
    assert!(
        result.markdown.contains("## Section Title"),
        "banner not promoted to heading: {}",
        result.markdown
    );
    // Label/value row → bold label + value.
    assert!(
        result.markdown.contains("**Field:** Value"),
        "label/value not linearized: {}",
        result.markdown
    );
    // Data grid → all columns preserved.
    assert!(
        result.markdown.contains("| Info | Col A | Col B | Col C |"),
        "grid header columns missing: {}",
        result.markdown
    );
    assert!(
        result.markdown.contains("| 2 | 3 |"),
        "grid data columns missing: {}",
        result.markdown
    );
    // The vMerge continuation cell is empty in the data row.
    assert!(
        result.markdown.contains("|  | 1 | 2 | 3 |"),
        "vMerge continuation not empty: {}",
        result.markdown
    );

    // Plain text linearizes without markdown markers but keeps grid tabs.
    assert!(result.plain_text.contains("Section Title"));
    assert!(result.plain_text.contains("Field"));
    assert!(!result.plain_text.contains("**"));
    assert!(result.plain_text.contains("Col A\tCol B\tCol C"));
}

/// Golden test: the merged/layout table renders to a stable, fully-preserved
/// Markdown layout. Update the expected file with a documented reason if the
/// hybrid rendering intentionally changes.
#[test]
fn test_docx_layout_table_golden() {
    let styles = r#"<?xml version="1.0" encoding="UTF-8"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/></w:style></w:styles>"#;
    let body = concat!(
        r#"<w:tbl>"#,
        r#"<w:tblGrid><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/><w:gridCol w:w="2000"/></w:tblGrid>"#,
        r#"<w:tr><w:tc><w:tcPr><w:gridSpan w:val="4"/></w:tcPr><w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>Section Title</w:t></w:r></w:p></w:tc></w:tr>"#,
        r#"<w:tr><w:tc><w:p><w:r><w:t>Field</w:t></w:r></w:p></w:tc><w:tc><w:tcPr><w:gridSpan w:val="3"/></w:tcPr><w:p><w:r><w:t>Value</w:t></w:r></w:p></w:tc></w:tr>"#,
        r#"<w:tr><w:tc><w:tcPr><w:vMerge w:val="restart"/></w:tcPr><w:p><w:r><w:t>Info</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Col A</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Col B</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Col C</w:t></w:r></w:p></w:tc></w:tr>"#,
        r#"<w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc><w:tc><w:p><w:r><w:t>1</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>2</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>3</w:t></w:r></w:p></w:tc></w:tr>"#,
        r#"</w:tbl>"#,
    );
    let data = build_docx_from_body(body, Some(styles));
    let result = anytomd::convert_bytes(&data, "docx", &ConversionOptions::default()).unwrap();
    let expected = include_str!("fixtures/expected/layout_table.docx.md");
    assert_eq!(normalize(&result.markdown), normalize(expected));
}
