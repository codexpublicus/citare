# Draft: a year between the case name and the volume is ignored in markup

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** Year printed before the volume is lost when the case name comes from markup

Ohio, California, Illinois, Indiana and other state courts print the year between the case
name and the volume: `Stanley v. Illinois (1972), 405 U.S. 645`. `find_case_name` reads that
year as the citation's own (`pre_cite_year`), but `find_case_name_in_html` doesn't, so the
same text in markup gets no year or the year of the next parenthetical:

```python
from eyecite import get_citations

markup = (
    "<p><em>Stanley v. Illinois</em> (1972), 405 U.S. 645, 651.</p>"
    "<p>4 <em>Id.,</em> (1990), 52 Ohio St.3d 155, 157.</p>"
)
for c in get_citations(markup_text=markup, clean_steps=["html", "inline_whitespace"]):
    if hasattr(c.metadata, "year"):
        print(c.matched_text(), c.year)
```

```text
405 U.S. 645 1990
52 Ohio St.3d 155 None
```

The plain-text check has the opposite problem: `re.match(r"\(\d{4}\)", word)` accepts any
word that starts with a year, so `Dr. Miles (1911). United States v. Arnold, Schwinn & Co., 388
U.S. 365 (1967).` gives `388 U.S. 365` the year 1911 and the defendant `Dr. Miles`.

**Suggested fix:** in both paths, take a year in parentheses as the citation's own when it is
the word right before the volume, alone or followed by a comma, and otherwise only a word that
is a year in parentheses and nothing else. On the ten-percent CourtListener sample this changes
4,258 years; 100 read at random are all the year printed before the volume. citare's write-up
is <https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#year-before-volume>.
