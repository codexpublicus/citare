# Draft: state courts' own statute forms

Not filed. The data is reporters-db's laws.json, so this is written for
[freelawproject/reporters-db](https://github.com/freelawproject/reporters-db), with eyecite as
the consumer that shows it. Checked against eyecite 2.7.8 with the reporters-db version
citation-bench pins.

---

**Title:** laws.json lacks the statute forms state courts write

laws.json names each state's code by its Bluebook abbreviation (`Conn. Gen. Stat.`, `Ga. Code
Ann.`, `42 Pa. Cons. Stat.`). State courts mostly write their own forms, and eyecite returns
each as an unknown citation for the `§`:

```python
from eyecite import get_citations

for text in [
    "General Statutes § 52-557n",   # Connecticut
    "Practice Book § 17-49",        # Connecticut court rules
    "OCGA § 16-9-93",               # Georgia
    "42 Pa.C.S. § 5322",            # Pennsylvania
    "Penal Law § 160.05",           # New York
    "(Pen. Code, § 211)",           # California
    "TEX. PENAL CODE ANN. § 1.03",  # Texas, small capitals
    "G. L. c. 93A, § 2",            # Massachusetts
]:
    print(text, [type(c).__name__ for c in get_citations(text)])
```

Every line prints `['UnknownCitation']`. `22 O.S. 1951, § 1066` (Oklahoma) is worse: a case
citation to volume 22 of Ohio State Reports at page 1951, then an unknown `§`.

In the CourtListener ten-percent sample eyecite's benchmark uses (7,632 opinions), 14,944
unknown citations are a bare `§`; about 9,100 of them have words in front naming what they
cite, and the most common are state codes: `OCGA` (Georgia), New York's consolidated laws by
name, `42 Pa.C.S.`, the California Style Manual's `Pen. Code, §`, `A.R.S.`, `HRS`, Connecticut's
`General Statutes`, `40 P.S.`, `N.C.G.S.`, `O.S.`, `M.R.S.`, `V.S.A.`, `N.D.C.C.` and Connecticut's
`Practice Book`. Adding them as variations of the existing entries (and new entries for
Purdon's `P.S.` and the Practice Book) finds 2,800 more statute citations there, with none
lost. The entries citare uses, with examples and the forms' counts, are in
[`data/laws-local.json`](../../data/laws-local.json) and [`laws-local.md`](../laws-local.md);
they are in laws.json's schema and could move over as they are, apart from `strings`
(prefilter strings for regexes without the reporter's name).

Several states' courts also cite their code by its own abbreviation with no `§` at all, and
eyecite returns nothing for these: `NRS 284.383` (Nevada), `ORS 163.375` (Oregon), `R.C.
2929.20` (Ohio), `SDCL 22-16-21` (South Dakota), `RCW 5.60.060` (Washington), `K.S.A. 60-1507`
(Kansas), `KRS 95.450` (Kentucky), `LSA-R.S. 22:655` (Louisiana), `MCL 750.110` (Michigan) and
`G.S. 163-140` (North Carolina). `735 ILCS 5/13-206` has a pattern under `Ill. Comp. Stat.`, but
it wants spaces around the slash and doesn't list `ILCS`. Over 12,337 CAP opinions these forms
are 14,094 statute citations. Three need the section's shape to stay apart from other codes:
`R.C.` with a period in the section (`R. C. § 2951` is the Revised Code of Alabama), `G.S.` with
a hyphen (`G.S. 1949, 79-1702` is Kansas), and Louisiana only as `La. R.S.` or `LSA-R.S.` (`R.S.`
is the federal Revised Statutes).

Courts in Florida, Wisconsin, Colorado, Missouri, North Dakota, Ohio and Idaho also put the
section before the code, which no laws.json pattern allows: `Section 54.28, Fla.Stat., F.S.A.`,
`sec. 103.465, Stats.`, `§ 25-8-505, C.R.S. 1973`, `Section 510.210 RSMo 1949, V.A.M.S.`,
`Section 24-02-26, N.D.C.C.`, `Section 4511.19, Revised Code` and `section 18-8004, Idaho Code`.
Over 12,337 CAP opinions these are 3,241 statute citations; eyecite returns an unknown `§` for
the `§` forms and nothing for the others. Ohio's General Code (`Section 4628, General Code`),
in force from 1910 to 1953, has no entry at all.
