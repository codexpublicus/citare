# Architecture Issues Inherited from eyecite

Issues discovered during analysis. Each needs a decision: fix, improve, or accept.

---

## Burndown

- [ ] **1. Redundant regex expansion (76 templates × 3,574 strings = 10K regexes)**
- [ ] **2. Ambiguous reporters silently dropped**
- [ ] **3. Nominative reporter hardcoded workaround**
- [ ] **4. Case name extraction is a 260-line heuristic with magic numbers**
- [ ] **5. Regex size bloat in WASM (5.2MB generated source)**
- [ ] **6. `{,N}` and other Python regex-isms leak through data**
- [ ] **7. Duplicate named capture groups require rewrite layer**
- [ ] **8. No fuzzy matching for OCR errors beyond enumerated variations**
- [ ] **9. `DISALLOWED_NAMES` is a hardcoded list of 100+ AG surnames**
- [ ] **10. Pin cite lookahead requires `fancy-regex` (slower engine)**

---

## Details

### 1. Redundant regex expansion

**Severity:** Efficiency / Size
**Source:** eyecite `tokenizers.py:_populate_reporter_extractors()`

Only 76 unique structural templates exist, but they're expanded to 10,257 individual regexes by substituting each reporter name (and each variation) into each template. The reporter name is stored in both the AC string list AND embedded in the regex pattern.

**Impact:** 5.2MB generated source, 7.8MB WASM binary. Conceptual complexity makes the system appear more complex than it is.

**Fix:** Replace per-reporter regexes with ~76 generic template regexes. AC already finds reporter strings — promote it from filter to primary matcher. Validate surrounding structure with generic template. Estimated 94% size reduction.

### 2. Ambiguous reporters silently dropped

**Severity:** Correctness
**Source:** eyecite `helpers.py:disambiguate_reporters()`

When `remove_ambiguous=True`, citations matching multiple reporters (24 reporter keys are ambiguous) are simply removed from the output. No fallback, no confidence score, no "best guess" — just dropped.

Even when `remove_ambiguous=False`, ambiguous citations have `edition_guess=None`, meaning downstream consumers get no reporter resolution.

**Impact:** Real citations are silently lost. A citation to "Met. 123" (which could be Kentucky or Massachusetts Metcalf) just disappears.

**Fix options:**
- Return all candidates with confidence scores
- Use contextual signals (jurisdiction of the document, surrounding citations) to disambiguate
- At minimum, return the ambiguous citation with all possible editions rather than dropping it

### 3. Nominative reporter hardcoded workaround

**Severity:** Maintainability / Correctness
**Source:** eyecite `tokenizers.py:54-64, 67-92`

Nine nominative reporter names (Thompson, Cooke, Holmes, etc.) are hardcoded as `NOMINATIVE_REPORTER_NAMES`. These are common surnames that are also historical reporter names. When a nominative reporter overlaps with another citation match, the nominative is deprioritized.

The comment says: "Cleaner way to do this would be via an attribute or named group from reporters-db. However; this tagging is currently not complete."

**Impact:** The hardcoded list may be incomplete. New nominative reporters added to reporters-db won't be handled. The fix should come from reporters-db, not from downstream consumers.

**Fix options:**
- Upstream: add a `nominative: true` flag to reporters-db edition data
- Citare: use the reporters-db flag if/when available, keep hardcoded list as fallback

### 4. Case name extraction heuristic

**Severity:** Accuracy
**Source:** eyecite `helpers.py:138-398`

260 lines of backward-scanning heuristic with:
- `BACKWARD_SEEK = 28` (magic number from "median case name length in the CL db, 2016-02-26")
- Hardcoded word lists for stop conditions
- Multiple fallback paths for different punctuation patterns
- Separate code paths for plaintiff v. defendant, in re, ex parte, etc.
- `strip_stop_words()` applies stop word removal twice with different regex flags

**Impact:** Case name extraction is the weakest part of the pipeline. Long or unusual case names get truncated. Short case names may capture preceding sentence text. The heuristic is fragile and hard to modify.

