#!/usr/bin/env python3
"""Scan identity/metadata differential checks; not executable prefix coverage."""
import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import subprocess
from oracle import Oracle


def fixtures():
    accepted = []
    for op in ['+', '*']:
        verb = f'({op}/)' + chr(92)
        for n in range(7):
            for bits in itertools.product([0, 1], repeat=n):
                noun = '0$0' if n == 0 else (',' if n == 1 else '') + ' '.join(map(str, bits))
                accepted.append((noun, verb, True))
        accepted += [(str(b), verb, True) for b in [0, 1]]
        for shape in ['0 3', '1 3', '2 0', '2 3', '3 2', '2 2 2', '1 0', '3 0 2']:
            accepted.append((shape + '$0 1', verb, True))
    rejected = [
        ('0 1 1 0', '+' + chr(92), False),
        ('0 1 1 0', '2 (+/)' + chr(92), False),
        ('0 1 1 0', '(-/)' + chr(92), False),
        ('0 1 1 0', '(+/\\)"1', False),
        ('2 3 4', '(+/)' + chr(92), False),
        ('9223372036854775807 1', '(+/)' + chr(92), False),
        ('1e16 1 _1e16', '(+/)' + chr(92), False),
        ('0.0 _0.0 1.0', '(+/)' + chr(92), False),
        ('_ __', '(+/)' + chr(92), False),
        ("'xy'", '(+/)' + chr(92), False),
        ('<0 1', '(+/)' + chr(92), False),
    ]
    return accepted + rejected


def contract_failures(actual, reference):
    """Metadata and witness flags only; never compare Unsupported as a pass."""
    errors = []
    if 'error' in reference:
        return ['candidate for J error']
    for key in ['type', 'shape']:
        if actual.get(key) != reference.get(key):
            errors.append(key)
    if actual.get('parallel_authorized') is not False:
        errors.append('parallel permission')
    if actual.get('inject_identity') is not False:
        errors.append('identity injection')
    return errors


def boolean_model(input_value, op):
    """Independent exact mathematical model, not a Rust prefix executor."""
    shape = input_value['shape']
    data = input_value['data']
    items = shape[0] if shape else 1
    out_shape = shape or [1]
    if items < 2 or not data:
        return {'type': 1, 'shape': out_shape, 'data': data}
    width = len(data) // items
    acc = data[:width]
    output = list(acc)
    for start in range(width, len(data), width):
        row = data[start:start + width]
        acc = [a + b if op == '+' else a * b for a, b in zip(acc, row)]
        output.extend(acc)
    return {'type': 4 if op == '+' else 1, 'shape': out_shape, 'data': output}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--source-directory', required=True)
    parser.add_argument('--source-revision', required=True)
    parser.add_argument('--reference-revision', required=True)
    parser.add_argument('--report', required=True)
    args = parser.parse_args()
    cases = fixtures()
    result = subprocess.run([args.binary], input=''.join(f'{noun}\t{verb}\n' for noun, verb, _ in cases),
                            text=True, capture_output=True, check=True)
    observed = [json.loads(line) for line in result.stdout.splitlines()]
    if len(observed) != len(cases):
        raise RuntimeError('Scan probe response count mismatch')
    oracle = Oracle()
    entries, failed, accepted = [], 0, 0
    try:
        for (noun, verb, expected), actual in zip(cases, observed):
            source = f'{verb} {noun}'
            reference = oracle.eval(source)
            contract = actual['contract']
            errors = []
            if (contract is not None) != expected:
                errors.append('recognition')
            if expected and contract is not None:
                errors += contract_failures(contract, reference)
                if reference != boolean_model(actual['input'], verb[1]):
                    errors.append('Boolean witness model')
                accepted += 1
            elif not actual['boundaries']:
                errors.append('missing analysis boundary')
            # Keep all runtime gaps explicit and outside metadata pass counts.
            if actual['runtime'] != {'error': 'unsupported'}:
                errors.append('unexpected executable prefix route')
            failed += bool(errors)
            entries.append({'source': source, 'reference': reference, 'observation': actual,
                            'expected_candidate': expected, 'failures': errors})
    finally:
        oracle.close()
    digest = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
    source_dir = Path(args.source_directory)
    report = {'platform': os.name, 'cases': len(cases), 'accepted_identity_checks': accepted,
              'rejected_analysis_checks': len(cases) - accepted, 'failed': failed,
              'runtime_prefix_coverage_boundaries': len(cases), 'executable_prefix_passes': 0,
              'binary_sha256': digest(args.binary), 'reference_sha256': digest(os.environ['J_LIBRARY']),
              'source_revision': args.source_revision, 'reference_revision': args.reference_revision,
              'source_hashes': {p: digest(source_dir / p) for p in ['jsrc/ap.c', 'jsrc/ar.c', 'jsrc/va2.c']},
              'entries': entries}
    Path(args.report).write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: report[k] for k in ['cases', 'accepted_identity_checks', 'rejected_analysis_checks',
                                           'failed', 'runtime_prefix_coverage_boundaries', 'executable_prefix_passes']}))
    return int(failed != 0)


if __name__ == '__main__':
    raise SystemExit(main())
