"""Runs eyecite's own test suite, from the ``upstream/eyecite`` submodule,
against citare's Python API (see ``eyecite_compat/suite.py``)."""

from __future__ import annotations

import pytest
from eyecite_compat import suite


def pytest_configure(config: pytest.Config) -> None:
    # A plugin, not conftest hooks: eyecite's test items live under
    # upstream/eyecite/tests, where this conftest's hooks don't apply.
    if not config.pluginmanager.is_registered(suite):
        config.pluginmanager.register(suite, "eyecite_compat.suite")
