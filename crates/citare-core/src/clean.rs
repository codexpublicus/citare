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

/// Elements that start a new block of text: HTML's block-level and table elements, plus
/// the block elements of CAP's case XML (`opinion`, `author`, `footnote` and the head
/// matter). Text on either side of one of these is separated by a space.
fn is_block(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "body"
            | "br"
            | "caption"
            | "center"
            | "col"
            | "colgroup"
            | "dd"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "head"
            | "header"
            | "hgroup"
            | "hr"
            | "html"
            | "legend"
            | "li"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "option"
            | "p"
            | "pre"
            | "section"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "title"
            | "tr"
            | "ul"
            // CAP case XML (`xml_harvard` in CourtListener)
            | "casebody"
            | "opinion"
            | "author"
            | "footnote"
            | "parties"
            | "docketnumber"
            | "court"
            | "decisiondate"
            | "otherdate"
            | "attorneys"
            | "judges"
            | "headnotes"
            | "syllabus"
            | "disposition"
            | "history"
            | "correction"
    )
}

/// Inline elements that mark a place in the running text rather than continuing it:
/// footnote references (`<sup>`, CAP's `<footnotemark>` and `<a class="footnotemark">`)
/// and CAP's page labels (`<a class="page-label">*289</a>`). They keep a space on either
/// side, as eyecite gives them, so a footnote number never runs into a page number before
/// it (`1 U.S. 1<sup>2</sup>` stays `1 U.S. 1 2`).
fn is_marker(elem: &scraper::node::Element) -> bool {
    matches!(elem.name(), "sup" | "footnotemark")
        || elem
            .classes()
            .any(|c| c == "footnotemark" || c == "page-label")
}

/// Whether eyecite drops the text directly inside this element.
fn hides_its_text(elem: &scraper::node::Element) -> bool {
    matches!(
        elem.name(),
        "style" | "script" | "head" | "link" | "page-number"
    ) || elem.attr("class") == Some("star-pagination")
}

