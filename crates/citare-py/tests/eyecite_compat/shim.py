"""An ``eyecite`` package made of citare's modules, for eyecite's tests.

``installed()`` puts ``eyecite`` and its submodules in ``sys.modules`` for
as long as it is entered, so eyecite's test modules import citare where they
import eyecite. Each ``eyecite.<name>`` is citare's module of that name
(``eyecite.test_factories`` is ``eyecite_compat.factories``). eyecite's
internals that its test modules import at the top
(``eyecite.helpers.filter_citations``, ``eyecite.find.extract_reference_citations``,
``eyecite.utils.maybe_balance_style_tags``) are added as functions that raise
``NotImplementedError``, so a module that imports one still loads and only
the tests that call it fail. Everything else eyecite has and citare doesn't
is missing, and importing it fails.
"""

from __future__ import annotations

import sys
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from types import ModuleType

import citare.annotate
import citare.clean
import citare.find
import citare.models
import citare.resolve
import citare.tokenizers
import citare.utils

import citare

from . import factories


def _internal(qualname: str) -> Callable[..., object]:
    def missing(*args: object, **kwargs: object) -> object:
        raise NotImplementedError(
            f"{qualname} is eyecite's internal; citare's Python API doesn't have it"
        )

    missing.__name__ = missing.__qualname__ = qualname.rsplit(".", 1)[1]
    return missing


#: eyecite's internals its test modules import at module level.
_INTERNALS = {
    "eyecite.find": ["extract_reference_citations"],
    "eyecite.helpers": ["filter_citations"],
    "eyecite.utils": ["maybe_balance_style_tags"],
}

_SOURCES: dict[str, ModuleType | None] = {
    "eyecite": citare,
    "eyecite.annotate": citare.annotate,
    "eyecite.clean": citare.clean,
    "eyecite.find": citare.find,
    "eyecite.helpers": None,
    "eyecite.models": citare.models,
    "eyecite.resolve": citare.resolve,
    "eyecite.tokenizers": citare.tokenizers,
    "eyecite.utils": citare.utils,
    "eyecite.test_factories": factories,
}


def _build() -> dict[str, ModuleType]:
    modules = {}
    for name, source in _SOURCES.items():
        module = ModuleType(name, getattr(source, "__doc__", None))
        if source is not None:
            module.__dict__.update(
                (k, v)
                for k, v in vars(source).items()
                if not k.startswith("__") or k in ("__all__", "__getattr__")
            )
        for attr in _INTERNALS.get(name, []):
            setattr(module, attr, _internal(f"{name}.{attr}"))
        modules[name] = module
    package = modules["eyecite"]
    package.__path__ = []  # a package, so only the submodules above import
    for name, module in modules.items():
        if name != "eyecite":
            setattr(package, name.rsplit(".", 1)[1], module)
    return modules


_MODULES = _build()


@contextmanager
def installed() -> Iterator[None]:
    """``import eyecite`` gives citare while this is entered."""
    saved = {
        name: sys.modules[name]
        for name in list(sys.modules)
        if name == "eyecite" or name.startswith("eyecite.")
    }
    for name in saved:
        del sys.modules[name]
    sys.modules.update(_MODULES)
    try:
        yield
    finally:
        for name in list(sys.modules):
            if name == "eyecite" or name.startswith("eyecite."):
                del sys.modules[name]
        sys.modules.update(saved)
