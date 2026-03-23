//! Annotation utilities, including offset mapping between text variants.

// ── SpanUpdater ───────────────────────────────────────────────────────────────

/// Maps character offsets from one version of a text to another using a
/// diff-based offset-tracking approach.
///
/// # Example
/// ```
/// # use citare_core::annotate::SpanUpdater;
/// let updater = SpanUpdater::new("foo bar", "foo baz bar");
/// // 'f' is still at offset 0
/// assert_eq!(updater.update(0, true), 0);
/// // 'b' in "bar" has moved from offset 4 to offset 8
/// assert_eq!(updater.update(4, true), 8);
/// ```
pub struct SpanUpdater {
    /// Sorted list of character offsets in `text_before` that mark the start
    /// of a new tracking region.
    offsets: Vec<usize>,
    /// One delta entry per entry in `offsets`.
    deltas: Vec<OffsetUpdate>,
}

enum OffsetUpdate {
    /// Relative shift: `new_offset = old_offset + delta`.
    Shift(i64),
    /// Absolute replacement: offsets in this deleted region map to a fixed
    /// position in `text_after`.
    Replace(usize),
}

impl SpanUpdater {
    /// Build a `SpanUpdater` by diffing `text_before` against `text_after`.
    ///
    /// The diff is character-level (Unicode scalar values).
    pub fn new(text_before: &str, text_after: &str) -> Self {
        // dmp::Dmp::diff_main works on &str and returns Vec<dmp::Diff>.
        // dmp::Diff has:
        //   .operation: i32  (-1 = delete, 0 = equal, 1 = insert)
        //   .text: String
        let dmp = dmp::new();
        let mut diffs = dmp.diff_main(text_before, text_after, false);
        // Apply semantic lossless cleanup so diffs align on word/phrase
        // boundaries rather than arbitrary character positions.
        dmp.diff_cleanup_semantic_lossless(&mut diffs);
        let diffs = diffs;

        let mut offsets: Vec<usize> = Vec::new();
        let mut deltas: Vec<OffsetUpdate> = Vec::new();
        let mut offset: usize = 0; // position in text_before
        let mut delta: i64 = 0; // cumulative shift

        for diff in &diffs {
            let amount = diff.text.chars().count();
            match diff.operation {
                0 => {
                    // Equal — start a new tracking region with current delta
                    offsets.push(offset);
                    deltas.push(OffsetUpdate::Shift(delta));
                    offset += amount;
                }
                1 => {
                    // Insert — only affects delta (text_before has no chars here)
                    delta += amount as i64;
                }
                -1 => {
                    // Delete — map deleted chars to the current absolute position
                    offsets.push(offset);
                    deltas.push(OffsetUpdate::Replace((offset as i64 + delta) as usize));
                    offset += amount;
                    delta -= amount as i64;
                }
                _ => {}
            }
        }

        Self { offsets, deltas }
    }

    /// Translate a character offset from `text_before` to the corresponding
    /// position in `text_after`.
    ///
    /// `use_right = true` uses bisect-right semantics (for span ends);
    /// `use_right = false` uses bisect-left semantics (for span starts).
    pub fn update(&self, offset: usize, use_right: bool) -> usize {
        if self.offsets.is_empty() {
            return offset;
        }
        let raw_index = if use_right {
            self.bisect_right(offset)
        } else {
            self.bisect_left(offset)
        };
        // Clamp to the last valid entry
        let index = if raw_index == 0 {
            0
        } else {
            raw_index - 1
        };
        match self.deltas[index] {
            OffsetUpdate::Shift(d) => (offset as i64 + d) as usize,
            OffsetUpdate::Replace(new_offset) => new_offset,
        }
    }

    /// Number of positions in `offsets` that are strictly less than `target`.
    fn bisect_left(&self, target: usize) -> usize {
        self.offsets.partition_point(|&x| x < target)
    }

    /// Number of positions in `offsets` that are less than or equal to `target`.
    fn bisect_right(&self, target: usize) -> usize {
        self.offsets.partition_point(|&x| x <= target)
    }
}

// ── annotate_citations ────────────────────────────────────────────────────────

/// Controls how unbalanced HTML tags inside a citation span are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnbalancedTags {
    /// Do not check for unbalanced tags (default, fastest).
    Unchecked,
    /// Skip annotations whose span contains unbalanced HTML that cannot be
    /// fixed by extending to include nearby style tags.
    Skip,
    /// Wrap every HTML tag inside the span with the annotation markers so the
    /// output remains well-formed.
    Wrap,
}

/// Callback type used by [`annotate_citations`] to build annotated spans.
///
/// Receives `(before, span_text, after)` and returns the annotated string.
pub type Annotator<'a> = &'a dyn Fn(&str, &str, &str) -> String;

