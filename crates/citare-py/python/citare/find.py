"""eyecite's ``find``: ``get_citations``."""

from __future__ import annotations

from typing import Any, Callable, Iterable

from . import _citare
from .clean import _steps
from .models import CitationBase, Document, _wrap
from .tokenizers import default_tokenizer

__all__ = ["get_citations"]


def get_citations(
    plain_text: str = "",
    remove_ambiguous: bool = False,
    tokenizer: Any = default_tokenizer,
    markup_text: str = "",
    clean_steps: Iterable[str | Callable[[str], str]] | None = None,
) -> list[CitationBase]:
    """Find the citations in ``plain_text``, or in ``markup_text`` (HTML or
    XML, whose ``<i>``/``<em>`` tags then help find case names).

    ``clean_steps`` are applied first (see ``clean_text``); spans index the
    cleaned text, which is each citation's ``document.plain_text``.
    ``remove_ambiguous`` drops citations whose reporter has more than one
    edition the year doesn't settle. What citare finds follows citare's own
    rules (docs/divergences.md), not eyecite's.
    """
    raw_document, raws = _citare.find(
        plain_text or "",
        remove_ambiguous,
        tokenizer,
        markup_text or "",
        _steps(clean_steps),
    )
    document = Document._from_raw(raw_document, clean_steps)
    return [_wrap(raw, document) for raw in raws]
