# State-code statute forms (`data/laws-local.json`)

reporters-db's laws data, which eyecite and citare build their statute citations from, has
each state's code under its Bluebook abbreviation: `Conn. Gen. Stat. §`, `Ga. Code Ann. §`,
`42 Pa. Cons. Stat. §`. State courts mostly write their own forms instead: `General Statutes §
52-557n`, `OCGA § 16-9-93`, `42 Pa.C.S. § 5322`, `Penal Law § 160.05`, `(Pen. Code, § 211)`.
Neither engine recognizes those, so the statute comes out as a bare `§` unknown citation and
the code is lost. `data/laws-local.json` adds the forms that came up most often, and
`crates/citare-core/build.rs` merges it into reporters-db's laws before building the data
bundle.

## Measured

Three samples, run through citare at `6665b6d` (v0.5.2 plus CI changes):

- the CourtListener ten-percent sample citation-bench pins (`courtlistener-10pct`, 7,632
  opinions), prepared by citation-bench's `tools/fetch_corpus.py`;
- 820 opinions from the Harvard Caselaw Access Project mirror: up to 15 cases from each of two
  volumes drawn from the newest quarter of 28 reporters with heavy state-statute citation
  (`a3d`, `p3d`, `ne3d`, `so3d`, `se2d`, `sw3d`, `nw2d`, `conn`, `conn-app`, `cal-4th`,
  `cal-app-4th`, `ny3d`, `ad3d`, `ga`, `ga-app`, `haw`, `ariz`, `idaho`, `nc`, `nc-app`,
  `pa-super`, `wis-2d`, `nd`, `tenn`, `md-app`, `vt`, `me`, `okla`), seeded;
- the 38 modern opinions (plus Brooks v. Powers, 138 A.3d 1012) of the stare.pub sample of
  2026-10-06, where 83 unique statutes and rules (428 occurrences) came through unrecognized.

Every unknown citation whose text is a `§` was grouped by the words in front of it, up to the
first lowercase word: `General Statutes`, `42 Pa.C.S.`, `OCGA`. The groups were then folded
into forms by hand (`OCGA` and `O.C.G.A.` are one form, `Penal Law` and `Judiciary Law` another).
The three samples have 14,944, 5,432 and 384 bare-`§` unknowns. 5,811, 1,676 and 84 of them
have nothing statute-like in front (`under § 341`, `Section 63 provides … § 63.`), and are the
opinion's own references to a section of whatever statute it is discussing. The forms with
the most of the rest:

