"""eyecite's ``resolve``: ``resolve_citations``."""

from __future__ import annotations

from collections import defaultdict
from typing import Callable, Hashable, Iterable

from . import _citare
from .models import (
    CitationBase,
    FullCitation,
    IdCitation,
    ReferenceCitation,
    Resource,
    ShortCaseCitation,
    SupraCitation,
)

__all__ = ["resolve_citations", "resolve_full_citation"]

ResourceType = Hashable
ResolvedFullCite = tuple[FullCitation, ResourceType]
ResolvedFullCites = list[ResolvedFullCite]
Resolutions = dict[ResourceType, list[CitationBase]]


def resolve_full_citation(full_citation: FullCitation) -> Resource:
    """The default full-citation resolver: the citation's own ``Resource``."""
    return Resource(full_citation)


def resolve_citations(
    citations: Iterable[CitationBase],
    resolve_full_citation: Callable[
        [FullCitation], ResourceType
    ] = resolve_full_citation,
    resolve_shortcase_citation: Callable[
        [ShortCaseCitation, ResolvedFullCites], ResourceType | None
    ]
    | None = None,
    resolve_supra_citation: Callable[
        [SupraCitation, ResolvedFullCites], ResourceType | None
    ]
    | None = None,
    resolve_reference_citation: Callable[
        [ReferenceCitation, ResolvedFullCites], ResourceType | None
    ]
    | None = None,
    resolve_id_citation: Callable[
        [IdCitation, ResourceType, Resolutions], ResourceType | None
    ]
    | None = None,
) -> Resolutions:
    """Group citations (in document order) by the resource each refers to.

    Returns a dict from resource to the citations resolved to it, in the
    order the resources were first cited; the citations are the objects
    passed in. ``resolve_full_citation`` maps a full citation to a resource
    (any hashable value; a falsy one leaves it unresolved). The other
    callbacks take eyecite's arguments; one left as ``None`` follows
    citare's rules over the resources the full-citation resolver returned,
    so returning your own ids from it is enough to group the short forms
    under them.
    """
    resolutions = _citare.resolve(
        list(citations),
        resolve_full_citation,
        resolve_shortcase_citation,
        resolve_supra_citation,
        resolve_reference_citation,
        resolve_id_citation,
    )
    return defaultdict(list, resolutions)
