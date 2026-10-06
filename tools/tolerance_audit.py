#!/usr/bin/env python3
"""FW-03 diagnostic, not an acceptance gate for J comparison semantics.

Observe pinned C J vs independent sequential Rust vs optimized Rust for
floating search and comparison at adjacent binary64 tolerance boundaries.
Known Rust fixed-near != J CCT discrepancies are reported, never waived as
passes, and the optimized route must agree with the independent Rust path.

This does not implement or test 9!:19 / !.t and does not license tolerant hash.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import sys

from conformance import equal, validate_cli_corpus
from search_three_way import _run, classify, ROOT


def j_float(value):
    """Use a round-trip binary64 decimal with J's underscore minus notation."""
    if math.isnan(value):
        return "_."
    if value == math.inf:
        return "_"
    if value == -math.inf:
        return "__"
    # J negative numbers use _, including exponents.
    literal = format(value, ".17g").replace("-", "_")
    # An integer-looking token would be an INT in J, not binary64.
    return literal if ("." in literal or "e" in literal) else literal + ".0"


def tolerance_cases():
    t = 2.0 ** -44
    one = 1.0
    below = one - t
    above = one + t
    left_neighbor = math.nextafter(below, 0.0)
    right_neighbor = math.nextafter(above, math.inf)
    near_b = one + 0.75 * t
    far_c = one + 1.5 * t

    pairs = [
        ("exact", one, one),
        ("lower_edge", one, below),
        ("upper_edge", one, above),
        ("outside_lower", one, left_neighbor),
        ("outside_upper", one, right_neighbor),
        ("half_chain", one, near_b),
        ("nontransitive_end", one, far_c),
        ("nontransitive_middle", near_b, far_c),
        ("zero_sign", 0.0, -0.0),
        ("positive_inf", math.inf, math.inf),
        ("opposite_inf", math.inf, -math.inf),
        ("nan", math.nan, math.nan),
    ]
    cases = []
    for label, left, right in pairs:
        a, b = j_float(left), j_float(right)
        # Compare floating scalar verb semantics separately from item search.
        cases.extend([
            (label + ":eq", f"({a}) = ({b})"),
            (label + ":first", f"({a}) i. ({b})"),
            (label + ":last", f"({a}) i: ({b})"),
            (label + ":member", f"({b}) e. ({a})"),
        ])
    return cases


def audit(binary, library, revision, report_path):
    if not binary.is_file() or not library.is_file():
        raise FileNotFoundError(f"missing binary or pinned C J library: {binary}, {library}")
    cases = tolerance_cases()
    corpus = [expr for _, expr in cases]
    validate_cli_corpus(corpus)
    oracle = _run(
        [sys.executable, str(ROOT / "tools/oracle.py")],
        "".join(json.dumps(s) + "\n" for s in corpus), len(corpus), "jsource",
    )
    rust_stdin = "\n".join(corpus) + "\n"
    rust_cmd = [str(binary), "--json"]
    reference = _run(
        rust_cmd + ["--semantic-reference"], rust_stdin, len(corpus), "rust-reference"
    )
    optimized = _run(rust_cmd, rust_stdin, len(corpus), "rust-optimized")
    results = []
    counts = {}
    route_errors = 0
    for (label, source), c, r, o in zip(cases, oracle, reference, optimized, strict=True):
        status = classify(c, r, o)
        counts[status] = counts.get(status, 0) + 1
        if not equal(r, o):
            route_errors += 1
        results.append({
            "label": label, "source": source, "classification": status,
            "jsource": c, "rust_reference": r, "rust_optimized": o,
        })
    report = {
        "kind": "FW-03 diagnostic only: CCT/Fit semantics not accepted",
        "reference_revision": revision,
        "reference_library": str(library),
        "reference_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
        "rust_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "cases": len(cases),
        "classifications": counts,
        "rust_reference_optimized_disagreements": route_errors,
        "observations": results,
        "limitations": [
            "J C binary under its default CCT, RustJ legacy fixed-near",
            "No dynamic CCT, no !.t, no boxed/sparse/Rank semantics",
            "A diagnostic mismatch is never an accepted conformance pass",
        ],
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/rustj")
    parser.add_argument("--reference-revision", required=True)
    parser.add_argument("--report", type=Path, default=ROOT / "reports/tolerance-audit.json")
    args = parser.parse_args()
    library = Path(os.environ.get(
        "J_LIBRARY", str(ROOT / ".reference/bin/linux/j64/libj.so")
    ))
    try:
        report = audit(args.binary.resolve(), library.resolve(),
                       args.reference_revision, args.report)
    except (OSError, RuntimeError) as exc:
        print(f"FW-03 diagnostic failed: {exc}", file=sys.stderr)
        return 1
    print(json.dumps({
        "cases": report["cases"],
        "classifications": report["classifications"],
        "rust_reference_optimized_disagreements": report[
            "rust_reference_optimized_disagreements"
        ],
        "gate": "DIAGNOSTIC ONLY; J CCT conformance not claimed",
        "c_rust_mismatch_labels": [
            row["label"] for row in report["observations"]
            if row["classification"] != "pass"
        ],
    }, indent=2))
    # Do not treat known language semantic gaps as passing. However, a newly
    # divergent optimized route is a regression against RustJ's own baseline.
    return int(report["rust_reference_optimized_disagreements"] != 0)


if __name__ == "__main__":
    sys.exit(main())
