#!/usr/bin/env python3
"""IO-25: pinned JMF bootstrap/mapping smoke, distinct from RustJ I/O support.

Runs under the separately built J C engine. This is NON-ACCEPTANCE until
the jlibrary bootstrap and original JMF mapping behavior are demonstrated.
It uses a temporary HOME and temporary mapped file, not the caller's data.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile

from file_io_audit import j_file_name
from oracle import Oracle

ROOT = Path(__file__).resolve().parents[1]
EMPTY_TRUE = {"type": 1, "shape": [], "data": [1]}


def jmf_expressions(name: str) -> tuple[str, ...]:
    """Separate RW/RO/COW mapping checks; none implies boxed payload support."""
    steps = [f"rustj_jmf_file=: <{name}",
             "createjmf_jmf_ rustj_jmf_file,<4096"]
    for kind, mode in (("rw", 0), ("ro", 1), ("cow", 2)):
        var = "rustj_jmf_" + kind
        steps.extend((
            f"map_jmf_ (<'{var}'),rustj_jmf_file,'';{mode}",
            f"'' -: {var}",
            f"unmap_jmf_ '{var}'",
        ))
    return tuple(steps)


def run(library: Path, revision: str, report_path: Path) -> dict:
    if not library.is_file():
        raise FileNotFoundError(f"missing pinned C library: {library}")
    variant = library.parent.name
    manifest_path = ROOT / ".reference" / ("manifest-" + variant + ".json")
    if not manifest_path.is_file():
        raise RuntimeError(f"missing pinned reference manifest: {manifest_path}")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("commit") != revision:
        raise RuntimeError("J C build manifest revision does not match requested pin")

    jlibrary = ROOT / ".reference" / "jlibrary"
    profile = jlibrary / "bin" / "profile.ijs"
    jmf = jlibrary / "addons" / "data" / "jmf" / "jmf.ijs"
    if not profile.is_file() or not jmf.is_file():
        raise FileNotFoundError("pinned J system profile or JMF add-on missing")

    result = {
        "scope": "pinned C JMF smoke only; no RustJ I/O implementation/boxed support",
        "acceptance": "non-acceptance; JMF loader and selected empty mapped-array cases only",
        "reference_revision": revision,
        "reference_variant": variant,
        "reference_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
        "stages": [],
        "status": "blocked",
    }
    with tempfile.TemporaryDirectory(prefix="rustj-jmf-audit-") as d:
        root = Path(d)
        backing_file = root / "array.jmf"
        previous_home = os.environ.get("HOME")
        previous_user = os.environ.get("USER")
        os.environ["HOME"] = str(root)
        os.environ["USER"] = "rustj-test-nonroot"
        oracle = None
        try:
            oracle = Oracle()
            startup = (
                f"BINPATH_z_=: {j_file_name(library.parent)}",
                f"0!:0 <{j_file_name(profile)}",
                # Verify the loaded stdlib actually provides ordinary J's loader.
                "4!:0 <'load'",
                "load 'jmf'",
            )
            mapped = jmf_expressions(j_file_name(backing_file))
            for index, expr in enumerate((*startup, *mapped)):
                kind = ("check" if index == 2 or expr.startswith("'' -: ")
                        or expr.startswith("unmap_jmf_ ") else "run")
                output = oracle.eval(expr) if kind == "check" else oracle.run(expr)
                if index == 2:  # 4!:0 <'load' should resolve to a function, not -1.
                    good = (isinstance(output, dict) and output.get("type") == 4
                            and output.get("shape") == []
                            and output.get("data", [-1])[0] != -1)
                elif expr.startswith("'' -: "):
                    good = output == EMPTY_TRUE
                elif expr.startswith("unmap_jmf_ "):
                    # JMF unmap returns 0 for successful unmap; JDo success alone
                    # would incorrectly accept its nonzero failure codes.
                    good = output == {"type": 4, "shape": [], "data": [0]}
                else:
                    good = output is None  # Oracle.run returns None on J success.
                result["stages"].append({
                    "name": ["binpath", "profile", "load_available", "jmf_load"][index]
                            if index < 4 else expr.split(" ")[0],
                    "expression": expr.replace(str(root), "<temporary-root>"),
                    "outcome": output,
                    "matches_source_expectation": good,
                })
                if not good:
                    result["status"] = "blocked"
                    result["blocker"] = ("bootstrap_or_addon" if index < 4
                                         else "jmf_execution_or_expectation")
                    # Capture J's detailed failure context *before* other
                    # evaluation can replace it. This is a diagnostic, never
                    # evidence that the mapped-array semantics failed.
                    if isinstance(output, dict) and "error" in output:
                        try:
                            result["j_error_context"] = oracle.eval("13!:12''")
                        except (ValueError, RuntimeError) as error:
                            result["j_error_context_unavailable"] = str(error)
                    break
            else:
                result["status"] = "observed_smoke_match"
        finally:
            if oracle is not None:
                oracle.close()
            if previous_home is None:
                os.environ.pop("HOME", None)
            else:
                os.environ["HOME"] = previous_home
            if previous_user is None:
                os.environ.pop("USER", None)
            else:
                os.environ["USER"] = previous_user

    result["stage_count"] = len(result["stages"])
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--reference-revision", required=True)
    p.add_argument("--report", type=Path, default=ROOT / "reports/jmf-smoke.json")
    p.add_argument("--library", type=Path, default=Path(os.environ.get(
        "J_LIBRARY", ROOT / ".reference/bin/linux/j64/libj.so")))
    p.add_argument("--gate", action="store_true", help="gate pinned C/JMF bootstrap only")
    a = p.parse_args()
    try:
        r = run(a.library.resolve(), a.reference_revision, a.report)
    except (OSError, RuntimeError, ValueError) as error:
        p.exit(1, f"JMF smoke setup failure (not a J semantic result): {error}\n")
    print(json.dumps({k: r.get(k) for k in (
        "reference_revision", "reference_variant", "status",
        "stage_count", "blocker")}, indent=2))
    if r["status"] != "observed_smoke_match":
        print(json.dumps(r["stages"][-1], ensure_ascii=False))
    return int(a.gate and r["status"] != "observed_smoke_match")


if __name__ == "__main__":
    raise SystemExit(main())