/// Insert annotation markers around each citation span in `plain_text`.
///
/// # Arguments
///
/// * `plain_text` — the text in which `annotations` offsets were computed.
/// * `annotations` — mutable `Vec` of `((start, end), before, after)` tuples.
///   Sorted in-place before processing.
/// * `source_text` — optional HTML source.  When non-empty and different from
///   `plain_text`, offsets are translated from plain-text space to source-text
///   space via a [`SpanUpdater`].
/// * `unbalanced_tags` — how to handle citation spans that contain unbalanced
///   HTML.
/// * `annotator` — optional closure that builds the annotated string from
///   `(before, span, after)`; defaults to simple concatenation.
/// * `offset_updater` — supply a pre-built [`SpanUpdater`] instead of building
///   one from `source_text`; when provided, `source_text` is used as the
///   working text directly.
pub fn annotate_citations(
    plain_text: &str,
    annotations: &mut [((usize, usize), String, String)],
    source_text: &str,
    unbalanced_tags: UnbalancedTags,
    annotator: Option<Annotator<'_>>,
    offset_updater: Option<&SpanUpdater>,
) -> String {
    // Determine the working text and which SpanUpdater (if any) to use.
    // `owned_updater` keeps the SpanUpdater alive for the loop below when we
    // build one on the fly from `source_text`.
    let owned_updater: Option<SpanUpdater>;
    let working_text: &str;
    let effective_updater: Option<&SpanUpdater>;

    if let Some(updater) = offset_updater {
        // Caller supplied a pre-built updater; use source_text directly.
        owned_updater = None;
        working_text = source_text;
        effective_updater = Some(updater);
    } else if !source_text.is_empty() && source_text != plain_text {
        // Build an updater from placeholder-collapsed source HTML.
        let placeholder_text = crate::utils::placeholder_markup(source_text);
        owned_updater = Some(SpanUpdater::new(plain_text, &placeholder_text));
        working_text = source_text;
        effective_updater = owned_updater.as_ref();
    } else {
        owned_updater = None;
        working_text = plain_text;
        effective_updater = None;
    }
    // Suppress unused-variable lint: owned_updater is kept alive intentionally.
    let _ = &owned_updater;

    annotations.sort();

    let mut out = String::new();
    let mut last_end: usize = 0;

    for ((raw_start, raw_end), before, after) in annotations.iter() {
        let mut start = *raw_start;
        let mut end = *raw_end;

        // Translate offsets from plain-text space → source-text space.
        if let Some(updater) = effective_updater {
            start = updater.update(start, true); // bisect_right for start
            end = updater.update(end, false); // bisect_left for end
        }

        // Skip or clip overlapping spans.
        if start < last_end {
            start = last_end;
            if start >= end {
                continue;
            }
        }

        let mut span_text = working_text[start..end].to_string();

        // Optionally validate / repair unbalanced HTML.
        match unbalanced_tags {
            UnbalancedTags::Unchecked => {}
            _ if crate::utils::is_balanced_html(&span_text) => {}
            UnbalancedTags::Wrap => {
                span_text = crate::utils::wrap_html_tags(&span_text, after, before);
            }
            UnbalancedTags::Skip => {
                let (new_start, new_end, new_span) =
                    crate::utils::maybe_balance_style_tags(start, end, working_text, 10);
                start = new_start;
                end = new_end;
                span_text = new_span;
                if !crate::utils::is_balanced_html(&span_text) {
                    continue; // give up on this annotation
                }
            }
        }

        let annotated = match annotator {
            Some(f) => f(before, &span_text, after),
            None => format!("{}{}{}", before, span_text, after),
        };

        out.push_str(&working_text[last_end..start]);
        out.push_str(&annotated);
        last_end = end;
    }

    // Append any trailing text after the last annotation.
    if last_end < working_text.len() {
        out.push_str(&working_text[last_end..]);
    }

    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_updater_no_change() {
        let updater = SpanUpdater::new("hello", "hello");
        assert_eq!(updater.update(0, true), 0);
        assert_eq!(updater.update(3, true), 3);
        assert_eq!(updater.update(5, true), 5);
    }

    #[test]
    fn test_span_updater_insertion() {
        // "foo bar" → "foo baz bar"  (inserted "baz " after "foo ")
        let updater = SpanUpdater::new("foo bar", "foo baz bar");
        // 'f' still at 0
        assert_eq!(updater.update(0, true), 0);
        // 'b' in "bar" was at offset 4, now at offset 8
        assert_eq!(updater.update(4, true), 8);
    }

    #[test]
    fn test_span_updater_deletion() {
        // "foo baz bar" → "foo bar"  (deleted "baz ")
        let updater = SpanUpdater::new("foo baz bar", "foo bar");
        // 'f' still at 0
        assert_eq!(updater.update(0, true), 0);
    }

    #[test]
    fn test_span_updater_empty_strings() {
        let updater = SpanUpdater::new("", "");
        assert_eq!(updater.update(0, true), 0);
    }

    #[test]
    fn test_span_updater_pure_insert() {
        // "" → "hello": everything is an insertion, offset 0 stays 0
        let updater = SpanUpdater::new("", "hello");
        assert_eq!(updater.update(0, true), 0);
    }

    #[test]
    fn test_span_updater_pure_delete() {
        // "hello" → "": all text deleted, any offset maps to 0
        let updater = SpanUpdater::new("hello", "");
        assert_eq!(updater.update(0, true), 0);
        assert_eq!(updater.update(3, true), 0);
    }

    #[test]
    fn test_span_updater_unicode() {
        // Ensure character-level (not byte-level) counting works
        let before = "café bar";
        let after = "café baz bar";
        let updater = SpanUpdater::new(before, after);
        // 'b' in "bar" was at char offset 5 (c,a,f,é,' ' = 5), now at 9
        assert_eq!(updater.update(5, true), 9);
    }

    // ── annotate_citations tests ───────────────────────────────────────────────

    #[test]
    fn test_annotate_basic() {
        let text = "foo 1 U.S. 1 bar";
        let mut annotations = vec![((4, 12), "<a>".to_string(), "</a>".to_string())];
        let result = annotate_citations(
            text,
            &mut annotations,
            "",
            UnbalancedTags::Unchecked,
            None,
            None,
        );
        assert_eq!(result, "foo <a>1 U.S. 1</a> bar");
    }

    #[test]
    fn test_annotate_multiple() {
        let text = "1 U.S. 1 and 2 U.S. 2";
        let mut annotations = vec![
            ((0, 8), "<a>".to_string(), "</a>".to_string()),
            ((13, 21), "<b>".to_string(), "</b>".to_string()),
        ];
        let result = annotate_citations(
            text,
            &mut annotations,
            "",
            UnbalancedTags::Unchecked,
            None,
            None,
        );
        assert_eq!(result, "<a>1 U.S. 1</a> and <b>2 U.S. 2</b>");
    }

    #[test]
    fn test_annotate_overlap_skip() {
        let text = "aaa bbb ccc";
        let mut annotations = vec![
            ((0, 7), "[".to_string(), "]".to_string()),
            ((4, 11), "{".to_string(), "}".to_string()),
        ];
        let result = annotate_citations(
            text,
            &mut annotations,
            "",
            UnbalancedTags::Unchecked,
            None,
            None,
        );
        // The second annotation's start is clipped to last_end (7), so the
        // space at index 7 is absorbed into the second annotated span.
        assert_eq!(result, "[aaa bbb]{ ccc}");
    }

    #[test]
    fn test_annotate_no_annotations() {
        let text = "no citations here";
        let mut annotations: Vec<((usize, usize), String, String)> = vec![];
        let result = annotate_citations(
            text,
            &mut annotations,
            "",
            UnbalancedTags::Unchecked,
            None,
            None,
        );
        assert_eq!(result, text);
    }

    #[test]
    fn test_annotate_custom_annotator() {
        let text = "foo 1 U.S. 1 bar";
        let mut annotations = vec![((4, 12), "BEFORE".to_string(), "AFTER".to_string())];
        let annotator = |before: &str, span: &str, after: &str| {
            format!("[{}|{}|{}]", before, span, after)
        };
        let result = annotate_citations(
            text,
            &mut annotations,
            "",
            UnbalancedTags::Unchecked,
            Some(&annotator),
            None,
        );
        assert_eq!(result, "foo [BEFORE|1 U.S. 1|AFTER] bar");
    }

    #[test]
    fn test_annotate_with_source_text() {
        // plain_text differs from source_text (HTML); offsets are in plain_text
        // coordinates and must be translated to source_text coordinates.
        let source = "foo <b>bar</b> 1 U.S. 1 baz";
        let plain = "foo bar 1 U.S. 1 baz";
        // In plain_text, "1 U.S. 1" starts at offset 8 (after "foo bar ")
        let mut annotations = vec![((8, 16), "<a>".to_string(), "</a>".to_string())];
        let result = annotate_citations(
            plain,
            &mut annotations,
            source,
            UnbalancedTags::Unchecked,
            None,
            None,
        );
        // The result is built from source_text and should contain both markers
        assert!(result.contains("<a>"), "missing opening tag in: {result}");
        assert!(result.contains("</a>"), "missing closing tag in: {result}");
        // Original HTML structure should be preserved around the annotation
        assert!(result.contains("<b>bar</b>"), "original HTML lost in: {result}");
    }

    #[test]
    fn test_annotate_unsorted_input_is_sorted() {
        // Provide annotations out of order; function must sort them first.
        let text = "aaa bbb ccc";
        let mut annotations = vec![
            ((8, 11), "<c>".to_string(), "</c>".to_string()),
            ((0, 3), "<a>".to_string(), "</a>".to_string()),
            ((4, 7), "<b>".to_string(), "</b>".to_string()),
        ];
        let result = annotate_citations(
            text,
            &mut annotations,
            "",
            UnbalancedTags::Unchecked,
            None,
            None,
        );
        assert_eq!(result, "<a>aaa</a> <b>bbb</b> <c>ccc</c>");
    }
}
