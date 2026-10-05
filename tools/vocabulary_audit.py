#!/usr/bin/env python3
"""Audit pinned C core spelling candidates/POS against Rust enqueue without executing verbs.

NuVoc guides the current documentation review. The reproducible spelling inventory
comes from ws.c, with POS observed by assignment + 4!:0 in the supplied C DLL.
A Rust Unsupported entry is an explicit coverage boundary, never a POS pass.
"""
import argparse
import ast
import hashlib
import json
import os
import re
from pathlib import Path
from oracle import Oracle
from frontend_stage_conformance import Probe


def source_spellings(source):
    first = source.split('static C spellintab2', 1)[1].split('};', 1)[0]
    words = set()
    for char, codes in re.findall(r"\['((?:\\.|[^'])+)'\s*-0x20\]\s*=\s*\{([^}]+)\}", first):
        base = ast.literal_eval("'" + char + "'")
        if base.isdigit():
            continue  # Uninflected numeric codes include invisible internal operators.
        values = [x.strip() for x in codes.split(',')]
        if len(values) != 3:
            raise ValueError('unexpected spellintab2 row')
        for suffix, code in zip(['', '.', ':'], values):
            if code != '0':
                words.add(base + suffix)
    second = source.split('static C spellintab3', 1)[1].split('};', 1)[0]
    for char, codes in re.findall(r'/\*\s*(\S)\s*\*/\s*\{([^}]+)\}', second):
        if char == 'e':
            continue
        values = [x.strip() for x in codes.split(',')]
        if len(values) != 4:
            raise ValueError('unexpected spellintab3 row')
        for suffix, code in zip(['..', '.:', ':.', '::'], values):
            if code != '0':
                words.add(char + suffix)
    words -= {'(', ')', '=.', '=:'}  # Structural tokens, not assignable entities.
    words.update(str(n).replace('-', '_') + ':' for n in range(-9, 10))
    words.add('__:')
    if not {'[.', '].', ']:', '/..', '$::', 'c.', 'f:', 't.', 'T.'}.issubset(words):
        raise ValueError('source spelling inventory missing modern forms')
    return sorted(words)


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--source-directory', type=Path, required=True)
    p.add_argument('--source-revision', required=True)
    p.add_argument('--reference-revision', required=True)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--report', type=Path, required=True)
    args = p.parse_args()
    ws = args.source_directory / 'jsrc/ws.c'
    o, probe = Oracle(), Probe(args.binary)
    entries, failures = [], []
    try:
        for word in source_spellings(ws.read_text()):
            error = o.run('vocabprobe=: ' + word)
            if error and error != {'error':'spelling error'}:
                raise RuntimeError(('unexpected C inventory error', word, error))
            # A nonzero spellin code does not prove an installed primitive.
            # w.c also requires ds(e) with a permanent usecount. Retain rejected
            # candidates without inventing POS or conflating them with support.
            pos = None if error else {0:'Noun', 1:'Adverb', 2:'Conjunction', 3:'Verb'}[o.name_class('vocabprobe')['class']]
            observed = probe.inspect(word)
            formed = o.words(word)['words_hex']
            if observed.get('raw_words') != formed:
                failures.append({'word': word, 'reason':'word formation mismatch'})
            queue = observed.get('enqueue')
            status = 'coverage_boundary'
            if error:
                status = 'source_code_only'
            elif isinstance(queue, list):
                if len(queue) != 1 or queue[0]['class'] != pos:
                    failures.append({'word':word, 'reason':'enqueue POS mismatch', 'expected':pos, 'actual':queue})
                else:
                    status = 'enqueue_pos_verified'
            elif queue != {'error':'unsupported'}:
                failures.append({'word':word, 'reason':'unexpected enqueue failure', 'actual':queue})
            entries.append({'word':word, 'c_pos':pos, 'c_error':error, 'rust_enqueue':queue, 'status':status})
        legacy = [{'word':w, 'c_result':o.run('vocabprobe=: '+w), 'rust_enqueue':probe.inspect(w).get('enqueue')}
                  for w in ['d.', 'D.', 'D:', 't:', '..', '.:', 's:', 'I:']]
        if any(x['c_result'] != {'error':'spelling error'} for x in legacy):
            raise RuntimeError('legacy spelling fixture no longer rejected by C')
    finally:
        probe.close()
        o.close()
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    report = {'platform':os.name, 'nuvoc_review_url':'https://code.jsoftware.com/mediawiki/index.php?title=NuVoc&oldid=60409',
              'nuvoc_review_date':'2026-10-05', 'source_review_revision':args.source_revision,
              'reference_revision':args.reference_revision, 'source_hashes':{rel:digest(args.source_directory/rel) for rel in ['jsrc/ws.c','jsrc/t.c','jsrc/v.c']},
              'reference_sha256':digest(Path(os.environ['J_LIBRARY'])), 'binary_sha256':digest(args.binary),
              'scope':'core spelling candidates from pinned ws.c plus finite/infinite constant functions; not full runtime coverage or a live NuVoc scraper',
              'entries':entries, 'legacy_rejections':legacy, 'failed':len(failures), 'failures':failures,
              'enqueue_pos_verified':sum(x['status']=='enqueue_pos_verified' for x in entries),
              'coverage_boundaries':sum(x['status']=='coverage_boundary' for x in entries),
              'source_code_only':sum(x['status']=='source_code_only' for x in entries)}
    args.report.write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')
    print(json.dumps({k:report[k] for k in ['platform','failed','enqueue_pos_verified','coverage_boundaries','source_code_only']}))
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
