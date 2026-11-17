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

Many courts also spell out `section` (`Idaho Code section 41-1839`, `Utah Code section
77-13-6`, `Indiana Code Section 35-34-1-8`, `Iowa Code section 321J.2`), and California's
courts name the code by subject in full (`Welfare and Institutions Code section 19050`, `Code
of Civil Procedure section 437c`). West Virginia's put a comma between the code and the
section (`W. Va. Code, 49-5-10(c)`), the District of Columbia's give the code's year (`D.C. Code
1967, § 16-501`), and `Va. Code § 38.1-327.2` has no `Ann.`. eyecite finds none of these; over
12,337 CAP opinions they are 2,279 statute citations. Indiana's sections also have one part more
than `law_section` takes when any part has a decimal: `Ind. Code § 35-44.1-3-1` gives section
`35-44.1-3`.
