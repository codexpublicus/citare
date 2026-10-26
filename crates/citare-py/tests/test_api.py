"""citare's own tests of the Python API: what eyecite's tests don't cover,
and where citare's behaviour is its own (string-index spans over Unicode,
resolver callbacks over citare's rules, errors from callbacks)."""

from concurrent.futures import ThreadPoolExecutor
from copy import copy

import pytest
from citare.tokenizers import (
    EXTRACTORS,
    AhocorasickTokenizer,
    HyperscanTokenizer,
    TokenExtractor,
    Tokenizer,
    default_tokenizer,
)

import citare
from citare import (
    annotate_citations,
    clean,
    clean_text,
    dump_citations,
    get_citations,
    models,
    resolve_citations,
)

TEXT = "Foo v. Bar, 1 U.S. 1, 2 (1999). Id. at 3. Foo, 1 U.S., at 4. Foo, supra, at 5."


def test_public_names():
    for name in citare.__all__:
        assert hasattr(citare, name)
    assert citare.__version__


def test_classes_follow_eyecite():
    citations = get_citations(TEXT)
    assert [type(c).__name__ for c in citations] == [
        "FullCaseCitation",
        "IdCitation",
        "ShortCaseCitation",
        "SupraCitation",
    ]
    full, short = citations[0], citations[2]
    assert isinstance(full, models.FullCitation)
    assert isinstance(full, models.CaseCitation)
    assert isinstance(full, models.ResourceCitation)
    assert isinstance(short, models.CaseCitation)
    assert not isinstance(short, models.FullCitation)
    assert full.groups == {"volume": "1", "reporter": "U.S.", "page": "1"}
    assert full.metadata.plaintiff == "Foo"
    assert full.metadata.pin_cite == "2"
    assert full.year == 1999
    assert full.corrected_reporter() == "U.S."
    assert full.edition_guess.short_name == "U.S."
    assert full.edition_guess.reporter.is_scotus
    assert full.all_editions == (full.edition_guess,)
    assert full.corrected_citation_full() == "Foo v. Bar, 1 U.S. 1, 2 (scotus 1999)"
    assert full.token == "1 U.S. 1"
    assert (full.token.start, full.token.end) == full.span()
    assert citations[3].formatted() == "Foo, supra, at 5"
    assert "FullCaseCitation('1 U.S. 1'" in repr(full)


def test_spans_are_string_indices():
    # Curly quotes and a section mark are several UTF-8 bytes each; spans
    # still index the Python string.
    text = "“Roe” — Foo v. Bar, 410 U.S. 113, 120 (1973). Mass. Gen. Laws ch. 1, § 2."
    case, law = get_citations(text)
    assert text[slice(*case.span())] == "410 U.S. 113"
    assert text[slice(*case.full_span())] == "Foo v. Bar, 410 U.S. 113, 120 (1973)"
    assert text[slice(*case.span_with_pincite())] == "410 U.S. 113, 120"
    assert text[slice(*law.span())] == "Mass. Gen. Laws ch. 1, § 2"


def test_metadata_is_a_dataclass():
    # citation-bench's eyecite adapter reads vars(cite.metadata).
    full = get_citations(TEXT)[0]
    fields = vars(full.metadata)
    assert fields["defendant"] == "Bar"
    assert "pin_cite_span_end" in fields
    full.metadata.resolved_case_name = "Foo v. Bar"
    assert full.dump()["metadata"]["resolved_case_name"] == "Foo v. Bar"


def test_markup_text():
    markup = "<p>See <i>Foo v. Bar</i>, 1 U.S. 1 (1999). <i>Id.</i> at 2.</p>"
    citations = get_citations(
        markup_text=markup, clean_steps=["html", "all_whitespace"]
    )
    document = citations[0].document
    assert document.markup_text == markup
    assert document.plain_text[slice(*citations[0].span())] == "1 U.S. 1"
    assert citations[0].metadata.plaintiff == "Foo"
    annotated = annotate_citations(
        document.plain_text,
        [(c.span(), "<a>", "</a>") for c in citations],
        source_text=markup,
        unbalanced_tags="skip",
    )
    assert "<a>1 U.S. 1</a>" in annotated
    # A Document built on its own cleans the same way.
    assert (
        models.Document(
            markup_text=markup, clean_steps=["html", "all_whitespace"]
        ).plain_text
        == document.plain_text
    )


def test_remove_ambiguous():
    text = "1 F. 1. 1 U.S. 1."
    assert len(get_citations(text, remove_ambiguous=True)) <= len(get_citations(text))


