# Draft: `Am. Jur.` is read only as the American Jurist

Not filed. Written for [freelawproject/reporters-db](https://github.com/freelawproject/reporters-db),
checked against the reporters-db commit citare 0.7.1 builds from, and eyecite 2.7.8.

---

**Title:** Add American Jurisprudence (`Am. Jur.`) to journals.json

reporters.json lists `Am. Jur.` once, as the American Jurist and Law Magazine (1829-1843, 28
volumes). Opinions from the late 1930s on cite American Jurisprudence, the encyclopedia (first
series 1936-1948, 58 volumes), with the same abbreviation, so eyecite reads them as case
citations of a reporter that ended in 1843:

```python
from eyecite import get_citations

c = get_citations("Smith v. United States ex rel. Gorlo, 7 Cir., 52 F.2d 848; 13 Am.Jur. 383.")[1]
print(type(c).__name__, c.edition_guess.reporter.name)
```

```text
FullCaseCitation American Jurist and Law Magazine
```

All 23 `Am. Jur.` citations in CourtListener's ten-percent sample are the encyclopedia: the
opinions cite second-series reporters, and the volumes match the encyclopedia's titles
(`20 Am. Jur. 299, Evidence, § 318`).

**Suggested fix:** add American Jurisprudence to journals.json under `Am. Jur.` (start 1936),
and let a citation whose abbreviation is both a journal's and a reporter's take the journal's
edition unless its year falls in the reporter's run. The American Jurist is a law magazine, so
it could also move to journals.json. citare's write-up is
<https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#am-jur>.
