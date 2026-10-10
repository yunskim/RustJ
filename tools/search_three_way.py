#!/usr/bin/env python3
"""FW-04: pinned J C / independent Rust reference / optimized Rust search.

Unlike the broad conformance corpus, this keeps exactly the same search
sentences and name bindings in three isolated subprocesses.  Passing this
supported-dense subset does NOT validate boxed, sparse, dynamic CCT/Fit, or
general ranked derived calls.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from conformance import equal, validate_cli_corpus

ROOT = Path(__file__).resolve().parents[1]


def search_cases():
    """Deterministic, bounded supported-dense first/last/member witnesses."""
    cases = [
        "(3 1 3 2) i. (3 4 1)",
        "(3 1 3 2) i: (3 4 1)",
        "(3 4 1) e. (3 1 3 2)",
        "(i.0) i. (3 4)",
        "(i.0) i: (3 4)",
        "(3 4) e. (i.0)",
        "(i.128) i. (17 17 199)",
        "(i.128) i: (17 17 199)",
        "(17 199) e. (i.128)",
        "(i.2 3) i. (2 3 $ 3 4 5 0 1 2)",
        "(i.2 3) e. (i.2 3)",
        "(i.2 0) i. (i.2 0)",
        "('abca') i. ('acx')",
        "('abca') i: ('acx')",
        "('acx') e. ('abca')",
        "(0 1 0 1) i. (1 2 0)",
        "(0 1 0 1) i: (1 2 0)",
        "(1 2 0) e. (0 1 0 1)",
        "(0.0 1.0 2.0) i. (2.0 3.0)",
        "(0.0 1.0 2.0) i: (2.0 3.0)",
        "(2.0 3.0) e. (0.0 1.0 2.0)",
        "keys=:i.256",
        "keys i. 17 255 999",
        "keys i. 17 255 999",
        "keys i: 17 255 999",
        "17 255 999 e. keys",
        "widekeys=:1000000 * i.256",
        "widekeys i. 0 1000000 4000000 _1",
        "widekeys i: 0 1000000 4000000 _1",
        "0 1000000 4000000 _1 e. widekeys",
    ]
    for n in (1, 3, 33, 65, 128):
        cases.extend([
            f"(i.{n}) i. (0 1 {n} _1)",
            f"(i.{n}) i: (0 1 {n} _1)",
            f"(0 1 {n} _1) e. (i.{n})",
        ])
    return cases


def classify(c, reference, optimized):
    c_ref = equal(c, reference)
    c_opt = equal(c, optimized)
    ref_opt = equal(reference, optimized)
    if c_ref and c_opt:
        return "pass"
    if c_ref:
        return "optimized_route_mismatch"
    if c_opt:
        return "rust_reference_mismatch"
    if ref_opt:
        return "rust_semantic_mismatch"
    return "all_three_differ"


def _run(argv, stdin, count, label):
    done = subprocess.run(
        argv, input=stdin, text=True, encoding="utf-8",
        capture_output=True, timeout=120, cwd=ROOT,
    )
    if done.returncode not in (0, 1):
        raise RuntimeError(f"{label} failed rc={done.returncode}: {done.stderr[-2000:]}")
    if label == "jsource" and done.returncode != 0:
        raise RuntimeError(f"jsource adapter failed: {done.stderr[-2000:]}")
    try:
        rows = [json.loads(line) for line in done.stdout.splitlines()]
    except ValueError as error:
        raise RuntimeError(f"non-JSON output from {label}: {error}") from error
    if len(rows) != count:
        raise RuntimeError(
            f"incomplete {label} output: expected {count}, received {len(rows)}; "
            f"stderr={done.stderr[-1000:]}"
        )
    return rows


def run(binary, library, revision, report_path):
    if not binary.is_file() or not library.is_file():
        raise FileNotFoundError(f"missing executable or pinned C library: {binary}, {library}")
    corpus = search_cases()
    validate_cli_corpus(corpus)
    encoded_oracle = "".join(json.dumps(sentence) + "\n" for sentence in corpus)
    encoded_rust = "\n".join(corpus) + "\n"
    report = {
        "scope": "supported-dense search only; no boxed/sparse/dynamic CCT/Fit/ranked derived proof",
        "reference_revision": revision,
        "reference_library": str(library),
        "reference_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
        "rust_binary": str(binary),
        "rust_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "cases": len(corpus),
        "passed": 0,
        "failed": 0,
        "classifications": {},
        "failures": [],
    }
    cmd_oracle = [sys.executable, str(ROOT / "tools/oracle.py")]
    cmd_rust = [str(binary), "--json"]
    oracle = _run(cmd_oracle, encoded_oracle, len(corpus), "jsource")
    reference = _run(cmd_rust + ["--semantic-reference"], encoded_rust, len(corpus), "rust-reference")
    optimized = _run(cmd_rust, encoded_rust, len(corpus), "rust-optimized")
    for index, (sentence, c, r, o) in enumerate(
        zip(corpus, oracle, reference, optimized, strict=True)
    ):
        status = classify(c, r, o)
        report["classifications"][status] = report["classifications"].get(status, 0) + 1
        if status == "pass":
            report["passed"] += 1
        else:
            report["failures"].append({
                "index": index, "source": sentence, "classification": status,
                "jsource": c, "rust_reference": r, "rust_optimized": o,
            })
    report["failed"] = len(report["failures"])
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/rustj")
    parser.add_argument("--reference-revision", required=True)
    parser.add_argument("--report", type=Path, default=ROOT / "reports/search-three-way.json")
    args = parser.parse_args()
    library = Path(os.environ.get(
        "J_LIBRARY", str(ROOT / ".reference/bin/linux/j64/libj.so")
    ))
    try:
        result = run(args.binary.resolve(), library.resolve(),
                     args.reference_revision, args.report)
    except (RuntimeError, OSError, subprocess.TimeoutExpired) as error:
        print(f"FW-04 search three-way harness failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps({
        key: result[key]
        for key in ("reference_revision", "cases", "passed", "failed", "classifications")
    }, indent=2))
    return int(result["failed"] != 0)


if __name__ == "__main__":
    sys.exit(main())
