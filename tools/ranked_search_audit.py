#!/usr/bin/env python3
"""FW-04 ranked search exploratory three-way diagnostic.

Rank changes how cells/frames are applied; not all forms are supported by
RustJ's closed A3 reference executor. Every discrepancy is recorded by its
exact source and result; this is NOT an acceptance gate or silent waiver.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys

from conformance import validate_cli_corpus
from search_three_way import ROOT, _run, classify


def ranked_search_cases():
    return [
        ("whole_first", '(i.3) (i."1 1) (i.3)'),
        ("whole_last", '(i.3) (i:"1 1) (i.3)'),
        ("whole_member", '(i.3) (e."1 1) (i.3)'),
        ("scalar_cells_first", '(i.3) (i."0 0) (i.3)'),
        ("scalar_cells_last", '(i.3) (i:"0 0) (i.3)'),
        ("scalar_cells_member", '(i.3) (e."0 0) (i.3)'),
        ("row_first", '(i.2 3) (i."1 1) (i.2 3)'),
        ("row_last", '(i.2 3) (i:"1 1) (i.2 3)'),
        ("row_member", '(i.2 3) (e."1 1) (i.2 3)'),
        ("left_broadcast", '(i.3) (i."1 1) (i.2 3)'),
        ("right_broadcast", '(i.2 3) (i."1 1) (i.3)'),
        ("scalar_left_broadcast", '3 (i."0 0) (i.2 3)'),
        ("frame_mismatch", '(i.2 3) (i."1 1) (i.3 3)'),
        ("empty_frame", '(i.0 3) (i."1 1) (i.0 3)'),
        ("empty_frame_last", '(i.0 3) (i:"1 1) (i.0 3)'),
        ("empty_frame_member", '(i.0 3) (e."1 1) (i.0 3)'),
        ("empty_frame_char", "(0 3 $ 'abc') (i.\"1 1) (0 3 $ 'abc')"),
        ("empty_frame_float", '(0 3 $ 1.5) (i."1 1) (0 3 $ 1.5)'),
        ("empty_frame_sum", '+/"1 (i.0 3)'),
        ("empty_frame_add", '(i.0 3) (+"1 1) (i.0 3)'),
    ]


def run(binary, library, revision, path):
    if not binary.is_file() or not library.is_file():
        raise FileNotFoundError(f"missing Rust executable or C library: {binary}, {library}")
    cases = ranked_search_cases()
    corpus = [expression for _, expression in cases]
    validate_cli_corpus(corpus)
    expected = _run(
        [sys.executable, str(ROOT / "tools/oracle.py")],
        "".join(json.dumps(expr) + "\n" for expr in corpus),
        len(corpus), "jsource",
    )
    text = "\n".join(corpus) + "\n"
    rust_cmd = [str(binary), "--json"]
    reference = _run(rust_cmd + ["--semantic-reference"], text, len(corpus), "rust-reference")
    optimized = _run(rust_cmd, text, len(corpus), "rust-optimized")
    counts = {}
    observations = []
    for (name, expression), c, r, o in zip(cases, expected, reference, optimized, strict=True):
        kind = classify(c, r, o)
        counts[kind] = counts.get(kind, 0) + 1
        observations.append({
            "name": name, "source": expression, "classification": kind,
            "jsource": c, "rust_reference": r, "rust_optimized": o,
        })
    report = {
        "kind": "FW-04 ranked search diagnostic; NOT an acceptance gate",
        "reference_revision": revision,
        "reference_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "cases": len(cases),
        "classifications": counts,
        "observations": observations,
        "limitations": [
            "Ranked call/derived-entity and empty-prototype behavior may be unsupported",
            "C/Rust triple matches do not prove all J Rank semantics",
            "No source-form differences are waived or counted as conformance",
        ],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/rustj")
    parser.add_argument("--reference-revision", required=True)
    parser.add_argument("--report", type=Path, default=ROOT / "reports/ranked-search-audit.json")
    args = parser.parse_args()
    library = Path(os.environ.get("J_LIBRARY", str(ROOT / ".reference/bin/linux/j64/libj.so")))
    try:
        report = run(args.binary.resolve(), library.resolve(), args.reference_revision, args.report)
    except (OSError, RuntimeError) as error:
        print(f"FW-04 ranked diagnostic failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps({
        "cases": report["cases"],
        "classifications": report["classifications"],
        "not_matching": [
            row["name"] for row in report["observations"] if row["classification"] != "pass"
        ],
        "gate": "DIAGNOSTIC ONLY: do not count mismatches as passes",
    }, indent=2))
    # This bounded corpus is a concrete supported subset. Once its
    # empty-frame semantics are implemented, any regression must fail CI;
    # the report still retains failures instead of silently waiving them.
    return int(any(
        row["classification"] != "pass" for row in report["observations"]
    ))


if __name__ == "__main__":
    sys.exit(main())
