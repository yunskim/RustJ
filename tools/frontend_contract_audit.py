"""Bounded frontend admission/handoff/error review, not a full conformance gate.

F/H/P/G/L are independent read-only inspections (frontend / handoff / binding / Graph / Logical). R is actual captured execution.
Only R and post-error state are compared with C; a stage rejection is not a J error.
"""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from oracle import Oracle

CASES = [
    ("array_expression", ["a=:1 2 3"], "a+2*3", []),
    ("late_verb", ["f=:+", "g=:f", "f=:*"], "g 3", []),
    ("computed_rank", [], '+/"(1+0) i.2 3', []),
    ("nonfinal_assignment", ["a=:1"], "a+a=:2", ["a"]),
    ("chained_assignment", [], "a=:b=:1", ["a", "b"]),
    ("computed_target", [], "'a'=:7", ["a"]),
    ("multiple_target", [], "'a b'=:3 4", ["a", "b"]),
    ("direct_locative", [], "a_base_=:7", ["a"]),
    ("indirect_locative", ["loc=:<'base'", "a=:7"], "a__loc", []),
    ("abandon_name", ["a=:7"], "a_:", ["a"]),
    ("abandon_verb_transfer", ["f=:+"], "g=:f_:", ["f"]),
    ("abandon_modifier_transfer", ["adv=:/"], "saved=:adv_:", ["adv"]),
    ("abandon_explicit_conjunction", ["c=:2 : 'u@:v'"], "h=:-c_:+", ["c"]),
    ("complex_literal", [], "1j2", []),
    ("extended_literal", [], "123x", []),
    ("rational_literal", [], "2r3", []),
    ("overflow_literal", [], "9223372036854775808", []),
    ("core_conjunction", [], "f=:+&2", []),
    ("modifier_value", [], "adv=:/", []),
    ("execute", [], '\". \'1+2\'', []),
    ("direct_definition", [], "f=:{{y+1}}", []),
    ("explicit_definition", [], "f=:3 : 'y+1'", []),
    ("nested_definition", [], "f=:{{\ninner=.{{y+1}}\ninner y\n}}", []),
    ("select_preparse", [], "f=:3 : 'select. y case. 1 do. 10 case. do. 20 end.'", []),
    ("syntax_failure", [], "1+)", []),
    ("domain_failure", [], "'a'+1", []),
    ("first_error", [], "missing+(1 2+1 2 3)", []),
    ("failed_assignment", ["a=:7"], "a=:1 2+1 2 3", ["a"]),
    ("effect_before_error", ["a=:1"], "(1 2+1 2 3)+a=:9", ["a"]),
    ("definition_error_location", ["f=:{{y+1 2 3}}"], "f 1 2", []),
    ("nested_error_location", ["inner=:{{y+1 2 3}}", "outer=:{{inner y}}"], "outer 1 2", []),
    ("caught_error_effect", ["count=:0", "f=:{{\ntry.\ncount=:count+1\n1 2+1 2 3\ncatch.\ncount\nend.\n}}"], "f 0", ["count"]),
    ("unsupported_not_caught", ["f=:{{try. +&2 y catch. 42 end.}}"], "f 3", []),
    ("invalid_control_redefinition", ["f=:42"], "f=:{{if. y do. y}}", ["f"]),
    ("definition_return_explicit", ["f=:3 : '+'"], "f 0", []),
    ("definition_return_direct", ["f=:{{+}}"], "f 0", []),
    ("definition_admission_valence", ["f=:4 : 'x+y'"], "f 0", []),
    ("definition_return_post_effect", ["count=:0", "saved=:99", "f=:3 : 'count=:count+1\ntry. local=.+ catch. 42 end.'"], "saved=:f 0", ["count", "saved"]),
]


def probe(binary, setup, source, after):
    operations = [("E", s) for s in setup]
    operations += [(stage, source) for stage in ("F", "H", "P", "G", "L", "R")]
    operations += [("E", s) for s in after]
    data = "".join(f"{stage} {s.encode().hex()}\n" for stage, s in operations)
    result = subprocess.run([str(binary)], input=data, text=True, encoding="utf-8",
                            capture_output=True, timeout=30, check=True)
    records = [json.loads(line) for line in result.stdout.splitlines()]
    if len(records) != len(operations):
        raise RuntimeError("Incomplete audit stream")
    return records[:len(setup)], dict(zip(("F", "H", "P", "G", "L", "R"), records[len(setup):len(setup)+6])), records[len(setup)+6:]


def semantic_outcome(outcome):
    return {"error": outcome["error"]} if "error" in outcome else outcome


INSPECTION_STAGES = {"F": "frontend", "H": "frontend-handoff", "P": "semantic-binding", "G": "j-graph", "L": "logical"}