**Fix options:**
- ML-based case name extraction (NER/sequence labeling) — this is a genuinely unstructured problem well-suited to ML
- Improved heuristic with better boundary detection
- Use HTML markup (when available) as primary signal, fall back to heuristic

### 5. Regex size bloat in WASM

**Severity:** Deployment
**Source:** `build.rs` generated output

5.2MB of generated Rust source containing 10K regex string literals. Compresses well (1.4MB gzipped WASM) but still large for a browser library.

**Impact:** Slower page loads, larger bundle size than necessary.

**Fix:** Addressed by fixing issue #1 (template consolidation). Also: add wasm-opt to build pipeline.

### 6. Python regex-isms leak through data

**Severity:** Compatibility
**Source:** reporters-db `regexes.json`, `laws.json`

The regex template variables from reporters-db use Python regex syntax:
- `{,3}` (valid in Python, invalid in Rust — requires `{0,3}`)
- Duplicate named capture groups (valid in PyPI `regex`, invalid everywhere else)
- Various escaping conventions

We already fix `{,N}` → `{0,N}` in the tokenizer. But new patterns added to reporters-db could introduce new Python-isms.

**Impact:** Silent regex compilation failures if not caught.

**Fix:** Add a validation pass in build.rs that compiles every generated regex and fails the build if any are invalid. Currently we log warnings and skip — should be an error.

### 7. Duplicate named capture groups require rewrite layer

**Severity:** Complexity
**Source:** eyecite `helpers.py` uses PyPI `regex` module (not stdlib `re`)

`SUPRA_ANTECEDENT_REGEX` and `POST_FULL_CITATION_REGEX` use the same group name in multiple alternation branches. Neither Rust `regex` nor `fancy-regex` supports this. We maintain a rewrite layer (`merge_branch_groups`) that renames groups to `_a`/`_b` suffixes and merges them back.

**Impact:** Extra complexity, potential for subtle bugs if the merge logic doesn't match Python's semantics.

**Fix options:**
- Accept the complexity (it works, it's tested)
- Rewrite the regexes to avoid alternation branches with shared groups (use sequential matching instead)
- Upstream: propose that eyecite/reporters-db avoid duplicate named groups

### 8. No fuzzy matching for OCR errors

**Severity:** Recall
**Source:** reporters-db `variations` field

Reporter variations are an exhaustive enumeration of known misspellings: `"A .2d"`, `"A,2d"`, `"Atl.2d"`. Novel OCR errors (e.g., `"A.Zd"` for `"A.2d"`) are missed.

**Impact:** Citations from OCR'd documents with unusual errors are not found.

**Fix options:**
- Edit-distance matching on AC hits (e.g., if AC finds something close to a known reporter within Levenshtein distance 1)
- Separate OCR correction pass before citation extraction
- Accept the limitation (eyecite has the same limitation)

### 9. DISALLOWED_NAMES hardcoded list

**Severity:** Maintainability
**Source:** eyecite `utils.py:10-108`

A hardcoded list of 100+ names (mostly US Attorney Generals) that cannot be reference citations. This prevents false positives like "Garland at 5" being treated as a reference to a case when it's referring to AG Garland.

**Impact:** The list needs manual maintenance when new AGs are appointed. It's US-centric.

**Fix options:**
- Accept it (it's a reasonable pragmatic solution)
- Make it configurable (allow users to add/remove names)
- Move to a separate data file rather than hardcoded in source

### 10. Pin cite lookahead requires fancy-regex

**Severity:** Performance / Complexity
**Source:** eyecite `regexes.py:229-234`

`PIN_CITE_REGEX` uses `(?= ...)` lookahead to avoid capturing the start of the next citation. This lookahead propagates to 5 metadata regexes. We use `fancy-regex` (slower than `regex`) for all metadata matching because of this.

**Impact:** Metadata regex matching is slower than it needs to be. `fancy-regex` adds binary size.

**Fix options:**
- Rewrite `PIN_CITE_REGEX` to avoid lookahead (use a consuming pattern with post-processing)
- Accept the performance cost (metadata matching is not in the hot path — only runs once per citation, not once per extractor)
- Hybrid: use `regex` for patterns without lookahead, `fancy-regex` only where needed (current approach)