/// Extract visible text from HTML.
///
/// Selects the same text nodes as eyecite's `clean.html`: every text node with non-blank
/// content (XPath `normalize-space()`) whose *direct* parent is not `<style>`, `<link>`,
/// `<head>`, `<page-number>`, `<script>` or an element with `class="star-pagination"`.
/// Text nodes are kept verbatim (not trimmed).
///
/// eyecite joins every pair of text nodes with a space, which splits a word or citation
/// where an inline element ends inside it: CAP wraps `Conn. Gen. Stat. § 10` in a citation
/// tag, so eyecite reads `§ 10 -236b(f)`, and every CAP citation link ends before its pin
/// cite (`410 U.S. 113 , 115`). citare adds a space before a text node only where the
/// markup separates it from the text before:
///
/// - at the start or end of a block-level element (`<p>`, `<div>`, `<br>`, table cells,
///   CAP's `<opinion>` and `<footnote>`, ...; see `is_block`);
/// - on either side of a footnote mark or page label (see `is_marker`);
/// - where any other element opens, since text glued to the start of an element is
///   usually a new word in real markup (`<em>Moy</em><em>v. United States</em>`,
///   `17<i>C.F.R.</i>`), unless the element's own text is suppressed: a
///   `<page-number>` can sit inside a word (`can<page-number>*642</page-number>not`);
/// - where whitespace-only text lies between.
///
/// Where an inline element closes, the text after it runs on with nothing added. See
/// docs/divergences.md#html-inline-join.
pub fn html(html_content: &str) -> String {
    let document = Html::parse_document(html_content);
    let mut out = String::with_capacity(html_content.len() / 2);
    // A break is due before the next text that is kept.
    let mut pending_break = false;

    // Depth-first, in document order; `true` marks the close of an element.
    let mut stack = vec![(document.tree.root(), false)];
    while let Some((node, closing)) = stack.pop() {
        match node.value() {
            Node::Element(elem) => {
                // Blocks and markers break on both sides. Any other element breaks where it
                // opens, unless its own text is hidden: a page number can sit inside a word.
                if is_block(elem.name()) || is_marker(elem) || (!closing && !hides_its_text(elem)) {
                    pending_break = true;
                }
                if !closing {
                    stack.push((node, true));
                    let mut children: Vec<_> = node.children().collect();
                    children.reverse();
                    stack.extend(children.into_iter().map(|c| (c, false)));
                }
            }
            Node::Text(text) => {
                // XPath normalize-space() only strips space, tab, CR and LF.
                if text
                    .text
                    .chars()
                    .all(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
                {
                    pending_break = true;
                    continue;
                }

                let suppressed = node.parent().is_some_and(|parent| match parent.value() {
                    Node::Element(elem) => hides_its_text(elem),
                    _ => false,
                });
                if suppressed {
                    continue;
                }

                if pending_break && !out.is_empty() {
                    out.push(' ');
                }
                pending_break = false;
                out.push_str(&text.text);
            }
            // Only elements are pushed again to close.
            Node::Document | Node::Fragment => {
                let mut children: Vec<_> = node.children().collect();
                children.reverse();
                stack.extend(children.into_iter().map(|c| (c, false)));
            }
            _ => {}
        }
    }

    out
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
        // Text nodes are kept verbatim; an opening inline tag adds a space, a closing one
        // nothing.
        assert_eq!(
            html("<p>In <i>Foo</i> v. <i>Bar,</i> 1 U.S. 1</p>"),
            "In  Foo v.  Bar, 1 U.S. 1"
        );
        assert_eq!(html("See <em>Bivens </em>v. x"), "See  Bivens v. x");
        // Whitespace-only text between inline elements is one space.
        assert_eq!(html("<i>Foo</i> <i>v.</i>\n<i>Bar</i>"), "Foo v. Bar");
        // Adjacent elements stay separate words (eyecite's test_markup_plaintiff...).
        assert_eq!(
            html("<em>Jin Fuey Moy</em><em>v. United States,</em> 254 U.S. 189"),
            "Jin Fuey Moy v. United States, 254 U.S. 189"
        );
    }

    #[test]
    fn test_html_inline_tag_splitting_a_citation() {
        // CAP wraps only part of a statute citation in its citation tag (D116).
        let markup = "<p>under <extracted-citation index=\"3\">Conn. Gen. Stat. \u{a7} 10</extracted-citation>-236b(f), and</p>";
        assert_eq!(
            html(markup),
            "under  Conn. Gen. Stat. \u{a7} 10-236b(f), and"
        );
        // CAP's citation links end before the pin cite.
        assert_eq!(
            html(r#"<p>See <a class="citation" href="/us/410/113">410 U.S. 113</a>, 115.</p>"#),
            "See  410 U.S. 113, 115."
        );
        assert_eq!(html("<p>(<em>Id</em>. at 5)</p>"), "( Id. at 5)");
        assert_eq!(html("<p>461 <i>A.</i>2d 138</p>"), "461  A.2d 138");
        for tag in [
            "a",
            "span",
            "em",
            "i",
            "b",
            "strong",
            "sub",
            "small",
            "u",
            "cite",
            "abbr",
            "code",
            "mark",
            "q",
            "s",
            "time",
            "var",
            "font",
            "cross_reference",
            "x-custom",
        ] {
            assert_eq!(
                html(&format!("<p>ab<{tag}>cd</{tag}>ef</p>")),
                "ab cdef",
                "{tag}"
            );
        }
    }

    #[test]
    fn test_html_block_boundaries() {
        for tag in [
            "p",
            "div",
            "li",
            "h1",
            "h6",
            "blockquote",
            "section",
            "article",
            "header",
            "footer",
            "pre",
            "dd",
            "dt",
            "center",
            "opinion",
            "author",
            "footnote",
        ] {
            assert_eq!(html(&format!("ab<{tag}>cd</{tag}>ef")), "ab cd ef", "{tag}");
        }
        assert_eq!(html("<p>one</p><p>two</p>"), "one two");
        assert_eq!(html("a<br>b<hr>c"), "a b c");
        assert_eq!(
            html("<ul><li>a</li><li>b</li></ul><ol><li>c</li></ol>"),
            "a b c"
        );
        assert_eq!(
            html("<table><tr><td>a</td><td>b</td></tr><tr><th>c</th></tr></table>"),
            "a b c"
        );
        assert_eq!(html("<dl><dt>a</dt><dd>b</dd></dl>"), "a b");
        // No leading or trailing separator.
        assert_eq!(html("<div><p>a</p></div>"), "a");
    }

    #[test]
    fn test_html_footnote_marks_and_page_labels_stay_separate() {
        assert_eq!(html("<p>1 U.S. 1<sup>2</sup> and</p>"), "1 U.S. 1 2  and");
        assert_eq!(
            html("<p>1 U.S. 1<footnotemark>2</footnotemark> and</p>"),
            "1 U.S. 1 2  and"
        );
        assert_eq!(
            html(r##"<p>1 U.S. 1<a class="footnotemark" href="#f2">2</a> and</p>"##),
            "1 U.S. 1 2  and"
        );
        assert_eq!(
            html(r##"<p>amend<a class="page-label" href="#p262">*262</a>ment</p>"##),
            "amend *262 ment"
        );
    }

    #[test]
    fn test_html_suppressed_text_leaves_no_gap() {
        // A page break inside a word or citation joins back up.
        assert_eq!(
            html(r#"<p>they can<page-number label="642">*642</page-number>not meet</p>"#),
            "they cannot meet"
        );
        assert_eq!(
            html(r#"<p>218 <page-number label="161">*161</page-number>So.2d 580</p>"#),
            "218 So.2d 580"
        );
        assert_eq!(
            html(r#"<p>the<span class="star-pagination">*2631 </span> assessment</p>"#),
            "the assessment"
        );
    }

    #[test]
    fn test_html_only_direct_parent_is_suppressed() {
        // eyecite only checks the direct parent and an exact class match.
        assert_eq!(
            html(
                r#"a <span class="star-pagination"><b>*5</b></span> b <span class="x star-pagination">*6</span>"#
            ),
            "a  *5 b  *6"
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
