"""eyecite's test_UtilsTest.py (eyecite v2.7.8). Provenance: engine-tests,
not ground truth; see eyecite_port/__init__.py.
"""

import re
from textwrap import dedent

import pytest
from citare.utils import dump_citations

from citare import clean_text, get_citations


@pytest.mark.parametrize(
    "steps,text,expected",
    [
        (["inline_whitespace"], "  word \t \n  word  ", " word \n word "),
        (["all_whitespace"], "  word \t \n  word  ", " word word "),
        (["underscores"], "__word__word_", "wordword_"),
        (["html"], " <style>ignore</style> <i> word </i> ", " word "),
        (
            ["html", "underscores", "inline_whitespace"],
            " <style>ignore</style> __ <i> word  word </i>",
            " word word ",
        ),
    ],
)
def test_clean_text(steps, text, expected):
    assert clean_text(text, steps) == expected


def test_clean_text_invalid():
    with pytest.raises(ValueError):
        clean_text("foo", ["invalid"])


@pytest.mark.xfail(
    strict=True,
    reason="citare's dump_citations (citare-core's) prints spans, sorted groups, "
    "the corrected reporter and edition guess, and no pin_cite_span_end; "
    "test_api.py checks its output",
)
def test_dump_citations():
    text = "blah. Foo v. Bar, 1 U.S. 2, 3-4 (1999). blah"
    cites = get_citations(text)
    dumped_text = dump_citations(cites, text)
    dumped_text = re.sub(r"\x1B.*?m", "", dumped_text)  # strip colors
    expected = dedent(
        """
    FullCaseCitation: blah. Foo v. Bar, 1 U.S. 2, 3-4 (1999). blah
      * groups
        * volume='1'
        * reporter='U.S.'
        * page='2'
      * metadata
        * pin_cite='3-4'
        * pin_cite_span_end=31
        * year='1999'
        * court='scotus'
        * plaintiff='Foo'
        * defendant='Bar'
      * year=1999
    """
    )
    assert dumped_text.strip() == expected.strip()
