"""Observe exact pinned J-C NAME/POS error traces without running RustJ.

This is a reference-only diagnostic, not a RustJ differential conformance gate.
Compare with RustJ only after the reference-first error has been pinned and read.
"""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
from unittest.mock import patch

from name_compatibility_audit import (
    SCRIPT_SETUPS, SCOPE_FIXTURES, reference_inputs, reference_trace,
    select_named_fixtures,
)
from oracle import Oracle


TARGET_CASES = (
    "named_locative_alias_pos_mismatch",
    "named_direct_definition_reparse_after_pos_restore",
    "named_direct_body_noun_before_call",
    "named_direct_body_rhs_nounifies_before_call",
    "named_function_pos_rebinding_first_error",
)

# Identify the exact diagnostic sentence (including its occurrence count), not
# merely the earliest error in a multi-sentence fixture. Setup can fail first,
# and the same source may be evaluated on both sides of a POS redefinition.
PROBE_LOCATORS = {
    "named_locative_alias_pos_mismatch": ("alias 4", 2),
    "named_direct_definition_reparse_after_pos_restore": ("call 4", 2),
    "named_direct_body_noun_before_call": ("call 4", 2),
    "named_direct_body_rhs_nounifies_before_call": ("call 4 + nounify 0", 1),
    "named_function_pos_rebinding_first_error": ("call 4 + nounify 0", 1),
}


def require_complete_reference_pair(libraries, metadata):
    """Refuse a first-error verdict when one pinned J-C variant is unobserved."""
    variants = [name for name, _ in libraries]
    missing = metadata.get("unrun_reference_variants", [])
    supported_pairs = (
        {"j64", "j64avx2"}, {"j.dll", "javx2.dll"},
    )
    if (missing or len(variants) != 2
            or not any(set(variants) == pair for pair in supported_pairs)):
        raise ValueError(
            "A reference-only first-error probe requires both J-C variants "
            "(j64/j64avx2 or j.dll/javx2.dll); "
            f"available={variants!r}, unrun={missing!r}"
        )



def raw_reference_trace(oracle, sources):
    """Run original J sentences once, without Oracle.eval's result assignment.

    A separate fresh interpreter is essential: running a sentence twice on
    the same interpreter could change global NAME state and error precedence.
    Successful raw JDo sentences cannot reveal noun data; only error types.
    """
    outcomes, transports = [], []
    for source in sources:
        script = source in SCRIPT_SETUPS
        outcome = oracle.run_script(source) if script else oracle.run(source)
        outcomes.append(outcome or {"silent": True})
        transports.append("script" if script else "raw-sentence")
    return outcomes, transports


def error_steps(sources, outcomes):
    return [
        {"index": index, "source": source, "error": outcome["error"]}
        for index, (source, outcome) in enumerate(zip(sources, outcomes))
        if isinstance(outcome, dict) and "error" in outcome
    ]



def select_probe_index(case, sources):
    """Return the unambiguous chosen source occurrence, or None for other cases."""
    if case not in PROBE_LOCATORS:
        return None
    source, occurrence = PROBE_LOCATORS[case]
    indexes = [i for i, candidate in enumerate(sources) if candidate == source]
    if len(indexes) < occurrence:
        raise ValueError(
            f"Probe source occurrence missing: {case}: {source!r} #{occurrence}"
        )
    return indexes[occurrence - 1]


def focused_probe_step(case, sources, wrapped, raw):
    """Preserve an exact probe's outcomes and errors preceding that probe."""
    index = select_probe_index(case, sources)
    if index is None:
        return None
    return {
        "index": index,
        "source": sources[index],
        "wrapped": wrapped[index],
        "raw": raw[index],
        "wrapped_errors_before": error_steps(sources[:index], wrapped[:index]),
        "raw_errors_before": error_steps(sources[:index], raw[:index]),
    }



