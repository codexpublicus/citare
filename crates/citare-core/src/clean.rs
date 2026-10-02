//! Text cleaning functions for pre-processing legal text.

use regex::Regex;
use scraper::{Html, Node};
use std::sync::OnceLock;
use thiserror::Error;

// ── Regex singletons ──────────────────────────────────────────────────────────

fn re_all_whitespace() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\u{200b}\s]+").unwrap())
}

fn re_inline_whitespace() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[ \t]+").unwrap())
}

fn re_underscores() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"__+").unwrap())
}

fn re_xml() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)^<\?xml.*?\?>").unwrap())
}

// ── Public cleaning functions ─────────────────────────────────────────────────

/// Collapse multiple whitespace chars (including zero-width space U+200B) into
/// a single space.
pub fn all_whitespace(text: &str) -> String {
    re_all_whitespace().replace_all(text, " ").into_owned()
}

/// Collapse multiple spaces or tabs into a single space (newlines preserved).
pub fn inline_whitespace(text: &str) -> String {
    re_inline_whitespace().replace_all(text, " ").into_owned()
}

/// Remove runs of two or more consecutive underscores.
pub fn underscores(text: &str) -> String {
    re_underscores().replace_all(text, "").into_owned()
}

/// Extract visible text from HTML.
///
/// Mirrors eyecite's `clean.html`, which selects every text node with
/// non-blank content (XPath `normalize-space()`) whose *direct* parent is not
/// `<style>`, `<link>`, `<head>`, `<page-number>`, `<script>` or an element
/// with `class="star-pagination"`, and joins those nodes with a single space.
/// Text nodes are kept verbatim (not trimmed), so whitespace around inline
/// tags survives: `In <i>Foo</i>` becomes `"In  Foo"`.
pub fn html(html_content: &str) -> String {
    let document = Html::parse_document(html_content);
    let mut parts: Vec<&str> = Vec::new();

    for node in document.tree.nodes() {
        let Node::Text(ref text) = node.value() else {
            continue;
        };

        // XPath normalize-space() only strips space, tab, CR and LF.
        if text
            .text
            .chars()
            .all(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
        {
            continue;
        }

        let suppressed = node.parent().is_some_and(|parent| match parent.value() {
            Node::Element(elem) => {
                matches!(
                    elem.name(),
                    "style" | "script" | "head" | "link" | "page-number"
                ) || elem.attr("class") == Some("star-pagination")
            }
            _ => false,
        });

        if !suppressed {
            parts.push(&text.text);
        }
    }

    parts.join(" ")
}

/// Remove the XML processing instruction from the start of a string.
pub fn xml(text: &str) -> String {
    re_xml().replace(text, "").into_owned()
}

// ── CleanStep / clean_text ────────────────────────────────────────────────────

/// Error type for [`clean_text`].
#[derive(Debug, Error)]
pub enum CleanError {
    #[error("unknown cleaning step name: {0}")]
    UnknownStep(String),
}

/// A single cleaning step applied by [`clean_text`].
pub enum CleanStep {
    Html,
    InlineWhitespace,
    AllWhitespace,
    Underscores,
    Xml,
    Custom(Box<dyn Fn(&str) -> String + Send + Sync>),
}

impl CleanStep {
    /// Look up a built-in step by name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "html" => Some(Self::Html),
            "inline_whitespace" => Some(Self::InlineWhitespace),
            "all_whitespace" => Some(Self::AllWhitespace),
            "underscores" => Some(Self::Underscores),
            "xml" => Some(Self::Xml),
            _ => None,
        }
    }

    fn apply(&self, text: &str) -> String {
        match self {
            Self::Html => html(text),
            Self::InlineWhitespace => inline_whitespace(text),
            Self::AllWhitespace => all_whitespace(text),
            Self::Underscores => underscores(text),
            Self::Xml => xml(text),
            Self::Custom(f) => f(text),
        }
    }
}

