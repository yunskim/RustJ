#!/usr/bin/env python3
"""IO-01/02: pinned J C file-foreign behavior diagnostic (NOT RustJ acceptance).

Only touches fresh per-probe files inside TemporaryDirectory. The J reference
library is loaded in a separate Python process through tools/oracle.py. This
probe records actual J behavior before RustJ's physical-I/O path exists.
"""
import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import tempfile

from oracle import Oracle

ROOT = Path(__file__).resolve().parents[1]
INITIAL = b"ABCDEFGH"


@dataclass(frozen=True)
class Probe:
    name: str
    j: str
    initial: bytes | None = INITIAL
    inspect: str = "run"             # "run" or "eval"
    noun: dict | None = None
    after: bytes | None = INITIAL
    after_size: int | None = None
    after_prefix: bytes | None = None
    after_suffix: bytes | None = None
    expect_error: bool = False
    expected_error: str | None = None


def cases() -> tuple[Probe, ...]:
    """Deterministic independent cases; every file begins in a fresh state.

    J's 1!:11/1!:12 examples use 'name';index (unboxed left name),
    while 1!:1/1!:4 use <'name'. See the J Files foreign reference.
    """
    return (
        Probe("full_read", "1!:1 {file}", inspect="eval",
              noun={"type": 2, "shape": [8], "data": list(INITIAL)}),
        Probe("file_size", "1!:4 {file}", inspect="eval",
              noun={"type": 4, "shape": [], "data": [8]}),
        Probe("partial_read", "1!:11 ({name};2 3)", inspect="eval",
              noun={"type": 2, "shape": [3], "data": list(b"CDE")}),
        Probe("negative_start_read", "1!:11 ({name};_2 2)", inspect="eval",
              noun={"type": 2, "shape": [2], "data": list(b"GH")}),
        Probe("zero_length_at_eof", "1!:11 ({name};8 0)"),
        Probe("zero_length_empty_file", "1!:11 ({name};0 0)", initial=b"", after=b""),
        Probe("read_past_eof", "1!:11 ({name};7 2)",
              expect_error=True, expected_error="index error"),
        Probe("read_negative_length", "1!:11 ({name};2 _1)",
              expect_error=True, expected_error="index error"),
        Probe("write_inside", "'xy' 1!:12 ({name};2)", after=b"ABxyEFGH"),
        Probe("write_outside_start", "'XY' 1!:12 ({name};_9)",
              expect_error=True, expected_error="index error"),
        Probe("write_beyond_eof", "'XY' 1!:12 ({name};10)",
              after=None, after_size=12, after_prefix=INITIAL, after_suffix=b"XY"),
        Probe("missing_file", "1!:11 ({name};0 1)", initial=None, after=None, expect_error=True),
        Probe("missing_file_bad_range", "1!:11 ({name};9 2)", initial=None, after=None,
              expect_error=True),
        Probe("zero_length_missing_file", "1!:11 ({name};0 0)", initial=None, after=None,
              expect_error=True),
        Probe("discarded_missing_read", "0 [ (1!:11 ({name};0 1))",
              initial=None, after=None, expect_error=True),
    )



@dataclass(frozen=True)
class OrderedStep:
    """One J expression plus the file state observable after that expression."""
    j: str
    after: bytes
    inspect: str = "run"
    noun: dict | None = None
    expect_error: bool = False
    expected_error: str | None = None


@dataclass(frozen=True)
class OrderedCase:
    name: str
    steps: tuple[OrderedStep, ...]


def ordered_cases() -> tuple[OrderedCase, ...]:
    """IO-02: effect liveness, error barriers, and state after exceptions.

    Steps use one J engine and one file; the fixture inspects the backing file
    after EVERY JDo. J exceptions are observable and must not erase earlier
    externally committed writes, or cause an unexecuted write to happen.
    """
    return (
        OrderedCase("discarded_write_has_effect", (
            OrderedStep("0 [ ('xy' 1!:12 ({name};2))", b"ABxyEFGH"),
            OrderedStep("1!:11 ({name};2 2)", b"ABxyEFGH", inspect="eval",
                        noun={"type": 2, "shape": [2], "data": list(b"xy")}),
        )),
        OrderedCase("right_error_prevents_left_write", (
            OrderedStep("('Z' 1!:12 ({name};1)) [ (1!:11 ({missing};0 1))",
                        INITIAL, expect_error=True),
            OrderedStep("1!:11 ({name};1 1)", INITIAL, inspect="eval",
                        noun={"type": 2, "shape": [1], "data": list(b"B")}),
        )),
        OrderedCase("completed_write_survives_later_error", (
            OrderedStep("'Z' 1!:12 ({name};1)", b"AZCDEFGH"),
            OrderedStep("1!:11 ({missing};0 1)", b"AZCDEFGH", expect_error=True),
            OrderedStep("1!:11 ({name};1 1)", b"AZCDEFGH", inspect="eval",
                        noun={"type": 2, "shape": [1], "data": list(b"Z")}),
        )),
        OrderedCase("bad_index_write_preserves_previous_effect", (
            OrderedStep("'x' 1!:12 ({name};2)", b"ABxDEFGH"),
            OrderedStep("'y' 1!:12 ({name};_9)", b"ABxDEFGH",
                        expect_error=True, expected_error="index error"),
            OrderedStep("1!:11 ({name};2 1)", b"ABxDEFGH", inspect="eval",
                        noun={"type": 2, "shape": [1], "data": list(b"x")}),
        )),
        OrderedCase("append_then_tail_read", (
            OrderedStep("'!' 1!:3 {file}", b"ABCDEFGH!"),
            OrderedStep("1!:11 ({name};_1 1)", b"ABCDEFGH!", inspect="eval",
                        noun={"type": 2, "shape": [1], "data": list(b"!")}),
        )),
        OrderedCase("truncate_then_full_read", (
            OrderedStep("'XY' 1!:2 {file}", b"XY"),
            OrderedStep("1!:1 {file}", b"XY", inspect="eval",
                        noun={"type": 2, "shape": [2], "data": list(b"XY")}),
        )),
    )


