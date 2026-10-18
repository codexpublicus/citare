"""eyecite's ``clean``: ``clean_text`` and the built-in cleaning steps."""

from __future__ import annotations

from typing import Any, Callable, Iterable

from . import _citare

__all__ = [
    "all_whitespace",
    "clean_text",
    "html",
    "inline_whitespace",
    "underscores",
    "xml",
]


def _step(name: str) -> Callable[[str], str]:
    def step(text: str) -> str:
        return _citare.clean(text, [name])

    step.__name__ = step.__qualname__ = name
    step.__doc__ = f"citare's ``{name}`` clean step."
    return step


html = _step("html")
inline_whitespace = _step("inline_whitespace")
all_whitespace = _step("all_whitespace")
underscores = _step("underscores")
xml = _step("xml")

cleaners_lookup = {
    "html": html,
    "inline_whitespace": inline_whitespace,
    "all_whitespace": all_whitespace,
    "underscores": underscores,
    "xml": xml,
}
_names = {func: name for name, func in cleaners_lookup.items()}


def _steps(steps: Iterable[Any] | None) -> list[Any] | None:
    """Pass the built-in step functions by name, so they run in Rust."""
    if steps is None:
        return None
    return [_names.get(s, s) if callable(s) else s for s in steps]


def clean_text(text: str, steps: Iterable[str | Callable[[str], str]]) -> str:
    """Apply each cleaning step to ``text`` in turn. A step is a name
    (``html``, ``inline_whitespace``, ``all_whitespace``, ``underscores``,
    ``xml``) or a callable taking and returning a string."""
    return _citare.clean(text, _steps(steps))
