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

## American Jurisprudence

reporters-db's only `Am. Jur.` is the American Jurist and Law Magazine, a reporter in
reporters.json, which ran from 1829 to 1843 in 28 volumes. Opinions from the late 1930s on cite
American Jurisprudence, the encyclopedia (first series 1936-1948, 58 volumes), as `Am. Jur.`, and
citare read it as a case citation of the American Jurist. In `Smith v. United States ex rel.
Gorlo, 7 Cir., 52 F.2d 848; 13 Am.Jur. 383` (CAP `a2d/31/0693-01`, Ray v. Bruce) it then joined
Smith's parallel citations across the semicolon and took Smith's name (#93).

The file's `journals` entries add a secondary source to the journals, and American Jurisprudence
is the first. A citation whose abbreviation is both a journal's and a reporter's is a journal
citation, as one that matched any journal already was, so a citation of `Am. Jur.` is a journal
citation whichever it is, and a journal citation never joins a run of case citations. Its
edition is the journal's unless its year falls in the reporter's run: `13 Am.Jur. 383` is
American Jurisprudence and `1 Am. Jur. 441 (1830)` the American Jurist. A journal citation also
keeps a section pin cite, `13 Am. Jur. 383, § 12`, as a case citation does.

Two other abbreviations are already both: `Cent. Law J.`, the Central Law Journal, and `N. Y.
L. J.` written with spaces, which matches both reporters-db's `N.Y.L.J.` reporter and its `N.Y.
L.J.` journal, both the New York Law Journal. They were journal citations without an edition
and now have the journal's.

## Format

`editions` entries name an edition by reporters.json key, cluster name and edition name, and
replace its regexes; the build fails if one no longer matches an edition. `reporters` entries add
a reporter reporters-db doesn't have, under a reporters.json key, with one edition; the build
fails if reporters-db gains a reporter of the same name under that key, so the local one can be
dropped. `variations` entries add an abbreviation for an edition, named like an `editions` entry,
that matches only through the entry's own regexes, unlike a reporters-db variation, which takes
its edition's; the build fails if the edition is gone or reporters-db gains the same variation.
`journals` entries add a secondary source under its abbreviation, as journals.json would; the
build fails if journals.json gains the abbreviation.
An entry's `examples` must be recognized as its edition with the given volume, and its
`rejects` must not be; `tests/reporters_local.rs` checks both.

## Measured

Before and after this file, over the CourtListener ten-percent sample citation-bench pins
(`courtlistener-10pct`, 7,632 opinions), citare found 159,909 and 159,908 citations. The one
that went is `No. 93 LA 11` above. No citation changed and none was added. citation-bench's
conformance vectors pass as before (270 of 278, no regressions). The sample has no `Mass. App.
Div.` citation, so the Massachusetts entries change none of its citations.

The Ohio entries change two of the sample's 159,914 citations, `127 Ohio 545` and `50 Ohio 107`,
whose corrected reporter goes from `Ohio` to `Ohio St.`; none is gained or lost, and the vectors
still pass 270 of 278. `50 Ohio 107`, in a Kansas opinion, is Putnam v. Valentine, `5 Ohio 187`,
misprinted, so its slug is wrong before and after.

The American Jurisprudence entry changes 24 of the sample's citations, and none is gained or
lost. The 23 `Am. Jur.` citations, in 18 opinions, are all the encyclopedia (15 of the opinions cite
second-series reporters, and the volumes match the encyclopedia's titles: `20 Am. Jur. 299,
Evidence, § 318`, `33 Am. Jur. 146` on libel). They become journal citations of American
Jurisprudence; five of them had taken the name and span of
the case citation before them (`Wharam v. Investment Underwriters`, `National Labor Relations
Board v. Jones & Laughlin Steel Corp.`), three a junk defendant (`In`, `and`) and two the year
of a later citation. `69 N. Y. L. J. 1170` takes the journal's edition. citation-bench's vectors
pass 269 of 278 before and after.
