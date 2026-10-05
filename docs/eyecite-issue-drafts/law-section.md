# Draft: statute section numbers with letters or Unicode hyphens

Not filed. The regex is reporters-db's `law_section` variable, so this is written for
[freelawproject/reporters-db](https://github.com/freelawproject/reporters-db), with eyecite
as the consumer that shows it. Checked against eyecite 2.7.8 with the reporters-db version
citation-bench pins.

---

**Title:** `law_section` stops at a letter or a Unicode hyphen in a section number

`law_section` in `regexes.json` is `\d+(?:[\-.:]\d+){,3}` (or a number with parenthesized
parts). Section numbers with a letter in them, which are common in the U.S. Code, the C.F.R.
and several state codes, don't fit it, and neither do sections written with U+2010 or U+2011
for the hyphen:

```python
from eyecite import get_citations

for text in [
    "See Mich. Comp. Laws § 600.2912a.",
    "See 42 U.S.C. § 2000e-5(f).",
    "See 17 C.F.R. § 240.10b-5.",
    "See Utah Code Ann. § 76‐9‐101.",
    "See N.Y. Gen. Bus. Law § 394-ccc(1).",
]:
    print(text, [(type(c).__name__, c.matched_text()) for c in get_citations(text)])
```

```text
See Mich. Comp. Laws § 600.2912a. [('FullLawCitation', 'Mich. Comp. Laws § 600')]
See 42 U.S.C. § 2000e-5(f). [('UnknownCitation', '§')]
See 17 C.F.R. § 240.10b-5. [('FullLawCitation', '17 C.F.R. § 240')]
See Utah Code Ann. § 76‐9‐101. [('FullLawCitation', 'Utah Code Ann. § 76')]
See N.Y. Gen. Bus. Law § 394-ccc(1). [('FullLawCitation', 'N.Y. Gen. Bus. Law § 394')]
```

Where the letter comes straight after the first number (`2000e`, `78j`, `1677b`), the section
can't end anywhere and the citation isn't found at all. In the 8,409 CourtListener opinions
eyecite's benchmark samples, allowing the forms below finds 793 statute citations that weren't
found and completes 85 that were cut short, in 231 opinions, without losing any; 250 of the new
ones are in 42 U.S.C., mostly Title VII (`§ 2000e-5`).

A section that allows them:

```text
(?P<section>(?:\d+(?:[a-z]{1,2}|[A-Z])?(?:[\-.:‐‑]\d+(?:[a-z]{1,2}|[A-Z])?|[\-‐‑][a-zA-Z]{1,3}){,3})|(?:\d+(?:\((?:[a-zA-Z]{1}|\d{1,2})\))+))
```

A part may end in up to two lowercase letters (`2912a`, `2000ee`) or one capital (`314A`), a
hyphen may introduce up to three letters (`394-ccc`, `66-k`), and U+2010 and U+2011 separate
parts like `-`. citare applies this as an override in its build
([codexpublicus/citare#48](https://github.com/codexpublicus/citare/issues/48)).
