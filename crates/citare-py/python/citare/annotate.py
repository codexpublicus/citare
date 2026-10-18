"""eyecite's ``annotate``: ``annotate_citations``."""

from __future__ import annotations

from typing import Any, Callable, Iterable

from . import _citare

__all__ = ["annotate_citations"]


def annotate_citations(
    plain_text: str,
    annotations: Iterable[tuple[tuple[int, int], Any, Any]],
    source_text: str = "",
    unbalanced_tags: str = "unchecked",
    use_dmp: bool = True,
    annotator: Callable[[Any, str, Any], str] | None = None,
    offset_updater: Any = None,
) -> str:
    """Insert each annotation's ``before`` and ``after`` around its span.

    Spans are string indices into ``plain_text`` (``citation.span()``).
    With ``source_text`` (the markup the plain text came from), they are
    carried onto the markup first. ``unbalanced_tags`` is ``unchecked``,
    ``skip`` or ``wrap``. ``annotator(before, span_text, after)`` builds
    each annotated span. ``use_dmp`` is accepted and ignored: citare always
    uses its own diff. ``offset_updater`` is not supported.
    """
    if offset_updater is not None:
        raise NotImplementedError("citare does not take an offset_updater")
    return _citare.annotate(
        plain_text, list(annotations), source_text or "", unbalanced_tags, annotator
    )
