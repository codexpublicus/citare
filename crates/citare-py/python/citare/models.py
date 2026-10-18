"""Citation classes with eyecite's names, attributes and methods.

Each citation wraps the citation citare found (``_raw``). Attributes are
plain Python values, so they can be read and changed like eyecite's, but
resolving, dumping and comparing use what citare found, not later changes.
"""

from __future__ import annotations

from collections import UserString
from dataclasses import dataclass, fields
from functools import cached_property
from typing import Any, Iterable

from . import _citare
from .utils import REPORTERS_THAT_NEED_PAGE_CORRECTION

Reporter = _citare.Reporter
Edition = _citare.Edition

__all__ = [
    "CaseCitation",
    "CitationBase",
    "Document",
    "Edition",
    "FullCaseCitation",
    "FullCitation",
    "FullJournalCitation",
    "FullLawCitation",
    "IdCitation",
    "ReferenceCitation",
    "Reporter",
    "Resource",
    "ResourceCitation",
    "ResourceType",
    "ShortCaseCitation",
    "SupraCitation",
    "Token",
    "UnknownCitation",
]


class Token(UserString):
    """The token a citation came from: its text, offsets and regex groups."""

    def __init__(self, data: str, start: int, end: int, groups: dict):
        super().__init__(data)
        self.start = start
        self.end = end
        self.groups = groups

    def __repr__(self) -> str:
        return f"Token({self.data!r}, start={self.start}, end={self.end})"


class Document:
    """The text citations were found in, eyecite's ``Document``.

    ``plain_text`` is the cleaned text that citation spans index. With
    ``markup_text`` alone, it is the markup run through ``clean_steps``
    (``html`` is added when missing).
    """

    def __init__(
        self,
        plain_text: str = "",
        markup_text: str | None = "",
        clean_steps: Iterable[Any] | None = None,
        **_ignored: Any,
    ):
        from .clean import _steps

        self.clean_steps = list(clean_steps or [])
        raw = _citare.document(
            plain_text or "", markup_text or "", _steps(self.clean_steps)
        )
        self._set(raw)

    @classmethod
    def _from_raw(cls, raw: Any, clean_steps: Iterable[Any] | None) -> Document:
        document = cls.__new__(cls)
        document.clean_steps = list(clean_steps or [])
        document._set(raw)
        return document

    def _set(self, raw: Any) -> None:
        self.plain_text: str = raw.plain_text
        self.markup_text: str | None = raw.markup_text
        self.source_text: str = raw.source_text

    def __repr__(self) -> str:
        return f"Document(plain_text={self.plain_text[:40]!r}...)"


class CitationBase:
    """Base class of the citations ``get_citations`` returns."""

    @dataclass(eq=True, unsafe_hash=True)
    class Metadata:
        parenthetical: str | None = None
        pin_cite: str | None = None
        pin_cite_span_start: int | None = None
        pin_cite_span_end: int | None = None

    def __init__(self, raw: Any, document: Document | None = None):
        self._raw = raw
        self.document = document
        self.index: int = raw.index
        self.span_start: int | None = raw.span_start
        self.span_end: int | None = raw.span_end
        self.full_span_start: int | None = raw.full_span_start
        self.full_span_end: int | None = raw.full_span_end
        self.groups: dict = raw.groups()
        names = {f.name for f in fields(self.Metadata)}
        values = {k: v for k, v in raw.metadata_fields() if k in names}
        if "pin_cite_span_start" in names:
            values["pin_cite_span_start"] = raw.pin_cite_span_start
            values["pin_cite_span_end"] = raw.pin_cite_span_end
        self.metadata = self.Metadata(**values)

    @cached_property
    def token(self) -> Token:
        raw = self._raw
        return Token(raw.matched_text, raw.token_start, raw.token_end, self.groups)

    def __repr__(self) -> str:
        return (
            f"{type(self).__name__}({self.matched_text()!r}"
            + (f", groups={self.groups!r}" if self.groups else "")
            + f", metadata={self.metadata!r})"
        )

    def __hash__(self) -> int:
        return hash(self._raw)

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, CitationBase):
            return NotImplemented
        return self._raw == other._raw

    def matched_text(self) -> str:
        """Text that identified this citation, such as '1 U.S. 1' or 'Id.'"""
        return self._raw.matched_text

    def span(self) -> tuple[int, int]:
        """Start and end offsets of ``matched_text()`` in the plain text."""
        token = self._raw
        return (
            self.span_start if self.span_start is not None else token.token_start,
            self.span_end if self.span_end is not None else token.token_end,
        )

    def span_with_pincite(self) -> tuple[int, int]:
        """Start and end offsets including the pin cite."""
        raw = self._raw
        starts = [self.metadata.pin_cite_span_start, self.span_start, raw.token_start]
        ends = [self.metadata.pin_cite_span_end, raw.token_end, self.span_end]
        return (
            min((v for v in starts if v is not None), default=raw.token_start),
            max((v for v in ends if v is not None), default=raw.token_end),
        )

    def full_span(self) -> tuple[int, int]:
        """Start and end offsets of the whole citation, with case name,
        pin cite and parenthetical where citare found them."""
        start, end = self.span()
        return (
            self.full_span_start if self.full_span_start is not None else start,
            self.full_span_end if self.full_span_end is not None else end,
        )

    def corrected_citation(self) -> str:
        return self.matched_text()

    def corrected_citation_full(self) -> str:
        return self.matched_text()

    def dump(self) -> dict:
        """Citation data for ``dump_citations``-style printing."""
        return {
            "groups": self.groups,
            "metadata": {
                k: v for k, v in self.metadata.__dict__.items() if v is not None
            },
        }

    def is_ambiguous(self) -> bool:
        """citare: a resource citation whose edition couldn't be narrowed
        to one (eyecite drops these with ``remove_ambiguous=True`` and
        otherwise gives no flag)."""
        return self._raw.is_ambiguous()


