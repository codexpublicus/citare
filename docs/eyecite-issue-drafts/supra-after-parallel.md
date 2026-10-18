# Draft: a supra after parallel citations resolves to nothing

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** Supra after parallel citations resolves to nothing

A decision cited in parallel gives one `FullCaseCitation` per reporter, and `get_citations`
already copies the case name and year from the first to the others. `resolve_citations` then
treats each as its own resource, so a supra whose antecedent matches the case name matches two
or three resources, and `_filter_by_matching_antecedent` returns `None`:

```python
from eyecite import get_citations, resolve_citations

for text in [
    "Smith v. Jones, 275 Conn. 38, 881 A.2d 194 (2005). The court agreed. Smith, supra, at 46. Id. at 47.",
    "Roe v. Wade, 410 U.S. 113, 93 S. Ct. 705, 35 L. Ed. 2d 147 (1973). The court agreed. Roe, supra, at 120.",
]:
    cites = get_citations(text)
    resolved = {id(c) for group in resolve_citations(cites).values() for c in group}
    print([(type(c).__name__, id(c) in resolved) for c in cites])
```

```text
[('FullCaseCitation', True), ('FullCaseCitation', True), ('SupraCitation', False), ('IdCitation', False)]
[('FullCaseCitation', True), ('FullCaseCitation', True), ('FullCaseCitation', True), ('SupraCitation', False)]
```

The id. citation after the supra is lost with it. State court opinions cite almost everything in
parallel, so this is most supras in them: in 138 A.3d 1012 (Brooks v. Powers, Conn. App. 2016)
all 52 are unresolved. In citare, a Rust port of eyecite with the same rule, resolving them as
below takes the supras that resolve on the ten-percent CourtListener sample eyecite's benchmark
uses from 1,227 of 5,140 to 2,263.

**Suggested fix:** remember which full case citations were found as parallel citations of one
another (adjacent, with the same full span start, the pairs `get_citations` copies the case name
across). When a supra's or short cite's matches are different resources that one such group
covers, resolve to the match in the short form's reporter, or else the first match. Two
different decisions with the same name stay ambiguous.

Each group should stand only for itself. Connecticut opinions repeat the regional cite after
every supra (`Haynes v. Middletown, supra, 314 Conn. at 312, 101 A.3d 249`), so the same
resource turns up later on its own, which is fine; but chaining groups through a shared
resource lets one misread pair (`Celotex, 477 U.S. at 321-25, 106 S.Ct. 2548; Anderson, 477
U.S. at 255-57, 106 S.Ct. 2505`, read as one case named Celotex) join two decisions.

Reference citations (`In Haynes at 312`, or an emphasized `Haynes` in markup) go through
`_resolve_reference_citation`, which needs exactly one resource too, so they fail the same way
after parallel citations and take the same fix. On the same sample, references resolved go from
3,060 of 6,363 to 6,128 with it.

citare resolves these this way; its write-up, with the corpus numbers,
is <https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#supra-after-parallel>.