def observe_reference_cases(libraries, fixtures, oracle_factory=Oracle):
    """Retain each sentence, transport, error, and post-error read in source order."""
    records = []
    for variant, library in libraries:
        # Oracle loads J_LIBRARY in its constructor. Do not leak the selection
        # to another audit in a shared Python process.
        with patch.dict(os.environ, {"J_LIBRARY": str(library)}):
            for name, sources in fixtures:
                oracle = oracle_factory()
                try:
                    outcomes, transports = reference_trace(oracle, sources)
                finally:
                    oracle.close()
                # A second fresh interpreter observes the *unwrapped* source
                # for syntax/domain errors. Oracle.eval prefixes a result
                # assignment to non-assignment queries, which may affect POS
                # parsing, so retain both error traces instead of guessing.
                raw_oracle = oracle_factory()
                try:
                    raw_outcomes, raw_transports = raw_reference_trace(
                        raw_oracle, sources)
                finally:
                    raw_oracle.close()
                if (len(outcomes) != len(sources)
                        or len(transports) != len(sources)
                        or len(raw_outcomes) != len(sources)
                        or len(raw_transports) != len(sources)):
                    raise RuntimeError(f"Incomplete J trace: {variant}/{name}")
                errors = error_steps(sources, outcomes)
                raw_errors = error_steps(sources, raw_outcomes)
                disagree = [
                    {"index": i, "source": source,
                     "wrapped_error": wrapped.get("error"),
                     "raw_error": raw.get("error")}
                    for i, (source, wrapped, raw)
                    in enumerate(zip(sources, outcomes, raw_outcomes))
                    if wrapped.get("error") != raw.get("error")
                ]
                records.append({
                    "variant": variant, "case": name, "sources": sources,
                    "reference": outcomes, "reference_transport": transports,
                    "first_error": errors[0] if errors else None,
                    "error_steps": errors,
                    "raw_reference": raw_outcomes,
                    "raw_reference_transport": raw_transports,
                    "raw_first_error": raw_errors[0] if raw_errors else None,
                    "raw_error_steps": raw_errors,
                    "transport_error_disagreements": disagree,
                    "focused_probe": focused_probe_step(
                        name, sources, outcomes, raw_outcomes),
                })
    return records



def reference_error_kind(outcome):
    """Return the J error kind, or None for a successful raw/wrapped sentence."""
    if not isinstance(outcome, dict):
        raise ValueError("J observation is not a dictionary")
    error = outcome.get("error")
    if error is not None and (not isinstance(error, str) or not error):
        raise ValueError("J observation has an invalid error kind")
    return error


