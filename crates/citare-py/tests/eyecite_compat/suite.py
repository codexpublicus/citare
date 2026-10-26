"""Collects eyecite's test suite from the ``upstream/eyecite`` submodule and
runs it against citare, with the divergences in ``eyecite_divergences.toml``
marked ``xfail(strict=True)``.

``tests/test_eyecite.py`` is the entry point: ``conftest.py`` collects it as
an ``EyeciteSuite``, whose children are eyecite's test modules, imported
from the submodule with ``eyecite`` resolving to citare (``shim.py``).
Nothing of eyecite's is copied into this repository.

A divergence names an eyecite test by its id under ``upstream/eyecite/tests``
(``test_FindTest.py::FindTest::test_find_citations``), optionally narrowed to
the ``subTest`` calls whose parameters match ``subtest``
(``{q = "..."}``). A module id (``test_RegexTest.py``) covers every test in
the module, or the module's import when that fails. The listed test or
subtest must fail: one that passes fails the run, and so does an entry that
matches nothing, or whose ``doc`` anchor isn't in ``docs/divergences.md``.
"""

from __future__ import annotations

import functools
import importlib.util
import inspect
import re
import sys
import unittest
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import pytest

from . import shim

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10
    import tomli as tomllib

TESTS_DIR = Path(__file__).resolve().parent.parent
REPO = TESTS_DIR.parent.parent.parent
UPSTREAM_TESTS = REPO / "upstream" / "eyecite" / "tests"
DIVERGENCES_FILE = TESTS_DIR / "eyecite_divergences.toml"
DIVERGENCES_DOC = REPO / "docs" / "divergences.md"

#: The module eyecite's test modules are imported under.
PACKAGE = "eyecite_upstream_tests"


@dataclass
class Divergence:
    test: str
    reason: str
    doc: str
    subtest: dict[str, Any] | None = None
    issue: str | None = None
    #: Whether the entry covers a collected test (or a module's import).
    found: bool = field(default=False, compare=False)
    #: How many subtests the entry has matched so far.
    hits: int = field(default=0, compare=False)

    def covers(self, test_id: str) -> bool:
        return test_id == self.test or (
            "::" not in self.test and test_id.startswith(self.test + "::")
        )

    def matches_subtest(self, msg: Any, params: dict[str, Any]) -> bool:
        assert self.subtest is not None
        values = dict(params)
        if msg is not None:
            values["msg"] = msg
        return all(k in values and values[k] == v for k, v in self.subtest.items())

    @property
    def message(self) -> str:
        return f"{self.reason} ({self.issue or self.doc})"


def load_divergences() -> list[Divergence]:
    data = tomllib.loads(DIVERGENCES_FILE.read_text(encoding="utf-8"))
    return [Divergence(**entry) for entry in data.get("divergence", [])]


DIVERGENCES = load_divergences()


def _test_id(case: unittest.TestCase) -> str:
    module = inspect.getmodule(type(case))
    path = Path(module.__file__).name if module and module.__file__ else "?"
    return f"{path}::{type(case).__name__}::{case._testMethodName}"


#: eyecite's test helpers whose calls run as subtests (``call="name(args)"``),
#: so a divergence can name one call and the test's other calls still run.
SUBTEST_HELPERS = {
    "ResolveTest": ("checkResolution", "checkReferenceResolution"),
}


def _helper_as_subtest(original, name: str):
    @functools.wraps(original)
    def call(self, *args, **kwargs):
        shown = [repr(a) for a in args] + [f"{k}={v!r}" for k, v in kwargs.items()]
        with self.subTest(call=f"{name}({', '.join(shown)})"):
            original(self, *args, **kwargs)

    return call


def _divergent_subtest(original):
    """Wrap ``TestCase.subTest``: a subtest that matches a divergence must
    fail, and its failure is reported as an xfail."""

    @contextmanager
    def subTest(self, msg=None, **params) -> Iterator[None]:
        test_id = _test_id(self)
        entry = next(
            (
                d
                for d in DIVERGENCES
                if d.subtest is not None
                and d.covers(test_id)
                and d.matches_subtest(msg, params)
            ),
            None,
        )
        args = (msg,) if msg is not None else ()
        with original(self, *args, **params):
            if entry is None:
                yield
                return
            entry.hits += 1
            try:
                yield
            except Exception as exc:  # noqa: BLE001 - any failure is the divergence
                pytest.xfail(f"{entry.message}: {type(exc).__name__}: {exc}"[:2000])
            pytest.fail(
                f"[XPASS(strict)] listed in {DIVERGENCES_FILE.name} but passes: "
                f"{entry.message}",
                pytrace=False,
            )

    return subTest


@contextmanager
def _test_module_package() -> Iterator[None]:
    """A namespace for eyecite's test modules that doesn't shadow anything."""
    if PACKAGE not in sys.modules:
        spec = importlib.util.spec_from_loader(PACKAGE, loader=None, is_package=True)
        package = importlib.util.module_from_spec(spec)
        package.__path__ = [str(UPSTREAM_TESTS)]
        sys.modules[PACKAGE] = package
    yield