class ResourceCitation(CitationBase):
    """A case, law or journal citation, full or short."""

    @dataclass(eq=True, unsafe_hash=True)
    class Metadata(CitationBase.Metadata):
        year: str | None = None
        month: str | None = None
        day: str | None = None

    def __init__(self, raw: Any, document: Document | None = None):
        super().__init__(raw, document)
        self.year: int | None = raw.year

    @cached_property
    def exact_editions(self) -> tuple:
        return tuple(self._raw.exact_editions)

    @cached_property
    def variation_editions(self) -> tuple:
        return tuple(self._raw.variation_editions)

    @cached_property
    def all_editions(self) -> tuple:
        return tuple(self._raw.all_editions)

    @cached_property
    def edition_guess(self) -> Edition | None:
        return self._raw.edition_guess

    def dump(self) -> dict:
        return {**super().dump(), "year": self.year}

    def corrected_reporter(self) -> str | None:
        """The edition guess's reporter, else the reporter as found. A law
        citation gives its code with the subject (citare #49)."""
        return self._raw.corrected_reporter()

    def corrected_citation(self) -> str:
        """The citation with the reporter corrected and the page
        standardized."""
        corrected = self.matched_text()
        reporter = self.groups.get("reporter")
        if self.edition_guess and reporter:
            corrected = corrected.replace(reporter, self.edition_guess.short_name)
        page = self.groups.get("page")
        corrected_page = self.corrected_page()
        if corrected_page and page and corrected_page != page:
            corrected = corrected.replace(page, corrected_page)
        return corrected

    def corrected_page(self) -> str | None:
        page = self.groups.get("page")
        if page is None:
            return None
        reporter = self.groups.get("reporter")
        if reporter:
            standard = self.edition_guess.short_name if self.edition_guess else ""
            if {reporter, standard} & REPORTERS_THAT_NEED_PAGE_CORRECTION:
                return page.replace("[U]", "(U)").replace("[A]", "(A)")
        return page

    def guess_edition(self) -> None:
        """A no-op: citare sets ``edition_guess`` while finding."""


class FullCitation(ResourceCitation):
    """A citation that fully identifies a resource."""


class FullLawCitation(FullCitation):
    """A citation to a code from reporters-db's laws.json (or citare's
    supplement)."""

    @dataclass(eq=True, unsafe_hash=True)
    class Metadata(FullCitation.Metadata):
        publisher: str | None = None

    def law(self) -> dict | None:
        """citare: the statute's jurisdiction, code, subject, title,
        chapter and section."""
        return self._raw.law()

    def corrected_citation_full(self) -> str:
        parts = [self.corrected_citation()]
        m = self.metadata
        if m.pin_cite:
            parts.append(f"{m.pin_cite}")
        publisher_date = " ".join(i for i in (m.publisher, m.month, m.day, m.year) if i)
        if publisher_date:
            parts.append(f" ({publisher_date})")
        if m.parenthetical:
            parts.append(f" ({m.parenthetical})")
        return "".join(parts)


