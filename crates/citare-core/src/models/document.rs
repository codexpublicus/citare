use super::token::{Token, Tokens};
use crate::clean::{clean_text, CleanStep};
use crate::tokenizer::AhocorasickTokenizer;

/// Represents a document being processed for citation extraction.
///
/// Construction logic and `SpanUpdater` integration are implemented in Phase 5.
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
    /// Emphasis tag spans: (tag_name, start, end).
    pub emphasis_tags: Vec<(String, usize, usize)>,
    /// The original source text supplied to the pipeline.
    pub source_text: String,
}

impl Document {
    /// Create a new Document, optionally applying cleaning steps and handling
    /// markup text.
    pub fn new(plain_text: &str, markup_text: &str, clean_steps: &[CleanStep]) -> Self {
        let text = if clean_steps.is_empty() {
            plain_text.to_string()
        } else {
            clean_text(plain_text, clean_steps).unwrap_or_else(|_| plain_text.to_string())
        };

        Document {
            plain_text: text,
            markup_text: if markup_text.is_empty() {
                None
            } else {
                Some(markup_text.to_string())
            },
            words: Vec::new(),
            citation_tokens: Vec::new(),
            emphasis_tags: Vec::new(),
            source_text: plain_text.to_string(),
        }
    }

    /// Tokenize the document's plain text, populating `words` and
    /// `citation_tokens`.
    pub fn tokenize(&mut self, tokenizer: &AhocorasickTokenizer) {
        let (words, citation_tokens) = tokenizer.tokenize(&self.plain_text);
        self.words = words;
        self.citation_tokens = citation_tokens;
    }
}
