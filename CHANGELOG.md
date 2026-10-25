# Changelog

Releases of citare. Each release is an annotated `vMAJOR.MINOR.PATCH` tag on
`main`; dependents pin it over git (`tag = "v0.4.0"`). Until 1.0, a minor
bump may break the API, and the breaking changes are listed first.

## v0.7.4 - 2026-10-25

### Fixes

- *(clean)* Don't add a space where an inline element ends ([#121](https://github.com/codexpublicus/citare/pull/121))

## v0.7.3 - 2026-10-23

### Fixes

- *(data)* Read the Pennsylvania Fiduciary Reporter's four series as courts write them ([#113](https://github.com/codexpublicus/citare/pull/113))
- *(data)* Cap only ended series, with sources, and tell N.M. App. from N.M. by year ([#109](https://github.com/codexpublicus/citare/pull/109))

## v0.7.2 - 2026-10-22

### Fixes

- *(data)* Read Bradf. vols 1-4 as Bradford's Surrogate Reports unless the year is Iowa's ([#106](https://github.com/codexpublicus/citare/pull/106))
- *(data)* Fold P.R.R. and Cai. R. into CAP's pr and cai ([#105](https://github.com/codexpublicus/citare/pull/105))
- *(data)* Give CAP's brad, cai, charlton-rep, nm-app and pr reporters editions ([#102](https://github.com/codexpublicus/citare/pull/102))
- *(core)* End a case name search at a sentence that ends in a parenthetical ([#98](https://github.com/codexpublicus/citare/pull/98))
- *(core)* Read 'Am. Jur.' as American Jurisprudence, a secondary source ([#97](https://github.com/codexpublicus/citare/pull/97))
- *(core)* Read a court parenthetical without a year into the citation ([#96](https://github.com/codexpublicus/citare/pull/96))

## v0.7.1 - 2026-10-18

### Fixes

- *(core)* Take a year between the case name and the volume as the citation's own ([#89](https://github.com/codexpublicus/citare/pull/89))

## v0.7.0 - 2026-10-18

### Breaking changes

- Pluggable resolvers, tokenizer option and dump_citations ([#81](https://github.com/codexpublicus/citare/pull/81))

### Features

- *(py)* Python package with eyecite's API ([#83](https://github.com/codexpublicus/citare/pull/83))

### Fixes

- *(data)* Read `Ohio` volumes 21-177 as the Ohio State Reports ([#86](https://github.com/codexpublicus/citare/pull/86))
- *(core)* Group nameless parallel runs and resolve an id. used as a case name ([#84](https://github.com/codexpublicus/citare/pull/84))
- *(core)* Give harvard_slug and slug the CAP reporter cited ([#79](https://github.com/codexpublicus/citare/pull/79))
- *(core)* Resolve references after parallel cites ([#77](https://github.com/codexpublicus/citare/pull/77))
- *(core)* Keep parallel citation groups to one decision ([#76](https://github.com/codexpublicus/citare/pull/76))

## v0.6.0 - 2026-10-17

### Breaking changes

- *(core)* `Edition` has a new public `slug` field, so code that builds an `Edition` with a struct literal has to set it

### Features

- *(core)* Give every reporter edition a stable slug ([#71](https://github.com/codexpublicus/citare/pull/71))

## v0.5.3 - 2026-10-08

### Features

- *(core)* Recognize state-code statutes ([#69](https://github.com/codexpublicus/citare/pull/69))

### Fixes

- *(core)* Resolve supras after parallel cites ([#65](https://github.com/codexpublicus/citare/pull/65))

## v0.5.2 - 2026-10-05

### Fixes

- *(core)* Extend section pins and statutes ([#58](https://github.com/codexpublicus/citare/pull/58))

### Performance

- *(core)* Make citation finding linear ([#59](https://github.com/codexpublicus/citare/pull/59))

## v0.5.1 - 2026-10-05

### Features

- Load reporter tables from a data bundle ([#50](https://github.com/codexpublicus/citare/pull/50))

## v0.5.0 - 2026-10-03

### Breaking changes

- *(core)* Map annotate spans in bytes only ([#45](https://github.com/codexpublicus/citare/pull/45))

### Features

- Markup input for WASM and CLI ([#38](https://github.com/codexpublicus/citare/pull/38))

### Fixes

- *(core)* Capture § pin cites on id. cites ([#41](https://github.com/codexpublicus/citare/pull/41))
- Make extraction output deterministic ([#42](https://github.com/codexpublicus/citare/pull/42))
- *(core)* Key CAP slugs by edition ([#36](https://github.com/codexpublicus/citare/pull/36))

## v0.4.1 - 2026-10-02

### Fixes

- *(core)* Keep spans on UTF-8 char boundaries ([#30](https://github.com/codexpublicus/citare/pull/30))

### Performance

- *(core)* Anchor the markup offset-map diff ([#31](https://github.com/codexpublicus/citare/pull/31))

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