class FullJournalCitation(FullCitation):
    """A citation to a journal from reporters-db's journals.json."""

    def corrected_citation_full(self) -> str:
        parts = [self.corrected_citation()]
        m = self.metadata
        if m.pin_cite:
            parts.append(f", {m.pin_cite}")
        if m.year:
            parts.append(f" ({m.year})")
        if m.parenthetical:
            parts.append(f" ({m.parenthetical})")
        return "".join(parts)


class CaseCitation(ResourceCitation):
    """A full or short case citation."""

    @dataclass(eq=True, unsafe_hash=True)
    class Metadata(ResourceCitation.Metadata):
        court: str | None = None


class FullCaseCitation(CaseCitation, FullCitation):
    """A full case citation: ``Foo v. Bar, 1 U.S. 1, 2 (1999)``."""

    @dataclass(eq=True, unsafe_hash=True)
    class Metadata(CaseCitation.Metadata):
        plaintiff: str | None = None
        defendant: str | None = None
        extra: str | None = None
        antecedent_guess: str | None = None
        resolved_case_name_short: str | None = None
        resolved_case_name: str | None = None

    def corrected_citation_full(self) -> str:
        parts = []
        m = self.metadata
        if m.plaintiff:
            parts.append(f"{m.plaintiff} v. ")
        if m.defendant:
            parts.append(f"{m.defendant}, ")
        parts.append(self.corrected_citation())
        if m.pin_cite:
            parts.append(f", {m.pin_cite}")
        if m.extra:
            parts.append(m.extra)
        publisher_date = " ".join(i for i in (m.court, m.year) if i)
        if publisher_date:
            parts.append(f" ({publisher_date})")
        if m.parenthetical:
            parts.append(f" ({m.parenthetical})")
        return "".join(parts)


class ShortCaseCitation(CaseCitation):
    """A short case citation: ``Foo, 1 U.S., at 2``."""

    @dataclass(eq=True, unsafe_hash=True)
    class Metadata(CaseCitation.Metadata):
        antecedent_guess: str | None = None

    def corrected_citation_full(self) -> str:
        parts = []
        if self.metadata.antecedent_guess:
            parts.append(f"{self.metadata.antecedent_guess}, ")
        parts.append(self.corrected_citation())
        return "".join(parts)


class SupraCitation(CitationBase):
    """A supra citation: ``Foo, supra, at 2``."""

    @dataclass(eq=True, unsafe_hash=True)
    class Metadata(CitationBase.Metadata):
        antecedent_guess: str | None = None
        volume: str | None = None

    def formatted(self) -> str:
        parts = []
        m = self.metadata
        if m.antecedent_guess:
            parts.append(f"{m.antecedent_guess}, ")
        if m.volume:
            parts.append(f"{m.volume} ")
        parts.append("supra")
        if m.pin_cite:
            parts.append(f", {m.pin_cite}")
        return "".join(parts)


class IdCitation(CitationBase):
    """An id. or ibid. citation: ``Id. at 2``."""

    @dataclass(eq=True, unsafe_hash=True)
    class Metadata(CitationBase.Metadata):
        pass

    def formatted(self) -> str:
        parts = ["id."]
        if self.metadata.pin_cite:
            parts.append(f", {self.metadata.pin_cite}")
        return "".join(parts)


class ReferenceCitation(CitationBase):
    """A case cited by name alone after its full citation: ``Roe at 240``."""

    @dataclass(eq=True, unsafe_hash=True)
    class Metadata(CitationBase.Metadata):
        plaintiff: str | None = None
        defendant: str | None = None
        resolved_case_name_short: str | None = None
        resolved_case_name: str | None = None

    name_fields = [
        "plaintiff",
        "defendant",
        "resolved_case_name_short",
        "resolved_case_name",
    ]


class UnknownCitation(CitationBase):
    """A section mark (``§``) citare couldn't place in a citation."""


_CLASSES = {
    cls.__name__: cls
    for cls in (
        FullCaseCitation,
        ShortCaseCitation,
        FullLawCitation,
        FullJournalCitation,
        SupraCitation,
        IdCitation,
        ReferenceCitation,
        UnknownCitation,
    )
}


def _wrap(raw: Any, document: Document | None) -> CitationBase:
    return _CLASSES[raw.type_name](raw, document)


class ResourceType:
    """What citations resolve to; any hashable object will do."""


@dataclass(frozen=True)
class Resource(ResourceType):
    """The default resource: a full citation. Resources are equal when
    their citations are (citare's equivalence, eyecite's rules)."""

    citation: FullCitation

    def __hash__(self) -> int:
        return hash(("Resource", hash(self.citation)))

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Resource) and self.citation == other.citation

