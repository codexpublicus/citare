//! Citare: Legal citation extraction library
//!
//! Inspired by [eyecite](https://github.com/freelawproject/eyecite)
//!
//! # Quick start
//!
//! ```rust
//! use citare_core::{get_citations, resolve_citations, annotate_citations, UnbalancedTags};
//!
//! let text = "Foo v. Bar, 1 U.S. 1 (1999). Id. at 2.";
//! let citations = get_citations(text, false, "", &[]);
//! let resolutions = resolve_citations(&citations);
//! let mut annotations: Vec<((usize, usize), String, String)> = citations
//!     .iter()
//!     .enumerate()
//!     .map(|(i, c)| {
//!         let (start, end) = c.span();
//!         ((start, end), format!("<a id=\"{}\">", i), "</a>".to_string())
//!     })
//!     .collect();
//! let annotated = annotate_citations(text, &mut annotations, "", UnbalancedTags::Unchecked, None, None);
//! ```

pub mod annotate;
pub mod clean;
pub mod find;
pub mod helpers;
pub mod models;
pub mod offsets;
pub mod regexes;
pub mod resolve;
pub mod tokenizer;
pub mod utils;

// ---------------------------------------------------------------------------
// Top-level re-exports — primary pipeline functions
// ---------------------------------------------------------------------------

/// Find citations in legal text.
///
/// See [`find::get_citations`] for full documentation.
pub use find::get_citations;

/// Find citations in a prepared [`Document`], keeping the document.
///
/// See [`find::get_citations_from_document`] for full documentation.
pub use find::get_citations_from_document;

/// Resolve a slice of citations to their resources.
///
/// See [`resolve::resolve_citations`] for full documentation.
pub use resolve::resolve_citations;

/// Wrap a full citation in a new [`Resource`].
///
/// See [`resolve::resolve_full_citation`] for full documentation.
pub use resolve::resolve_full_citation;

/// The resolved-citation map type: `HashMap<Resource, Vec<Citation>>`.
pub use resolve::Resolutions;

/// Insert annotation markers around citation spans in text.
///
/// See [`annotate::annotate_citations`] for full documentation.
pub use annotate::annotate_citations;

/// Controls how unbalanced HTML tags inside a citation span are handled.
pub use annotate::UnbalancedTags;

/// Maps character offsets between two versions of a text (e.g. plain vs. HTML).
pub use annotate::SpanUpdater;

/// Convert byte spans to Unicode code point or UTF-16 offsets.
///
/// See [`offsets`] for details.
pub use offsets::{CitationSpans, OffsetMap, OffsetUnit};

/// Apply a sequence of cleaning steps to text.
///
/// See [`clean::clean_text`] for full documentation.
pub use clean::clean_text;

/// A single cleaning step applied by [`clean_text`].
pub use clean::CleanStep;

// ---------------------------------------------------------------------------
// Key model types
// ---------------------------------------------------------------------------

/// The primary citation enum — all variants live here.
pub use models::Citation;

/// A resolved resource that one or more citations point to.
pub use models::Resource;

/// Token variants produced by the tokenizer.
pub use models::Token;

/// A parsed legal document with tokenized words.
pub use models::Document;

/// A single edition of a reporter (e.g. "2d" series of "F.").
pub use models::Edition;

/// A top-level reporter entry from reporters-db.
pub use models::Reporter;

// ---------------------------------------------------------------------------
// Tokenizer types
// ---------------------------------------------------------------------------

/// The high-performance Aho-Corasick tokenizer (recommended).
pub use tokenizer::AhocorasickTokenizer;

/// The simple (non-Aho-Corasick) tokenizer.
pub use tokenizer::Tokenizer;
