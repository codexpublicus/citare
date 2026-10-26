# Draft: a supra's last word alone can't pick between cases

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** Supra resolution uses only the word before "supra", so `X v. United States, supra` resolves to nothing

`SupraCitation.metadata.antecedent_guess` is the one word before `supra`, and
`_filter_by_matching_antecedent` resolves the supra only when the earlier full citations whose
plaintiff or defendant contains that word are one resource. When the word ends a name several
cited cases share, the supra resolves to nothing, though the case name before it tells them apart:

```python
from eyecite import get_citations, resolve_citations

for text in [
    "Mulholland v. United States, 28 Fed. Cl. 320 (1993). American Auto. Association v. "
    "United States, 367 U.S. 687 (1961). See Mulholland v. United States, supra.",
    "Franchi v. Farmholme, Inc., 191 Conn. 201 (1983); Swanson v. Groton, Inc., 1 Conn. 2 "
    "(1990). Franchi v. Farmholme, Inc., supra.",
    "Weishan v. Commissioner, 97 T.C. 1 (1991); Lindsey v. Commissioner, 98 T.C. 2 (1992). "
    "See Weishan v. Comm'r, supra.",
]:
    cites = get_citations(text)
    resolved = {id(c) for group in resolve_citations(cites).values() for c in group}
    print([(type(c).__name__, c.metadata.antecedent_guess if hasattr(c.metadata, "antecedent_guess") else None, id(c) in resolved) for c in cites][-1])
```

```text
('SupraCitation', 'States', False)
('SupraCitation', 'Inc.', False)
('SupraCitation', 'r', False)
```

The guess is the end of a name many cases share (`States`, `Co.`, `Inc.`, `Commissioner`) or,
for `Comm'r`, a single letter that nearly every name contains. In a sample of 1,000 opinions
from the Caselaw Access Project and CourtListener (771 supras), 211 supras matched an earlier
full citation and were left unresolved because they matched more than one case, and 178 of them
had a case name with a "v." before `supra`.

**Suggested fix:** find the case name before a supra with the same backward scan
`find_case_name` uses for full citations, and when the antecedent guess matches more than one
resource, keep the matches whose plaintiff is the supra's plaintiff, then, if still more than
one, those whose defendant is also the supra's defendant. Compare names as words, lowercased
without punctuation, so `O’Neil` and `O'Neil` match and `Commissioner` matches `Commissioner of
Internal Revenue`. The scan has to stop at an earlier `supra` or `Id.`: `find_case_name` passes
over `supra` on purpose (`Canning v. Lensink, supra, 221 Conn. 351`), and in `Smith v. Jones,
supra. Jones, supra.` it would give the second supra the first one's plaintiff. A supra the
current rule resolves keeps its resolution. In citare, a Rust citation extractor that had
eyecite's rule, this resolves 121 more of the 771 supras, and 1,219 more of 13,544 in 65,670
New York opinions, with none moving to a different case.
