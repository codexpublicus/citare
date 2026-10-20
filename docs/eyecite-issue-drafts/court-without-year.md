# Draft: a court parenthetical without a year is ignored

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** Court is lost when its parenthetical has no year (`389 F.2d 885 (5th Cir.), cert. denied, ...`)

A citation followed by subsequent history often prints only the court in its parenthetical and
leaves the year to the history. `POST_FULL_CITATION_REGEX` reads a court only from a
parenthetical that ends in a year, so the citation gets no court, and its full span stops
before the parenthetical:

```python
from eyecite import get_citations

text = (
    "Flaksa v. Little River Marine Construction Co., 389 F.2d 885 (5th Cir.), "
    "cert. den., 392 U.S. 928, 88 S.Ct. 2287, 20 L.Ed.2d 1387 (1968)."
)
c = get_citations(text)[0]
print(c.metadata.court, c.full_span())
```

```text
None (0, 60)
```

The court is `ca5`, and the full span would end at 71, after `(5th Cir.)`.

**Suggested fix:** when the post-citation regex finds no year, accept a parenthetical right
after the citation (or its pin cite) whose whole text is a court's `citation_string`, and end
the full span after it. Requiring the whole string, not a prefix, keeps `(per curiam)` and
`(a)` out. On the ten-percent CourtListener sample this gives 289 citations a court; of 70 read
in context, 67 are right and 3 are `(D. C.)` or `(D.C.)`, which courts-db gives to the
District of Columbia's criminal court whether or not there is a year. citare's write-up is
<https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#court-without-year>.
