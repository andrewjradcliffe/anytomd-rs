//! Shared model and rendering for extracted document comments.
//!
//! DOCX and PPTX converters collect comments into [`Comment`] values during
//! parsing (gated by `ConversionOptions::extract_comments`). After the document
//! body is rendered, [`append_comments`] appends a `# Comments` section to both
//! the Markdown and plain-text output.
//!
//! The rendered Markdown structure is:
//!
//! ```text
//! # Comments
//!
//! ## 1
//! - **author**: <identity> (<date>)
//! - **comment**: <body>
//! - **source**: <commented-on text>
//! ```
//!
//! The plain-text form mirrors the same layout with all Markdown markers
//! stripped (`Comments` / `1` / `author:` / `comment:` / `source:`).

/// A single extracted comment, ready for rendering.
///
/// Field values are already normalized by the converter that produced them
/// (whitespace collapsed, `source` capped, `author` formatted). The renderer
/// does not transform them further except for the reply prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Comment {
    /// Commenter identity, pre-formatted as `Name` or `Name (date)`.
    pub(crate) author: String,
    /// Comment body text (plain, whitespace-collapsed, uncapped).
    pub(crate) body: String,
    /// The text the comment was anchored to (DOCX range, capped) or a slide
    /// label (PPTX). May be empty.
    pub(crate) source: String,
    /// Whether this comment is a reply within a thread.
    pub(crate) is_reply: bool,
}

/// Maximum number of characters retained for a comment's `source` text.
pub(crate) const SOURCE_CAP: usize = 200;

/// Collapse all runs of whitespace (spaces, tabs, newlines) into single spaces
/// and trim the ends.
///
/// Used to flatten multi-paragraph comment bodies and ranged source text into a
/// single line suitable for a Markdown list item.
pub(crate) fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Truncate `s` to at most `max_chars` Unicode scalar values, appending `…`
/// when truncation occurs.
///
/// Truncation always lands on a character boundary, so multi-byte characters
/// (CJK, emoji) are never split mid-byte.
pub(crate) fn cap_text(s: &str, max_chars: usize) -> String {
    // `nth(max_chars)` yields the byte index of the character just past the cap
    // (0-indexed), which is exactly the truncation point and always a char
    // boundary. `None` means the string is within the cap.
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => {
            let mut out = String::with_capacity(idx + '…'.len_utf8());
            out.push_str(&s[..idx]);
            out.push('…');
            out
        }
        None => s.to_string(),
    }
}

/// Format the `author` field from a raw author name and date string.
///
/// - Empty author becomes `Unknown`.
/// - A present date is emitted verbatim in parentheses (no validation).
/// - An empty date yields the author name alone (no empty parentheses).
pub(crate) fn format_author(author: &str, date: &str) -> String {
    let author = author.trim();
    let author = if author.is_empty() { "Unknown" } else { author };
    let date = date.trim();
    if date.is_empty() {
        author.to_string()
    } else {
        format!("{author} ({date})")
    }
}

/// Render the comment body for display, prefixing replies with `(reply)`.
fn body_display(c: &Comment) -> String {
    if c.is_reply {
        if c.body.is_empty() {
            "(reply)".to_string()
        } else {
            format!("(reply) {}", c.body)
        }
    } else {
        c.body.clone()
    }
}

/// Render comments as a Markdown `# Comments` section.
fn render_comments_md(comments: &[Comment]) -> String {
    let mut out = String::from("# Comments\n");
    for (i, c) in comments.iter().enumerate() {
        out.push_str(&format!("\n## {}\n", i + 1));
        out.push_str(&format!("- **author**: {}\n", c.author));
        out.push_str(&format!("- **comment**: {}\n", body_display(c)));
        out.push_str(&format!("- **source**: {}\n", c.source));
    }
    out
}