def _run_ordered_case(oracle: Oracle, case: OrderedCase, directory: Path) -> dict:
    """Each ordered case is isolated; only its steps share file/J state."""
    path = directory / (case.name + ".dat")
    missing = directory / (case.name + "-missing.dat")
    if path.exists() or missing.exists():
        raise FileExistsError("ordered test cases must never reuse file paths")
    path.write_bytes(INITIAL)
    results = []
    for step in case.steps:
        expression = step.j.format(
            name=j_file_name(path), file=j_file_argument(path), missing=j_file_name(missing))
        observed = oracle.eval(expression) if step.inspect == "eval" else oracle.run(expression)
        actual = path.read_bytes() if path.is_file() else None
        has_error = isinstance(observed, dict) and "error" in observed
        checks = {
            "error_presence": has_error == step.expect_error,
            "file_content": actual == step.after,
            "missing_file_untouched": not missing.exists(),
        }
        if step.expected_error is not None:
            checks["error_class"] = has_error and observed["error"] == step.expected_error
        elif step.expect_error:
            checks["not_argument_or_syntax_failure"] = (has_error and
                observed["error"] not in {"length error", "rank error", "syntax error"})
        if step.noun is not None:
            checks["noun_value"] = observed == step.noun
        results.append({
            "j": expression.replace(str(path), "<temporary-file>").replace(
                str(missing), "<missing-file>"),
            "outcome": observed,
            "after_hex": _as_json_bytes(actual),
            "checks": checks,
            "status": "matches_source_expectation" if all(checks.values()) else "requires_review",
        })
    return {
        "case": case.name,
        "steps": results,
        "status": ("matches_source_expectation" if all(
            step["status"] == "matches_source_expectation" for step in results
        ) else "requires_review"),
    }


def j_file_name(path: Path) -> str:
    """Unboxed J string name for indexed file foreign x;y argument pairs."""
    return "'" + str(path).replace("'", "''") + "'"


def j_file_argument(path: Path) -> str:
    """Box only whole-file foreign arguments (1!:1 / 1!:4)."""
    return "<" + j_file_name(path)


def _as_json_bytes(value):
    return None if value is None else value.hex()


def _probe_one(oracle: Oracle, probe: Probe, directory: Path) -> dict:
    path = directory / (probe.name + ".dat")
    if probe.initial is not None:
        path.write_bytes(probe.initial)
    expression = probe.j.format(file=j_file_argument(path), name=j_file_name(path))
    outcome = oracle.eval(expression) if probe.inspect == "eval" else oracle.run(expression)
    exists = path.exists()
    data = path.read_bytes() if exists else None
    observed_error = isinstance(outcome, dict) and "error" in outcome
    expected_noun = probe.noun
    checks = {
        "error_presence": observed_error == probe.expect_error,
        "file_exists": exists == (probe.initial is not None),
    }
    if probe.expected_error is not None:
        checks["error_class"] = isinstance(outcome, dict) and outcome.get("error") == probe.expected_error
    elif probe.expect_error:
        # A malformed J foreign argument yields length/rank/syntax error too.
        # Merely seeing *some* error is not proof a file access was attempted.
        checks["not_argument_or_syntax_failure"] = (
            isinstance(outcome, dict) and
            outcome.get("error") not in {"length error", "rank error", "syntax error"}
        )
    if not probe.expect_error and expected_noun is not None:
        checks["noun_value"] = outcome == expected_noun
    if probe.after is not None:
        checks["file_content"] = data == probe.after
    if probe.after_size is not None:
        checks["file_length"] = data is not None and len(data) == probe.after_size
    if probe.after_prefix is not None:
        checks["file_prefix"] = data is not None and data.startswith(probe.after_prefix)
    if probe.after_suffix is not None:
        checks["file_suffix"] = data is not None and data.endswith(probe.after_suffix)
    return {
        "case": probe.name,
        "j": expression.replace(str(path), "<temporary-file>"),
        "outcome": outcome,
        "file_after_hex": _as_json_bytes(data),
        "checks": checks,
        "status": "matches_source_expectation" if all(checks.values()) else "requires_review",
    }


