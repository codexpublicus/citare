# Draft: a year outside the only exact edition doesn't count

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** `guess_edition` keeps a lone exact edition whose dates rule out the citation's year

`ResourceCitation.guess_edition` takes the exact editions whenever there are any and filters by
year only when there are several. With one exact edition and a variation edition whose dates
fit the year, it still guesses the exact one:

```python
from eyecite import get_citations

c = get_citations("Smith v. Jones, 5 How. 100 (1838)")[0]
print(c.edition_guess.reporter.name, [e.reporter.name for e in c.variation_editions])
```

```text
Howard's Supreme Court Reports ['Mississippi Reports, Howard']
```

Howard's Supreme Court Reports run from 1843 to 1860; the Mississippi Reports' Howard, whose
edition reporters-db names `Howard` with `How.` as a variation, from 1834 to 1843. 32
reporters-db abbreviations are both an exact edition and a variation of another edition
(`How.`, `Gilm.`, `Wash.`, `B.R.`, `Allen` and others), and the same happens wherever the two
editions' dates don't overlap.

**Suggested fix:** when the year falls outside every exact edition and inside a variation's,
guess from the variation editions. A wrong year makes it wrong where eyecite was right: in the
CAP volumes citare measured, `7 B.R. 128 (Bkrtcy.W.D.Pa.1980)` picks up 1974 from elsewhere in
its sentence and becomes the Baltimore City Reports. That was one citation in 3,774,274, and the
rule changed one other, which was wrong both ways. citare's write-up is
<https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#year-rules-out-exact>.