/// Render comments as a plain-text section (Markdown markers stripped).
fn render_comments_plain(comments: &[Comment]) -> String {
    let mut out = String::from("Comments\n");
    for (i, c) in comments.iter().enumerate() {
        out.push_str(&format!("\n{}\n", i + 1));
        out.push_str(&format!("author: {}\n", c.author));
        out.push_str(&format!("comment: {}\n", body_display(c)));
        out.push_str(&format!("source: {}\n", c.source));
    }
    out
}

/// Append a rendered section to a buffer, separated by one blank line.
///
/// If the buffer has no meaningful content, it is replaced by the section
/// (no leading blank lines). Otherwise trailing whitespace is trimmed and a
/// single blank line is inserted before the section.
fn append_section(buf: &mut String, section: &str) {
    let keep = buf.trim_end().len();
    if keep == 0 {
        *buf = section.to_string();
        return;
    }
    buf.truncate(keep);
    buf.push_str("\n\n");
    buf.push_str(section);
}

/// Append the `# Comments` section to both Markdown and plain-text output.
///
/// No-op when `comments` is empty (the section is omitted entirely).
pub(crate) fn append_comments(
    markdown: &mut String,
    plain_text: &mut String,
    comments: &[Comment],
) {
    if comments.is_empty() {
        return;
    }
    append_section(markdown, &render_comments_md(comments));
    append_section(plain_text, &render_comments_plain(comments));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(author: &str, body: &str, source: &str, is_reply: bool) -> Comment {
        Comment {
            author: author.to_string(),
            body: body.to_string(),
            source: source.to_string(),
            is_reply,
        }
    }

    // ---- collapse_ws ----

    #[test]
    fn test_collapse_ws_newlines_and_tabs_to_space() {
        assert_eq!(collapse_ws("a\nb\tc"), "a b c");
    }

    #[test]
    fn test_collapse_ws_runs_collapsed_and_trimmed() {
        assert_eq!(collapse_ws("  a   \n\n  b  "), "a b");
    }

    #[test]
    fn test_collapse_ws_empty() {
        assert_eq!(collapse_ws("   \n\t "), "");
    }

    #[test]
    fn test_collapse_ws_cjk_preserved() {
        assert_eq!(collapse_ws("한국어\n中文"), "한국어 中文");
    }

    // ---- cap_text ----

    #[test]
    fn test_cap_text_under_limit_unchanged() {
        assert_eq!(cap_text("hello", 200), "hello");
    }

    #[test]
    fn test_cap_text_exact_limit_unchanged() {
        let s = "a".repeat(200);
        assert_eq!(cap_text(&s, 200), s);
    }

    #[test]
    fn test_cap_text_over_limit_truncated_with_ellipsis() {
        let s = "a".repeat(201);
        let out = cap_text(&s, 200);
        assert_eq!(out.chars().count(), 201); // 200 + ellipsis
        assert!(out.ends_with('…'));
        assert_eq!(&out[..200], &"a".repeat(200));
    }

    #[test]
    fn test_cap_text_cjk_not_split_mid_char() {
        // 250 CJK chars (3 bytes each) — must truncate on a char boundary.
        let s = "한".repeat(250);
        let out = cap_text(&s, 200);
        assert_eq!(out.chars().count(), 201);
        assert!(out.ends_with('…'));
        // Round-trips as valid UTF-8 (no panic on slice) and is all 한 + …
        assert_eq!(out.chars().filter(|&ch| ch == '한').count(), 200);
    }

    // ---- format_author ----

    #[test]
    fn test_format_author_name_and_date() {
        assert_eq!(
            format_author("Jane Smith", "2024-01-15T09:30:00Z"),
            "Jane Smith (2024-01-15T09:30:00Z)"
        );
    }

    #[test]
    fn test_format_author_empty_date_name_only() {
        assert_eq!(format_author("Jane Smith", ""), "Jane Smith");
        assert_eq!(format_author("Jane Smith", "   "), "Jane Smith");
    }

    #[test]
    fn test_format_author_empty_author_unknown() {
        assert_eq!(format_author("", ""), "Unknown");
        assert_eq!(format_author("   ", ""), "Unknown");
    }

    #[test]
    fn test_format_author_unknown_with_date() {
        assert_eq!(format_author("", "2024-01-15"), "Unknown (2024-01-15)");
    }

    // ---- body_display ----

    #[test]
    fn test_body_display_non_reply() {
        assert_eq!(body_display(&c("A", "Hello", "src", false)), "Hello");
    }

    #[test]
    fn test_body_display_reply_prefixed() {
        assert_eq!(body_display(&c("A", "Agreed.", "src", true)), "(reply) Agreed.");
    }

    #[test]
    fn test_body_display_reply_empty_body() {
        assert_eq!(body_display(&c("A", "", "src", true)), "(reply)");
    }

    // ---- render_comments_md ----

    #[test]
    fn test_render_comments_md_matches_spec() {
        let comments = vec![
            c(
                "Jane Smith (2024-01-15T09:30:00Z)",
                "Please revise this.",
                "the quick brown fox",
                false,
            ),
            c("Unknown", "Agreed.", "jumped over", true),
        ];
        let expected = "# Comments\n\
            \n## 1\n\
            - **author**: Jane Smith (2024-01-15T09:30:00Z)\n\
            - **comment**: Please revise this.\n\
            - **source**: the quick brown fox\n\
            \n## 2\n\
            - **author**: Unknown\n\
            - **comment**: (reply) Agreed.\n\
            - **source**: jumped over\n";
        assert_eq!(render_comments_md(&comments), expected);
    }

    // ---- render_comments_plain ----

    #[test]
    fn test_render_comments_plain_matches_layout() {
        let comments = vec![
            c(
                "Jane Smith (2024-01-15T09:30:00Z)",
                "Please revise this.",
                "the quick brown fox",
                false,
            ),
            c("Unknown", "Agreed.", "jumped over", true),
        ];
        let expected = "Comments\n\
            \n1\n\
            author: Jane Smith (2024-01-15T09:30:00Z)\n\
            comment: Please revise this.\n\
            source: the quick brown fox\n\
            \n2\n\
            author: Unknown\n\
            comment: (reply) Agreed.\n\
            source: jumped over\n";
        assert_eq!(render_comments_plain(&comments), expected);
        // Plain text must not contain Markdown markers.
        assert!(!render_comments_plain(&comments).contains('#'));
        assert!(!render_comments_plain(&comments).contains('*'));
    }

    // ---- append_comments ----

    #[test]
    fn test_append_comments_empty_is_noop() {
        let mut md = "body\n".to_string();
        let mut plain = "body\n".to_string();
        append_comments(&mut md, &mut plain, &[]);
        assert_eq!(md, "body\n");
        assert_eq!(plain, "body\n");
    }

    #[test]
    fn test_append_comments_separated_by_blank_line() {
        let mut md = "# Title\n\nSome body.\n".to_string();
        let mut plain = "Some body.\n".to_string();
        append_comments(&mut md, &mut plain, &[c("A", "B", "C", false)]);
        assert!(md.contains("Some body.\n\n# Comments\n"));
        assert!(plain.contains("Some body.\n\nComments\n"));
        assert!(md.ends_with("- **source**: C\n"));
    }

    #[test]
    fn test_append_comments_empty_body_no_leading_blank() {
        // Comments-only document: body empty -> section with no leading blank lines.
        let mut md = String::new();
        let mut plain = String::new();
        append_comments(&mut md, &mut plain, &[c("A", "B", "C", false)]);
        assert!(md.starts_with("# Comments\n"));
        assert!(plain.starts_with("Comments\n"));
    }

    #[test]
    fn test_append_comments_whitespace_only_body_treated_as_empty() {
        let mut md = "\n\n".to_string();
        let mut plain = "  \n".to_string();
        append_comments(&mut md, &mut plain, &[c("A", "B", "C", false)]);
        assert!(md.starts_with("# Comments\n"));
        assert!(plain.starts_with("Comments\n"));
    }
}
