# Draft: an id. citation used as a case name resolves to the citation before it

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** `Id.` standing for the case name of the next citation resolves to the previous one

Older opinions write `Id.` for "the same case name" before a full citation of another decision
in the same case. In `Grain Cleaner Co. v. Smith, 110 N. Y. 83, 17 N. E. 671; Id., 34 Hun, 91`
the id. citation is the name of `34 Hun, 91`, the General Term's decision, and refers to it.
`resolve_citations` resolves every id. citation to the citation before it, so this one goes
with `17 N. E. 671`, the Court of Appeals' decision:

```python
from eyecite import get_citations, resolve_citations

for text in [
    "Grain Cleaner Co. v. Smith, 110 N. Y. 83, 17 N. E. 671; Id., 34 Hun, 91.",
    "First National Benefit Society v. Stuart, 9 Cir., 134 F.2d 438; Id., 9 Cir., 152 F.2d 298.",
]:
    for resource, cites in resolve_citations(get_citations(text)).items():
        print([c.matched_text() for c in cites])
```

```text
['110 N. Y. 83']
['17 N. E. 671', 'Id.,']
['34 Hun, 91']
['134 F.2d 438', 'Id.,']
['152 F.2d 298']
```

`find_case_name` gives `34 Hun, 91` the defendant `Id.` and a full span that starts at the id.
citation, so the full citation knows which id. citation is its name (`filter_citations` logs
`Unknown overlap case` for each pair, because the two spans overlap). Which way the id.
citation resolves depends on how the citations sort: in `In re Murray (1990), 52 Ohio St.3d
155, 157; Stanley v. Illinois (1972), 405 U.S. 645, 651. Id., (1990), 52 Ohio St.3d 155` the
second Ohio citation is named `Stanley v. Illinois`, its full span starts before the id.
citation, and the id. citation resolves to Murray, but for the wrong reason.

**Suggested fix:** resolve an id. citation with no pin cite to the full case citation right
after it when that citation's case name is the id. citation and at most a court or year in
parentheses (`Id., (1990),`, `Id. (Tex.Civ.App.)`, `Id., 9 Cir.,`). On the ten-percent
CourtListener sample this changes 13 id. citations, each to the decision the text names.
citare's write-up is
<https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#id-as-case-name>.
