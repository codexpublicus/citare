"""citare's stand-in for ``eyecite.test_factories``.

eyecite's tests build expected citations with ``eyecite.test_factories``,
which calls the citation constructors with a hand-made token. citare's
citations wrap the citation citare-core found, so they can't be built that
way. These factories take eyecite's arguments and return an object of the
same citare class with the attributes eyecite's tests read: ``groups``,
``metadata``, ``year`` and ``corrected_reporter()``.

Comparing and hashing go through citare-core, so each factory also finds
the citation it stands for (``volume reporter page`` for a case or journal,
the source text for the others) and compares as that citation. Each call
finds its citation at a new offset, as separate citations in one document
would be, so the citations citare tells apart by position (a case citation
without a page) stay apart, as eyecite's factory objects do. When the text
doesn't give exactly one citation of the class, the object compares equal
only to itself.
"""

from __future__ import annotations

import itertools
from typing import Any

from citare.tokenizers import EDITIONS_LOOKUP

from citare import get_citations, models


class _Unfound:
    """The raw citation of a factory object whose text citare doesn't find
    as one citation of its class: equal only to itself."""

    def __init__(self, text: str, index: int):
        self.matched_text = text
        self.index = index
        self.token_start = 0
        self.token_end = 99

    def corrected_reporter(self) -> None:
        return None

    def is_ambiguous(self) -> bool:
        return False


_offsets = itertools.count()


def _raw_for(cls: type, text: str, index: int) -> Any:
    found = get_citations(" " * next(_offsets) + text)
    if len(found) == 1 and type(found[0]) is cls:
        return found[0]._raw
    return _Unfound(text, index)


def _get_year(text: str) -> int | None:
    # eyecite.helpers.get_year: the first four digits, if a plausible year.
    try:
        year = int(text[:4])
    except (TypeError, ValueError):
        return None
    return year if 1600 < year < 2100 else None


def _build(
    cls: type,
    raw_text: str,
    index: int,
    groups: dict,
    metadata: dict,
    **attrs: Any,
) -> models.CitationBase:
    cite = cls.__new__(cls)
    cite._raw = _raw_for(cls, raw_text, index)
    cite.document = None
    cite.index = index
    cite.span_start = attrs.pop("span_start", None)
    cite.span_end = attrs.pop("span_end", None)
    cite.full_span_start = attrs.pop("full_span_start", None)
    cite.full_span_end = attrs.pop("full_span_end", None)
    cite.groups = groups
    cite.metadata = cls.Metadata(**metadata)
    for name, value in attrs.items():
        setattr(cite, name, value)
    return cite


def resource_citation(
    cls, source_text, reporter, short=False, year=None, index=0, **kwargs
):
    metadata = kwargs.pop("metadata", {})
    groups = kwargs.pop("groups", {})
    groups.setdefault("reporter", kwargs.pop("reporter_found", reporter))
    raw_text = kwargs.pop("_raw_text", None) or (
        source_text if isinstance(source_text, str) else ""
    )
    if year:
        metadata["year"] = str(year)
    elif "year" in metadata:
        year = _get_year(metadata["year"])
    # eyecite: edition = EDITIONS_LOOKUP[reporter][0], so the expected
    # corrected reporter is that edition's short name.
    editions = EDITIONS_LOOKUP.get(reporter)
    edition = editions[0] if editions else None
    kwargs.setdefault("exact_editions", (edition,) if edition else ())
    kwargs.setdefault("edition_guess", edition)
    corrected = edition.short_name if edition else groups["reporter"]
    cite = _build(cls, raw_text, index, groups, metadata, **kwargs)
    cite.year = year
    cite.corrected_reporter = lambda: corrected
    return cite


def case_citation(
    source_text=None, page="1", reporter="U.S.", volume="1", short=False, **kwargs
):
    metadata = kwargs.setdefault("metadata", {})
    groups = kwargs.setdefault("groups", {})
    if reporter == "U.S.":
        metadata.setdefault("court", "scotus")
    if short:
        metadata.setdefault("pin_cite", page)
    groups.setdefault("volume", volume)
    groups.setdefault("page", page)
    found_as = groups.get("reporter", kwargs.get("reporter_found", reporter))
    at = " at " if short else " "
    kwargs["_raw_text"] = f"{volume} {found_as}{at}{page}"
    if page == "__":
        # eyecite's CitationToken turns a missing page into None.
        groups["page"] = None
    cls = models.ShortCaseCitation if short else models.FullCaseCitation
    return resource_citation(cls, source_text, reporter, short, **kwargs)


def law_citation(source_text=None, reporter="Mass. Gen. Laws", **kwargs):
    if not isinstance(source_text, str):
        kwargs["_raw_text"] = f"{reporter} ch. 1, § 1"
    return resource_citation(models.FullLawCitation, source_text, reporter, **kwargs)


def journal_citation(
    source_text=None, page="1", reporter="Minn. L. Rev.", volume="1", **kwargs
):
    groups = kwargs.setdefault("groups", {})
    groups.setdefault("volume", volume)
    groups.setdefault("page", page)
    kwargs["_raw_text"] = f"{volume} {groups.get('reporter', reporter)} {page}"
    return resource_citation(
        models.FullJournalCitation, source_text, reporter, **kwargs
    )


def _simple(cls):
    def factory(source_text=None, index=0, **kwargs: Any) -> models.CitationBase:
        metadata = kwargs.pop("metadata", {})
        groups = kwargs.pop("groups", {})
        text = source_text if isinstance(source_text, str) else ""
        if metadata.get("pin_cite"):
            text = f"{text} {metadata['pin_cite']}"
        return _build(cls, text, index, groups, metadata, **kwargs)

    factory.__name__ = factory.__qualname__ = (
        cls.__name__.replace("Citation", "").lower() + "_citation"
    )
    return factory


id_citation = _simple(models.IdCitation)
reference_citation = _simple(models.ReferenceCitation)
unknown_citation = _simple(models.UnknownCitation)
supra_citation = _simple(models.SupraCitation)
