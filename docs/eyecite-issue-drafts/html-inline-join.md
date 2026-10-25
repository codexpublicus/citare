# Draft: `clean.html` adds a space wherever an inline element ends

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8. The closest existing report is
[#210](https://github.com/freelawproject/eyecite/issues/210), which saw the same space between
`Twombly` and its comma and was closed by widening the supra antecedent regex; the join itself
is unchanged.

---

**Title:** `clean.html` puts a space inside a token wherever inline markup ends, which drops pin cites after linked citations

`clean.html` joins every visible text node with a space, so any token that an inline element
ends in the middle of comes out split. CAP's case HTML links each citation it recognized and
the link ends before the pin cite, so the pin cite after almost every linked citation is lost.
CAP also ends some statute links partway through the section number:

```python
from eyecite import clean_text, get_citations

html = (
    '<p>The board relied on <a class="citation">Conn. Gen. Stat. § 10</a>-236b(f), '
    'and see <a class="citation">410 U.S. 113</a>, 115 (1973).</p>'
)
print(repr(clean_text(html, ["html", "inline_whitespace"])))
for c in get_citations(markup_text=html, clean_steps=["html", "inline_whitespace"]):
    print(c.matched_text(), c.groups, c.metadata.pin_cite, getattr(c.metadata, "extra", None))
```

```text
'The board relied on Conn. Gen. Stat. § 10 -236b(f), and see 410 U.S. 113 , 115 (1973).'
Conn. Gen. Stat. § 10 {'reporter': 'Conn. Gen. Stat.', 'section': '10'} None None
410 U.S. 113 {'volume': '410', 'reporter': 'U.S.', 'page': '113'} None , 115
```

The page reads `§ 10-236b(f)` and `410 U.S. 113, 115`. On the same text without the markup,
`get_citations` gives `410 U.S. 113` the pin cite `115`; through `markup_text` the pin cite goes
to `extra` as `, 115`. The statute's section would stay `10` either way, since the law regex
doesn't take `236b`, but the cleaned text is wrong regardless of what reads it.

The same join turns `<em>Id</em>. at 263` into `Id . at 263`, which isn't found at all,
`104 <i>L.Ed.</i>2d 487` into the reporter `L.Ed. 2d`, and `In <em>Twombly, supra</em>, at
553-554` into a supra with no pin cite (eyecite's own `test_reference_filtering` expects that
last one).

**Root cause:** `eyecite/clean.py` lines 47 to 58 at v2.7.8
(<https://github.com/freelawproject/eyecite/blob/v2.7.8/eyecite/clean.py#L47-L58>) select the
text nodes with an XPath query and return `" ".join(text)`. Adjacent text nodes in the same
paragraph get a space whether or not the markup separates them.

**Suggested fix:** join text nodes with nothing inside a block and put the space only where the
markup separates text: at block-level elements (`p`, `div`, `br`, `li`, table cells, headings,
`blockquote`, `pre`), around footnote marks (`sup`, CAP's `footnotemark` and
`a.footnotemark`) and CAP's `a.page-label`, where an element opens, and where whitespace-only
text lies between. The space where an element opens matters: real markup writes
`<em>Jin Fuey Moy</em><em>v. United States,</em>` (`test_markup_plaintiff_and_antecedent_guesses`)
and `17<i>C.F.R.</i>`, and joining those loses more than it fixes. Text eyecite already drops
(`page-number`, star pagination) can join with nothing, so `can<page-number>*642</page-number>not`
reads `cannot`.

citare made that change and measured it over 1,279 HTML opinions (500 CAP cases and eyecite's
one-percent CourtListener sample). It found a pin cite for 3,045 citations that had none and
lost 2. It gained 10 citations and lost 18 out of 26,222, most of the losses being
`(<i>Id.</i>)` and `<em>Id.</em>;`, which the tokenizer misses in plain text too and found
only because of the added space. citare's write-up is
<https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#html-inline-join>.