def run(library: Path, reference_revision: str, report: Path) -> dict:
    if not library.is_file():
        raise FileNotFoundError(f"missing pinned J C library: {library}")
    # The build-reference manifest prevents accidental results from an unrelated
    # libj.so. A hash alone identifies bytes, not the provenance of a build.
    variant = library.parent.name
    manifest_path = ROOT / ".reference" / ("manifest-" + variant + ".json")
    if not manifest_path.is_file():
        raise RuntimeError(f"missing pinned reference manifest: {manifest_path}")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("commit") != reference_revision:
        raise RuntimeError(f"reference revision mismatch: {manifest.get('commit')} != {reference_revision}")
    report_data = {
        "scope": "J C file-foreign diagnostic and ordered-effect probes only; no RustJ sync/optimized path",
        "acceptance": "non-acceptance: no independent Rust implementation or mapped/Jd oracle",
        "reference_revision": reference_revision,
        "reference_variant": variant,
        "reference_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
        "cases": [],
        "ordered_effect_cases": [],
    }
    oracle = Oracle()
    try:
        with tempfile.TemporaryDirectory(prefix="rustj-file-io-oracle-") as d:
            for probe in cases():
                report_data["cases"].append(_probe_one(oracle, probe, Path(d)))
            for case in ordered_cases():
                report_data["ordered_effect_cases"].append(
                    _run_ordered_case(oracle, case, Path(d)))
    finally:
        oracle.close()
    statuses = [item["status"] for item in report_data["cases"]]
    report_data["checked_cases"] = len(statuses)
    report_data["matching_source_expectation"] = statuses.count("matches_source_expectation")
    report_data["requires_review"] = statuses.count("requires_review")
    ordered_statuses = [item["status"] for item in report_data["ordered_effect_cases"]]
    report_data["ordered_checked_cases"] = len(ordered_statuses)
    report_data["ordered_matching"] = ordered_statuses.count("matches_source_expectation")
    report_data["ordered_requires_review"] = ordered_statuses.count("requires_review")
    report_data["ordered_checked_steps"] = sum(
        len(item["steps"]) for item in report_data["ordered_effect_cases"])
    errors = {c["case"]: c["outcome"].get("error") for c in report_data["cases"]
              if isinstance(c["outcome"], dict) and c["outcome"].get("error")}
    report_data["observed_error_labels"] = errors
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(report_data, indent=2, ensure_ascii=False) + "\n",
                      encoding="utf-8")
    return report_data


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference-revision", required=True)
    parser.add_argument("--report", type=Path, default=ROOT / "reports/file-io-audit.json")
    parser.add_argument("--library", type=Path, default=Path(os.environ.get(
        "J_LIBRARY", ROOT / ".reference/bin/linux/j64/libj.so")))
    parser.add_argument("--gate", action="store_true",
                        help="fail on expectation mismatch, after pin-specific behavior is validated")
    args = parser.parse_args()
    try:
        result = run(args.library.resolve(), args.reference_revision, args.report)
    except (RuntimeError, OSError, ValueError) as error:
        parser.exit(1, f"IO C-oracle setup/execution failed: {error}\n")
    print(json.dumps({k: result[k] for k in (
        "reference_revision", "reference_variant", "checked_cases",
        "matching_source_expectation", "requires_review", "ordered_checked_cases",
        "ordered_matching", "ordered_requires_review", "ordered_checked_steps")}, indent=2))
    # Keep failing case details visible in CI logs even without downloading a ZIP.
    # All filesystem paths are already scrubbed by _probe_one.
    for item in result["cases"]:
        if item["status"] == "requires_review":
            print(json.dumps({key: item[key] for key in
                  ("case", "outcome", "checks", "file_after_hex")},
                  ensure_ascii=False))
    for case in result["ordered_effect_cases"]:
        if case["status"] == "requires_review":
            print(json.dumps({key: case[key] for key in ("case", "steps")},
                             ensure_ascii=False))
    # This gates only the C-oracle fixtures, never RustJ implementation coverage.
    return int(args.gate and (
        result["requires_review"] > 0 or result["ordered_requires_review"] > 0))


if __name__ == "__main__":
    raise SystemExit(main())
