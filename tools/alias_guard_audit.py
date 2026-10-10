"""Finite native-Windows C comparison of the explicit Rust call-lease API."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from oracle import Oracle


FIXTURES = [(primitive, valence, x, y)
            for primitive in ["+", "-", "*", "%"]
            for valence, x, y in [
                ("monad", "", "1+i.6"),
                ("monad", "", "i.0"),
                ("dyad", "2 3$i.6", "2 3$1+i.6"),
                ("dyad", "3", "1+i.6"),
                ("dyad", "i.0", "i.0"),
                ("dyad", "i.2", "i.3"),
            ]] + [("+", mode, "", "") for mode in ["effect-miss", "noun-miss", "missing-miss"]]


def reference_probe(oracle, fixture):
    primitive, valence, x, y = fixture
    setup = [f"f=:{primitive}", "g=:f", "h=:g", "h 2"]
    if valence == "effect-miss":
        setup += ["count=:0", "change=:{{ count=:count+1\nf=:*\nu }}",
                  "argument=:_2 change"]
    elif valence in {"noun-miss", "missing-miss"}:
        setup += ["f=:7" if valence == "noun-miss" else "f=:later", "argument=:_2"]
    for source in setup:
        result = oracle.eval(source)
        if "error" in result:
            raise RuntimeError(f"C setup failed: {source!r}: {result}")
    query = ("h argument" if valence.endswith("-miss") else
             f"h ({y})" if valence == "monad" else f"({x}) h ({y})")
    # guard is a Rust-only assertion. C provides value/error/namespace outcomes.
    expected = {"guard": "miss" if valence.endswith("-miss") else "valid",
                "result": oracle.eval(query)}
    if valence == "effect-miss":
        expected["count"] = oracle.eval("count")
    return setup, query, expected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--rust-revision", required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "win32":
        parser.error("Use native Windows Python, J DLLs and Rust probe")
    records = []
    for variant in ["j.dll", "javx2.dll"]:
        library = args.assets_root.resolve() / "target/cj-windows/j64" / variant
        os.environ["J_LIBRARY"] = str(library)
        for fixture in FIXTURES:
            oracle = Oracle()
            try:
                setup, query, expected = reference_probe(oracle, fixture)
            finally:
                oracle.close()
            run = subprocess.run([str(args.binary), *fixture], capture_output=True,
                                 text=True, encoding="utf-8", timeout=30, check=True)
            actual = json.loads(run.stdout)
            records.append({"variant": variant, "fixture": fixture, "setup": setup,
                            "query": query, "expected": expected, "actual": actual,
                            "status": "matched" if expected == actual else "mismatch"})
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    counts = dict(Counter(record["status"] for record in records))
    report = {"scope": "Bounded explicit Rust alias-call lease versus C values/errors; guard outcomes are Rust-only assertions, not observed C events or compiler dispatch",
              "platform": sys.platform, "rust_revision": args.rust_revision,
              "rust_probe_sha256": sha(args.binary),
              "reference_revision": "ded7793fe5795d79eda8e7138dce94aa056edf78",
              "reference_sha256": {variant: sha(args.assets_root.resolve() / "target/cj-windows/j64" / variant)
                                   for variant in ["j.dll", "javx2.dll"]},
              "fixtures": len(FIXTURES), "observations": len(records), "counts": counts,
              "records": records}
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({key: report[key] for key in ["fixtures", "observations", "counts"]}))
    return int(counts.get("mismatch", 0) != 0)


if __name__ == "__main__":
    raise SystemExit(main())
