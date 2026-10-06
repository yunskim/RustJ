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


def cases() -> tuple[Probe, ...]:
    """Deterministic independent cases; every file begins in a fresh state."""
    return (
        Probe("full_read", "1!:1 {file}", inspect="eval",
              noun={"type": 2, "shape": [8], "data": list(INITIAL)}),
        Probe("file_size", "1!:4 {file}", inspect="eval",
              noun={"type": 4, "shape": [], "data": [8]}),
        Probe("partial_read", "1!:11 ({file};2 3)", inspect="eval",
              noun={"type": 2, "shape": [3], "data": list(b"CDE")}),
        Probe("negative_start_read", "1!:11 ({file};_2 2)", inspect="eval",
              noun={"type": 2, "shape": [2], "data": list(b"GH")}),
        Probe("zero_length_at_eof", "1!:11 ({file};8 0)"),
        Probe("zero_length_empty_file", "1!:11 ({file};0 0)", initial=b"", after=b""),
        Probe("read_past_eof", "1!:11 ({file};7 2)", expect_error=True),
        Probe("read_negative_length", "1!:11 ({file};2 _1)", expect_error=True),
        Probe("write_inside", "'xy' 1!:12 ({file};2)", after=b"ABxyEFGH"),
        Probe("write_outside_start", "'XY' 1!:12 ({file};_9)", expect_error=True),
        Probe("write_beyond_eof", "'XY' 1!:12 ({file};10)",
              after=None, after_size=12, after_prefix=INITIAL, after_suffix=b"XY"),
        Probe("missing_file", "1!:11 ({file};0 1)", initial=None, after=None, expect_error=True),
        Probe("missing_file_bad_range", "1!:11 ({file};9 2)", initial=None, after=None,
              expect_error=True),
        Probe("zero_length_missing_file", "1!:11 ({file};0 0)", initial=None, after=None,
              expect_error=True),
        Probe("discarded_missing_read", "0 [ (1!:11 ({file};0 1))",
              initial=None, after=None, expect_error=True),
    )


def j_file_argument(path: Path) -> str:
    """Box a file name as a J literal; never interpolate unescaped source."""
    return "<'" + str(path).replace("'", "''") + "'"


def _as_json_bytes(value):
    return None if value is None else value.hex()


def _probe_one(oracle: Oracle, probe: Probe, directory: Path) -> dict:
    path = directory / (probe.name + ".dat")
    if probe.initial is not None:
        path.write_bytes(probe.initial)
    expression = probe.j.format(file=j_file_argument(path))
    outcome = oracle.eval(expression) if probe.inspect == "eval" else oracle.run(expression)
    exists = path.exists()
    data = path.read_bytes() if exists else None
    observed_error = isinstance(outcome, dict) and "error" in outcome
    expected_noun = probe.noun
    checks = {
        "error_presence": observed_error == probe.expect_error,
        "file_exists": exists == (probe.initial is not None),
    }
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
        "scope": "J C file-foreign diagnostic only; no RustJ sync/optimized path",
        "acceptance": "non-acceptance: no independent Rust implementation or mapped/Jd oracle",
        "reference_revision": reference_revision,
        "reference_variant": variant,
        "reference_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
        "cases": [],
    }
    oracle = Oracle()
    try:
        with tempfile.TemporaryDirectory(prefix="rustj-file-io-oracle-") as d:
            for probe in cases():
                report_data["cases"].append(_probe_one(oracle, probe, Path(d)))
    finally:
        oracle.close()
    statuses = [item["status"] for item in report_data["cases"]]
    report_data["checked_cases"] = len(statuses)
    report_data["matching_source_expectation"] = statuses.count("matches_source_expectation")
    report_data["requires_review"] = statuses.count("requires_review")
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
        "matching_source_expectation", "requires_review")}, indent=2))
    # Diagnostic mode always reports mismatches but never claims acceptance.
    return int(args.gate and result["requires_review"] > 0)


if __name__ == "__main__":
    raise SystemExit(main())
