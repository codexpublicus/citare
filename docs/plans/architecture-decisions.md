# Architecture Decisions

Decisions made after analyzing eyecite's architecture and comparing with alternatives.

## Core Principle

**Don't replace what works. Improve what doesn't.**

Citation finding (operations 1+2) is a lookup problem fully specified by reporters-db. Keep regex/AC. Case name extraction (operation 3) is an unstructured problem — candidate for ML.

---

## Decided: Short Term (v0.3)

### 1. Consolidate 10K regexes → 76 generic templates + AC primary matcher

**Decision:** Fix
**Rationale:** 94% size reduction (5.2MB → ~300KB generated source). No accuracy change. AC already finds reporter strings — promote from filter to matcher, validate surrounding structure with shared template regex.

### 2. Return ambiguous reporters instead of dropping them

**Decision:** Fix
**Rationale:** Silently dropping citations is a correctness bug. Return all candidate editions and let the consumer decide. Add an `ambiguous: bool` flag to citation output.

### 3. Make build.rs fail on invalid regex (not warn-and-skip)

**Decision:** Fix
**Rationale:** Silent failures are dangerous. If reporters-db adds a pattern that doesn't compile in Rust, the build should fail loudly, not silently skip the reporter.

---

## Decided: Medium Term (v0.4+)

### 4. ML model for case name extraction

**Decision:** Fix with ML
**Rationale:** The 260-line backward-scanning heuristic is the system's weakest link. Case name boundaries are an unstructured problem well-suited to sequence labeling. Train a small BiLSTM-CRF or distilled transformer (~2MB) on CourtListener data. Must run in WASM (ONNX Runtime).

### 5. Add wasm-opt to build pipeline

**Decision:** Fix
**Rationale:** Free size reduction, no downside.

---

## Decided: Accept (for now)

### 6. Duplicate named capture groups rewrite layer

**Decision:** Accept
**Rationale:** Working, tested, complexity is contained in `merge_branch_groups()`. Would be cleaner to rewrite the regexes to avoid alternation branches, but not worth the risk of subtle behavioral changes.

### 7. No fuzzy matching for OCR errors

**Decision:** Accept
**Rationale:** Matches eyecite behavior. Could add edit-distance matching later if recall data shows it's needed.

### 8. DISALLOWED_NAMES hardcoded list

**Decision:** Accept, make configurable later
**Rationale:** Pragmatic solution. Move to a data file when we need non-US support.

### 9. Pin cite lookahead requires fancy-regex

**Decision:** Accept
**Rationale:** Not in hot path (once per citation, not once per extractor). Could rewrite to avoid lookahead but risk is not worth the modest performance gain.

### 10. Nominative reporter hardcoded list

**Decision:** Track upstream
**Rationale:** Should be a flag in reporters-db, not downstream hardcoding. Keep hardcoded list as fallback until reporters-db adds the flag.

---

## Rejected: Full ML replacement of citation finding

**Decision:** Reject
**Rationale:**
- Citation finding is a lookup problem, not a learning problem
- reporters-db fully specifies what to find — nothing to learn
- ML would be bigger (5-20MB model), slower (50-500ms), less accurate on rare reporters (99.5% vs 100%)
- New reporters require retraining vs. adding a JSON entry
- Legal domain requires deterministic, auditable results
- Browser/WASM deployment constraints make model size critical

**Exception:** ML is appropriate for case name extraction (#4) where the problem IS unstructured.
