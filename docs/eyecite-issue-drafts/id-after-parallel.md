# Draft: an id. citation after parallel citations is checked against the last printing only

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** `Id. at <page>` after parallel citations is unresolved when the pin cite is in the first printing

`_resolve_id_citation` checks an id. citation's pin cite against the citation before it, which
after a decision cited in parallel is the run's last printing. A pin cite into the first
printing is out of range there, and the id. citation resolves to nothing:

```python
from eyecite import get_citations, resolve_citations

for text in [
    "Ray v. Bruce, 128 Neb. 783, 260 N.W. 287 (1935). Id. at 790.",
    "Ray v. Bruce, 128 Neb. 783, 260 N.W. 287 (1935). Id. at 290.",
]:
    cites = get_citations(text)
    for resource, group in resolve_citations(cites).items():
        print(resource.citation.matched_text(), [type(c).__name__ for c in group])
```

```text
128 Neb. 783 ['FullCaseCitation']
260 N.W. 287 ['FullCaseCitation']
128 Neb. 783 ['FullCaseCitation']
260 N.W. 287 ['FullCaseCitation', 'IdCitation']
```

`Id. at 790` is page 790 of 128 Neb. Over 7,632 CourtListener opinions (the benchmark's
ten-percent sample) about 400 id. citations are left unresolved this way, counting the pinless
`Id.`s chained after them.

**Suggested fix:** when the pin cite is out of range of the last resolution and that resolution
is one of the parallel citations just before the id. citation, try the others in order and take
the first the pin cite can be in. citare's write-up, with the measurement, is
<https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#id-after-parallel>.
