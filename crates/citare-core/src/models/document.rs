use super::token::{Token, Tokens};

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
