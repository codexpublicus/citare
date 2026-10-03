# Draft: `Id. §` pin cites, resolution and corpus evidence

Not filed. [freelawproject/eyecite#299](https://github.com/freelawproject/eyecite/issues/299)
already reports the missing pin cite, and [PR #343](https://github.com/freelawproject/eyecite/pull/343)
captures it. What #299 doesn't cover is what the missing pin cite does to resolution, and how
often it comes up. This is written as a comment on #299, with a note for #343; it could also
be a new issue titled "`Id. §` resolves to the preceding case, and breaks the id. chain".

Checked against eyecite 2.7.8, eyecite main at
[`0513e7f`](https://github.com/freelawproject/eyecite/commit/0513e7fec46db49d86d9c8f6a854feba231b50a3)
(identical output), and #343's head at `09551f3`.

---

The missing pin cite in #299 also changes how `Id. § N` resolves, in three ways. Repro:

```python
from eyecite import get_citations, resolve_citations

for text in [
    "See 42 U.S.C. § 1983. Id. § 1988. Id. at § 1983(b).",
    "Foo v. Bar, 1 U.S. 1 (1999). Id. § 5.",
    "See 83 Stat. 841. Id. § 4(b).",
]:
    cites = get_citations(text)
    groups = {id(c): g for g, members in enumerate(resolve_citations(cites).values()) for c in members}
    print(text)
    for c in cites:
        print(f"  {type(c).__name__:17} {c.matched_text()!r:20} pin_cite={c.metadata.pin_cite!r:14} resource={groups.get(id(c))}")
```

eyecite 2.7.8, and main at `0513e7f`:

```
See 42 U.S.C. § 1983. Id. § 1988. Id. at § 1983(b).
  FullLawCitation   '42 U.S.C. § 1983'   pin_cite=None           resource=0
  IdCitation        'Id.'                pin_cite=None           resource=0
  UnknownCitation   '§'                  pin_cite=None           resource=None
  IdCitation        'Id.'                pin_cite=None           resource=None
  UnknownCitation   '§'                  pin_cite=None           resource=None
Foo v. Bar, 1 U.S. 1 (1999). Id. § 5.
  FullCaseCitation  '1 U.S. 1'           pin_cite=None           resource=0
  IdCitation        'Id.'                pin_cite=None           resource=0
  UnknownCitation   '§'                  pin_cite=None           resource=None
See 83 Stat. 841. Id. § 4(b).
  FullLawCitation   '83 Stat. 841'       pin_cite=None           resource=0
  IdCitation        'Id.'                pin_cite=None           resource=0
  UnknownCitation   '§'                  pin_cite=None           resource=None
```

1. **The second `Id.` is unresolved.** `resolve_citations` sets `last_resolution` for every
   citation, and the `UnknownCitation` for the `§` resolves to nothing, so each `Id. § N` cuts
   the id. chain. Expected: both id. citations resolve to `42 U.S.C. § 1983`.
2. **`Id. § 5` after a case resolves to the case.** That's only because the pin cite is lost.
   With the pin cite, `_has_invalid_pin_cite` treats it like `Id. ¶ 4` (a pin cite that doesn't
   start with a page number can't be checked against a case) and leaves it unresolved.
   Expected: unresolved, as for `Id. ¶ 4`.
3. **`Id. § 4(b)` after `83 Stat. 841` resolves**, which is right, but only for the same
   reason. Once the pin cite is kept, the same rule applies to any full citation with a numeric
   `page`, and a section pin cite into the Statutes at Large or a session law becomes
   "invalid". Expected: resolved; the page range check is for case reporters, so a section pin
   cite should only be ruled out after a `FullCaseCitation`.

#343's head (`09551f3`) fixes 1 and 2, and hits 3:

```
See 42 U.S.C. § 1983. Id. § 1988. Id. at § 1983(b).
  FullLawCitation   '42 U.S.C. § 1983'   pin_cite=None           resource=0
  IdCitation        'Id.'                pin_cite='§ 1988'       resource=0
  IdCitation        'Id.'                pin_cite='at § 1983'    resource=0
Foo v. Bar, 1 U.S. 1 (1999). Id. § 5.
  FullCaseCitation  '1 U.S. 1'           pin_cite=None           resource=0
  IdCitation        'Id.'                pin_cite='§ 5'          resource=None
See 83 Stat. 841. Id. § 4(b).
  FullLawCitation   '83 Stat. 841'       pin_cite=None           resource=0
  IdCitation        'Id.'                pin_cite='§ 4'          resource=None
```

It also drops the subsection: `§ 4` for `§ 4(b)`, `at § 1983` for `at § 1983(a)`.

**How often this comes up.** I searched two samples for `Id.`/`Ibid.` followed by `§` (allowing
`,` or `at` between them): one volume from each of the 401 reporters in the Caselaw Access
Project (70,896 cases), and the CourtListener sample eyecite's benchmark uses (8,411 opinions).
That gave 1,520 matches; in 1,281 eyecite finds the `Id.`. Each was labeled by reading the
opinion, 100 by hand.

- The `§` is the id. citation's pin cite in 1,268 of the 1,281. The other 13 are digests that
  end a paragraph with `Id.` and start the next with `§`; 10 of those have a line break between
  them.
- What the section belongs to: a statute or code in 884, a regulation in 179, a treatise,
  restatement or encyclopedia in 166, a document such as a plan or agreement in 24, a
  constitution or court rule in 9. Only 3 are pin cites into a case (a Texas reporter numbered
  by section).
- eyecite 2.7.8 resolves 1,091 of the 1,281 correctly (85.2%). With the pin cite kept, the id.
  chain intact and section pin cites ruled out only after a case (codexpublicus/citare, which
  does all three), 1,241 (96.9%).
- Of the 29 that eyecite resolves to a case, 26 belong to something else: `47 Am.Jur.2d
  Judgments § 645, citing ... Aetna Casualty & Surety Co. v. Jones, 220 Conn. 285 ... Id. at
  § 648` resolves to *Aetna*; `"In determining whether a legal action should be dismissed under
  [the TCPA] ..." Id. § 27.006(a)` resolves to the last Texas case cited.
- Of the 158 that eyecite leaves unresolved because the chain was cut, 141 belong to the
  statute or regulation the chain started from.

The labeled matches, with a snippet of each opinion, are in codexpublicus/citare's
[`docs/evidence/id-section-pin.md`](https://github.com/codexpublicus/citare/blob/main/docs/evidence/id-section-pin.md),
and the inputs above are citation-bench vectors (`vectors/divergences/`).

**Not covered here, for a separate issue if wanted:** eyecite doesn't find `Id.` directly after
an opening parenthesis, bracket or brace (`(Id. § 2441)`), because `ID_REGEX` requires a space
before it. That accounts for 222 of the 239 matches where no `Id.` is found.
