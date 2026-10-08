# Draft: codes cited by subject lose the subject, and New York's split varies by process

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8 with the reporters-db version citation-bench pins.

---

**Title:** `N.Y. Gen. Bus. Law § 394` splits differently from run to run, and normalizes to `N.Y. Law`

reporters-db has two entries that match a New York code cited by name: `N.Y.`
(`$reporter $law_subject § $law_section`) and `N.Y. Law`
(`(?P<reporter>N\.Y\. $law_subject Law) § $law_section`). Both match `N.Y. Gen. Bus. Law §
394` at the same span with different groups. `Tokenizer.tokenize` keeps the first of two
overlapping tokens, and `AhocorasickTokenizer.get_extractors` returns a `set`, so which one
comes first depends on the process:

```python
from eyecite import get_citations

c = get_citations("See N.Y. Gen. Bus. Law § 394 (2020).")[0]
print(c.groups, c.corrected_reporter())
```

Ten runs gave, five times each:

```text
{'reporter': 'N.Y. Gen. Bus. Law', 'subject': 'Gen. Bus.', 'section': '394'} N.Y. Law
{'reporter': 'N.Y.', 'subject': 'Gen. Bus. Law', 'section': '394'} N.Y.
```

Either way `corrected_reporter()` drops the code's name. The same holds for every code
reporters-db keys without its subject: `Cal. Civ. Code § 1714` normalizes to `Cal. Code`,
`Tex. Penal Code Ann. § 22.01` to `Tex. Code Ann.`, `Md. Code Ann., Cts. & Jud. Proc. § 5-101`
to `Md. Code Ann.`. Two citations to different codes then have the same normalized reporter
and section, and a link to the statute can't be built from it.

Suggested fixes:

1. Make the overlap deterministic, preferring the more specific entry: for law tokens with
   the same span, the longer `reporter` group (`N.Y. Law` over `N.Y.`, the split `Cal. Code`
   and `Tex. Code Ann.` already give).
2. Have `corrected_reporter()` for a `FullLawCitation` with a `subject` group put the subject
   back: `N.Y. Gen. Bus. Law`, `Cal. Civ. Code`, `Tex. Penal Code Ann.`, `Md. Code Ann., Cts. &
   Jud. Proc.`.

citare does both (codexpublicus/citare#49).
