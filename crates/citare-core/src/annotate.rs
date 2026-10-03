//! Annotation utilities, including offset mapping between text variants.

// ── SpanUpdater ───────────────────────────────────────────────────────────────

/// Maps UTF-8 byte offsets from one version of a text to another using a
/// diff-based offset-tracking approach.
///
/// Byte offsets are the only unit: citation spans are byte offsets, and
/// [`annotate_citations`] and the [`crate::Document`] span maps slice text
/// with what this returns. There is one constructor, so a map built in one
/// place agrees with a map of the same texts built in another.
///
/// # Example
/// ```
/// # use citare_core::annotate::SpanUpdater;
/// let updater = SpanUpdater::new_bytes("café bar", "café XYZ bar");
/// // 'c' is still at offset 0
/// assert_eq!(updater.update(0, true), 0);
/// // 'b' in "bar" has moved from byte 6 to byte 10 ("é" is two bytes)
/// assert_eq!(updater.update(6, true), 10);
/// ```
#[derive(Debug, Clone)]
pub struct SpanUpdater {
    /// Sorted list of byte offsets in `text_before` that mark the start of a
    /// new tracking region.
    offsets: Vec<usize>,
    /// One delta entry per entry in `offsets`.
    deltas: Vec<OffsetUpdate>,
}

#[derive(Debug, Clone, Copy)]
enum OffsetUpdate {
    /// Relative shift: `new_offset = old_offset + delta`.
    Shift(i64),
    /// Absolute replacement: offsets in this deleted region map to a fixed
    /// position in `text_after`.
    Replace(usize),
}

impl SpanUpdater {
    /// Build a `SpanUpdater` whose offsets are UTF-8 byte offsets, matching
    /// the spans that citations carry.
    ///
    /// The diff runs over characters, so every region boundary is a char
    /// boundary in both texts, and is measured in bytes. No cleanup pass is
    /// applied, mirroring eyecite's `SpanUpdater` (`cleanup="No"`).
    ///
    /// The diff is Myers' algorithm, whose running time is the text length
    /// times the number of edits. Between an opinion's markup and its plain
    /// text both grow with the input, so one diff over a long opinion is
    /// quadratic: 400 KB of markup took minutes (#28). Above
    /// `WHOLE_DIFF_MAX` bytes the texts are cut at anchors (see
    /// [`diff_anchors`]) and each piece is diffed on its own; below it, the
    /// whole diff is cheap and runs as before.
    pub fn new_bytes(text_before: &str, text_after: &str) -> Self {
        let dmp = dmp::new();
        let anchors = if text_before.len() + text_after.len() <= WHOLE_DIFF_MAX {
            Vec::new()
        } else {
            diff_anchors(text_before, text_after)
        };
        let mut diffs = Vec::new();
        let (mut a, mut b) = (0, 0);
        for (anchor_a, anchor_b) in anchors {
            diffs.extend(dmp.diff_main(&text_before[a..anchor_a], &text_after[b..anchor_b], false));
            (a, b) = (anchor_a, anchor_b);
        }
        diffs.extend(dmp.diff_main(&text_before[a..], &text_after[b..], false));
        Self::from_diffs(&diffs)
    }

