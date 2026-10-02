# Changelog

Releases of citare. Each release is an annotated `vMAJOR.MINOR.PATCH` tag on
`main`; dependents pin it over git (`tag = "v0.4.0"`). Until 1.0, a minor
bump may break the API, and the breaking changes are listed first.

## v0.4.0 - 2026-10-02

### Breaking changes

- A case citation whose page is a run of underscores (`1 U.S. ___`) now has
  `groups["page"] = None`, so the CLI and WASM JSON carry `"page": null`
  instead of `"___"`, as eyecite does. `matched_text` and the spans keep the
  underscores. Such a citation hashes by its position in the document instead
  of its memory address, so it now equals its own clones. (#14)
- `Document` has two new public fields, `plain_to_markup` and
  `markup_to_plain`. Code that builds a `Document` with a struct literal has
  to set them; `Document::new` is unaffected. `source_text` now holds the
  markup when markup was given. (#15)
- `clean::html` follows eyecite's lxml extraction: text nodes are kept
  verbatim instead of trimmed (`In <i>Foo</i>` becomes `"In  Foo"`), and only
  a text node's direct parent is checked against the suppressed tags and an
  exact `class="star-pagination"`. Cleaned HTML, and the byte spans computed
  on it, differ from v0.3.0. (#15)
- Reference citations take their metadata from the regex groups that
  matched, as eyecite's `groupdict()` does: `pin_cite` is now set, and only
  the party name that matched is filled in instead of both names being
  copied from the full citation. (#17, #19)
- Reporter names match with relaxed whitespace (`N. Y. S. 2d`,
  `18 U. S. C. § 4241`), and `all_editions` no longer lists one edition twice.
  Text that produced nothing before can now produce citations, and a
  citation that only looked ambiguous because of a duplicate edition no
  longer is. (#20)
- `JsCitation` (citare-wasm) has new public fields; see Features. The JSON
  change is additive, but a Rust struct literal of it has to set them. (#16)

### Features

- Markup input. `get_citations` with `markup_text` cleans the markup into
  plain text (before, markup-only input found no citations), keeps span maps
  between the two, reads party names from `<em>`/`<i>` tags, and finds
  reference citations wherever a party name is set in an emphasis tag. A
  missing `html` clean step is added with a warning. (#15, #19, fixes #5)
- Unicode-aware offsets. `OffsetMap`, `OffsetUnit` and `CitationSpans`
  convert byte spans to code points (eyecite, Python) or UTF-16 units
  (JavaScript), and `Citation::pin_cite_span` returns the pin cite bounds.
  WASM citations carry `utf16` and `chars` span objects next to the byte
  `span`, plus byte `full_span` and pin cite bounds. The CLI takes
  `--offsets bytes|chars|utf16` (default `bytes`) on `find` and `resolve`,
  and both now print `full_span`, `pin_cite_span_start` and
  `pin_cite_span_end`. (#16, fixes #11)
- Upstream sync to eyecite 2.7.8, reporters-db v3.2.66 (`e095e6b`) and
  courts-db `5a7a532`: 1,236 reporters, 1,368 editions, 2,809 courts. (#20)

### Fixes

- Blank pages are `None`, as above. (#14, fixes #7)
- Reference citations carry eyecite's metadata, and `v. Bar, 1 U.S. 1` sets
  `antecedent_guess` to `Bar` the way eyecite does. (#17, fixes #6)
- When two extractors matched the same span, the merged edition list never
  reached `all_tokens`, so the result depended on hash-set order. (#20)
- The workspace passes `cargo fmt` and clippy again. (#13)

### Performance

- `get_citations` reuses one process-wide tokenizer instead of building the
  automata and regexes on every call (#4), caches compiled metadata regexes,
  skips the reference regex when no party name follows the citation, and
  precomputes court lookup keys at build time instead of parsing courts.json
  at runtime (#18). A single short input in a release build goes from about
  61 ms to about 2 ms per call.
- The regex crates build at opt-level 2 in dev builds, which takes
  `cargo test` from about 41 s to about 10 s. (#18)

### Parity

- citation-bench eyecite-2.7.8 find vectors: 202/202 (147/202 at v0.3.0).
- citation-bench v2 set: 268/278.
- 278 tests.

## v0.3.0 - 2026-03-24

- Consolidate 10K regexes into 268 templates plus a reporter map (37%
  smaller generated source).
- The build fails on an invalid regex instead of skipping it.
- Ambiguous reporters are exposed through `is_ambiguous()` instead of being
  dropped.
- 222 tests; 150/150 cross-validation against the eyecite test suite.
- WASM: 6.0 MB raw, 1.3 MB gzipped.

## v0.2.0 - 2026-03-23

- `harvard_slug` on `Edition` (366/399 Harvard CAP reporters matched),
  exposed in the CLI, WASM and Rust API.
- ReportersMetadata.json embedded at build time.
- Lazy per-extractor regex compilation (0.1 s cold start).

## v0.1.0 - 2026-03-23

- Citation finding (case, law, journal, short, supra, id, reference),
  resolution of short forms to full citations, annotation and text cleaning.
- CLI (`citare find/resolve/annotate`) and WASM bindings (`getCitations`,
  `resolveCitations`, `annotateCitations`).
- Build-time regex generation from reporters-db and courts-db.