def test_ambiguous_flag_and_law_fields():
    law = get_citations("N.Y. Gen. Bus. Law § 394")[0]
    assert law.law()["section"] == "394"
    assert law.corrected_reporter() == "N.Y. Gen. Bus. Law"
    assert get_citations("1 U.S. 1")[0].is_ambiguous() is False


# ---------------------------------------------------------------------------
# Resolution
# ---------------------------------------------------------------------------


def test_default_resolution_keeps_the_objects():
    citations = get_citations(TEXT)
    groups = resolve_citations(citations)
    assert len(groups) == 1
    ((resource, cites),) = groups.items()
    assert isinstance(resource, models.Resource)
    assert resource.citation is citations[0]
    assert [id(c) for c in cites] == [id(c) for c in citations]


def test_resolutions_in_document_order():
    text = "1 F.2d 1. 2 U.S. 2. 1 F.2d, at 3. 3 F.3d 4. 2 U.S., at 5."
    citations = get_citations(text)
    groups = resolve_citations(citations)
    assert [r.citation.matched_text() for r in groups] == [
        "1 F.2d 1",
        "2 U.S. 2",
        "3 F.3d 4",
    ]


def test_full_resolver_ids_group_short_forms():
    citations = get_citations(TEXT)
    groups = resolve_citations(
        citations, resolve_full_citation=lambda c: f"case:{c.corrected_citation()}"
    )
    assert list(groups) == ["case:1 U.S. 1"]
    assert groups["case:1 U.S. 1"] == citations


def test_unresolved_full_citation():
    text = "Foo v. Bar, 1 U.S. 1. Id. at 2. Baz v. Qux, 2 U.S. 3. Id. at 4."
    citations = get_citations(text)
    groups = resolve_citations(
        citations,
        resolve_full_citation=lambda c: None if c.groups["volume"] == "1" else "two",
    )
    assert {k: [c.matched_text() for c in v] for k, v in groups.items()} == {
        "two": ["2 U.S. 3", "Id."]
    }


def test_short_form_callbacks_get_eyecite_arguments():
    seen = {}

    def shortcase(citation, resolved_full_cites):
        seen["shortcase"] = (citation, list(resolved_full_cites))
        return resolved_full_cites[0][1]

    def supra(citation, resolved_full_cites):
        seen["supra"] = citation
        return None

    def id_(citation, last_resolution, resolutions):
        seen["id"] = (last_resolution, {k: list(v) for k, v in resolutions.items()})
        return last_resolution

    citations = get_citations(TEXT)
    groups = resolve_citations(
        citations,
        resolve_full_citation=lambda c: "R",
        resolve_shortcase_citation=shortcase,
        resolve_supra_citation=supra,
        resolve_id_citation=id_,
    )
    short, full_cites = seen["shortcase"]
    assert short is citations[2]
    assert full_cites == [(citations[0], "R")]
    assert seen["supra"] is citations[3]
    assert seen["id"] == ("R", {"R": [citations[0]]})
    assert groups["R"] == citations[:3]


def test_callback_errors_propagate():
    def boom(citation):
        raise RuntimeError("boom")

    with pytest.raises(RuntimeError, match="boom"):
        resolve_citations(get_citations(TEXT), resolve_full_citation=boom)
    with pytest.raises(TypeError):
        resolve_citations(get_citations(TEXT), resolve_full_citation=lambda c: [1])


def test_resolve_rejects_foreign_citations():
    with pytest.raises(TypeError):
        resolve_citations(["1 U.S. 1"])


def test_citations_from_several_texts():
    citations = get_citations("Foo v. Bar, 1 U.S. 1.") + get_citations(
        "Foo, 1 U.S., at 2."
    )
    groups = resolve_citations(citations)
    assert list(groups.values()) == [citations]


def test_two_candidates_in_one_document():
    """eyecite's test_supra_resolution and test_short_resolution cases with
    two candidates, as one document: neither short form resolves. Found in
    separate texts, as eyecite's checkResolution finds them, they resolve
    (eyecite_divergences.toml)."""
    for short in ("Foo, supra, at 2.", "Foo, 1 U.S., at 2."):
        text = f"Foo v. Bar, 1 U.S. 1. Foo v. Bar, 1 U.S. 2. {short}"
        citations = get_citations(text)
        assert len(citations) == 3
        groups = resolve_citations(citations)
        assert [len(v) for v in groups.values()] == [1, 1]


# ---------------------------------------------------------------------------
# Cleaning and annotating
# ---------------------------------------------------------------------------


def test_clean_steps_callables():
    assert clean_text("a  b", [str.upper, "inline_whitespace"]) == "A B"
    # The built-in step functions run as the named steps.
    assert clean_text(
        " <i>a</i>  b ", [clean.html, clean.all_whitespace]
    ) == clean_text(" <i>a</i>  b ", ["html", "all_whitespace"])
    citations = get_citations("1 U.S.\n1", clean_steps=[lambda t: t.replace("\n", " ")])
    assert citations[0].matched_text() == "1 U.S. 1"