    fn from_diffs(diffs: &[dmp::Diff]) -> Self {
        let mut offsets: Vec<usize> = Vec::new();
        let mut deltas: Vec<OffsetUpdate> = Vec::new();
        let mut offset: usize = 0; // position in text_before
        let mut delta: i64 = 0; // cumulative shift

        for diff in diffs {
            let amount = diff.text.len();
            match diff.operation {
                0 => {
                    // Equal — start a new tracking region with current delta
                    offsets.push(offset);
                    deltas.push(OffsetUpdate::Shift(delta));
                    offset += amount;
                }
                1 => {
                    // Insert — only affects delta (text_before has no bytes here)
                    delta += amount as i64;
                }
                -1 => {
                    // Delete — map deleted bytes to the current absolute position
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

    /// Translate a byte offset from `text_before` to the corresponding
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
        let index = if raw_index == 0 { 0 } else { raw_index - 1 };
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

/// Combined length (bytes) up to which [`SpanUpdater::new_bytes`] diffs
/// the two texts whole.
const WHOLE_DIFF_MAX: usize = 16 * 1024;
/// Words shorter than this (in bytes) are not used as anchors.
const ANCHOR_MIN_WORD: usize = 2;
/// Minimum length (bytes of the first text) of a piece between anchors.
const ANCHOR_MIN_GAP: usize = 256;

/// Points `(a, b)` where `text_a[a..]` and `text_b[b..]` can be diffed
/// apart from what precedes them, in increasing order in both texts.
///
/// An anchor is the start of a word (a maximal run of alphanumeric
/// characters) that occurs exactly once in each text. Every word of the
/// plain text comes from the markup's visible text, so a word unique in
/// both is the same passage in both. Of those pairs, the longest chain that
/// is in order in both texts is kept, and each gap between two anchors is
/// searched again for words unique within that gap: the rules of patience
/// diff. The chain is then thinned to one anchor per `ANCHOR_MIN_GAP`
/// bytes. Anchoring takes time linear in the input (times the nesting
/// depth, which stays small), and leaves Myers with pieces a few hundred
/// bytes long.
fn diff_anchors(text_a: &str, text_b: &str) -> Vec<(usize, usize)> {
    // A stack of byte ranges (start_a, end_a, start_b, end_b) still to
    // search, rather than recursion: a long run of short gaps nests deeply.
    let mut pending = vec![(0, text_a.len(), 0, text_b.len())];
    let mut chain = Vec::new();
    while let Some((start_a, end_a, start_b, end_b)) = pending.pop() {
        let found = unique_word_chain(&text_a[start_a..end_a], &text_b[start_b..end_b]);
        let (mut a, mut b) = (start_a, start_b);
        for (next_a, next_b) in found {
            let (next_a, next_b) = (start_a + next_a, start_b + next_b);
            pending.push((a, next_a, b, next_b));
            chain.push((next_a, next_b));
            (a, b) = (next_a, next_b);
        }
        if (a, b) != (start_a, start_b) {
            pending.push((a, end_a, b, end_b));
        }
    }
    // Anchors from disjoint gaps are in order in both texts, so sorting by
    // the first text's offset restores the chain.
    chain.sort_unstable();

    let mut anchors: Vec<(usize, usize)> = Vec::new();
    let mut last_a = 0;
    for (a, b) in chain {
        if a >= last_a + ANCHOR_MIN_GAP {
            anchors.push((a, b));
            last_a = a;
        }
    }
    anchors
}

/// The longest in-order chain of `(a, b)` starts of words that occur once
/// in `text_a` and once in `text_b`. A word at the very start of either
/// text is left out, so every anchor splits the range it was found in.
fn unique_word_chain(text_a: &str, text_b: &str) -> Vec<(usize, usize)> {
    use std::collections::HashMap;

    if text_a.len() < ANCHOR_MIN_GAP || text_b.len() < ANCHOR_MIN_WORD {
        return Vec::new();
    }

    // word -> (count in a, start in a, count in b, start in b)
    let mut words: HashMap<&str, (u32, usize, u32, usize)> = HashMap::new();
    for (start, word) in alphanumeric_words(text_a) {
        let entry = words.entry(word).or_insert((0, start, 0, 0));
        entry.0 += 1;
    }
    for (start, word) in alphanumeric_words(text_b) {
        if let Some(entry) = words.get_mut(word) {
            entry.2 += 1;
            entry.3 = start;
        }
    }

    let mut pairs: Vec<(usize, usize)> = words
        .into_values()
        .filter(|&(count_a, a, count_b, b)| count_a == 1 && count_b == 1 && a > 0 && b > 0)
        .map(|(_, a, _, b)| (a, b))
        .collect();
    pairs.sort_unstable();

    // Longest chain increasing in b (patience sorting, O(n log n)).
    let mut tails: Vec<usize> = Vec::new(); // index into pairs
    let mut prev: Vec<Option<usize>> = vec![None; pairs.len()];
    for (i, &(_, b)) in pairs.iter().enumerate() {
        let pos = tails.partition_point(|&t| pairs[t].1 < b);
        prev[i] = pos.checked_sub(1).map(|p| tails[p]);
        if pos == tails.len() {
            tails.push(i);
        } else {
            tails[pos] = i;
        }
    }
    let mut chain = Vec::with_capacity(tails.len());
    let mut cursor = tails.last().copied();
    while let Some(i) = cursor {
        chain.push(pairs[i]);
        cursor = prev[i];
    }
    chain.reverse();
    chain
}

/// Runs of alphanumeric characters at least `ANCHOR_MIN_WORD` bytes long,
/// with their byte offsets.
fn alphanumeric_words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.len() >= ANCHOR_MIN_WORD)
        .map(move |word| (word.as_ptr() as usize - text.as_ptr() as usize, word))
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
        // Build an updater from placeholder-collapsed source HTML: the same
        // map `Document::plain_to_markup` holds for these texts, so the
        // wrappers land where the citations' markup spans say.
        let placeholder_text = crate::utils::placeholder_markup(source_text);
        owned_updater = Some(SpanUpdater::new_bytes(plain_text, &placeholder_text));
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

        // Spans from get_citations always fit, but callers (the WASM
        // binding, for one) can pass any offsets. Skip one that would
        // panic: out of range, reversed, or inside a character.
        if start > end
            || !working_text.is_char_boundary(end)
            || !working_text.is_char_boundary(start)
        {
            continue;
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

        // Balancing style tags can move `start` back before the end of the
        // previous annotation; eyecite's `plain_text[last_end:start]` is
        // then empty.
        if last_end < start {
            out.push_str(&working_text[last_end..start]);
        }
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
        let updater = SpanUpdater::new_bytes("hello", "hello");
        assert_eq!(updater.update(0, true), 0);
        assert_eq!(updater.update(3, true), 3);
        assert_eq!(updater.update(5, true), 5);
    }

    #[test]
    fn test_span_updater_insertion() {
        // "foo bar" → "foo XYZ bar"  (inserted "XYZ " after "foo ")
        let updater = SpanUpdater::new_bytes("foo bar", "foo XYZ bar");
        // 'f' still at 0
        assert_eq!(updater.update(0, true), 0);
        // 'b' in "bar" was at offset 4, now at offset 8
        assert_eq!(updater.update(4, true), 8);
        // With no cleanup pass, "foo baz bar" diffs as "foo ba" + "z ba" +
        // "r", so "bar" stays at 4, as in eyecite.
        assert_eq!(
            SpanUpdater::new_bytes("foo bar", "foo baz bar").update(4, true),
            4
        );
    }

    #[test]
    fn test_span_updater_deletion() {
        // "foo baz bar" → "foo bar"  (deleted "baz ")
        let updater = SpanUpdater::new_bytes("foo baz bar", "foo bar");
        // 'f' still at 0
        assert_eq!(updater.update(0, true), 0);
    }

    #[test]
    fn test_span_updater_empty_strings() {
        let updater = SpanUpdater::new_bytes("", "");
        assert_eq!(updater.update(0, true), 0);
    }

    #[test]
    fn test_span_updater_pure_insert() {
        // "" → "hello": everything is an insertion, offset 0 stays 0
        let updater = SpanUpdater::new_bytes("", "hello");
        assert_eq!(updater.update(0, true), 0);
    }

    #[test]
    fn test_span_updater_pure_delete() {
        // "hello" → "": all text deleted, any offset maps to 0
        let updater = SpanUpdater::new_bytes("hello", "");
        assert_eq!(updater.update(0, true), 0);
        assert_eq!(updater.update(3, true), 0);
    }

    #[test]
    fn test_span_updater_bytes_counts_utf8_bytes() {
        // "é" is two bytes, so "bar" starts at byte 6 before and byte 10
        // after the insertion of "XYZ ".
        let updater = SpanUpdater::new_bytes("café bar", "café XYZ bar");
        assert_eq!(updater.update(6, true), 10);
        assert_eq!(updater.update(0, true), 0);
    }

    #[test]
    fn test_span_updater_bytes_markup_to_plain() {
        let markup = "<p>In <i>Foo</i> v. Bar</p>";
        let plain = "In Foo v. Bar";
        let to_plain = SpanUpdater::new_bytes(markup, plain);
        let foo = markup.find("Foo").unwrap();
        assert_eq!(to_plain.update(foo, true), plain.find("Foo").unwrap());
        let to_markup = SpanUpdater::new_bytes(plain, markup);
        assert_eq!(
            to_markup.update(plain.find("Bar").unwrap(), true),
            markup.find("Bar").unwrap()
        );
    }

    #[test]
    fn test_span_updater_bytes_long_markup_is_anchored() {
        // Past WHOLE_DIFF_MAX the diff runs piecewise between anchors; every
        // word must still map to itself in both directions.
        let mut markup = String::new();
        let mut plain = String::new();
        for i in 0..2000 {
            markup.push_str(&format!(
                r#"<p class="x{i}">Para{i} <i>see</i> {i} U.S. {i}</p>"#
            ));
            plain.push_str(&format!("Para{i} see {i} U.S. {i} "));
        }
        assert!(markup.len() + plain.len() > WHOLE_DIFF_MAX);
        assert!(!diff_anchors(&plain, &markup).is_empty());
        let to_markup = SpanUpdater::new_bytes(&plain, &markup);
        let to_plain = SpanUpdater::new_bytes(&markup, &plain);
        for i in (0..2000).step_by(97) {
            let word = format!("Para{i} ");
            let p = plain.find(&word).unwrap();
            let m = markup.find(&word).unwrap();
            assert_eq!(to_markup.update(p, true), m, "plain -> markup for {word}");
            assert_eq!(to_plain.update(m, true), p, "markup -> plain for {word}");
        }
    }

    #[test]
    fn test_diff_anchors_are_unique_words_in_order() {
        let a = format!(
            "{} alpha {} beta {}",
            "x ".repeat(200),
            "y ".repeat(200),
            "z "
        );
        let b = format!("<{}> alpha <{}> beta <z>", "q".repeat(300), "q".repeat(300));
        let anchors = diff_anchors(&a, &b);
        assert_eq!(anchors.len(), 2);
        for (pa, pb) in anchors {
            assert_eq!(a[pa..].split(' ').next(), b[pb..].split(' ').next());
        }
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
        let annotator =
            |before: &str, span: &str, after: &str| format!("[{}|{}|{}]", before, span, after);
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
        assert!(
            result.contains("<b>bar</b>"),
            "original HTML lost in: {result}"
        );
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
