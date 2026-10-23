# Volume caps

A volume cap is a `(?P<volume>...)` group in `data/reporters-local.json` that takes fewer
volumes than reporters-db's regex for the same edition. A citation past the cap isn't read as
that edition, and if no other edition takes it, it isn't read at all. A cap set from what the
Caselaw Access Project (CAP) holds is wrong for a series that is still published or that runs
past CAP's last volume, so the owner's ruling D80 (2026-10-22, from the audit in
[citare#108](https://github.com/codexpublicus/citare/issues/108)) sets the rule below.

## The rule

1. **A cap that says a volume can't exist** belongs only on a series that has ended, with its
   last volume cited from a source other than CAP: the publisher, the court, Bluebook T1 or a
   library catalogue record. The entry's `notes` in `reporters-local.json` link the source.
2. **A running series gets no cap**, whatever CAP holds.
3. **A cap that only tells two reporters with one abbreviation apart** is a disambiguation cap.
   Its note says so, and it gives way to the citation's year when the citation has one.
4. **Caps are not taken from stare.pub's artifacts.** `idx.stare.pub/manifest.json` and
   `reporters.json` list CAP's volumes, so a cap derived from them is "CAP doesn't have it"
   under another name, and it feeds itself: a cap drops citations, the next index lacks them,
   and the next derivation confirms the cap.
5. **Each index build reports what the caps cost.** stare-builder's index-build report counts,
   per cap or filter, the CourtListener citations it drops, and how many of those have a
   cluster date inside the series' own dates. That subset is where real volumes hide.

A cap with no outside source for the last volume may stay when a volume past it goes to another
edition, so it loses no citation; its note says the cap is CAP's holdings only.

## The caps

| Edition | Cap | Kind | Source for the last volume |
|---|---|---|---|
| `Ohio` (Ohio Reports) | 1-20 | Closed series | Ohio Reports 1821-1851 ([Cardiff Index to Legal Abbreviations](https://www.legalabbrevs.cardiff.ac.uk/titles/tview/id/4397/tsearch//ttype/Exact)); the Ohio State Reports begin again at volume 1 in 1852 ([1 Ohio St.](https://cite.case.law/ohio-st/1/)) |
| `Ohio` as a variation of `Ohio St.` | 21-177 | Closed series; gates only the spelling `Ohio` | Ohio State Reports vols 1-177, then Ohio St. 2d ([University of Akron case citation guide](https://libguides.uakron.edu/LARW/casecitations)) |
| `Bradf.` (Iowa Reports, Bradford) | 1-3 | CAP's holdings only | None found outside CAP. A volume past 3 goes to Bradford's Surrogate Reports, so the cap loses no citation |
| `Brad.`, `Bradf.` (Bradford's Surrogate Reports) | 1-4 | Closed series | 4 volumes, cases 1849-1857 ([HathiTrust](https://catalog.hathitrust.org/Record/008916078)) |
| `Cai.`, `Cai. R.` (Caines' Reports) | 1-3 | Closed series | "Caine's reports, 3 v." in the contents of the New York Common Law Reports ([archive.org](https://archive.org/details/reportsofcasesad04newyiala)) |
| `P.R.`, `P.R.R.` (Puerto Rico Reports) | 1-100 | Closed series | v. 3 (1899) to v. 100 (1972); vols 1-2 were published in Spanish only ([Berkeley Law catalogue](https://lawcat.berkeley.edu/record/183605); [Washington State Law Library](https://lawlibrary.courts.wa.gov/cgi-bin/koha/opac-detail.pl?biblionumber=13950)) |
| `Charlton Rep.` (T.U.P. Charlton's Reports) | 1 | Closed series | One volume, 1824, covering 1805-1810 ([VIAF](https://viaf.org/viaf/53766484); [Cardiff Index to Legal Abbreviations](https://www.legalabbrevs.cardiff.ac.uk/titles/view/id/4942)) |
| `Fid. Rep.` and its variations (Fiduciary Reporter, first series) | 1-30 | Closed series | Vol. 1 (1951) to v. 30 (1980), then the second series ([Jenkins Law Library catalogue](https://encore.jenkinslaw.org/iii/encore/record/C__Rb111056); [Berkeley Law catalogue](https://lawcat.berkeley.edu/record/281833)) |
| `Fid. Rep. 2d`, `Pa. Fid.` and their variations (Fiduciary Reporter, second series) | 1-30 | Closed series | 2nd ser., v. 1 (1981) to v. 30 (2010) ([Jenkins Law Library catalogue](https://encore.jenkinslaw.org/iii/encore/record/C__Rb111056); [Berkeley Law catalogue](https://lawcat.berkeley.edu/record/281833)) |
| `Fid. Rep. 3d` and its variations (Fiduciary Reporter, third series) | 1-13 | Closed series | 3rd ser., v. 1 (2011) to v. 13 (2022), then the fourth series from 2023 ([Jenkins Law Library catalogue](https://encore.jenkinslaw.org/iii/encore/record/C__Rb111056)) |
| `Conn. Super. Ct.` | three digits | Disambiguation | The numbered Reports (1986-1994) against the Superior Court's neutral citation, whose volume is a year |
| `Mass. App. Div.` (numbered series) | three digits | Disambiguation | CAP's numbered series (vols 1-15) against the annual series, whose volume is a year |

The Fiduciary Reporter's fourth series (`Fid. Rep. 4th`, 2023-) is still published and takes
any volume.

The two disambiguation caps give way to the year in the only way they can: the other series'
volume is the year itself, so a four-digit volume is read as the year series and a one- to
three-digit one as the numbered series.

reporters-db's own regexes cap a few editions too: `\d{1,3}` on `Ark.`, `Ark. App.`, `Me.`,
`N.H.`, `Pa. Super.` and `Ohio`, and `$full_cite_single_volume` (volume 1 or none) on `Dallam`,
`Gilmer`, `Litt. Sel. Cas.`, `McCahon`, `N.B.R. (Supp.)`, `Robards`, `Smith (N. H.)` and `Tapp.
Rep.`. These are reporters-db's calls, and CourtListener's 2026-09-30 citations don't argue
with any of them (citare#108).

## New Mexico Appellate Reports

citare 0.7.2 capped `N.M. App.` at CAP's ten volumes (2011-2016). The series is still
published, as unofficial hardbound volumes ([UNM Law Library
guide](https://libguides.law.unm.edu/c.php?g=244202&p=2316952); [Rule 23-112
NMRA](https://supremecourt.nmcourts.gov/wp-content/uploads/sites/2/2024/02/Rule-23-112-NMRA-1.pdf)
tells filers not to cite them), and I found no source for its current last volume, so the edition
now takes any volume.

The cap also did disambiguation work. New Mexico opinions write a Court of Appeals case in the
New Mexico Reports with the court in the reporter, `84 N.M.App. 83` (CAP `nm/84`), which
reporters-db doesn't have; citare didn't read it before or after the cap. `reporters-local.json`
now adds `N.M. App.` as a variation of the New Mexico Reports with the Appellate Reports'
regex, so one extractor finds both editions, and the Appellate Reports' start year (2011)
decides between them (`guess_edition` in `crates/citare-core/src/helpers.rs`):

- A citation with a year before 2011 is the New Mexico Reports, slug `nm`: `84 N.M.App. 83, 500
  P.2d 1 (1972)`.
- A citation with a year of 2011 or later is the New Mexico Appellate Reports, slug `nm-app`,
  whatever its volume: `12 N.M. App. 40 (2015)`.
- A citation without a year is the New Mexico Appellate Reports too, slug `nm-app`, and isn't
  marked ambiguous, so `--remove-ambiguous` keeps it. A New Mexico Reports citation written this
  way without a year takes the wrong slug, but it is read, where before it wasn't; making it
  ambiguous instead would let `--remove-ambiguous` drop real Appellate Reports citations.

A year citare can't read counts as none: `(Ct.App.1972)`, written without spaces, gives no year
today.

Over the opinion text of 1,626 CAP volumes (4,499,400 citations; the volumes measured in
`docs/reporters-local.md`, which include all of `nm` and `nm-app`), the change gains four
citations and loses or changes none. All four are New Mexico Reports cases written `N.M.App.`:
`110 N.M.App. 228, 794 P.2d 371 (1990)` (`nm/110`) and `96 N.M.App. 18, 627 P.2d 413 (1980)`
(`nm/95`) take `nm`, and `84 N.M.App. 83` (`nm/84`) and `94 N.M.App. 656, 615 P.2d 271, 1980`
(`nm/95`, a year citare doesn't read outside a parenthetical) take `nm-app`.
