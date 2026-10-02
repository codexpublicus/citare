use std::sync::OnceLock;

use regex::Regex;

use super::token::{Token, Tokens};
use crate::annotate::SpanUpdater;
use crate::clean::{clean_text, html, CleanStep};
use crate::tokenizer::AhocorasickTokenizer;
use crate::utils::placeholder_markup;

/// Represents a document being processed for citation extraction.
///
/// Ported from eyecite's `models.Document`. All offsets (citation spans,
/// span maps and `emphasis_tags`) are UTF-8 byte offsets.
#[derive(Debug, Clone)]
pub struct Document {
    /// Plain text of the document (markup stripped).
    pub plain_text: String,
    /// Original markup text, if the document was provided as HTML/XML.
    pub markup_text: Option<String>,
    /// Citation tokens paired with their index in `words`.
    pub citation_tokens: Vec<(usize, Token)>,
    /// All tokens (and raw string spans) in document order.
    pub words: Tokens,
    /// Maps offsets in `plain_text` to offsets in `markup_text`. Only set
    /// when the document was built from markup alone.
    pub plain_to_markup: Option<SpanUpdater>,
    /// Maps offsets in `markup_text` to offsets in `plain_text`. Only set
    /// when the document was built from markup alone.
    pub markup_to_plain: Option<SpanUpdater>,
    /// Emphasis (`<em>`/`<i>`) tags in `markup_text`:
    /// (stripped inner text, start, end) of the whole element.
    pub emphasis_tags: Vec<(String, usize, usize)>,
    /// The original source text supplied to the pipeline (the markup, when
    /// markup was given).
    pub source_text: String,
}

fn re_emphasis_tag() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // eyecite: r"<(em|i)[^>]*>(.*?)</\1>" with IGNORECASE | DOTALL. The
    // regex crate has no backreferences, so spell out both alternatives in
    // the same order.
    RE.get_or_init(|| Regex::new(r"(?is)<em[^>]*>(.*?)</em>|<i[^>]*>(.*?)</i>").unwrap())
}

impl Document {
    /// Create a new Document.
    ///
    /// Mirrors eyecite's `Document.__post_init__`:
    ///
    /// - `plain_text` only: `clean_steps` are applied to it.
    /// - `markup_text` only: `clean_steps` are applied to the markup to get
    ///   the plain text (with `html` prepended, and a warning logged, if the
    ///   caller left it out), then diff-based span maps between the two
    ///   texts are built and the markup's emphasis tags are recorded.
    /// - both: `plain_text` is taken to be the cleaned `markup_text`. eyecite
    ///   rejects `clean_steps` here as ambiguous; citare logs a warning and
    ///   ignores them.
    /// - neither: an empty document (eyecite raises `ValueError`).
    pub fn new(plain_text: &str, markup_text: &str, clean_steps: &[CleanStep]) -> Self {
        let mut document = Document {
            plain_text: plain_text.to_string(),
            markup_text: None,
            words: Vec::new(),
            citation_tokens: Vec::new(),
            plain_to_markup: None,
            markup_to_plain: None,
            emphasis_tags: Vec::new(),
            source_text: plain_text.to_string(),
        };

        if markup_text.is_empty() {
            if !clean_steps.is_empty() {
                document.plain_text =
                    clean_text(plain_text, clean_steps).unwrap_or_else(|_| plain_text.to_string());
            }
            return document;
        }

        document.markup_text = Some(markup_text.to_string());
        document.source_text = markup_text.to_string();

        if !plain_text.is_empty() {
            if !clean_steps.is_empty() {
                log::warn!(
                    "Both `markup_text` and `plain_text` were passed; ignoring \
                     `clean_steps` because it is not clear which to apply them to"
                );
            }
            return document;
        }

        // eyecite#328: markup always needs the `html` step to become plain
        // text. Rather than fail, add it up front and warn.
        let has_html = clean_steps.iter().any(|s| matches!(s, CleanStep::Html));
        let cleaned = if has_html {
            clean_text(markup_text, clean_steps)
        } else {
            log::warn!("`html` has been added to `markup_text` clean_steps list");
            clean_text(&html(markup_text), clean_steps)
        };
        document.plain_text = cleaned.unwrap_or_default();

        // Replace tags (and their attributes) with same-length placeholders
        // so the plain -> markup diff isn't thrown off by attribute text.
        let placeholder = placeholder_markup(markup_text);
        document.plain_to_markup = Some(SpanUpdater::new_bytes(&document.plain_text, &placeholder));
        document.markup_to_plain = Some(SpanUpdater::new_bytes(markup_text, &document.plain_text));
        document.identify_emphasis_tags();

        document
    }

