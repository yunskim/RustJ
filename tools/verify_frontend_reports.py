"""Verify current Windows frontend audit evidence against its recorded files.

Known gaps stay visible; file integrity is not language conformance.
Run after rebuilding the probes and regenerating these reports.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path


REPORTS = [
    "name-abandon-windows", "string-assignment-windows", "definition-calls-windows",
    "definition-loops-windows", "definition-nested-windows", "definition-name-scopes-windows",
    "frontend-contract-audit-windows", "name-effects-windows", "numeric-overflow-windows",
    "numeric-syntax-j64-windows", "numeric-syntax-avx2-windows", "numeric-integer-dtype-windows",
    "numeric-scientific-windows",
    "numeric-ratio-windows",
    "numeric-extended-windows",
]


def checked_hash(root, relative, expected):
    # Constrain recorded paths lexically; installed reference DLLs may be
    # trusted symlinks into the versioned asset cache outside this directory.
    root = Path(os.path.abspath(root))
    path = Path(os.path.abspath(root / relative.replace("\\", "/")))
    if not path.is_relative_to(root):
        raise ValueError(f"Report path escapes its evidence root: {relative}")
    if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
        raise ValueError(f"Evidence hash mismatch: {path}")
    return 1


def verify_report(root, assets, path):
    report = json.loads(path.read_text(encoding="utf-8"))
    checks = 0
    for field in ["rust_source_sha256", "audit_source_sha256"]:
        for relative, expected in report.get(field, {}).items():
            checks += checked_hash(root, relative, expected)
    for relative, expected in report.get("source_hashes", {}).items():
        checks += checked_hash(assets / "target/jref", relative, expected)
    reference = report["reference_sha256"]
    if isinstance(reference, str):
        dll = "javx2.dll" if path.stem == "numeric-syntax-avx2-windows" else "j.dll"
        checks += checked_hash(assets / "target/cj-windows/j64", dll, reference)
        checks += checked_hash(root, "target/debug/examples/frontend_probe.exe", report["binary_sha256"])
    else:
        for dll, expected in reference.items():
            checks += checked_hash(assets / "target/cj-windows/j64", dll, expected)
        if "rust_binary_sha256" in report:
            binary, expected = "target/debug/rustj.exe", report["rust_binary_sha256"]
        else:
            probe = "name_effect_probe" if path.stem == "name-effects-windows" else "frontend_contract_probe"
            binary, expected = f"target/debug/examples/{probe}.exe", report["probe_sha256"]
        checks += checked_hash(root, binary, expected)
    if report.get("failed", 0):
        raise ValueError(f"Audit records failures: {path}")
    for counts in report.get("stage_admission_counts", {}).values():
        if any(counts.get(category, 0) for category in ["verifier-defect", "backend-failure"]):
            raise ValueError(f"Internal stage failure: {path}")
    counts = report.get("counts", {})
    if set(counts) - {"matched", "unsupported_gap", "runtime_gap"}:
        raise ValueError(f"Unrecognized audit status: {path}")
    if counts and sum(counts.values()) != report["observations"]:
        raise ValueError(f"Incomplete observation counts: {path}")
    if report.get("fixture_set") in {"numeric-overflow", "integer-dtype", "scientific", "real-ratio", "extended-integer"} and set(counts) != {"matched"}:
        raise ValueError(f"Strict numeric audit contains gaps: {path}")
    return checks


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets-root", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    checks = 0
    for name in REPORTS:
        checks += verify_report(args.repo_root, args.assets_root, args.repo_root / "reports" / (name + ".json"))
    print(f"Verified {len(REPORTS)} reports and {checks} source/binary/DLL hashes; known gaps remain gaps")


if __name__ == "__main__":
    main()
