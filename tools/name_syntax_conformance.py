#!/usr/bin/env python3
"""Bounded ASCII name grammar oracle; locale/debug lookup is not runtime conformance."""
import argparse
from collections import Counter
import hashlib
import itertools
import json
import os
from pathlib import Path
import random

from frontend_stage_conformance import Probe
from oracle import Oracle


def name_cases():
    words = ['a' + ''.join(t) for n in range(7) for t in itertools.product('a0_', repeat=n)]
    words += ['foo_0_', 'foo_00_', 'foo_123456789012345678_',
              'foo_1234567890123456789_', 'foo__bar_baz', 'foo__bar___1',
              'foo___bar', 'foo____1', 'foo_a9_', 'foo_0a_']
    rng = random.Random(20261005)
    words += ['a' + ''.join(rng.choice('a0_A9b') for _ in range(rng.randrange(1, 21)))
              for _ in range(1000)]
    # Every generated name also appears as a by-value/abandon name. Append
    # precisely _: so the C/Rust validators see the same underlying name.
    return sorted(set(words + [word + '_:' for word in words]))


def run(args):
    if os.name != 'nt':
        raise RuntimeError('Run on native Windows with the pinned J DLL.')
    oracle, probe = Oracle(), Probe(args.binary)
    entries, failures, counts = [], [], Counter()
    try:
        for word in name_cases():
            formed = oracle.words(word)
            observed = probe.inspect(word)
            c_result = oracle.run('vocabprobe=: ' + word)
            actual = observed.get('enqueue')
            local = []
            if formed != {'words_hex': [word.encode().hex()]} or observed.get('raw_words') != formed.get('words_hex'):
                local.append('word formation mismatch')
            if c_result == {'error': 'ill-formed name'}:
                status = 'invalid_name_verified'
                if actual != c_result:
                    local.append('invalid name error mismatch')
            else:
                # C parses before locale/debug lookup. Accepted sentences and
                # subsequent value/locale errors establish lexical validity,
                # never Rust runtime equivalence or lookup support.
                if c_result is not None and c_result.get('error') not in {'value error', 'J error 30'}:
                    local.append('unexpected C result outside lexical/lookup seam')
                if '_:' in word or '__' in word or word.endswith('_'):
                    status = 'valid_unsupported_name'
                    if actual != {'error': 'unsupported'}:
                        local.append('valid name was rejected or lookup support invented')
                else:
                    status = 'simple_name_verified'
                    if not isinstance(actual, list) or len(actual) != 1 or actual[0]['class'] != 'Name':
                        local.append('simple name classification mismatch')
            counts[status] += 1
            entry = {'word': word, 'c_result': c_result, 'rust_error': actual.get('error') if isinstance(actual, dict) else None,
                     'status': status, 'failures': local}
            entries.append(entry)
            if local:
                failures.append(entry)
    finally:
        probe.close()
        oracle.close()
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    report = {'platform': os.name, 'scope': 'bounded ASCII simple/direct/indirect/debug/by-value name grammar; not locale lookup, allocation limits, abandon effects or runtime support',
              'seed': 20261005, 'source_revision': args.source_revision, 'reference_revision': args.reference_revision,
              'source_hashes': {rel: digest(args.source_directory / rel) for rel in ['jsrc/ws.c', 'jsrc/w.c', 'jsrc/sn.c', 'jsrc/jerr.h']},
              'reference_sha256': digest(Path(os.environ['J_LIBRARY'])), 'binary_sha256': digest(args.binary),
              'cases': len(entries), 'checks': dict(counts), 'entries': entries, 'failed': len(failures), 'failures': failures}
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
