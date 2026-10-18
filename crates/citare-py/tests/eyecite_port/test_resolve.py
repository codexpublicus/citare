"""eyecite's test_ResolveTest.py (eyecite v2.7.8): the checkResolution
cases and the reference resolution cases that don't need eyecite's
internal ``extract_reference_citations``. Provenance: engine-tests, not
ground truth; see eyecite_port/__init__.py.
"""

from collections import defaultdict

import pytest
from citare.models import FullCitation, Resource
from citare.resolve import resolve_citations

from citare import get_citations


def format_resolution(resolution):
    return {
        k.citation.matched_text(): [i.matched_text() for i in v]
        for k, v in resolution.items()
    }


def assertResolution(citations, expected_resolution_dict):
    actual_resolution_dict = resolve_citations(citations)
    assert format_resolution(actual_resolution_dict) == format_resolution(
        expected_resolution_dict
    )


def checkReferenceResolution(expected_indices, citation_text):
    citations = get_citations(citation_text)
    resolution_index_map = {
        cite.corrected_citation(): idx for idx, cite in enumerate(citations)
    }
    resolved_citations = resolve_citations(citations)
    formatted_resolution = format_resolution(resolved_citations)
    result = {
        key: [resolution_index_map[value] for value in values]
        for key, values in formatted_resolution.items()
    }
    assert expected_indices == list(result.values())


def checkResolution(*expected_resolutions):
    expected_resolution_dict = defaultdict(list)
    citations = []
    resources = []
    for i, cite_text in expected_resolutions:
        cites = get_citations(cite_text)
        assert len(cites) == 1, f"Failed to find exactly one cite in {cite_text!r}"
        cite = cites[0]
        citations.append(cite)
        if i is None:
            continue
        assert i <= len(resources), f"Invalid row {(i, cite_text)!r}"
        if i == len(resources):
            assert isinstance(cite, FullCitation), f"Invalid row {(i, cite_text)!r}"
            resources.append(Resource(citation=cite))
        expected_resolution_dict[resources[i]].append(cites[0])
    assertResolution(citations, expected_resolution_dict)


PARALLEL = (
    "checkResolution finds each citation in its own text, so both full "
    "citations start at offset 0 under the same case name, which citare reads "
    "as one decision cited in parallel and resolves the short form to it "
    "(docs/divergences.md, supra-after-parallel and parallel-groups). In one "
    "document the two citations would have different case-name offsets."
)


