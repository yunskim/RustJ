"""Probe native Windows C NAME semantics; this is not RustJ conformance."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys

from oracle import Oracle

ROOT = Path(__file__).resolve().parents[1]
CASES = [
    ("noun_snapshot", ["a=:1", "b=:a", "a=:2"], "b", "1", None),
    ("verb_late_lookup", ["f=:+", "g=:f", "f=:*"], "g 3", "3-2", None),
    ("undefined_function_later_defined", ["g=:later", "later=:-"], "g 3", "_3", None),
    ("expected_pos_mismatch", ["f=:+", "g=:f", "f=:7"], "g 3", None, "domain error"),
    ("sentence_lookup_order", ["a=:1"], "a + a=:2", "4", None),
    ("direct_locative_call", ["f_probe_=:+", "g=:f_probe_", "f_probe_=:*"], "g 3", "3-2", None),
    ("fixed_function", ["f=:+", "g=:f f.", "f=:*"], "g 3", "3", None),
    ("noun_abandon", ["a=:7"], "a_:", "7", None),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets-root", type=Path, default=ROOT,
                        help="Checkout containing target/jref and target/cj-windows")
    parser.add_argument("--output", type=Path,
                        default=ROOT / "reports/name-system-research-windows.json")
    args = parser.parse_args()
    if sys.platform != "win32":
        parser.error("Use native Windows Python and native J DLLs.")
    assets = args.assets_root.resolve()
    records = []
    for variant in ["j.dll", "javx2.dll"]:
        os.environ["J_LIBRARY"] = str(assets / "target/cj-windows/j64" / variant)
        for name, setup, query, expected, error in CASES:
            oracle = Oracle()
            try:
                for line in setup:
                    result = oracle.run(line)
                    if result is not None:
                        raise AssertionError((variant, name, line, result))
                actual = oracle.eval(query)
                wanted = {"error": error} if error else oracle.eval(expected)
                if actual != wanted:
                    raise AssertionError((variant, name, actual, wanted))
                if name == "noun_abandon":
                    if oracle.name_class("a") != oracle.name_class("neverbound"):
                        raise AssertionError("Abandoned noun remains bound")
                records.append({"variant": variant, "case": name, "setup": setup,
                                "query": query, "actual": actual, "passed": True})
            finally:
                oracle.close()
    report = {
        "platform": sys.platform,
        "scope": "C name semantics research; not RustJ conformance or parallel execution",
        "source_revision": "13994ffa1ed5f06f79fad6e9822a7ed2d29b1528",
        "reference_revision": "ded7793fe5795d79eda8e7138dce94aa056edf78",
        "revision_note": "Recorded research asset revisions; SHA256 identifies actual supplied files.",
        "source_sha256": {f: hashlib.sha256((assets / "target/jref/jsrc" / f).read_bytes()).hexdigest()
                          for f in ["p.c", "sc.c", "s.c", "sn.c", "cx.c", "jtype.h"]},
        "reference_sha256": {f: hashlib.sha256((assets / "target/cj-windows/j64" / f).read_bytes()).hexdigest()
                             for f in ["j.dll", "javx2.dll"]},
        "cases": len(records), "passed": len(records), "records": records,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print("C_NAME_RESEARCH_PASSED", len(records))


if __name__ == "__main__":
    main()
