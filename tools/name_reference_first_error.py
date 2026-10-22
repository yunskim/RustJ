"""Observe exact pinned J-C NAME/POS error traces without running RustJ.

This is a reference-only diagnostic, not a RustJ differential conformance gate.
Compare with RustJ only after the reference-first error has been pinned and read.
"""
import argparse
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
    "named_function_pos_rebinding_first_error",
)


def require_complete_reference_pair(libraries, metadata):
    """Refuse a first-error verdict when one pinned J-C variant is unobserved."""
    variants = [name for name, _ in libraries]
    missing = metadata.get("unrun_reference_variants", [])
    if missing or len(variants) != 2 or len(set(variants)) != 2:
        raise ValueError(
            "A reference-only first-error probe requires both J-C variants; "
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
                })
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    roots = parser.add_mutually_exclusive_group(required=True)
    roots.add_argument("--assets-root", type=Path)
    roots.add_argument("--reference-root", type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument(
        "--case", action="append", metavar="NAME",
        help="A selected NAME scope fixture; defaults to three alias/direct-POS cases",
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
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "mode": report["mode"], "status": report["status"],
        "fixtures": report["fixtures"], "observations": report["observations"],
        "first_errors": [
            {"variant": item["variant"], "case": item["case"],
             "first_error": item["first_error"],
             "raw_first_error": item["raw_first_error"],
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
