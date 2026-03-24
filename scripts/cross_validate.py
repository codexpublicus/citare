#!/usr/bin/env python3
"""Cross-validate citare (Rust) against eyecite (Python).

Runs both tools on the same inputs and compares citation extraction results.
Requires: eyecite installed, citare-cli built.

Usage:
    python3 scripts/cross_validate.py
"""

import json
import subprocess
import sys
from pathlib import Path

# Add eyecite to path
EYECITE_DIR = Path(__file__).parent.parent / "upstream" / "eyecite"
sys.path.insert(0, str(EYECITE_DIR))

CITARE_CLI = None  # Will be found dynamically


def find_citare_cli():
    """Find the citare CLI binary."""
    project_root = Path(__file__).parent.parent
    # Try release first, then debug
    for profile in ["release", "debug"]:
        path = project_root / "target" / profile / "citare"
        if path.exists():
            return str(path)
    # Try building it
    print("Building citare-cli...")
    result = subprocess.run(
        ["cargo", "build", "-p", "citare-cli",
         "--manifest-path", str(project_root / "Cargo.toml")],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print(f"Failed to build citare-cli: {result.stderr}")
        sys.exit(1)
    return str(project_root / "target" / "debug" / "citare")


def run_eyecite(text):
    """Run eyecite on text, return normalized citation list."""
    from eyecite import get_citations
    from eyecite.models import ResourceCitation

    citations = get_citations(plain_text=text)
    results = []
    for cite in citations:
        # Clear pin_cite spans for comparison (eyecite tests do this too)
        cite.metadata.pin_cite_span_start = None
        cite.metadata.pin_cite_span_end = None

        entry = {
            "type": type(cite).__name__,
            "matched_text": cite.matched_text(),
            "span": list(cite.span()),
            "groups": {
                k: v for k, v in cite.groups.items()
                if v is not None
            },
        }
        if isinstance(cite, ResourceCitation):
            entry["year"] = cite.year
            entry["corrected_reporter"] = cite.corrected_reporter()
        results.append(entry)
    return results


def run_citare(text, cli_path):
    """Run citare CLI on text, return normalized citation list."""
    result = subprocess.run(
        [cli_path, "find", "--text", text],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        return [{"error": result.stderr.strip()}]

    try:
        citations = json.loads(result.stdout)
    except json.JSONDecodeError:
        return [{"error": f"Invalid JSON: {result.stdout[:200]}"}]

    results = []
    for cite in citations:
        entry = {
            "type": cite["type"],
            "matched_text": cite["matched_text"],
            "span": cite["span"],
            "groups": {
                k: v for k, v in cite.get("groups", {}).items()
                if v is not None
            },
        }
        if cite.get("year") is not None:
            entry["year"] = cite["year"]
        # Map corrected_reporter from metadata or groups
        meta = cite.get("metadata", {})
        if cite["type"] in ("FullCaseCitation", "ShortCaseCitation",
                            "FullLawCitation", "FullJournalCitation"):
            entry["corrected_reporter"] = (
                cite["groups"].get("reporter", "")
            )
        results.append(entry)
    return results


def normalize(results):
    """Normalize results for comparison."""
    for r in results:
        # Sort groups keys
        if "groups" in r:
            r["groups"] = dict(sorted(r["groups"].items()))
    return results


# Test cases: (description, input_text)
TEST_CASES = [
    ("basic citation", "1 U.S. 1"),
    ("with year", "1 U.S. 1 (1999)"),
    ("with parties", "Foo v. Bar, 1 U.S. 1 (1999)"),
    ("with court", "Foo v. Bar, 1 U.S. 12, 347-348 (4th Cir. 1982)"),
    ("second edition", "22 A.2d 332"),
    ("variation reporter", "22 U. S. 332 (1975)"),
    ("short citation", "Foo v. Bar, 1 U.S. 1 (1999). Bar, 1 U.S., at 2"),
    ("id citation", "1 U.S. 1 (1999). Id. at 2."),
    ("supra citation", "Foo v. Bar, 1 U.S. 1 (1999). Foo, supra, at 2."),
    ("multiple citations", "1 U.S. 1 and 2 F.3d 100"),
    ("law citation", "Mass. Gen. Laws ch. 1, § 2"),
    ("with month/day", "Foo v. Bar, 164 A.3d 1189 (Pa. Feb. 9, 2017)"),
    ("with parenthetical", "Foo v. Bar, 1 U.S. 1 (1999) (discussing something)"),
    ("pin cite", "1 U.S. 1, 4-5 (1999)"),
    ("placeholder page", "585 U.S. ___ (2018)"),
    ("roman numeral page", "1 U.S. xlii"),
    ("journal citation", "1 Minn. L. Rev. 1"),
    ("parallel citation",
     "Foo v. Bar, 1 U.S. 12, 1 S. Ct. 2 (1982)"),
    ("long text with multiple types",
     "In Foo v. Bar, 1 U.S. 1 (1999), the Court held something. "
     "See also Baz v. Qux, 2 F.3d 100 (2d Cir. 2000). Id. at 102. "
     "Foo, supra, at 5."),
]


def main():
    cli_path = find_citare_cli()
    print(f"Using citare CLI: {cli_path}\n")

    passed = 0
    failed = 0
    errors = 0
    mismatches = []

    for desc, text in TEST_CASES:
        try:
            eyecite_result = normalize(run_eyecite(text))
            citare_result = normalize(run_citare(text, cli_path))
        except Exception as e:
            print(f"  ERROR: {desc}: {e}")
            errors += 1
            continue

        # Compare type counts
        eyecite_types = [r["type"] for r in eyecite_result]
        citare_types = [r["type"] for r in citare_result]

        if eyecite_types == citare_types:
            # Compare matched text
            eyecite_texts = [r["matched_text"] for r in eyecite_result]
            citare_texts = [r["matched_text"] for r in citare_result]
            if eyecite_texts == citare_texts:
                print(f"  PASS: {desc}")
                passed += 1
            else:
                print(f"  DIFF: {desc}")
                print(f"    eyecite texts: {eyecite_texts}")
                print(f"    citare texts:  {citare_texts}")
                failed += 1
                mismatches.append((desc, "matched_text", eyecite_texts, citare_texts))
        else:
            print(f"  DIFF: {desc}")
            print(f"    eyecite types: {eyecite_types}")
            print(f"    citare types:  {citare_types}")
            failed += 1
            mismatches.append((desc, "types", eyecite_types, citare_types))

    print(f"\n{'='*60}")
    print(f"Results: {passed} passed, {failed} failed, {errors} errors")
    print(f"Total: {len(TEST_CASES)} test cases")

    if mismatches:
        print(f"\nMismatches:")
        for desc, field, expected, got in mismatches:
            print(f"  {desc} ({field}):")
            print(f"    eyecite: {expected}")
            print(f"    citare:  {got}")

    return 0 if failed == 0 and errors == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
