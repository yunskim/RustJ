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

# Boundary probes pin observable runtime behavior. They do not test a future
# admission verifier or parser continuation and must never be reported as such.
BOUNDARY_FIXTURES = [
    ("pure_array_region", ["a=:i.3", "b=:a+a*a", "b", "a"]),
    ("shape_unknown_is_not_a_parse_demand", ["n=:4", "i.n"]),
    ("literal_rank_constructor", ['(+/"1) i.2 3']),
    ("computed_rank_constructor", ['(+/"(1+0)) i.2 3']),
    ("assignment_then_name_read", ["a=:1", "a+a=:2", "a"]),
    ("modifier_write_then_left_read", ["a=:1", "adv=:1 : 'a=:u'", "a+2 adv", "a"]),
    ("right_error_precedes_missing_left_name", ["missing+(1 2+1 2 3)"]),
    ("effect_survives_later_error", ["count=:0", "adv=:1 : 'count=:count+u'",
                                   "(1 2+1 2 3)+5 adv", "count"]),
    ("dynamic_source_request", ['". \'1+2\'']),
    ("late_target_pos_change", ["f=:+", "g=:f", "f=:7", "g 3"]),
]

# Both definition spellings must preserve the same NAME timing and scope rules.
SCOPE_FIXTURES = [
    ("explicit_local_fallback", ["t=:10", "add=:1 : 't=.t+u'", "2 add", "3 add", "t"]),
    ("direct_local_fallback", ["t=:10", "add=:{{ t=.t+u }}", "2 add", "3 add", "t"]),
    ("explicit_local_function_escape", ["t=:+", "make=:1 : 0\nt=.u\nt/\n)",
                                        "res=:-make", "res 1 2 3", "t=:-", "res 1 2 3"]),
    ("direct_local_function_escape", ["t=:+", "make=:{{ t=.u\nt/ }}",
                                      "res=:-make", "res 1 2 3", "t=:-", "res 1 2 3"]),
    ("explicit_global_write", ["a=:10", "set=:1 : 'a=:u'", "3 set", "a"]),
    ("direct_global_write", ["a=:10", "set=:{{ a=:u }}", "3 set", "a"]),
    ("explicit_local_global_collision", ["a=:10", "bad=:1 : 0\na=.u\na=:7\na\n)", "3 bad", "a"]),
    ("direct_local_global_collision", ["a=:10", "bad=:{{ a=.u\na=:7\na }}", "3 bad", "a"]),
    ("top_level_local_copula_is_global", ["outside=.7", "outside"]),
    ("callee_does_not_capture_caller_local", ["f=:+", "inner=:1 : 'u 7'",
                                             "outer=:{{ f=.u\nf inner }}", "-outer", "f 7"]),
]

