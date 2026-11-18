# Draft: `69 id. 76`, "id." standing for the reporter, is read as an id. citation

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** `18 id. 481` ("id." for the reporter just cited) is an id. citation resolving to the wrong case

Opinions into the twentieth century cite a run of cases in one reporter by writing `id.` for the
reporter after the first, and `id.` with a page for a second case in the same volume:

```python
from eyecite import get_citations, resolve_citations

for text in [
    "Langdon v. The Mayor, 92 N.Y. 427; Thomas v. Thomas, 18 id. 481.",
    "Smith v. Edwards, 88 N.Y. 92; Delaney v. McCormack, id. 174.",
]:
    for resource, cites in resolve_citations(get_citations(text)).items():
        print([type(c).__name__ for c in cites])
```

```text
['FullCaseCitation', 'IdCitation']
['FullCaseCitation', 'IdCitation']
```

`18 id. 481` is 18 N.Y. 481 and `id. 174` after `Delaney v. McCormack` is 88 N.Y. 174, each a
different decision; eyecite reads both as id. citations of the citation before them, with the
page as the pin cite. Over 35,755 CAP opinions from older New York and Illinois reports, about
16,000 id. citations are of these two forms, and in citation strings like `Langdon v. The Mayor,
92 N. Y. 427; Thomas v. Thomas, 18 id. 481; Hoag v. Hoag, 35 id. 471` each one goes to the
first case.

**Suggested fix:** read a volume, `id.` and a page as a full case citation in the reporter of
the case citation before it (the first reporter of a parallel group), and `id.` and a page
right after a case name as one in that citation's volume as well. After a citation the
tokenizer didn't recognize, the reporter is unknown, and the `id.` is better left out than
resolved. citare's write-up, with the measurement, is
<https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#reporter-id>.