def assess_reference_probe_consensus(records, fixture_names, variants):
    """Classify *only* agreement on focused J-C error kinds.

    This is evidence readiness, not a pass of RustJ NAME semantics. A later
    J-C differential must check values and post-error effects separately.
    """
    observed = Counter((record.get("case"), record.get("variant"))
                       for record in records)
    expected = {(case, variant)
                for case in fixture_names for variant in variants}
    extra = set(observed) - expected
    duplicate = {key for key, count in observed.items() if count != 1}
    cases = []
    for case in fixture_names:
        reasons = []
        raw_kinds, wrapped_kinds = [], []
        for variant in variants:
            count = observed[(case, variant)]
            if count != 1:
                reasons.append(
                    f"{variant}: expected one observation, found {count}")
                continue
            record = next(r for r in records if
                          r.get("case") == case and r.get("variant") == variant)
            sources = record.get("sources")
            probe = record.get("focused_probe")
            if not isinstance(sources, list) or not isinstance(probe, dict):
                reasons.append(f"{variant}: missing source list or focused probe")
                continue
            try:
                index = select_probe_index(case, sources)
            except ValueError as error:
                reasons.append(f"{variant}: {error}")
                continue
            if index is None:
                reasons.append(f"{variant}: no defined probe locator")
                continue
            if (probe.get("index") != index or probe.get("source") != sources[index]):
                reasons.append(f"{variant}: focused probe points to the wrong sentence")
                continue
            if (len(record.get("reference", [])) != len(sources)
                    or len(record.get("raw_reference", [])) != len(sources)):
                reasons.append(f"{variant}: incomplete sentence trace")
                continue
            if (probe.get("wrapped_errors_before") is None
                    or probe.get("raw_errors_before") is None):
                reasons.append(f"{variant}: missing prerequisite error traces")
                continue
            if (probe["wrapped_errors_before"]
                    or probe["raw_errors_before"]):
                reasons.append(f"{variant}: an earlier setup sentence failed")
                continue
            if (record["reference"][index] != probe.get("wrapped")
                    or record["raw_reference"][index] != probe.get("raw")):
                reasons.append(f"{variant}: probe outcome differs from stored trace")
                continue
            try:
                raw_kinds.append(reference_error_kind(probe["raw"]))
                wrapped_kinds.append(reference_error_kind(probe["wrapped"]))
            except ValueError as error:
                reasons.append(f"{variant}: {error}")

        if len(raw_kinds) == len(variants):
            if len(set(raw_kinds)) != 1 or len(set(wrapped_kinds)) != 1:
                reasons.append("pinned J-C variants disagree on the target error kind")
            if any(raw != wrapped for raw, wrapped in
                   zip(raw_kinds, wrapped_kinds)):
                reasons.append("raw JDo and wrapped Oracle.eval disagree at the target")

        case_status = (
            "blocked" if reasons else
            "agreed_error" if raw_kinds[0] is not None else "agreed_no_error"
        )
        cases.append({
            "case": case,
            "status": case_status,
            "raw_error_kind": raw_kinds[0] if not reasons else None,
            "wrapped_error_kind": wrapped_kinds[0] if not reasons else None,
            "reasons": reasons,
        })

    if extra:
        cases.append({
            "case": None, "status": "blocked",
            "raw_error_kind": None, "wrapped_error_kind": None,
            "reasons": ["unexpected observation keys: " + repr(sorted(extra))],
        })
    if duplicate and not any(item["status"] == "blocked" for item in cases):
        raise AssertionError("duplicate records must block consensus")
    return {
        "scope": "focused J-C error kinds only; not RustJ conformance",
        "complete": bool(cases) and all(item["status"] != "blocked"
                                      for item in cases),
        "cases": cases,
    }



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    roots = parser.add_mutually_exclusive_group(required=True)
    roots.add_argument("--assets-root", type=Path)
    roots.add_argument("--reference-root", type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument(
        "--case", action="append", metavar="NAME",
        help="A selected NAME scope fixture; defaults to five POS cases",
    )
    args = parser.parse_args()
    try:
        libraries, metadata = reference_inputs(args.assets_root, args.reference_root)
        require_complete_reference_pair(libraries, metadata)
        requested = args.case if args.case else TARGET_CASES
        fixtures = select_named_fixtures(SCOPE_FIXTURES, requested)
        records = observe_reference_cases(libraries, fixtures)
    except (ValueError, OSError, RuntimeError) as error:
        parser.exit(1, f"J-C reference-only probe failed: {error}\n")

    report = {
        "mode": "reference-only",
        "status": "OBSERVED_J_REFERENCE_NOT_RUST_CONFORMANCE",
        "scope": "Selected NAME scope cases and pinned J-C variants only",
        "fixture_set": "name-scopes",
        "selection": {"partial": True, "cases": [name for name, _ in fixtures]},
        "reference_metadata": metadata,
        "reference_sha256": {
            variant: hashlib.sha256(library.read_bytes()).hexdigest()
            for variant, library in libraries
        },
        "fixtures": len(fixtures),
        "observations": len(records),
        "records": records,
        "focused_error_consensus": assess_reference_probe_consensus(
            records, [name for name, _ in fixtures],
            [variant for variant, _ in libraries]),
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "mode": report["mode"], "status": report["status"],
        "fixtures": report["fixtures"], "observations": report["observations"],
        "focused_error_consensus": report["focused_error_consensus"],
        "first_errors": [
            {"variant": item["variant"], "case": item["case"],
             "first_error": item["first_error"],
             "raw_first_error": item["raw_first_error"],
             "focused_probe": item["focused_probe"],
             "transport_error_disagreements":
                 item["transport_error_disagreements"]}
            for item in records
        ],
    }))
    # Success means a complete reference-only trace was recorded, NOT that
    # RustJ matched it or that the ignored regression can be released.
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