/// Apply a sequence of cleaning steps to `text` and return the result.
pub fn clean_text(text: &str, steps: &[CleanStep]) -> Result<String, CleanError> {
    let mut result = text.to_string();
    for step in steps {
        result = step.apply(&result);
    }
    Ok(result)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_whitespace() {
        assert_eq!(all_whitespace("foo  bar"), "foo bar");
        assert_eq!(all_whitespace("foo\t\nbar"), "foo bar");
        assert_eq!(all_whitespace("foo\u{200b}bar"), "foo bar");
    }

    #[test]
    fn test_inline_whitespace() {
        assert_eq!(inline_whitespace("foo  bar"), "foo bar");
        assert_eq!(inline_whitespace("foo\t\tbar"), "foo bar");
        // newlines are preserved
        assert_eq!(inline_whitespace("foo\nbar"), "foo\nbar");
    }

    #[test]
    fn test_underscores() {
        assert_eq!(underscores("foo__bar"), "foobar");
        assert_eq!(underscores("foo___bar"), "foobar");
        // single underscore is preserved
        assert_eq!(underscores("foo_bar"), "foo_bar");
    }

    #[test]
    fn test_html_extraction() {
        assert_eq!(html("<p>hello</p>"), "hello");
        assert_eq!(html("<p>hello</p><style>css here</style>"), "hello");
        assert_eq!(html("<p>hello</p><script>js here</script>"), "hello");
    }

    #[test]
    fn test_html_star_pagination() {
        let input = r#"<p>visible</p><span class="star-pagination">*5</span><p>also visible</p>"#;
        let result = html(input);
        assert!(result.contains("visible"));
        assert!(result.contains("also visible"));
        assert!(!result.contains("*5"));
    }

    #[test]
    fn test_html_keeps_text_node_whitespace() {
        // Matches eyecite/lxml: text nodes are joined verbatim with " ".
        assert_eq!(
            html("<p>In <i>Foo</i> v. <i>Bar,</i> 1 U.S. 1</p>"),
            "In  Foo  v.  Bar,  1 U.S. 1"
        );
        assert_eq!(html("See <em>Bivens </em>v. x"), "See  Bivens  v. x");
    }

    #[test]
    fn test_html_only_direct_parent_is_suppressed() {
        // eyecite only checks the direct parent and an exact class match.
        assert_eq!(
            html(
                r#"a <span class="star-pagination"><b>*5</b></span> b <span class="x star-pagination">*6</span>"#
            ),
            "a  *5  b  *6"
        );
        assert_eq!(
            html("<html><head><title>t</title></head><body>b</body></html>"),
            "t b"
        );
    }

    #[test]
    fn test_xml_strip() {
        assert_eq!(xml(r#"<?xml version="1.0"?>hello"#), "hello");
        assert_eq!(xml("no xml here"), "no xml here");
    }

    #[test]
    fn test_clean_text_steps() {
        // InlineWhitespace collapses double space; Underscores removes "__"
        let result = clean_text(
            "foo  bar __baz",
            &[CleanStep::InlineWhitespace, CleanStep::Underscores],
        )
        .unwrap();
        assert_eq!(result, "foo bar baz");
    }

    #[test]
    fn test_clean_text_underscores_only() {
        let result = clean_text("foo__bar", &[CleanStep::Underscores]).unwrap();
        assert_eq!(result, "foobar");
    }

    #[test]
    fn test_clean_step_from_name() {
        assert!(CleanStep::from_name("html").is_some());
        assert!(CleanStep::from_name("inline_whitespace").is_some());
        assert!(CleanStep::from_name("bogus").is_none());
    }

    #[test]
    fn test_clean_step_custom() {
        let result = clean_text(
            "hello world",
            &[CleanStep::Custom(Box::new(|s| s.to_uppercase()))],
        )
        .unwrap();
        assert_eq!(result, "HELLO WORLD");
    }
}