class EyeciteModule(pytest.Module):
    """One of eyecite's test modules, imported with ``eyecite`` as citare."""

    def _getobj(self):
        name = f"{PACKAGE}.{self.path.stem}"
        if name in sys.modules:
            return sys.modules[name]
        with _test_module_package(), shim.installed():
            spec = importlib.util.spec_from_file_location(name, self.path)
            module = importlib.util.module_from_spec(spec)
            sys.modules[name] = module
            try:
                spec.loader.exec_module(module)
            except BaseException:
                del sys.modules[name]
                raise
        for obj in vars(module).values():
            if (
                isinstance(obj, type)
                and issubclass(obj, unittest.TestCase)
                and obj.__module__ == name
            ):
                obj.subTest = _divergent_subtest(unittest.TestCase.subTest)
                for helper in SUBTEST_HELPERS.get(obj.__name__, ()):
                    setattr(
                        obj, helper, _helper_as_subtest(getattr(obj, helper), helper)
                    )
        return module

    def collect(self):
        entry = next((d for d in DIVERGENCES if d.test == self.path.name), None)
        try:
            _ = self.obj  # imports the module
        except Exception as exc:
            if entry is None:
                raise
            entry.found = True
            yield ImportDivergence.from_parent(
                self, name="<import>", entry=entry, error=exc
            )
            return
        yield from super().collect()


class ImportDivergence(pytest.Item):
    """A listed module that doesn't import against citare."""

    def __init__(self, *, entry: Divergence, error: BaseException, **kwargs):
        super().__init__(**kwargs)
        self.entry = entry
        self.error = error

    def runtest(self) -> None:
        pytest.xfail(f"{self.entry.message}: {type(self.error).__name__}: {self.error}")

    def reportinfo(self):
        return self.path, None, self.name


class DivergenceList(pytest.Item):
    """Checks ``eyecite_divergences.toml``: every entry names a collected
    test and links to a section of docs/divergences.md."""

    def runtest(self) -> None:
        anchors = set(
            re.findall(r'<a id="([^"]+)"></a>', DIVERGENCES_DOC.read_text("utf-8"))
        )
        problems = []
        for d in DIVERGENCES:
            path, _, anchor = d.doc.partition("#")
            if path != "docs/divergences.md" or anchor not in anchors:
                problems.append(
                    f"{d.test}: doc {d.doc!r} is not a docs/divergences.md section"
                )
            if not d.found:
                problems.append(f"{d.test}: matches no eyecite test")
        if problems:
            pytest.fail(
                f"{DIVERGENCES_FILE.name}:\n  " + "\n  ".join(problems), pytrace=False
            )

    def reportinfo(self):
        return DIVERGENCES_FILE, None, self.name


class EyeciteSuite(pytest.File):
    """eyecite's tests in ``upstream/eyecite/tests``."""

    def collect(self):
        if not any(UPSTREAM_TESTS.glob("test_*.py")):
            raise pytest.UsageError(
                f"eyecite's tests aren't in {UPSTREAM_TESTS}; "
                "run `git submodule update --init upstream/eyecite`"
            )
        for path in sorted(UPSTREAM_TESTS.glob("test_*.py")):
            yield EyeciteModule.from_parent(
                self, path=path, name=path.name, nodeid=f"{self.nodeid}::{path.name}"
            )
        yield DivergenceList.from_parent(self, name=DIVERGENCES_FILE.name)


def upstream_id(item: pytest.Item) -> str | None:
    """An item's id under upstream/eyecite/tests, if it is one of eyecite's."""
    for node in item.listchain():
        if isinstance(node, EyeciteModule):
            return item.nodeid.split("::", 1)[1]
    return None


ENTRY = TESTS_DIR / "test_eyecite.py"


def pytest_pycollect_makemodule(module_path: Path, parent: pytest.Collector):
    if module_path == ENTRY:
        return EyeciteSuite.from_parent(parent, path=module_path)
    return None


_SUBTEST_ENTRIES = pytest.StashKey[list]()


@pytest.hookimpl(tryfirst=True)
def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    """Mark the eyecite tests listed as divergences ``xfail(strict=True)``.
    Subtest entries are checked where the subtest runs. This runs before
    ``-k`` and ``--deselect`` drop items, so every entry finds its test."""
    for item in items:
        test_id = upstream_id(item)
        if test_id is None or isinstance(item, ImportDivergence):
            continue
        for d in DIVERGENCES:
            if not d.covers(test_id):
                continue
            d.found = True
            if d.subtest is None:
                item.add_marker(pytest.mark.xfail(reason=d.message, strict=True))
            else:
                item.stash.setdefault(_SUBTEST_ENTRIES, []).append(d)


@pytest.hookimpl(wrapper=True)
def pytest_runtest_call(item: pytest.Item):
    """Run an eyecite test with ``eyecite`` resolving to citare, and fail it
    when a subtest divergence listed for it didn't match any subtest."""
    if upstream_id(item) is None:
        return (yield)
    entries = item.stash.get(_SUBTEST_ENTRIES, [])
    before = [d.hits for d in entries]
    with shim.installed():
        result = yield
    unmatched = [d for d, hits in zip(entries, before) if d.hits == hits]
    if unmatched:
        pytest.fail(
            f"{DIVERGENCES_FILE.name} lists subtests of this test that didn't run: "
            + "; ".join(repr(d.subtest) for d in unmatched),
            pytrace=False,
        )
    return result
