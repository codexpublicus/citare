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
}
