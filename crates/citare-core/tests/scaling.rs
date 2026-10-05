//! Finding, resolving and annotating citations must take time linear in
//! the length of the text (#44). Each test times the pipeline over a text
//! and over the same text repeated eight times, and fails if the long text
//! takes more than ten times as long.
//!
//! Absolute times vary with the machine and its load, so only the ratio of
//! two times taken in the same run is checked. The short and long texts are
//! timed in alternation, and the fastest of several rounds is kept for each,
//! which drops the rounds a busy machine slowed down.
//!
//! Repeating a text is the hard case for each step that went quadratic:
//! every party name recurs after every citation, and no word is unique in
//! the markup, which is what anchors the markup diff.

use std::time::{Duration, Instant};

use citare_core::{
    annotate_citations, get_citations, get_citations_from_document, resolve_citations, CleanStep,
    Document, UnbalancedTags,
};

const REPEAT: usize = 8;
const MAX_RATIO: f64 = 10.0;
const ROUNDS: usize = 3;

/// eyecite's long-opinion test fixture (86 KB, 170 citations).
fn opinion() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../upstream/eyecite/tests/assets/opinion.txt"
    );
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e} (run `git submodule update --init`)"))
}

/// The text's first `max_len` bytes, cut at a paragraph break.
fn prefix(text: &str, max_len: usize) -> &str {
    let end = text[..max_len].rfind("\n\n").unwrap_or(max_len);
    &text[..end]
}

/// The text as HTML: one `<p>` per paragraph, with "Id." and "supra" in
/// `<i>` tags.
fn to_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .split("\n\n")
        .map(|para| {
            let para = para
                .replace("Id.", "<i>Id.</i>")
                .replace("supra", "<i>supra</i>");
            format!("<p class=\"opinion\">{para}</p>\n")
        })
        .collect()
}

/// Find, resolve and annotate the citations in `text`, which is markup
/// when `markup` is set.
fn pipeline(text: &str, markup: bool) -> usize {
    let (plain, citations) = if markup {
        let mut document = Document::new("", text, &[CleanStep::Html]);
        let citations = get_citations_from_document(&mut document, false);
        (document.plain_text, citations)
    } else {
        (text.to_string(), get_citations(text, false, "", &[]))
    };
    let resolutions = resolve_citations(&citations);
    let mut annotations: Vec<_> = citations
        .iter()
        .map(|c| (c.span(), "<a>".to_string(), "</a>".to_string()))
        .collect();
    let source = if markup { text } else { "" };
    let annotated = annotate_citations(
        &plain,
        &mut annotations,
        source,
        UnbalancedTags::Unchecked,
        None,
        None,
    );
    citations.len() + resolutions.len() + annotated.len()
}

fn time(text: &str, markup: bool) -> Duration {
    let start = Instant::now();
    std::hint::black_box(pipeline(text, markup));
    start.elapsed()
}

/// How many times longer the pipeline takes over `text` repeated
/// [`REPEAT`] times than over `text` once.
fn scaling_ratio(text: &str, markup: bool) -> f64 {
    let long = text.repeat(REPEAT);
    pipeline(text, markup); // load the data and build the tokenizer
    let (mut short_best, mut long_best) = (Duration::MAX, Duration::MAX);
    for _ in 0..ROUNDS {
        short_best = short_best.min(time(text, markup));
        long_best = long_best.min(time(&long, markup));
    }
    let ratio = long_best.as_secs_f64() / short_best.as_secs_f64();
    eprintln!(
        "{} KB: {short_best:?}; {REPEAT}x: {long_best:?}; ratio {ratio:.1}",
        text.len() / 1024
    );
    ratio
}

#[test]
fn plain_text_scales_linearly() {
    let text = opinion();
    let text = prefix(&text, 32 * 1024);
    let ratio = scaling_ratio(text, false);
    assert!(
        ratio <= MAX_RATIO,
        "{REPEAT}x the text took {ratio:.1}x the time (limit {MAX_RATIO}x)"
    );
}

#[test]
fn markup_scales_linearly() {
    let text = opinion();
    let html = to_html(prefix(&text, 16 * 1024));
    let ratio = scaling_ratio(&html, true);
    assert!(
        ratio <= MAX_RATIO,
        "{REPEAT}x the markup took {ratio:.1}x the time (limit {MAX_RATIO}x)"
    );
}