# Only these assignment setup blocks need script input (JDo has no interactive
# block-input callback). Keep original source identical on both sides and never
# script-wrap value queries, whose noun results must remain observable.
DEFINITION_FIXTURES = [
    ("verb_direct", ["f=:{{ y+1 }}", "f 41", "f 1 2 3"]),
    ("verb_explicit", ["f=:3 : 'y+1'", "f 41"]),
    ("verb_dyad", ["f=:4 : 'x+y'", "2 f 3", "f 3"]),
    ("verb_sections", ["f=:3 : 0\ny+1\n:\nx+y\n)", "f 4", "2 f 3"]),
    ("verb_local_fallback", ["g=:10", "f=:3 : 'g=.g+y'", "f 2", "f 3", "g"]),
    ("verb_late_global", ["g=:10", "f=:3 : 'g+y'", "g=:20", "f 2"]),
    ("verb_late_function", ["op=:+", "f=:3 : 'op y'", "op=:-", "f 3"]),
    ("verb_snapshot", ["a=:i.4", "f=:3 : 0\nn=.y\ncopy=.n\nn=.n+10\ncopy\n)", "f a", "a"]),
    ("verb_effect", ["count=:0", "f=:3 : 0\ncount=:count+y\ncount\n)", "count", "f 2", "count"]),
    ("verb_frames", ["y=:99", "g=:3 : 'y'", "f=:3 : 'y+g y+1'", "f 2", "y"]),
    ("verb_private_caller", ["a=:10", "g=:3 : 'a+y'", "f=:3 : 0\na=.99\ng y\n)", "f 2", "a"]),
    ("verb_error_cleanup", ["y=:99", "f=:3 : 0\nt=.y\n1 2+1 2 3\n)", "f 2", "y", "t+0"]),
    ("verb_branch_return", ["f=:3 : 0\nif. y>0 do.\n42 return.\nelse.\n7 return.\nend.\n1 2+1 2 3\n)", "f 1", "f _1"]),
    ("verb_elseif", ["f=:3 : 0\nif. y=0 do.\n10\nelseif. y=1 do.\n20\nelse.\n30\nend.\n)", "f 0", "f 1", "f 2"]),
    ("verb_while", ["f=:3 : 0\nn=.y\nwhile. n>0 do.\nn=.n-1\nend.\nn\n)", "f 4", "f 0"]),
    ("verb_whilst", ["f=:3 : 0\nn=.y\nwhilst. n>0 do.\nn=.n-1\nend.\nn\n)", "f 4", "f 0"]),
    ("verb_break_continue", ["f=:3 : 0\nn=.0\ns=.0\nwhile. n<y do.\nn=.n+1\nif. n=2 do. continue. end.\nif. n=4 do. break. end.\ns=.s+n\nend.\ns\n)", "f 8"]),
    ("verb_first_atom_condition", ["f=:3 : 'if. y do. 42 else. 7 end.'", "f 0 1", "f 1 0", "f i.0", "f 'x'"]),
    ("verb_test_preserves_result", ["f=:3 : 0\n42\nif. y do. 7 end.\n)", "f 0", "f 1"]),
    ("verb_recursion", ["f=:3 : 0\nif. y=0 do. 1 return. end.\ny*f y-1\n)", "f 5", "f 3"]),
    ("verb_catch_error", ["f=:3 : 0\ntry.\n1 2+1 2 3\ncatch.\n42\nend.\n)", "f 0"]),
    ("verb_catch_skip", ["f=:3 : 'try. y+1 catch. 1 2+1 2 3 end.'", "f 4"]),
    ("verb_catch_condition", ["f=:3 : 'try. if. + do. 1 end. catch. 42 end.'", "f 0"]),
    ("verb_nested_catch", ["f=:3 : 'try. try. 1 2+1 2 3 catch. 1 2+1 2 3 end. catch. 42 end.'", "f 0"]),
    ("verb_catchd", ["f=:3 : 'try. 1 2+1 2 3 catchd. 42 end.'", "f 0"]),
    ("verb_catch_effect", ["g=:0", "f=:3 : 0\ntry.\ng=:y\n1 2+1 2 3\ncatch.\ng\nend.\n)", "f 7", "g"]),
    ("verb_catch_scope", ["f=:3 : 'try. y+1 catch. 42 end. 1 2+1 2 3'", "f 0"]),
    ("verb_empty_branch", ["f=:3 : 'if. y do. 42 end.'", "f 0"]),
    ("verb_empty_return", ["f=:3 : 'return. 42'", "f 0"]),
    ("verb_unbound_implicit_global_fallback", ["u=:10", "x=:10",
        "f=:3 : 'u+y'", "f 2", "f=:3 : 'x+y'", "f 2"]),
    ("frontend_e2e_demonstration", ["a=:1 2 3", "a+2*3", "g=:10",
        "explicit=:3 : 0\nt=.y+g\nt\n)", "direct=:{{ t=.y+g\nt }}",
        "explicit 2", "direct 2", "g=:20", "explicit 2", "direct 2",
        "pair=:4 : 'x+y'", "2 pair 3", "ddpair=:{{ x+y }}", "2 ddpair 3"]),
]

SCRIPT_SETUPS = {source for name, sources in SCOPE_FIXTURES
                 if name in {"explicit_local_function_escape", "explicit_local_global_collision"}
                 for source in sources if " : 0\n" in source}
SCRIPT_SETUPS.update(source for _, sources in DEFINITION_FIXTURES
                     for source in sources if " : 0\n" in source)


def reference_trace(oracle, sources):
    outcomes, transports = [], []
    for source in sources:
        script = source in SCRIPT_SETUPS
        outcomes.append((oracle.run_script(source) or {"silent": True})
                        if script else oracle.eval(source))
        transports.append("script" if script else "sentence")
    return outcomes, transports


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
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument("--boundary-fixtures-only", action="store_true")
    selection.add_argument("--scope-fixtures-only", action="store_true")
    selection.add_argument("--definition-fixtures-only", action="store_true")
    args = parser.parse_args()
    if sys.platform != "win32":
        parser.error("Use native Windows Python, J DLLs and Rust binary")
    assets = args.assets_root.resolve()
    fixtures = (DEFINITION_FIXTURES if args.definition_fixtures_only else
                BOUNDARY_FIXTURES if args.boundary_fixtures_only else
                SCOPE_FIXTURES if args.scope_fixtures_only else FIXTURES)
    records = []
    for variant in ["j.dll", "javx2.dll"]:
        os.environ["J_LIBRARY"] = str(assets / "target/cj-windows/j64" / variant)
        for name, sources in fixtures:
            oracle = Oracle()
            try:
                reference, transports = reference_trace(oracle, sources)
            finally:
                oracle.close()
            for route in ["direct", "semantic-reference"]:
                actual = rust_trace(args.binary, route, sources)
                status, differences = compare_trace(sources, reference, actual)
                records.append({"case": name, "variant": variant, "route": route,
                                "status": status, "sources": sources, "reference": reference,
                                "reference_transport": transports,
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
        "fixture_set": ("definition-calls" if args.definition_fixtures_only else
                        "semantic-boundaries" if args.boundary_fixtures_only else
                        "name-scopes" if args.scope_fixtures_only else "names"),
        "fixtures": len(fixtures), "observations": len(records), "counts": counts, "records": records,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"fixtures": len(fixtures), "observations": len(records), "counts": counts}))
    # Completing an audit is not a conformance pass. Counts always expose gaps.
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
