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
`conn-super-ct`. It is a disambiguation cap, not a last volume; [Volume caps](volume-caps.md)
states the rule every cap in this file follows and lists each one with its source.

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

## Five CAP reporters without an edition

stare's index (v2026.10.18) had five CAP reporters that no citare edition took the slug of:
`brad`, `cai`, `charlton-rep`, `nm-app` and `pr` (#99). Each is in reporters-db under another
abbreviation, as another reporter's variation, or not at all, so a citation in the form CAP
gives its own cases took another reporter's slug or none. The file adds each as a reporter of
its own, under CAP's short name, so the edition's `slug` and `harvard_slug` are CAP's.

I measured how opinions cite them from CAP's case metadata (`CasesMetadata.json`'s `cites_to`,
whose `case_paths` name the CAP case each citation resolves to) for every volume of the five,
Iowa's `bradf`, `cai-cas`, `charlton`, Barb., Cow., Ga., Ga. App., Hill, Johns., N.M., N.Y.,
Redf. and Wend. The counts below are of citations CAP resolves to a case unless they say
otherwise.

- **Bradford's Surrogate Reports** (`brad`, vols 1-4, New York Surrogate's Court, 1849-1857),
  cited by CAP as `1 Brad. 1`. reporters-db's only `Brad.` is a variation of Iowa's Bradford
  (`Bradf.`, 1839-1841, CAP's `bradf`, vols 1-3), and 113 of 114 `Brad.` citations name a case in
  `brad`. Opinions write `Bradf.` for it more often still: 600 of 614 `Bradf.` citations name a
  case in `brad`, 14 one in `bradf`. The file caps Iowa's Bradford at volume 3 and adds
  Bradford's Surrogate Reports under `Bradf.` too, for volumes 1 to 4, with the slug `brad`;
  volumes 1 to 3 take it unless the year is in Iowa's run ([below](#bradf-volumes-1-to-3)).
- **Caines' Reports** (`cai`, vols 1-3, New York Supreme Court of Judicature, 1803-1805), cited
  by CAP as `1 Cai. 1` and, for a few cases, `1 Cai. 22a`. reporters-db has the reporter as
  `Cai. R.` (released as `cai-r`, now `cai`) and gives `Cai.` to Caines' Cases (`cai-cas`) as a
  variation, so `1 Cai. 1` took `cai-cas`. Of 117 `Cai.` citations, 114 name a case in `cai` and
  3 one in `cai-cas`, and the Bluebook (T1) also gives `Cai.` to Caines' Reports and `Cai. Cas.`
  to Caines' Cases. A volume above 3 isn't Caines' Reports and stays a variation of Caines'
  Cases.
- **T.U.P. Charlton's Reports** (`charlton-rep`, vol 1, Georgia superior courts, 1805-1824),
  cited by CAP as `1 Charlton Rep. 13`. reporters-db has only R.M. Charlton's reports (1811-1837,
  CAP's `charlton`, cited `1 Charlton 1`), with `Charlton Rep.` among its variations. No
  `cites_to` entry names a case in either: Georgia opinions cite the two without a volume
  (`T. U. P. Charlton, 311` 14 times and `R. M. Charlton, 1` 29 times in the Ga. volumes' text),
  which no reporter regex reads, so `Charlton Rep.` is CAP's own form, and the file takes only
  volume 1 of it.
- **New Mexico Appellate Reports** (`nm-app`, vols 1-10, 2011-2016), which reporters-db doesn't
  have. CAP cites every one of its cases by the neutral citation alone (`2011-NMCA-108`,
  `2016-NMSC-021`), whose volume is a year and whose slugs are `nmca` and `nmsc`, and no
  `cites_to` entry reads `N.M. App.`. The edition gives CAP's slug a name and reads `5 N.M. App.
  1` if one is written. It took CAP's ten volumes only until the owner's ruling D80 removed the
  cap: the series is still published, and the edition's 2011 start year now tells it from the New
  Mexico Reports written `84 N.M.App. 83` (CAP `nm/84`, 0077-02), which a variations entry adds
  ([Volume caps](volume-caps.md#new-mexico-appellate-reports)).
- **Puerto Rico Reports** (`pr`, vols 3-100, the English translation, 1899-1972), cited by CAP
  as `3 P.R. 2`. reporters-db has the reporter as `P.R.R.` (released as `p-r-r`, now `pr`) with `P.R.`
  as a variation of it, the Pacific Reporter and Penrose and Watts, so `45 P.R. 100` was
  ambiguous and took the Pacific Reporter's slug, `p`. All 85 `P.R.` and `P. R.` citations name a
  case in `pr`. The file caps the edition at volume 100, the series' last (sources in
  [Volume caps](volume-caps.md)), so a higher volume stays ambiguous as before.

reporters-db's `Cai. R.` and `P.R.R.` are the same books as CAP's `cai` and `pr`, and citare 0.7.0
and 0.7.1 released them as `cai-r` and `p-r-r`, slugs CAP has no volumes under. The Puerto Rico
Supreme Court's own opinions write `P.R.R.` (30,866 of the `cites_to` entries in `pr`, which CAP
doesn't resolve), so most citations of the Puerto Rico Reports named no CAP volume. The owner's
ruling D78 folds both into CAP's slug; no index file or case URL uses `cai-r` or `p-r-r`, so the
fold moves no published path. Each keeps its reporters-db edition, so `45 P.R.R. 100` is still
corrected to `P.R.R.`, as eyecite's own tests expect (`test_disambiguate_citations`), and shares
CAP's slug the way `Day.` shares `day` (`citare_data_build::slugs::BY_REPORTER` and
`SAME_BOOKS`): `45 P.R.R. 100` and `45 Puerto Rico 100` are `pr`, and `2 Cai. R. 169` and
`2 Cai.R. 169` are `cai`, each with the same `harvard_slug`. The file caps the two editions at the
series' volumes, as it does `P.R.` and `Cai.`, so `101 P.R.R. 1` and `4 Cai. R. 1` are no longer
citations. A year outside the series' run (`45 P.R.R. 100 (1850)`) leaves the edition as it is,
since no other reporter is cited as `P.R.R.` or `Cai. R.`. stare.pub's `reporters.json` lists
`p-r-r` and `cai-r`; stare keeps them for one release, marked as replaced by `pr` and `cai`.

I compared citare before and after the fold over the opinion text of 1,626 CAP volumes, the
1,360 measured below and Iowa's (`bradf`, `greene`, `morris`, `iowa`), which have 3,774,274 full case,
short case, law and journal citations. 36,469 change, in `slug` and `harvard_slug` and nothing
else: 36,435 `P.R.R.` citations (written `P. R. R.`, `P.R. R.` and `45 P.R.R. at 100` among them)
go from `p-r-r` with no `harvard_slug` to `pr`, and 34 `Cai. R.` citations from `cai-r` to `cai`.
Two are lost, `127 P.R.R. 746` (CAP `pr/62`, 0760-01) and `179 P.R.R. 511` (`pr/82`, 0695-01),
volumes the series doesn't have, each in a run of `P.R.R.` citations of volumes below 100.

An exact edition comes before a variation's when citare guesses a citation's edition, as in
eyecite, so `P.R.` would have made `1 P.R. 1 (1831)`, which eyecite's own tests read as Penrose
and Watts (1829-1832) through its `P.R.` variation, the Puerto Rico Reports. citare now passes
over the exact editions when the citation's year falls outside all of them and inside a
variation's (`guess_edition` in `crates/citare-core/src/helpers.rs`).

I compared citare before and after over the opinion text of every CAP volume measured above,
1,360 volumes with 4,000,932 citations. 170 citations changed and none of the rest: 93 `P.R.` and
`P. R.` citations went from the Pacific Reporter's `p` to `pr`, 31 `4 Bradf.` and 24 `Brad.`
citations from `bradf` to `brad`, and 21 `Cai.` citations from `cai-cas` to `cai`. Two were
lost, `8 Brad., 467` (CAP `ga/71`) and `8 Bradf., 629` (`ny/25`), whose volume neither Bradford
has; four `N.M.App.` citations of the New Mexico Reports that the volume cap now rejects were
never found. One went wrong: `In re Seneca Trails, Inc., 7 B.R. 128 (Bkrtcy.W.D.Pa.1980)` (CAP
`nm/97`, 0692-01) takes the year 1974 from elsewhere in its sentence, which the Bankruptcy
Reporter (1979-) rules out, so it became the Baltimore City Reports. At the citation-bench
commit CI pins, the vectors pass as before: eyecite's 270 of 278, the divergences 93 of 94, the
CAP slugs 11 of 12 and the statutes 75 of 78.

<a id="bradf-volumes-1-to-3"></a>
### `Bradf.` volumes 1 to 3

Volumes 1 to 3 of `Bradf.` are in both Bradfords, and #99 left them Iowa's, the reporter
reporters-db and the released slug `bradf` give the name, until Iowa's own opinions were
measured (the owner's ruling D73, 2026-10-22). I added CAP's Iowa volumes (`bradf`, `greene`,
`morris` and all 261 of `iowa`, 1839-1968) to the 1,360 above and counted `cites_to` entries.

- Iowa's opinions never cite Iowa's Bradford with a volume. They write `Bradford, 16`, 58 times,
  which no reporter regex reads. CAP resolves no citation in them to `bradf`.
- Every `Bradf.` or `Brad.` with a volume in Iowa's opinions is the New York reports: `1 Brad.,
  476` and `2 Brad., 133` (resolved to `brad`), `4 Brad., 260`, `3 Brad. 500`, and `1 Bradf. 52`,
  `1 Bradford, 437`, `3 Bradf. (Sur.) 144`, `2 Bradf. (N. Y.) 339` and `1 Bradf. (N. Y. Sur.)
  476`, whose pages Iowa's volumes don't have (they end at pages 20, 67 and 55).
- Over all 1,626 volumes, there are 793 `Bradf.` citations of volumes 1 to 4. CAP resolves 610
  to `brad` and 14 to `bradf`, and all 14 are New York opinions citing a New York surrogate's case
  at a page Iowa's volume also has: `2 Bradf. 42` eight times, five of them named `Weir v.
  Fitzgerald` (CAP `brad/2`, 0042-01), `Phyfe v. Phyfe, 3 Bradf. 45` (`brad/3`, 0045-01) four
  times, `Ex parte Dawson, 3 Bradf. 30` and `Glover v. Holley, 2 Bradf., 29`.

So none of the 793 is Iowa's Bradford, and the file gives `Bradf.` volumes 1 to 3 to Bradford's
Surrogate Reports. Iowa's edition stays, with the slug `bradf`, and still takes a citation whose
year is in its run (`1 Bradf. 3 (1840)`); `citare_data_build::slugs::SORTS_LAST` sorts it after
Bradford's Surrogate Reports, so a citation without such a year (`1 Bradf. 356`) is `brad`.
Bradford's Surrogate Reports' regex for volumes 1 to 3 is the same as Iowa's, so one extractor
finds both editions in that order; the build orders editions only within an extractor.
`data/edition-slugs.json` gives `Bradf.` both slugs, `brad` and `bradf`.

Before and after over the 1,626 volumes' opinion text (3,774,274 citations), 217 citations change,
all from `bradf` to `brad`: 130 in Redfield's Surrogate Reports, 55 in N.Y., 31 in Barb. and one
in Ga. (`2 Bradf. 107`). None is in an Iowa opinion. A `Bradf.` citation of volumes 1 to 3 without
a year is now ambiguous between the two, as `172 Wash. 688` is, so `--remove-ambiguous` drops it.

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
a reporter reporters-db doesn't have under a reporters.json key, with one edition; the build
fails if reporters-db gains a reporter of the same name under that key, so the local one can be
dropped. The reporter may be one reporters-db has under another key (`Cai.` and `Cai. R.` are
both Caines' Reports); the two then share CAP's slug (`citare_data_build::slugs::SAME_BOOKS`). `variations` entries
add an abbreviation for an edition, named like an `editions` entry or a `reporters` entry's,
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