class TestResolve:
    def test_issue_167(self):
        checkResolution((0, "25 Texas L.Rev. 199"))

    def test_full_resolution(self):
        # Test resolving a single, full citation
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
        )
        # Test resolving two full citations to the same document
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (0, "Foo v. Bar, 1 U.S. 1."),
        )
        # Test resolving two full citations with missing page numbers but
        # otherwise identical. These should not resolve to the same document.
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. ____."),
            (1, "Foo v. Bar, 1 U.S. ____."),
        )
        # Test resolving multiple full citations to different documents
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (1, "Smith v. Jones, 1 F.2d 1."),
        )
        # Supra and short cites should resolve if there are redundant full
        # cites -- redundant cites don't create ambiguity
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (0, "Foo v. Bar, 1 U.S. 1."),
            (0, "Foo, 1 U.S., at 2."),
            (0, "Foo, supra at 2."),
        )

    def test_supra_resolution(self):
        # Test resolving a supra citation
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (0, "Foo, supra, at 2."),
        )

    def test_short_resolution(self):
        # Test resolving a short form citation
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (0, "1 U.S., at 2."),
        )
        # Test resolving a short form citation with an antecedent
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (0, "Foo, 1 U.S., at 2."),
        )
        # Test resolving a short form citation when its reporter and
        # volume match two possible candidates. We expect its antecedent
        # guess to provide the correct tiebreaker.
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (1, "Wrong v. Wrong, 1 U.S. 2."),
            (0, "Foo, 1 U.S., at 2."),
        )
        # Test resolving a short form citation when its reporter and
        # volume match two possible candidates, and when it lacks a
        # meaningful antecedent. We expect the short form citation to not
        # be resolved.
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (1, "Boo v. Far, 1 U.S. 2."),
            (None, "1 U.S., at 2."),
        )
        # Test resolving a short form citation when its reporter and
        # volume are erroneous. We expect the short form citation to not
        # be resolved.
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (None, "2 F.2d, at 2."),
        )

    def test_ambigous_short_cite(self):
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (1, "See Foo, 1 U.S. 2."),
            (None, "Foo, 1 U.S., at 2."),
        )

    def test_id_resolution(self):
        # Test resolving an Id. citation
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (0, "Id."),
            (0, "Id. at 2."),
        )
        # Test resolving an Id. citation when the previous citation
        # resolution failed. We expect the Id. citation to also not be
        # resolved.
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (None, "2 F.2d, at 2."),
            (None, "Id. at 2."),
        )
        # Test resolving an Id. citation when the previous citation is to an
        # unknown document. Since we can't resolve those documents,
        # we expect the Id. citation to also not be matched.
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (None, "Blah § 2."),
            (None, "Id. at 2."),
        )
        # Test resolving an Id. citation when it is the first citation
        # found. Since there is nothing before it, we expect no matches to
        # be returned.
        checkResolution(
            (None, "Id. at 2."),
        )
        # Id. cites will not match if their pin cite is
        # invalid relative to the target full cite.
        checkResolution(
            # too high:
            (0, "Foo v. Bar, 1 U.S. 100."),
            (None, "Id. at 500."),
            # too low:
            (0, "Foo v. Bar, 1 U.S. 100."),  # reset
            (None, "Id. at 50."),
            # edge case -- pin cites with non-digits at beginning are
            # assumed not to match cites with a "page" value:
            (0, "Foo v. Bar, 1 U.S. 100."),  # reset
            (None, "Id. at ¶ 100."),
            # edge case -- cites without a "page" group are assumed to match:
            (1, "Ala. Code § 92"),
            (1, "Id. at 2000"),
        )
        # Test resolving an Id. citation with a pin cite when the previous
        # citation only has a placeholder page. We expect this to fail.
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. ___"),
            (None, "Id. at 100."),
        )

    def test_non_case_resolution(self):
        """Test law and journal resolution."""
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (0, "Id. at 2."),
            (1, "Mass. Gen. Laws ch. 1, § 2"),
            (1, "Id."),
            (0, "Foo, supra, at 2."),
            (2, "1 Minn. L. Rev. 1."),
            (2, "Id. at 2."),
            (0, "Foo, 1 U.S., at 2."),
        )

    def test_complex_resolution(self):
        """
        Tests whether resolution works with a more complex string.
        Inspired by: https://github.com/freelawproject/courtlistener/blob/d65d4c1e11328fd9f24dabd2aa9a792b4e725832/cl/citations/tests.py#L546
        """
        checkResolution(
            (0, "Blah blah Foo v. Bar 1 U.S. 1, 77 blah blah."),
            (1, "Asdf asdf Qwerty v. Uiop 2 F.3d 500, 555."),
            (0, "Also check out Foo, 1 U.S. at 99."),
            (1, "Then let's cite Qwerty, supra, at 567."),
            (0, "See also Foo, supra, at 101 as well."),
            (2, "Another full citation is Lorem v. Ipsum 1 U. S. 50."),
            (1, "Quoting Qwerty, “something something”, 2 F.3d 500, at 559."),
            (None, "This case is similar to Fake, supra,"),
            (1, "and Qwerty supra, as well."),
            (1, "This should resolve to the foregoing. Ibid."),
            (1, "This should also convert appropriately, see Id. at 567."),
            (
                None,
                "But this fails because the pin cite is too low, see Id. at 400.",
            ),
            (
                None,
                "This should fail to resolve because the reporter and citation is ambiguous, 1 U. S., at 51.",
            ),
            (2, "However, this should succeed, Lorem, 1 U.S., at 52."),
        )

    @pytest.mark.parametrize(
        "expected,text",
        [
            ([[0, 1]], "Foo v. Bar, 1 U.S. 1 ... Foo at 2"),
            ([[0]], "Foo at 2. .... ; Foo v. Bar, 1 U.S. 1"),
            (
                [[0, 1]],
                "Foo v. Bar 1 U.S. 12, 347-348. something something, In Foo at 62, we see that",
            ),
            (
                [[0, 2], [1]],
                "Foo v. Bar 1 U.S. 12, 347-348; 12 U.S. 1. someting; In Foo at 2, we see that",
            ),
            (
                [[0, 2], [1]],
                "Foo v. Bar 1 U.S. 12, 347-348; In Smith, 12 U.S. 1 (1999) we saw something else. someting. In Foo at 2, we see that",
            ),
        ],
    )
    def test_reference_resolution(self, expected, text):
        checkReferenceResolution(expected, text)

    @pytest.mark.xfail(reason=PARALLEL, strict=True)
    def test_supra_resolution_two_candidates(self):
        # Test resolving a supra citation when its antecedent guess matches
        # two possible candidates. We expect the supra citation to not
        # be resolved.
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (1, "Foo v. Bar, 1 U.S. 2."),
            (None, "Foo, supra, at 2."),
        )

    @pytest.mark.xfail(reason=PARALLEL, strict=True)
    def test_short_resolution_two_candidates(self):
        # Test resolving a short form citation when its reporter and
        # volume match two possible candidates, and when its antecedent
        # guess also matches multiple possibilities. We expect the short
        # form citation to not be resolved.
        checkResolution(
            (0, "Foo v. Bar, 1 U.S. 1."),
            (1, "Foo v. Bar, 1 U.S. 2."),
            (None, "Foo, 1 U.S., at 2."),
        )

    def test_two_candidates_in_one_document(self):
        """The two cases above, as one document: neither short form
        resolves (citare's own check, not eyecite's)."""
        for short in ("Foo, supra, at 2.", "Foo, 1 U.S., at 2."):
            text = f"Foo v. Bar, 1 U.S. 1. Foo v. Bar, 1 U.S. 2. {short}"
            citations = get_citations(text)
            assert len(citations) == 3
            groups = resolve_citations(citations)
            assert [len(v) for v in groups.values()] == [1, 1]
