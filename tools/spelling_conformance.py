#!/usr/bin/env python3
"""Finite spelling/enqueue differential, not runtime or full numeric grammar coverage."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path

from frontend_stage_conformance import Probe
from oracle import Oracle


def spelling_cases():
    # All graphic ASCII bases except quote (requires paired string grammar),
    # crossed with uninflected, one-inflection and two-inflection forms.
    words = [chr(base) + suffix for base in range(33, 127) if base != ord("'")
             for suffix in ['', '.', ':', '..', '.:', ':.', '::']]
    words += ['99:', '1.5:', '_99:', '_a:', 'abc.', 'NB..', 'NB.:',
              'foo_:', 'foo_bar_:', 'foo__:', 'foo_bar__:',
              'with', 'foo_bar', '1j2', '2r3', '123x']
    return words


def run(args):
    if os.name != 'nt':
        raise RuntimeError('Run on native Windows with the pinned J DLL.')
    oracle, probe = Oracle(), Probe(args.binary)
    entries, failures, counts = [], [], Counter()
    name_boundaries = {'foo_:', 'foo_bar_:', 'foo_bar__:'}
    numeric_boundaries = {'1j2', '2r3', '123x'}
    try:
        for word in spelling_cases():
            # Replenish names because name_: can abandon its binding. These
            # controls establish validity; they do not emulate enqueue lookup.
            for declaration in ['foo=:2', 'foo_bar=:2']:
                if oracle.run(declaration):
                    raise RuntimeError('C control declaration failed')
            expected_words = oracle.words(word)
            observed = probe.inspect(word)
            expected = oracle.run('vocabprobe=: ' + word)
            actual = observed.get('enqueue')
            local = []
            if expected_words != {'words_hex': [word.encode().hex()]}:
                local.append('fixture is not one parse-visible C word')
            if observed.get('raw_words') != expected_words.get('words_hex'):
                local.append('word formation mismatch')
            if expected and expected['error'] in {'spelling error', 'ill-formed number', 'ill-formed name', 'syntax error'}:
                status = 'error_class_verified'
                if actual != expected:
                    local.append('C/Rust enqueue error class mismatch')
            elif word in name_boundaries or word in numeric_boundaries:
                status = 'valid_unsupported_boundary'
                if expected is not None and expected != {'error': 'value error'}:
                    local.append('valid control is no longer accepted by C')
                if actual != {'error': 'unsupported'}:
                    local.append('valid unsupported syntax was reclassified')
            else:
                status = 'accepted_enqueue_control'
                if expected not in [None, {'error': 'value error'}] or not isinstance(actual, list):
                    local.append('accepted control enqueue mismatch')
            counts[status] += 1
            if expected:
                counts[expected['error']] += 1
            entry = {'word': word, 'c_result': expected, 'rust_enqueue': actual,
                     'status': status, 'failures': local}
            entries.append(entry)
            if local:
                failures.append(entry)
    finally:
        probe.close()
        oracle.close()
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    report = {'platform': os.name, 'scope': 'fixed ASCII spelling and enqueue errors; valid unimplemented syntax stays unsupported; not full name/numeric grammar or execution coverage',
              'source_revision': args.source_revision, 'reference_revision': args.reference_revision,
              'source_hashes': {rel: digest(args.source_directory / rel) for rel in ['jsrc/ws.c', 'jsrc/w.c']},
              'reference_sha256': digest(Path(os.environ['J_LIBRARY'])),
              'binary_sha256': digest(args.binary), 'cases': len(entries), 'matrix_cases': 651,
              'checks': dict(counts), 'entries': entries, 'failed': len(failures), 'failures': failures}
    args.report.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: report[k] for k in ['cases', 'checks', 'failed']}))
    return bool(failures)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-directory', type=Path, required=True)
    parser.add_argument('--source-revision', required=True)
    parser.add_argument('--reference-revision', required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    return run(parser.parse_args())


if __name__ == '__main__':
    raise SystemExit(main())
