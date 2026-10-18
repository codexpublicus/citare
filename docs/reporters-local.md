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

## Massachusetts Appellate Division

CAP files the Reports of Massachusetts Appellate Division in two series: `mass-app-div`, the
numbered volumes 1 to 15, and `mass-app-div-annual`, the annual volumes numbered by year
(1980 to 2015). reporters-db has one `Mass. App. Div.` that takes any volume, so `1995 Mass.
App. Div. 12` took the numbered series' slug, a directory with no volume 1995 (#75). The file
caps that edition at a three-digit volume and adds the annual series as a reporter of its own,
`Reports of Massachusetts Appellate Division (Annual)` (CAP's name), whose volume is a year. The
two then work like the Ohio Reports and the Ohio neutral citation: each citation matches one
edition, and its `slug` and `harvard_slug` are that series'.

## Ohio Reports and Ohio State Reports

The Ohio Reports (1821-1851) have 20 volumes, CAP's `ohio`; the Ohio State Reports (1852-1964)
have 177, CAP's `ohio-st`. reporters-db's Ohio Reports take any one- to three-digit volume, so
`127 Ohio 545` (Troop A Riding Academy v. Miller, 1934, which CAP files as `127 Ohio St. 545`)
took the slug `ohio`, a directory with no volume 127 (#80).

I measured how opinions write `<volume> Ohio <page>` from CAP's case metadata
(`CasesMetadata.json`'s `cites_to`, the citation as the opinion prints it) for every volume of
Ohio St., Ohio App., Ohio Law Abs., Ohio N.P., Ohio C.C., Ohio Cir. Dec., Ohio Misc., Ill., Ind.
and Mass., and N.E.2d volumes 1 to 250. Of the 142 citations with a volume above 20, 103 give the
first page of an Ohio State Reports case at that volume, 37 a page inside one, one a page past
the end of its volume and one volume 295, which neither series has. Illinois courts wrote 72 of
them and Indiana courts 21; 65 of the 142 date from the 1880s and 1890s. The same volumes cite
`Ohio St.` 89,809 times.

The file caps the Ohio Reports at volume 20 and adds `Ohio` as a variation of the Ohio State
Reports that takes volumes 21 to 177 only. `127 Ohio 545` keeps its reporter, `Ohio`, as the
text has it, and is a variation match (`variation_editions`, not `exact_editions`) whose edition,
`slug` and `harvard_slug` are the Ohio State Reports'. A volume of 1 to 20 stays the Ohio Reports:
the two series both have those volumes, and `cites_to` gives the first page of an Ohio Reports
case (the page in its official citation; CAP's `ohio` volumes are an 1872 reprint, whose
`first_page` differs) for 6,805 such citations against 520 that give only the first page of an Ohio State Reports
case. A volume above 177 is neither and is no longer a citation.

## Format

`editions` entries name an edition by reporters.json key, cluster name and edition name, and
replace its regexes; the build fails if one no longer matches an edition. `reporters` entries add
a reporter reporters-db doesn't have, under a reporters.json key, with one edition; the build
fails if reporters-db gains a reporter of the same name under that key, so the local one can be
dropped. `variations` entries add an abbreviation for an edition, named like an `editions` entry,
that matches only through the entry's own regexes, unlike a reporters-db variation, which takes
its edition's; the build fails if the edition is gone or reporters-db gains the same variation.
An entry's `examples` must be recognized as its edition with the given volume, and its
`rejects` must not be; `tests/reporters_local.rs` checks both.

## Measured

Before and after this file, over the CourtListener ten-percent sample citation-bench pins
(`courtlistener-10pct`, 7,632 opinions), citare found 159,909 and 159,908 citations. The one
that went is `No. 93 LA 11` above. No citation changed and none was added. citation-bench's
conformance vectors pass as before (270 of 278, no regressions). The sample has no `Mass. App.
Div.` citation, so the Massachusetts entries change none of its citations.
