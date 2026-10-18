"""eyecite's own tests, ported to run against citare's Python API.

Provenance: engine-tests. These are eyecite v2.7.8's tests
(upstream/eyecite/tests, https://github.com/freelawproject/eyecite/tree/v2.7.8/tests),
with ``import eyecite`` swapped for ``import citare``. They check that code
written against eyecite's API runs unchanged; they are not ground truth for
what citare should find. Where citare deliberately differs
(docs/divergences.md), the case is marked ``xfail`` with the reason.
"""