| Form | Example | CourtListener | CAP | stare.pub | All | Opinions | Added as |
|---|---|---:|---:|---:|---:|---:|---|
| Georgia | `OCGA §`, `O.C.G.A. §` | 263 | 248 | 2 | 513 | 87 | `Ga. Code Ann.` |
| New York laws by name | `Penal Law §`, `Judiciary Law §` | 460 | 24 | 4 | 488 | 222 | `N.Y. Law`, subject from the list of consolidated laws |
| Pennsylvania Consolidated Statutes | `42 Pa.C.S. §`, `42 Pa.C.S.A. §` | 348 | 69 | 55 | 472 | 95 | `Pa. Cons. Stat.` |
| California Style Manual | `Pen. Code, §`, `Code Civ. Proc., §` | 186 | 222 | 0 | 408 | 136 | `Cal. Code`, subject as written |
| Arizona | `A.R.S. §` | 142 | 254 | 0 | 396 | 52 | `Ariz. Rev. Stat.` |
| Hawaii | `HRS §` | 135 | 188 | 0 | 323 | 14 | `Haw. Rev. Stat.` |
| Connecticut | `General Statutes §`, `General Statutes (Rev. to 2013) §`, `C.G.S.A. §` | 75 | 227 | 12 | 314 | 67 | `Conn. Gen. Stat.` |
| Idaho and Indiana | `I.C. §` | 119 | 159 | 0 | 278 | 47 | not added: the same abbreviation in two states |
| Sentencing Guidelines | `U.S.S.G. §`, `USSG §` | 272 | 0 | 0 | 272 | 66 | not added: a federal guideline, not a state code |
| Purdon's Pennsylvania Statutes | `40 P.S. §` | 222 | 19 | 30 | 271 | 56 | `Pa. Stat. Ann.` (new) |
| North Carolina | `G.S. §`, `N.C.G.S. §` | 96 | 100 | 1 | 197 | 39 | `N.C. Gen. Stat.` |
| Oklahoma | `12 O.S.2011 §`, `22 O.S. 1951, §` | 121 | 60 | 1 | 182 | 41 | `Okla. Stat.` |
| Texas in small capitals | `TEX. PENAL CODE ANN. §` | 162 | 0 | 11 | 173 | 55 | `Tex. Code Ann.` |
| Massachusetts | `G. L. c. 93A, §`, `G.L.c. 94C, §` | 138 | 4 | 8 | 150 | 23 | `Mass. Gen. Laws` |
| Maine | `19-A M.R.S. §`, `15 M.R.S.A. §` | 105 | 0 | 54 | 159 | 19 | `Me. Rev. Stat. Ann.` |
| Texas without a space | `Tex. Prop.Code Ann. §`, `Tex. Fam.Code §` | 116 | 30 | 0 | 146 | 49 | `Tex. Code Ann.` |
| Vermont | `24 V.S.A. §` | 48 | 95 | 0 | 143 | 26 | `Vt. Stat. Ann.` |
| North Dakota | `N.D.C.C. §` | 41 | 71 | 0 | 112 | 14 | `N.D. Cent. Code` |
| Connecticut court rules | `Practice Book §` | 39 | 56 | 6 | 101 | 33 | `Conn. Practice Book` (new) |
| New York codes not named `Law` | `Family Ct Act §`, `CPLR §` | 81 | 1 | 0 | 82 | 33 | `N.Y.`, subject as written |
| Tennessee | `T.C.A. §` | 49 | 0 | 0 | 49 | 22 | `Tenn. Code Ann.` |
| Wisconsin in small capitals | `WIS. STAT. §` | 35 | 4 | 1 | 40 | 6 | `Wis. Stat.` |
| Michigan | `MCL §`, `M.C.L.A. §` | 39 | 0 | 0 | 39 | 8 | `Mich. Comp. Laws` |
| West Virginia spelled out | `West Virginia Code §` | 28 | 0 | 7 | 35 | 8 | `W. Va. Code` |
| New Mexico | `NMSA 1978, §` | 27 | 0 | 0 | 27 | 7 | `N.M. Stat. Ann.` |
| Alabama | `Ala. Code 1975, §` | 19 | 6 | 0 | 25 | 15 | `Ala. Code` |
| Louisiana | `La.Rev.Stat. §`, `La. R.S. §` | 20 | 0 | 0 | 20 | 8 | `La. Stat. Ann.` |
| Rhode Island | `G.L. 1956 §` | 14 | 0 | 4 | 18 | 7 | `R.I. Gen. Laws` |
| Arkansas | `A.C.A. §` | 13 | 1 | 0 | 14 | 6 | `Ark. Code Ann.` |

The Massachusetts row counts only `§`s straight after `G. L. c. N,`; the grouping pass
lumped them with other `c. N, §` forms (`Laws of 2001, ch. 10, §`), so they were counted
apart.

Also common, and not statutes or not codes: constitutions (`art. I, § 2`, 359), Restatements
(`Restatement (Second) of Torts § 46`, 212), treatises (`Williston on Contracts § 17.5`) and
session laws (`Laws of 2001, ch. 10, § 6`). Maryland's article abbreviations (`CP § 8-301`,
`CS § 8-209.2`, `CR § 14-101`) are left out: two capitals in front of a `§` are too easily
something else to read as Maryland's code without more context.

## Results

The same samples with the supplement (citare at this branch), compared citation by
citation:

| Sample | Statute citations before | After | Gained | Changed | Lost | Bare-`§` unknowns before → after |
|---|---:|---:|---:|---:|---:|---|
| CourtListener ten-percent | 8,451 | 11,251 | 2,800 (708 opinions) | 225 | 0 | 14,944 → 12,144 |
| CAP | 1,303 | 3,038 | 1,735 (258 opinions) | 13 | 0 | 5,432 → 3,697 |
| stare.pub | 80 | 255 | 175 (22 opinions) | 2 | 0 | 384 → 209 |