    /// Record every `<em>`/`<i>` element in `markup_text` as
    /// (stripped inner text, start, end).
    fn identify_emphasis_tags(&mut self) {
        let Some(markup) = self.markup_text.as_deref() else {
            return;
        };
        self.emphasis_tags = re_emphasis_tag()
            .captures_iter(markup)
            .map(|caps| {
                let whole = caps.get(0).unwrap();
                let inner = caps
                    .get(1)
                    .or_else(|| caps.get(2))
                    .map_or("", |m| m.as_str());
                (inner.trim().to_string(), whole.start(), whole.end())
            })
            .collect();
    }

    /// Tokenize the document's plain text, populating `words` and
    /// `citation_tokens`.
    pub fn tokenize(&mut self, tokenizer: &AhocorasickTokenizer) {
        let (words, citation_tokens) = tokenizer.tokenize(&self.plain_text);
        self.words = words;
        self.citation_tokens = citation_tokens;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text_only_applies_clean_steps() {
        let doc = Document::new("foo  bar", "", &[CleanStep::InlineWhitespace]);
        assert_eq!(doc.plain_text, "foo bar");
        assert_eq!(doc.source_text, "foo  bar");
        assert!(doc.markup_text.is_none());
        assert!(doc.plain_to_markup.is_none());
    }

    #[test]
    fn test_markup_only_is_cleaned_into_plain_text() {
        let markup = "<p>In <i>Foo</i> v. <i>Bar,</i> 1 U.S. 1</p>";
        let doc = Document::new("", markup, &[CleanStep::Html, CleanStep::InlineWhitespace]);
        assert_eq!(doc.plain_text, "In Foo v. Bar, 1 U.S. 1");
        assert_eq!(doc.source_text, markup);
        assert_eq!(doc.markup_text.as_deref(), Some(markup));
    }

    #[test]
    fn test_markup_without_html_step_adds_it() {
        // eyecite#328: leaving out `html` must not fail.
        let markup = "<p>See 1 U.S. 1</p>";
        let doc = Document::new("", markup, &[CleanStep::AllWhitespace]);
        assert_eq!(doc.plain_text, "See 1 U.S. 1");
        let doc = Document::new("", markup, &[]);
        assert_eq!(doc.plain_text, "See 1 U.S. 1");
    }

    #[test]
    fn test_markup_span_maps_round_trip() {
        let markup = r#"<p id="x1">In <i>Foo</i> v. <i>Bar,</i> 1 U.S. 1</p>"#;
        let doc = Document::new("", markup, &[CleanStep::Html, CleanStep::InlineWhitespace]);
        let to_markup = doc.plain_to_markup.as_ref().unwrap();
        let to_plain = doc.markup_to_plain.as_ref().unwrap();
        for word in ["In", "Foo", "Bar", "1 U.S. 1"] {
            let p = doc.plain_text.find(word).unwrap();
            let m = markup.find(word).unwrap();
            assert_eq!(to_markup.update(p, true), m, "plain -> markup for {word}");
            assert_eq!(to_plain.update(m, true), p, "markup -> plain for {word}");
        }
    }

    #[test]
    fn test_emphasis_tags() {
        let markup = "<em>Grant </em>v. <EM class=\"a\">Esquire, Inc. </EM> and <i>Foo</i>";
        let doc = Document::new("", markup, &[CleanStep::Html]);
        let names: Vec<_> = doc.emphasis_tags.iter().map(|t| t.0.as_str()).collect();
        assert_eq!(names, ["Grant", "Esquire, Inc.", "Foo"]);
        let (_, start, end) = &doc.emphasis_tags[2];
        assert_eq!(&markup[*start..*end], "<i>Foo</i>");
    }

    #[test]
    fn test_both_texts_keep_plain_text() {
        let doc = Document::new("1 U.S. 1", "<p>1 U.S. 1</p>", &[]);
        assert_eq!(doc.plain_text, "1 U.S. 1");
        assert_eq!(doc.source_text, "<p>1 U.S. 1</p>");
        assert!(doc.plain_to_markup.is_none());
    }
}