def test_clean_step_errors_propagate():
    def boom(text):
        raise RuntimeError("boom")

    with pytest.raises(RuntimeError, match="boom"):
        clean_text("x", [boom])
    with pytest.raises(RuntimeError, match="boom"):
        get_citations("1 U.S. 1", clean_steps=[boom])
    with pytest.raises(TypeError):
        clean_text("x", [lambda t: 1])
    with pytest.raises(ValueError):
        get_citations("1 U.S. 1", clean_steps=["nope"])


def test_annotator_gets_original_objects():
    text = "See 1 U.S. 1 and “2 U.S. 2”."
    citations = get_citations(text)
    marks = [((c.span()), {"n": i}, ("end", i)) for i, c in enumerate(citations)]
    out = annotate_citations(
        text,
        marks,
        annotator=lambda before, span, after: f"[{before['n']}:{span}:{after[1]}]",
    )
    assert out == "See [0:1 U.S. 1:0] and “[1:2 U.S. 2:1]”."


def test_annotate_errors():
    with pytest.raises(ValueError):
        annotate_citations(
            "1 U.S. 1", [((0, 8), "<a>", "</a>")], unbalanced_tags="nope"
        )
    with pytest.raises(TypeError):
        annotate_citations("1 U.S. 1", [((0, 8), 1, 2)])
    with pytest.raises(NotImplementedError):
        annotate_citations("1 U.S. 1", [], offset_updater=object())

    def boom(before, span, after):
        raise RuntimeError("boom")

    with pytest.raises(RuntimeError, match="boom"):
        annotate_citations("1 U.S. 1", [((0, 8), "<a>", "</a>")], annotator=boom)


def test_dump_citations():
    text = "blah. Foo v. Bar, 1 U.S. 2, 3-4 (1999). blah"
    dumped = dump_citations(get_citations(text), text, color=False)
    assert dumped.startswith(
        "FullCaseCitation: blah. Foo v. Bar, 1 U.S. 2, 3-4 (1999). blah\n"
    )
    assert '    * pin_cite="3-4"' in dumped
    assert "  * year=1999" in dumped
    assert "\x1b[" in dump_citations(get_citations(text), text)


# ---------------------------------------------------------------------------
# Tokenizers
# ---------------------------------------------------------------------------


def test_extractors():
    assert len(EXTRACTORS) > 1000
    extractor = EXTRACTORS[0]
    copied = copy(extractor)
    copied.regex = "changed"
    assert copied.regex == "changed"
    assert extractor.regex != "changed"


def test_tokenizers_find_the_same_citations():
    text = (
        "Foo v. Bar, 1 U.S. 1 (1999); 2 F.3d 3; Mass. Gen. Laws ch. 1, § 2. Id. at 4."
    )
    expected = [c.matched_text() for c in get_citations(text)]
    for tokenizer in (
        default_tokenizer,
        AhocorasickTokenizer(),
        AhocorasickTokenizer(EXTRACTORS),
        HyperscanTokenizer(cache_dir="unused"),
    ):
        assert [
            c.matched_text() for c in get_citations(text, tokenizer=tokenizer)
        ] == expected


def test_simple_tokenizer_from_some_extractors():
    us = [e for e in EXTRACTORS if any("U.S." in s for s in e.strings)]
    tokenizer = Tokenizer(us)
    assert [
        c.matched_text()
        for c in get_citations("1 U.S. 1; 2 F.3d 3", tokenizer=tokenizer)
    ] == ["1 U.S. 1"]


def test_custom_extractor():
    # An id-like token of the caller's own.
    extractor = TokenExtractor(r"\b(ditto)\b", token_type="id", strings=["ditto"])
    tokenizer = AhocorasickTokenizer(list(EXTRACTORS) + [extractor])
    citations = get_citations("1 U.S. 1. ditto at 2.", tokenizer=tokenizer)
    assert [type(c).__name__ for c in citations] == ["FullCaseCitation", "IdCitation"]


def test_bad_tokenizer():
    with pytest.raises(TypeError):
        get_citations("1 U.S. 1", tokenizer=object())
    with pytest.raises(ValueError):
        TokenExtractor("x", token_type="nope")


def test_threads():
    texts = [f"Foo v. Bar, {i} U.S. {i} ({1900 + i})." for i in range(1, 50)]
    with ThreadPoolExecutor(8) as pool:
        results = list(pool.map(get_citations, texts))
    assert [r[0].groups["volume"] for r in results] == [str(i) for i in range(1, 50)]
