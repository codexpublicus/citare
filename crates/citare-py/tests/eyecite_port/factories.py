"""Expected citations for the eyecite tests run against citare.

eyecite's tests build expected citations with ``eyecite.test_factories``
and compare their groups, metadata, year and corrected reporter with what
``get_citations`` found. citare's citation objects wrap what citare found,
so they can't be built by hand; these factories take the same arguments
and build the attributes the comparison reads.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from citare.tokenizers import EDITIONS_LOOKUP

from citare import models


@dataclass
class Expected:
    cls: type
    groups: dict = field(default_factory=dict)
    metadata: dict = field(default_factory=dict)
    year: int | None = None
    corrected_reporter: str | None = None


def _get_year(text: str) -> int | None:
    # eyecite.helpers.get_year: the first four digits, if a plausible year.
    try:
        year = int(text[:4])
    except (TypeError, ValueError):
        return None
    return year if 1600 < year < 2100 else None


def resource_citation(cls, source_text, reporter, short=False, year=None, **kwargs):
    metadata = kwargs.pop("metadata", {})
    groups = kwargs.pop("groups", {})
    groups.setdefault("reporter", kwargs.pop("reporter_found", reporter))
    if year:
        metadata["year"] = str(year)
    elif "year" in metadata:
        year = _get_year(metadata["year"])
    # eyecite: edition = EDITIONS_LOOKUP[reporter][0]; the expected
    # corrected reporter is that edition's short name.
    editions = EDITIONS_LOOKUP.get(reporter)
    corrected = editions[0].short_name if editions else reporter
    return Expected(cls, groups, metadata, year, corrected)


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
    cls = models.ShortCaseCitation if short else models.FullCaseCitation
    return resource_citation(cls, source_text, reporter, short, **kwargs)


def law_citation(source_text=None, reporter="Mass. Gen. Laws", **kwargs):
    return resource_citation(models.FullLawCitation, source_text, reporter, **kwargs)


def journal_citation(
    source_text=None, page="1", reporter="Minn. L. Rev.", volume="1", **kwargs
):
    groups = kwargs.setdefault("groups", {})
    groups.setdefault("volume", volume)
    groups.setdefault("page", page)
    return resource_citation(
        models.FullJournalCitation, source_text, reporter, **kwargs
    )


def _simple(cls):
    def factory(source_text=None, index=0, **kwargs: Any) -> Expected:
        return Expected(cls, kwargs.get("groups", {}), kwargs.get("metadata", {}))

    return factory


id_citation = _simple(models.IdCitation)
reference_citation = _simple(models.ReferenceCitation)
unknown_citation = _simple(models.UnknownCitation)
supra_citation = _simple(models.SupraCitation)


def comparison_attrs(cite: models.CitationBase) -> dict:
    """eyecite's run_test_pairs ``get_comparison_attrs`` for a found
    citation. Groups that didn't match (``None``) are left out, since the
    factories don't list them."""
    cite.metadata.pin_cite_span_start = None
    cite.metadata.pin_cite_span_end = None
    out = {
        "groups": {k: v for k, v in cite.groups.items() if v is not None},
        "metadata": cite.metadata,
    }
    if isinstance(cite, models.ResourceCitation):
        out["year"] = cite.year
        out["corrected_reporter"] = cite.corrected_reporter()
    return out


def expected_attrs(expected: Expected) -> dict:
    out = {
        "groups": {k: v for k, v in expected.groups.items() if v is not None},
        "metadata": expected.cls.Metadata(**expected.metadata),
    }
    if issubclass(expected.cls, models.ResourceCitation):
        out["year"] = expected.year
        out["corrected_reporter"] = expected.corrected_reporter
    return out
