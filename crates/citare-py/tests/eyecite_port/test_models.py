"""eyecite's test_ModelsTest.py (eyecite v2.7.8). Provenance: engine-tests,
not ground truth; see eyecite_port/__init__.py.

eyecite builds most of these citations with test_factories; citare's
citations can only come from get_citations, so the factory calls become
the texts they stand for. test_persistent_hash is left out: it pins
eyecite's hash values, and citare's hashes are its own (stable across
runs, but different numbers).
"""

from citare.models import Resource

from citare import get_citations


def cite(text):
    citations = get_citations(text)
    assert len(citations) == 1, text
    return citations[0]


def test_citation_comparison():
    for text in ["1 U.S. 1", "1 Minn. L. Rev. 1", "Mass. Gen. Laws ch. 1, § 2"]:
        a, b = cite(text), cite(text)
        assert a == b
        assert hash(a) == hash(b)


def test_resource_comparison():
    resources = [Resource(cite("2 U.S. 2")), Resource(cite("2 U.S. 2"))]
    assert resources[0] == resources[1]
    assert hash(resources[0]) == hash(resources[1])


def test_resource_comparison_with_missing_page_cites():
    citations = get_citations("2 U.S. __; 2 U.S. __")
    resources = [Resource(citations[0]), Resource(citations[1])]
    assert resources[0] != resources[1]
    assert hash(resources[0]) != hash(resources[1])


def test_citation_comparison_with_missing_page_cites():
    citations = get_citations("2 U.S. __; 2 U.S. __")
    assert citations[0] != citations[1]
    assert hash(citations[0]) != hash(citations[1])


def test_citation_comparison_with_corrected_reporter():
    citations = [cite("2 U.S. 4"), cite("2 U. S. 4")]
    assert citations[0] == citations[1]
    assert hash(citations[0]) == hash(citations[1])


def test_citation_comparison_with_different_source_text():
    citations = [cite("foobar 2 U.S. 4"), cite("foo 2 U.S. 4")]
    assert citations[0] == citations[1]
    assert hash(citations[0]) == hash(citations[1])


def test_citation_comparison_with_nominative_reporter():
    citations = [cite("5 U.S. 137"), cite("5 U.S. (1 Cranch) 137")]
    assert citations[0] == citations[1]
    assert hash(citations[0]) == hash(citations[1])


def test_citation_comparison_with_different_reporter():
    citations = [cite("2 F. Supp. 4"), cite("2 U. S. 4")]
    assert citations[0] != citations[1]
    assert hash(citations[0]) != hash(citations[1])


def test_tax_court_citation_comparison():
    citations = [cite("T.C.M. (RIA) ¶ 95,342"), cite("T.C.M. (RIA) ¶ 95,342")]
    assert citations[0] == citations[1]
    assert hash(citations[0]) == hash(citations[1])


def test_id_citation_comparison():
    citations = get_citations("1 U.S. 1. Id., at 123. Id., at 123.")[1:]
    assert citations[0] != citations[1]
    assert hash(citations[0]) != hash(citations[1])


def test_unknown_citation_comparison():
    citations = get_citations("See §99. And §99.")
    assert [type(c).__name__ for c in citations] == ["UnknownCitation"] * 2
    assert citations[0] != citations[1]
    assert hash(citations[0]) != hash(citations[1])


def test_missing_page_cite_conversion():
    assert get_citations("2 U.S. __")[0].groups["page"] is None


def test_hash_function_identity():
    citation = cite("1 U.S. 1")
    resource = Resource(cite("1 U.S. 1"))
    assert hash(citation) == citation.__hash__()
    assert hash(resource) == resource.__hash__()


def test_corrected_full_citation_includes_closing_parenthesis():
    journal_citation = get_citations(
        "Originalism without Foundations, 65 N.Y.U. L. Rev. 1373 (1990)"
    )[0]
    assert journal_citation.corrected_citation_full() == "65 N.Y.U. L. Rev. 1373 (1990)"
    full_case_citation = get_citations(
        "Meritor Sav. Bank v. Vinson, 477 U.S. 57, 60 (1986)"
    )[0]
    assert (
        full_case_citation.corrected_citation_full()
        == "Meritor Sav. Bank v. Vinson, 477 U.S. 57, 60 (scotus 1986)"
    )


def test_page_correction():
    tests = [
        ("2024 N.Y. Slip Op. 51192(U)", "2024 NY Slip Op 51192(U)", "51192(U)"),
        ("2024 NYSlipOp 51192[U]", "2024 NY Slip Op 51192(U)", "51192(U)"),
        ("11 Misc 3d 134[A]", "11 Misc. 3d 134(A)", "134(A)"),
        ("83 Misc.3d 126(A)", "83 Misc. 3d 126(A)", "126(A)"),
        ("11 U.S. 11[2]", "11 U.S. 11", "11"),
        (
            "Tex. Civ. Prac. & Rem. Code Ann. § 171.023",
            "Tex. Code Ann. § 171.023",
            None,
        ),
    ]
    for citation, corrected_citation, corrected_page in tests:
        c = get_citations(citation)[0]
        assert c.corrected_citation() == corrected_citation, citation
        assert c.corrected_page() == corrected_page, citation
