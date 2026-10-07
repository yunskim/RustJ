"""Compare bounded ordered NAME execution (including post-state) on Windows."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from name_compatibility_audit import compare_trace
from oracle import Oracle

# Setup/check are ordinary execution; exactly one marked sentence uses the new
# plan route. No Unsupported fallback is permitted in the audit adapter.
FIXTURES = [
    ("noun_take", ["a=:7"], "a_:", ["a+0"]),
    ("group_take", ["a=:7"], "(a_:)", ["a+0"]),
    ("right_to_left", ["a=:7"], "a_:+a", ["a+0"]),
    ("deleted_later_lookup", ["a=:7"], "a+a_:", ["a+0"]),
    ("deleted_before_failed_write", ["a=:7", "b=:99"], "b=:a+a_:", ["a+0", "b"]),
    ("earlier_length", ["a=:7"], "a_:+1 2+1 2 3", ["a+0"]),
    ("retained_delete_domain", ["a=:7"], "a_: + 'x'", ["a+0"]),
    ("take_write", ["a=:7"], "b=:a_:+a", ["a+0", "b"]),
    ("target_suffix", [], "a_:=:9", ["a"]),
    ("array_alias", ["a=:i.6", "b=:a"], "c=:a_: + b", ["a+0", "b", "c"]),
    ("reshape", ["a=:i.6"], "b=:2 3$a_:", ["a+0", "b"]),
    ("search", ["a=:3 1 4"], "a_: i. 1 9", ["a+0"]),
    ("verb_transfer", ["f=:+"], "saved=:f_:", ["f 0", "saved 3"]),
    ("adverb_transfer", ["f=:/"], "saved=:(f_:)", ["f 0", "+saved 1 2 3"]),
    ("conjunction_transfer", ["f=:@:"], "saved=:(f_:)", ["f 0", "h=:-saved+", "h 3"]),
    ("explicit_verb_transfer", ["f=:3 : 'y+1'"], "saved=:f_:", ["f 0", "saved 3"]),
    ("explicit_adverb_transfer", ["f=:1 : 'u/'"], "saved=:f_:", ["f 0", "+saved 1 2 3"]),
    ("explicit_conjunction_transfer", ["f=:2 : 'u@:v'"], "saved=:f_:", ["f 0", "h=:-saved+", "h 3"]),
    ("late_inner_alias", ["base=:+", "alias=:base"], "saved=:alias_:", ["base=:-", "saved 3"]),
]


def audit(assets, probe):
    records = []
    for variant in ("j.dll", "javx2.dll"):
        os.environ["J_LIBRARY"] = str(assets / "target/cj-windows/j64" / variant)
        for name, setup, effect, after in FIXTURES:
            sources = setup + [effect] + after
            oracle = Oracle()
            try:
                reference = [oracle.eval(source) for source in sources]
            finally:
                oracle.close()
            for mode, route in [("effect", "ordered-semantic"), ("array", "ordered-logical")]:
                commands = (["eval\t" + source for source in setup] + [mode + "\t" + effect]
                            + ["eval\t" + source for source in after])
                process = subprocess.run([str(probe.resolve())], input="\n".join(commands) + "\n",
                                         text=True, capture_output=True, check=True)
                actual = [json.loads(line) for line in process.stdout.splitlines()]
                status, differences = compare_trace(sources, reference, actual)
                records.append(dict(case=name, variant=variant, route=route, sources=sources, effect_index=len(setup),
                                    reference=reference, rust=actual, status=status, differences=differences))
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets-root", type=Path, required=True)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "win32":
        parser.error("Requires native Windows Python, Rust probe and C DLLs")
    records = audit(args.assets_root.resolve(), args.probe)
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    report = dict(scope="Bounded top-level ordered NAME plan execution; setup/check use ordinary execution; no replay/fallback",
                  revision=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                  platform=sys.platform, fixtures=len(FIXTURES), observations=len(records),
                  counts=dict(Counter(record["status"] for record in records)), records=records,
                  probe_sha256=sha(args.probe),
                  rust_source_sha256={str(path): sha(path) for path in sorted(Path("src").rglob("*.rs"))},
                  audit_source_sha256={str(path): sha(path) for path in [Path(__file__),
                      Path("examples/name_effect_probe.rs"), Path("tools/oracle.py"),
                      Path("tools/name_compatibility_audit.py"), Path("Cargo.toml"), Path("Cargo.lock")]},
                  source_revision="13994ffa1ed5f06f79fad6e9822a7ed2d29b1528",
                  reference_revision="ded7793fe5795d79eda8e7138dce94aa056edf78",
                  reference_sha256={name: sha(args.assets_root / "target/cj-windows/j64" / name)
                                    for name in ("j.dll", "javx2.dll")})
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({key: report[key] for key in ("fixtures", "observations", "counts")}))
    return int(any(record["status"] != "matched" for record in records))


if __name__ == "__main__":
    raise SystemExit(main())
