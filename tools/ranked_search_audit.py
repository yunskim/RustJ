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


def rank_adversarial_cases():
    """RK-06 exploratory zero-cardinality witnesses, NOT an acceptance set.

    In particular, [0,3] has no rank-1 cells; [2,0] has two empty rank-1
    cells. A zero inside a larger frame is neither of those cases.
    """
    return [
        ("zero_frame_front_reduce", '+/"1 (i.0 3)'),
        ("empty_cells_nonzero_frame_reduce", '+/"1 (i.2 0)'),
        ("empty_cells_nonzero_frame_add", '(i.2 0) (+"1 1) (i.2 0)'),
        ("zero_frame_middle_reduce", '+/"1 (i.2 0 3)'),
        ("zero_frame_middle_add", '(i.2 0 3) (+"1 1) (i.2 0 3)'),
        ("zero_frame_middle_search", '(i.2 0 3) (i."1 1) (i.2 0 3)'),
        ("zero_frame_tail_rank0_add", '(i.2 3 0) (+"0 0) (i.2 3 0)'),
        ("zero_frame_front_ravel", ',"1 (i.0 3)'),
        ("one_side_empty_left_add", '(i.0 3) (+"1 1) (i.3)'),
        ("one_side_empty_right_add", '(i.3) (+"1 1) (i.0 3)'),
        ("one_side_empty_left_search", '(i.0 3) (i."1 1) (i.3)'),
        ("one_side_empty_right_search", '(i.3) (i."1 1) (i.0 3)'),
        ("frame_mismatch_empty_vs_full", '(i.0 3) (+"1 1) (i.2 3)'),
        ("middle_frame_mismatch", '(i.2 0 3) (+"1 1) (i.2 2 3)'),
        ("negative_rank_reduce", '+/"_1 (i.0 3)'),
        ("oversized_rank_reduce", '+/"99 (i.0 3)'),
        ("negative_rank_add", '(i.0 3) (+"_1 _1) (i.0 3)'),
        ("oversized_rank_add", '(i.0 3) (+"99 99) (i.0 3)'),
        ("empty_type_mismatch", "(0 3 $ 'abc') (+\"1 1) (i.0 3)"),
        # cr.c::jtrank2ex: distinguish EVINHOMO fill-type retry from
        # a computational-domain fallback and from real-cell execution.
        ("empty_type_mismatch_reversed", '(i.0 3) (+"1 1) (0 3 $ \'abc\')'),
        ("empty_char_float_fill", "(0 3 $ 'abc') (+\"1 1) (0 3 $ 1.5)"),
        ("empty_char_bool_fill", "(0 3 $ 'abc') (+\"1 1) (0 3 $ 1=1)"),
        ("empty_char_left_real_right", "(0 3 $ 'abc') (+\"1 1) (i.3)"),
        ("empty_char_right_real_left", "(i.3) (+\"1 1) (0 3 $ 'abc')"),
        ("nonempty_char_int_domain", "(2 3 $ 'abc') (+\"1 1) (i.2 3)"),
        ("positive_frame_empty_char_cells", "(2 0 $ 'abc') (+\"1 1) (i.2 0)"),
        ("nested_rank_nonempty_cells", '(i.2 3) ((+"0 0)"1 1) (i.2 3)'),
        ("empty_division", '(i.0 3) (%"1 1) (i.0 3)'),
        ("empty_zero_cell_rank0_reduce", '+/"0 (i.2 0)'),
        ("empty_bool_member", '(0 3 $ 1) (e."1 1) (0 3 $ 1)'),
        ("empty_char_index", "(0 3 $ 'abc') (i.\"1 1) (0 3 $ 'abc')"),
        ("nested_rank_empty", '(i.0 3) ((+"0 0)"1 1) (i.0 3)'),
    ]


def run(binary, library, revision, path, adversarial=False):
    if not binary.is_file() or not library.is_file():
        raise FileNotFoundError(f"missing Rust executable or C library: {binary}, {library}")
    cases = rank_adversarial_cases() if adversarial else ranked_search_cases()
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
        "kind": ("RK-06 adversarial Rank diagnostic; NOT an acceptance gate"
                 if adversarial else "FW-04 bounded Rank three-way regression"),
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


def diagnostic_summary(report, adversarial):
    """Expose the exact C/Rust divergence; an exploratory CI pass is not acceptance."""
    mismatches = [
        row for row in report["observations"] if row["classification"] != "pass"
    ]
    summary = {
        "cases": report["cases"],
        "classifications": report["classifications"],
        "not_matching": [row["name"] for row in mismatches],
        "gate": ("EXPLORATORY: mismatches remain open, NOT accepted"
                 if adversarial else "BOUNDED REGRESSION: fail on any mismatch"),
    }
    if adversarial:
        summary["mismatch_observations"] = mismatches
    return summary


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/rustj")
    parser.add_argument("--reference-revision", required=True)
    parser.add_argument("--report", type=Path, default=ROOT / "reports/ranked-search-audit.json")
    parser.add_argument("--adversarial", action="store_true",
                        help="RK-06 exploratory corpus; report mismatches without acceptance")
    args = parser.parse_args()
    library = Path(os.environ.get("J_LIBRARY", str(ROOT / ".reference/bin/linux/j64/libj.so")))
    try:
        report = run(args.binary.resolve(), library.resolve(), args.reference_revision,
                     args.report, adversarial=args.adversarial)
    except (OSError, RuntimeError) as error:
        print(f"FW-04 ranked diagnostic failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(diagnostic_summary(report, args.adversarial), indent=2))
    # This bounded corpus is a concrete supported subset. Once its
    # empty-frame semantics are implemented, any regression must fail CI;
    # the report still retains failures instead of silently waiving them.
    return int(not args.adversarial and any(
        row["classification"] != "pass" for row in report["observations"]
    ))


if __name__ == "__main__":
    sys.exit(main())