def inspection_outcome(stage, value):
    if value.get("inspection_stage") != INSPECTION_STAGES[stage] or value.get("execution_performed") is not False:
        raise ValueError("Inspection stage must not claim execution")
    if "error" not in value:
        if value.get("ok") is not True:
            raise ValueError("Missing representation admission result")
        return "accepted-representation"
    if value.get("j_handler_eligible") is not False:
        raise ValueError("Read-only inspection cannot enter a J handler")
    category = value.get("category")
    if category not in {"j-language", "unsupported-capability", "verifier-defect", "backend-failure"}:
        raise ValueError("Unknown failure category")
    return category


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--assets-root", type=Path, required=True)
    p.add_argument("--probe", type=Path, required=True)
    p.add_argument("--report", type=Path, required=True)
    args = p.parse_args()
    if sys.platform != "win32":
        p.error("Native Windows audit only")
    rust = {}
    cases = [(name, setup, source, [s + "+0" for s in after]) for name, setup, source, after in CASES]
    for name, setup, source, after in cases:
        setup_rust, stages, post = probe(args.probe, setup, source, after)
        for value in stages.values():
            for key in ("context_hex", "render_hex", "names_hex", "capture_hex", "requirements_hex"):
                if key in value:
                    value[key.removesuffix("_hex")] = bytes.fromhex(value.pop(key)).decode()
            result = value.get("result", {})
            for key in ("context_hex", "render_hex"):
                if key in result:
                    result[key.removesuffix("_hex")] = bytes.fromhex(result.pop(key)).decode()
        for stage in INSPECTION_STAGES:
            inspection_outcome(stage, stages[stage])
        rust[name] = (setup_rust, stages, post)
    comparisons = []
    for variant in ("j.dll", "javx2.dll"):
        os.environ["J_LIBRARY"] = str(args.assets_root.resolve() / "target/cj-windows/j64" / variant)
        for name, setup, source, after in cases:
            oracle = Oracle()
            try:
                setup_results = [oracle.eval(s) for s in setup]
                if name in {"complex_literal", "extended_literal", "rational_literal"}:
                    # The noun bridge does not serialize these families. Record
                    # C acceptance/type explicitly, never invent a value match.
                    reference = oracle.run("contractvalue=: " + source)
                    if reference is None:
                        reference = {"accepted_noun_type": oracle.eval("3!:0 contractvalue")["data"][0]}
                else:
                    reference = oracle.eval(source)
                post = [oracle.eval(s) for s in after]
            finally:
                oracle.close()
            setup_rust, stages, rust_post = rust[name]
            actual = stages["R"].get("result", stages["R"])
            matches = ([semantic_outcome(x) for x in setup_rust] == setup_results and
                       semantic_outcome(actual) == reference and
                       [semantic_outcome(x) for x in rust_post] == post)
            comparisons.append({"case": name, "variant": variant, "source": source,
                                "setup": setup, "setup_reference": setup_results, "setup_rust": setup_rust,
                                "reference": reference, "stages": stages,
                                "post_sources": after, "post_reference": post, "post_rust": rust_post,
                                "status": "matched" if matches else "runtime_gap"})
    report = {"scope": "Bounded admission, handoff and error audit; gaps are findings, not passes. C diagnostic locations are not compared.",
              "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
              "rust_source_sha256": {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                                     for p in sorted(Path("src").rglob("*.rs"))},
              "probe_sha256": hashlib.sha256(args.probe.read_bytes()).hexdigest(),
              "audit_source_sha256": {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                                      for p in [Path(__file__), Path("examples/frontend_contract_probe.rs")]},
              "reference_sha256": {name: hashlib.sha256((args.assets_root / "target/cj-windows/j64" / name).read_bytes()).hexdigest()
                                   for name in ("j.dll", "javx2.dll")},
              "source_revision": "13994ffa1ed5f06f79fad6e9822a7ed2d29b1528",
              "reference_revision": "ded7793fe5795d79eda8e7138dce94aa056edf78",
              "cases": len(CASES), "observations": len(comparisons),
              "counts": dict(Counter(r["status"] for r in comparisons)),
              "stage_admission_counts": {stage: dict(Counter(inspection_outcome(stage, values[1][stage]) for values in rust.values())) for stage in INSPECTION_STAGES},
              "records": comparisons}
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2)+"\n", encoding="utf-8")
    print(json.dumps({key: report[key] for key in ("cases", "observations", "counts", "stage_admission_counts")}))
    if any(category in {"verifier-defect", "backend-failure"} for counts in report["stage_admission_counts"].values() for category in counts):
        raise SystemExit("Internal stage failure must not be classified as a runtime gap")


if __name__ == "__main__":
    main()
