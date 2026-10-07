"""Bounded C/Rust NAME audit: record incompatibilities without counting them as passes."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from name_system_research import CASES
from oracle import Oracle

# Each fixture gets a fresh Engine and C instance. Every sentence, including
# setup and post-error reads, is compared. This does not observe internal events.
FIXTURES = [(name, setup + [query]) for name, setup, query, _, _ in CASES] + [
    ("array_alias_snapshot", ["a=:i.6", "b=:a", "a=:a+1", "a", "b"]),
    ("reshape_snapshot", ["a=:i.6", "b=:2 3$a", "a=:a+10", "b"]),
    ("failed_assignment_preserves_binding", ["a=:7", "a=:1 2+1 2 3", "a"]),
    ("verb_alias_chain", ["f=:+", "g=:f", "h=:g", "f=:*", "h 3"]),
    ("local_shadow", ["a=:7", "f=:3 : 'a=.9'", "f 0", "a"]),
    ("local_unbound_fallback", ["a=:7", "f=:3 : 'a=.a+1'", "f 0", "a"]),
    ("modifier_local_shadow", ["a=:7", "adv=:1 : 'a=.u+2'", "5 adv", "a"]),
    ("modifier_local_unbound_fallback", ["a=:7", "adv=:1 : 'a=.a+u'", "2 adv", "a"]),
    ("named_adverb_capture", ["a=:/", "f=:+a", "a=:\\", "f 1 2 3"]),
    ("nameless_adverb_alias", ["a=:/", "b=:a", "a=:\\", "+b 1 2 3"]),
    ("conjunction_alias", ["c=:@:", "d=:c", "c=:&", "(+d-) 3"]),
    ("base_locative", ["a=:7", "a__"]),
    ("indirect_locative_rebind", ["loc=:<'probe'", "f_probe_=:+", "f_other_=:*",
                                  "g=:f__loc", "loc=:<'other'", "g 3"]),
    ("locative_execution_context", ["a=:9", "a_probe_=:7", "f_probe_=:3 : 'a'", "f_probe_ 0"]),
    ("computed_single_assignment", ["'a'=:7", "a"]),
    ("multiple_assignment", ["'a b'=:3 4", "a", "b"]),
    ("undefined_function_call", ["g=:later", "g 3"]),
    ("invalid_name", ["foo_ 1"]),
]


def compare_trace(sources, expected, actual):
    if len(expected) != len(sources) or len(actual) > len(sources):
        raise RuntimeError("Incomplete observation stream")
    differences = [{"index": i, "source": source, "reference": c, "rust": r}
                   for i, (source, c, r) in enumerate(zip(sources, expected, actual)) if c != r]
    if not differences and len(actual) == len(sources):
        return "matched", differences
    if not differences:
        return "observation_gap", differences
    # Later results after failed setup are secondary. Classify by the first
    # divergence, but preserve the complete trace for review.
    first = differences[0]
    if first["rust"] == {"error": "unsupported"}:
        return "unsupported_gap", differences
    return "semantic_mismatch", differences


def rust_trace(binary, route, sources):
    command = [str(binary), "--json"]
    if route == "semantic-reference":
        command.append("--semantic-reference")
    result = subprocess.run(command, input="\n".join(sources) + "\n", text=True,
                            encoding="utf-8", capture_output=True, timeout=30)
    if result.returncode not in (0, 1):
        raise RuntimeError((command, result.returncode, result.stderr))
    return [json.loads(line) for line in result.stdout.splitlines()]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--rust-revision", required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "win32":
        parser.error("Use native Windows Python, J DLLs and Rust binary")
    assets = args.assets_root.resolve()
    records = []
    for variant in ["j.dll", "javx2.dll"]:
        os.environ["J_LIBRARY"] = str(assets / "target/cj-windows/j64" / variant)
        for name, sources in FIXTURES:
            oracle = Oracle()
            try:
                reference = [oracle.eval(source) for source in sources]
            finally:
                oracle.close()
            for route in ["direct", "semantic-reference"]:
                actual = rust_trace(args.binary, route, sources)
                status, differences = compare_trace(sources, reference, actual)
                records.append({"case": name, "variant": variant, "route": route,
                                "status": status, "sources": sources, "reference": reference,
                                "rust": actual, "differences": differences,
                                "unobserved_suffix": sources[len(actual):]})
    sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    counts = dict(Counter(record["status"] for record in records))
    report = {
        "scope": "Bounded observable sentence/value/error audit; not full NAME, event, Graph/A3 or compiled-route conformance",
        "platform": sys.platform, "rust_revision": args.rust_revision,
        "rust_binary_sha256": sha(args.binary),
        "source_revision": "13994ffa1ed5f06f79fad6e9822a7ed2d29b1528",
        "reference_revision": "ded7793fe5795d79eda8e7138dce94aa056edf78",
        "revision_note": "Recorded asset revisions; DLL hashes identify the actual oracle, not a same-source rebuild",
        "reference_sha256": {name: sha(assets / "target/cj-windows/j64" / name)
                             for name in ["j.dll", "javx2.dll"]},
        "fixtures": len(FIXTURES), "observations": len(records), "counts": counts, "records": records,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"fixtures": len(FIXTURES), "observations": len(records), "counts": counts}))
    # Completing an audit is not a conformance pass. Counts always expose gaps.
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
