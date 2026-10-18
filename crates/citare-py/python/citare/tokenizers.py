"""eyecite's ``tokenizers``: the tokenizers and token extractors.

``EXTRACTORS`` is citare's built-in extractors, built on first use.
"""

from __future__ import annotations

from typing import Any

from . import _citare

Tokenizer = _citare.Tokenizer
AhocorasickTokenizer = _citare.AhocorasickTokenizer
TokenExtractor = _citare.TokenExtractor

__all__ = [
    "AhocorasickTokenizer",
    "EDITIONS_LOOKUP",
    "EXTRACTORS",  # noqa: F822 - built on first use by __getattr__
    "HyperscanTokenizer",
    "TokenExtractor",
    "Tokenizer",
    "default_tokenizer",
]

#: The tokenizer ``get_citations`` uses by default: citare's shared one.
default_tokenizer = AhocorasickTokenizer()


def HyperscanTokenizer(  # noqa: N802 - eyecite's class name
    extractors: list | None = None, cache_dir: Any = None
) -> Any:
    """citare has no Hyperscan tokenizer; this is ``AhocorasickTokenizer``,
    which finds the same citations. ``cache_dir`` is ignored."""
    return AhocorasickTokenizer(extractors)


class _EditionsLookup:
    """eyecite's ``EDITIONS_LOOKUP``: reporter string (canonical or a
    variation) -> its editions. Lookups only; it can't be iterated."""

    def __getitem__(self, reporter: str) -> list:
        editions = _citare.editions_lookup(reporter)
        if not editions:
            raise KeyError(reporter)
        return editions

    def __contains__(self, reporter: object) -> bool:
        return isinstance(reporter, str) and bool(_citare.editions_lookup(reporter))

    def get(self, reporter: str, default: Any = None) -> Any:
        return _citare.editions_lookup(reporter) or default


EDITIONS_LOOKUP = _EditionsLookup()


_extractors: list | None = None


def __getattr__(name: str) -> Any:
    global _extractors
    if name == "EXTRACTORS":
        if _extractors is None:
            _extractors = _citare.extractors()
        return _extractors
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