The changed ones are codes cited by subject, whose normalized reporter now keeps the
subject (`Tex. Code Ann.` → `Tex. Penal Code Ann.`, `N.Y.` → `N.Y. U.C.C.`; #49), and 81
`U.S.C.` citations whose reporter group no longer ends in a line break. Outside statutes,
85 case citations went away and 17 changed. The 85 are Oklahoma statutes read as cases:
`22 O.S. 1951, § 1066` was volume 22 of Ohio State Reports (`O.S.`) at page 1951. In the 17, a
statute in front of a short citation or case name had been taken for its antecedent or party
(`A.R.S. § 13-1421. Gilfillan, 196 Ariz. at 400` had antecedent guess `A.R.S. § 13-1421.
Gilfillan`), and now isn't. Speed is unchanged: over the ten-percent sample, two runs each
alternating, 9.6 and 10.1 ms per opinion before and 10.2 and 9.9 after.

I read 40 of the gained citations, drawn at random, in context:
[`evidence/state-code-forms.md`](evidence/state-code-forms.md). All 40 have the right
jurisdiction, code and title or chapter, and 39 the right section; the 40th, `23 M.R.S.A.
§§ 1915-1917`, has the range `1915-1917` as its section, as reporters-db's section grammar
reads any hyphenated number.

Still unrecognized after the supplement: the families left out above, and in Massachusetts
the `§`s after other `c. N,` forms (`St. 2002, c. 10, §`, session laws).

<a id="no-section-sign"></a>
## Codes cited by abbreviation with no `§` (#115)

Courts in several states cite their code by its own abbreviation and the section number, with
no `§`: `NRS 284.383`, `ORS 163.375`, `R.C. 2929.20`, `735 ILCS 5/13-206`. reporters-db's entries
for these codes all have the one pattern `$reporter § $law_section` under the Bluebook name, and
neither engine found any of them. The forms came from a 10,000-decision export of stare.pub's
CAP and CourtListener text ([#115](https://github.com/codexpublicus/citare/issues/115)), and
`data/laws-local.json` now has each one as a cluster under its reporters-db key, with a regex
that takes the abbreviation and the section and no `§`.

Three of the abbreviations are also another code's, so their entries ask for the shape of the
state's section numbers:

- `R.C.` needs a period in the section (`2929.20`), as every section of the Ohio Revised Code
  has one. `R. C. § 2951` in nineteenth-century Alabama reports is the Revised Code of Alabama,
  and `R.C.` inside `I.R.C. § 514` is the Internal Revenue Code; the first version of the entry
  read both as Ohio.
- `G.S.` and `N.C.G.S.` with no `§` need a hyphen in the section (`163-140`). Kansas courts cite
  their old General Statutes as `G.S. 1949, 79-1702`.
- `R.S.` alone is left out: it is also the federal Revised Statutes. Louisiana's is taken as
  `La. R.S.`, `La.R.S.` and `LSA-R.S.`.

`K.S.A.` has a section pattern of its own: Kansas sections can have a letter inside them
(`79-5a01`) or a comma before the last part (`12-16,102`), which `$law_section` doesn't allow,
and Kansas courts put a supplement's year between the abbreviation and the section (`K.S.A.
1989 Supp. 21-4603(2)`). `ILCS` takes the act and section with or without spaces around the
slash; reporters-db's pattern for `Ill. Comp. Stat.` asks for ` / `. Michigan's `MSA`
(Michigan Statutes Annotated), which follows nearly every `MCL` cite in older opinions (`MCL
750.110; MSA 28.305`), isn't in reporters-db and isn't added.

I measured on two samples, with citare at `2acf651` (v0.7.5 plus CI changes) against this branch:

- a CAP statute sample of 12,337 opinions: the 47 volumes the examples in #115 to #118 come from,
  and one more volume, drawn at random (seed 222), of each of their 29 reporters (one, N.E.2d
  285, isn't on data.stare.pub), 75 volumes in all, from stare.pub's volume zips;
- citation-bench's stare.pub sample (`stare-pub-v2026.10.22`, 1,000 opinions).

| Code | Forms | CAP | Opinions | stare.pub | Opinions |
|---|---|---:|---:|---:|---:|
| `Kan. Stat. Ann.` | `K.S.A. 60-1507`, `K.S.A. 1989 Supp. 21-4603(2)` | 3,943 | 296 | 0 | 0 |
| `Or. Rev. Stat.` | `ORS 163.375` | 3,041 | 318 | 66 | 8 |
| `Wash. Rev. Code` | `RCW 5.60.060(2)(a)` | 2,410 | 107 | 246 | 7 |
| `Nev. Rev. Stat.` | `NRS 284.383` | 1,628 | 203 | 0 | 0 |
| `La. Stat. Ann.` | `LSA-R.S. 22:655`, `La.R.S. 13:5101` | 1,016 | 238 | 1 | 1 |
| `Ohio Rev. Code Ann.` | `R.C. 2929.20(J)`, `R. C. 4509.46` | 704 | 59 | 58 | 11 |
| `Mich. Comp. Laws` | `MCL 750.110`, `MCLA 768.35` | 597 | 179 | 0 | 0 |
| `Ky. Rev. Stat. Ann.` | `KRS 95.450` | 405 | 130 | 1 | 1 |
| `Ill. Comp. Stat.` | `735 ILCS 5/13-206` | 254 | 39 | 310 | 42 |
| `S.D. Codified Laws` | `SDCL 22-16-21` | 94 | 9 | 0 | 0 |
| `N.C. Gen. Stat.` | `G.S. 163-140` | 2 | 2 | 32 | 23 |
| All | | 14,094 | 1,577 | 714 | 93 |

Statute citations go from 15,262 to 29,356 on the CAP sample and from 1,174 to 1,888 on the
stare.pub sample, and none is lost. The bare-`§` unknowns the new citations take in go from
20,878 to 20,812 and from 1,409 to 1,408. Outside statutes, the CAP sample loses three case
citations and four references, and the stare.pub sample one reference, all of them wrong
before: `NRS 41 A. 100` (Nevada chapter 41A) was volume 41 of the Atlantic Reporter, twice, with
four references to it; `RCW 9 A. 76-.050` the same; and `(West 2018)` after an `ILCS` cite was a
reference to `People v. West`. 75 citations change a field on the CAP sample and 3 on the
stare.pub sample, nearly all because a statute in front of a citation is now a citation: a short
cite's antecedent guess was the statute (`K.S.A. 22-3434. 240 Kan. at 332` had antecedent
`K.S.A. 22-3434.`) and is now none, and a case's full span no longer starts inside the statute.
Four citations lose a year they had taken from an earlier citation across what is now a
statute citation: `4 Kan. App. 2d 141, Syl. ¶ 1.` had 1988 (the case is from 1979),
`1978 PA 368`, a Michigan public act read as a case, had 1978, and two repeated Westlaw cites
(`2010 WL 2978091, at *9.`) had their volume's year; and both citations of `Kinzua Lbr. Co. v.
Daggett, 203 Or 585, 281 P2d 221 (1955)` now get 1955. Ten case names (five cases, each
with a parallel citation) now start with the statute's last subsection, `ORS 197.829(1) and
Clark v. Jackson County` giving plaintiff `1)  Clark`; citare does the same after a `§` statute
today (`42 U.S.C. § 1983(1) and Clark v. Jackson County`), so it is a case-name bug these forms
now reach, not one they add. Speed is unchanged.

I read 40 of the new citations, drawn at random, in context:
[`evidence/codes-without-section-sign.md`](evidence/codes-without-section-sign.md). All 40 have
the right jurisdiction, code and section.

## What a law citation gives

`Citation::law()` returns the parts of a statute citation in fields of their own, and the CLI
and WASM output carry them as `law` (absent on other citations):

```json
{
  "type": "FullLawCitation",
  "matched_text": "42 Pa.C.S. § 5322",
  "groups": {"reporter": "Pa.C.S.", "section": "5322", "title": "42"},
  "law": {
    "jurisdiction": "Pennsylvania",
    "code": "Pa. Cons. Stat.",
    "subject": null,
    "title": "42",
    "chapter": null,
    "section": "5322"
  }
}
```

- `jurisdiction`: reporters-db's jurisdiction for the code (`Connecticut`, `New York`,
  `United States`), from the data bundle's new section 10.
- `code`: the code's standard abbreviation, whichever spelling the text used (`Conn. Gen.
  Stat.` for `General Statutes`, `C.G.S.A.` and `Conn. Gen. Stat.`; `Ga. Code Ann.` for `OCGA`).
  A code cited by subject keeps the subject: `N.Y. Penal Law`, `N.Y. Gen. Bus. Law`, `Cal. Pen.
  Code`, `Cal. Civ. Proc. Code`, `Tex. Fam. Code Ann.`, `Md. Code Ann., Cts. & Jud. Proc.`. The
  subject is as written, so `Pen. Code` and `Penal Code` give `Cal. Pen. Code` and `Cal. Penal
  Code`; a small-capitals subject is put in title case (`TEX. PENAL CODE ANN.` gives `Tex. Penal
  Code Ann.`). `reporter_normalized` in citation-bench, and `corrected_reporter()`, are this code.
- `subject`, `title`, `chapter`: the subject of a code cited by subject, the title of a code
  cited by title (`42`, `19-A`), the chapter of one cited by chapter (`93A`).
- `section`: the section number without its subsections (`52-557n`, `1653`); a pin cite such
  as `(6)(A)` stays in `metadata.pin_cite`.

A link to statute text needs the jurisdiction, the code (and its subject, title or chapter)
and the section, and these fields are those. The jurisdiction and code names are reporters-db's
laws keys and jurisdictions, plus the two new keys `Pa. Stat. Ann.` and `Conn. Practice Book`.

## The file

`data/laws-local.json` has laws.json's schema, keyed the same way. An entry under a key
reporters-db has is one more cluster of that key: whatever it leaves out (name, cite type,
jurisdiction, dates, regexes) comes from reporters-db's first cluster, so it is the same
edition, found through more spellings. An entry under a new key gives them all. Two fields
are citare's own:

- `strings`: prefilter strings for a regex that doesn't contain the reporter's name (`Law§`
  for the New York laws), compared with whitespace removed. Without them, the regex runs on
  every document.
- `examples`: citations the entry has to recognize. `crates/citare-core/tests/law_local.rs`
  checks each one gives a law citation with that key as its edition, a jurisdiction and a
  section.

`notes` says why each entry exists. To add a form: measure it as above, add the entry with
examples, add its expected fields to `statute_fields` in `law_local.rs`, and add real
inputs to citation-bench's `vectors/statutes/`.
