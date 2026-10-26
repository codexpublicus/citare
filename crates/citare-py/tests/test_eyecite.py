"""eyecite's own test suite against citare's Python API.

conftest.py collects this file as eyecite's test modules from the
``upstream/eyecite`` submodule (``upstream/eyecite/tests``), imported with
``eyecite`` resolving to citare; nothing of eyecite's is copied here. The
tests citare deliberately fails are listed in ``eyecite_divergences.toml``.
See ``eyecite_compat/suite.py``.
"""
