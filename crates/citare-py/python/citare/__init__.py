"""citare's citation extraction with eyecite's Python API.

``import citare`` in place of ``import eyecite``: the functions, their
arguments and the citation classes have eyecite's names and shapes. What
citare finds and how it resolves follow citare's rules, which differ from
eyecite's where docs/divergences.md says so.
"""

from ._citare import __version__
from .annotate import annotate_citations
from .clean import clean_text
from .find import get_citations
from .resolve import resolve_citations
from .utils import dump_citations

__all__ = [
    "__version__",
    "annotate_citations",
    "clean_text",
    "dump_citations",
    "get_citations",
    "resolve_citations",
]
