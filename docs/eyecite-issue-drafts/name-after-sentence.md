# Draft: a case name runs back into a sentence that ends in a parenthetical

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** Case name includes the end of the previous sentence when it ends in `).`

`find_case_name` stops at a word that starts with `(` once the candidate has more than three
words, so a sentence that ends in a parenthetical before the case name ends up in it:

```python
from eyecite import get_citations

for text in [
    "Dr. Miles (1911). United States v. Arnold, Schwinn & Co., 388 U.S. 365 (1967).",
    "Witherspoon v. Illinois, 391 U.S. 510 (1968). Whan v. Texas, 403 U.S. 946 (1971).",
]:
    c = get_citations(text)[-1]
    print(repr(c.metadata.plaintiff), repr(c.metadata.defendant), c.year)
```

```text
'' 'Dr. Miles' 1911
'' 'Illinois' 1968
```

The case names are `United States v. Arnold, Schwinn & Co.` (1967) and `Whan v. Texas` (1971).

**Suggested fix:** stop the backward search at a word that ends with `)` or `]` followed by
`.`, `?` or `!` (and maybe a closing quotation mark), and start the case name at the first
capitalized word after it. When there is none, give the citation a full span start of its own,
so `is_parallel_citation` doesn't match it to another nameless citation by an equal `None`. On
the ten-percent CourtListener sample this changes 145 citations; of 40 read in context, 34 are
right, 6 are still wrong with less junk, and none is newly wrong. citare's write-up is
<https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#name-after-sentence>.
