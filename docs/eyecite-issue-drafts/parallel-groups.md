# Draft: a citation of a second decision takes the first decision's case name

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** A citation of a second decision takes the case name of the citation before it

`find_case_name` and `find_case_name_in_html` search back from a full case citation for a
`v.` or another stop word, and pass over any citations on the way. That is how the second
citation of `Smith v. Jones, 275 Conn. 38, 881 A.2d 194` gets the name `Smith v. Jones`. But
when a second decision has no `v.` of its own between it and the citations before it, the
search runs on into the first decision's name, and the second citation gets the first one's
name, full span start and year:

```python
from eyecite import get_citations

for markup in [
    "<p>(citing <i>Sutton v. Elrod,</i> 724 So.2d 551 (Ala.Civ.App.1998); <i>Beasley,</i> 564 So.2d 959 (Ala.Civ.App.1990)).</p>",
    "<p>In the case of <i>Zink v. McManus</i> (49 Hun, 583; affd., 121 N. Y. 259) both the owner and occupant.</p>",
]:
    for c in get_citations(markup_text=markup, clean_steps=["html", "inline_whitespace"]):
        print(c.matched_text(), c.metadata.plaintiff, c.metadata.defendant, c.full_span()[0])

text = ("in Daubert, 509 U.S. 579, 113 S.Ct. 2786, 125 L.Ed.2d 469, and Kumho Tire, "
        "526 U.S. 137, 119 S.Ct. 1167, 143 L.Ed.2d 238.")
for c in get_citations(text):
    print(c.matched_text(), c.metadata.defendant, c.full_span()[0])
```

```text
724 So.2d 551 Sutton Elrod 8
564 So.2d 959 Sutton Elrod 8
49 Hun, 583 Zink McManus 15
121 N. Y. 259 Zink McManus 15
509 U.S. 579 Daubert 3
113 S.Ct. 2786 Daubert 3
125 L.Ed.2d 469 Daubert 3
526 U.S. 137 Daubert 3
119 S.Ct. 1167 Daubert 3
143 L.Ed.2d 238 Daubert 3
```

`Beasley, 564 So.2d 959` is named `Sutton v. Elrod`, the affirmance `121 N. Y. 259` is grouped
with the Hun citation, and Kumho Tire is named Daubert. The same happens after `cert. den.`,
`and`, `Matter of`, and a year that closes the first citation (`209 P. 1102 (1922), 214 P.
146 (1923)`), and when one `<em>` holds a whole string cite. A later supra of the second
decision (`Kumho Tire, supra`, `Beasley, supra`) then resolves to nothing. On the ten-percent
CourtListener sample eyecite's benchmark uses, 448 of 19,518 runs of citations that share a
full span start take in a second decision this way.

**Suggested fix:** pass over an earlier citation only when nothing but pin cites and
punctuation stand between the two and they are in different reporters (a decision is printed
once in each), and otherwise end the search there, taking the name from what follows the
earlier citation. citare does this; its write-up, with the corpus measurement and the forms of
pin cite that have to stay grouped (`301(4)(b)`, `703 and n. 6`, `418(II)`, `2 OBR 380`), is
<https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#parallel-groups>.
