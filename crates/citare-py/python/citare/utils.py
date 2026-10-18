"""eyecite's ``utils``: ``dump_citations`` and the constants the models use."""

from __future__ import annotations

from typing import Iterable

from . import _citare

REPORTERS_THAT_NEED_PAGE_CORRECTION = {"NY Slip Op", "Misc. 3d"}

__all__ = ["dump_citations"]


def dump_citations(
    citations: Iterable, text: str, context_chars: int = 30, color: bool = True
) -> str:
    """Describe each citation for debugging: its line of text, groups,
    metadata, year and edition guess (citare-core's ``dump_citations``).

    ``color`` is citare's: eyecite always prints ANSI colors. The spans
    printed are citare's UTF-8 byte offsets, not string indices.
    """
    return _citare.dump(list(citations), text, context_chars, color)
