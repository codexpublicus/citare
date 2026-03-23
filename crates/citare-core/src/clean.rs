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

/// Extract visible text from HTML, skipping `<style>`, `<script>`, `<head>`,
/// `<link>`, `<page-number>` elements, and elements with
/// `class="star-pagination"`.
pub fn html(html_content: &str) -> String {
    let document = Html::parse_document(html_content);
    let mut parts: Vec<String> = Vec::new();

    for node in document.tree.nodes() {
        // We only care about text nodes.
        let Node::Text(ref text) = node.value() else {
            continue;
        };

        let content = text.text.trim();
        if content.is_empty() {
            continue;
        }

        // Walk ancestors; skip if any ancestor is a suppressed element.
        let mut suppressed = false;
        let mut ancestor = node.parent();
        while let Some(a) = ancestor {
            if let Node::Element(ref elem) = a.value() {
                let name = elem.name();
                if matches!(name, "style" | "script" | "head" | "link" | "page-number") {
                    suppressed = true;
                    break;
                }
                // class="star-pagination"
                if elem
                    .attr("class")
                    .map(|c| c.split_whitespace().any(|cls| cls == "star-pagination"))
                    .unwrap_or(false)
                {
                    suppressed = true;
                    break;
                }
            }
            ancestor = a.parent();
        }

        if !suppressed {
            parts.push(content.to_string());
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
