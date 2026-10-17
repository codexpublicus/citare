# Neutral-citation volumes (`data/reporters-local.json`)

reporters-db gives the Louisiana, North Dakota, New Mexico, Pennsylvania, South Dakota and
Vermont neutral citations (`LA`, `ND`, `NM`, `PA`, `SD`, `VT`) no regex of their own, so they
take the default `$volume $reporter,? $page`, where the volume is any number. A neutral
citation's volume is its year (`2021 ND 123`). `data/reporters-local.json` replaces those
editions' regexes with `$volume_year $reporter,? $page`, a volume of 1700 to 2099, and
`crates/citare-core/build.rs` applies it before building the data bundle.

This does two things:

- Citation matching is tighter. `No. 93 LA 11`, a Chicago License Appeal Commission docket
  number, was a Louisiana neutral citation with volume 93 and no longer matches.
- Each neutral citation shares the slug of the CAP reporter with the same letters (`nd` for
  both `2021 ND 123` and `50 N.D. 1`), which the slug checks in `citare_data_build::slugs`
  accept only because the neutral citation's volumes are now years and the CAP reporter's
  last volume is below 1700 (`data/cap_volumes.json`).

The file also caps the Connecticut Superior Court Reports (1986–1994, `Conn. Super. Ct.`) at a
three-digit volume. reporters-db takes any volume for it, which also matches the Superior
Court's neutral citation (`1999 Conn. Super. Ct. 13301-M`, volume a year), so the two can share
`conn-super-ct`.

Each entry names its edition by reporters.json key, cluster name and edition name, and the
build fails if one no longer matches an edition. Its `examples` must be recognized as that
edition with the given volume, and its `rejects` must not be; `tests/reporters_local.rs`
checks both.

## Measured

Before and after this file, over the CourtListener ten-percent sample citation-bench pins
(`courtlistener-10pct`, 7,632 opinions), citare found 159,909 and 159,908 citations. The one
that went is `No. 93 LA 11` above. No citation changed and none was added. citation-bench's
conformance vectors pass as before (270 of 278, no regressions).
