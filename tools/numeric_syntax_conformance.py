#!/usr/bin/env python3
"""Numeric-word recognition oracle; valid unsupported payloads and unresolved grammar are separate."""
import argparse
from collections import Counter
import hashlib
import itertools
import json
import os
from pathlib import Path
from frontend_stage_conformance import Probe
from oracle import Oracle


def numeric_cases():
    parts = ['0','1','2','_2','_','__','_.','1.0','1e2','1E2',
             '1x','2x','1j2','1j','1r2','2r','_r3','2r_',
             '2r3x','2x3','1p2','2b10','2b','2b.','2b_1',
             '2ad90','_2ad90','2ar1','1z','1f','2xx','1r2r3']
    words = parts + [' '.join(t) for t in itertools.product(parts, repeat=2)]
    words += ['1jj2','1ax2','_r','_r_3','_r_','__r','__r_3',
              '2r__','2r0','0r0','1xr2','1x2x3','2b102','16bff',
              '2b1.1','2b_1.1','2b1..1','2b1_1','1j2b10',
              '1p2b10','1r2j3r4','1jnan','1jinf','1j_','1j_.',
              '9223372036854775808','_9223372036854775809',
              '0.0 1x','1j0 1x','2b10 1x','1x _','1x __',
              '99:', '_99:', '1.5:']
    rows = [(w,'reviewed') for w in sorted(set(words))]
    quad = ['2fq','2.fq','2.5fq','_2fq','2efq','2E3fq','2fqz',
            '2fqbad','2fqfq','1e401fq','1e_401fq','1e9223372036854775807fq',
            '1e_9223372036854775808fq','1e9223372036854775808fq',
            '1e_9223372036854775809fq','_','__','_.','_fq','_.fq','_.5',
            '1','1.0','1E2','1e_2','2r3','1x','1j2','2fs','2fh']
    precision = set(quad + [' '.join(t) for t in itertools.product(quad, repeat=2)])
    rows += [(w,'precision-reviewed') for w in sorted(precision)]
    hex_parts = ['0X10','_0X10','0X1.8','0X.8','0X1P2','0X1P_2',
                 '0X1P9999','0X1P_9999','0X','0X.','0Xz','0X1P',
                 '0X1P_','0X1Pz','1X10','0X1..2','0X10r2',
                 'NaN','nan','NAN','Infinity','inf','INF']
    platform = ['1j' + h for h in hex_parts]
    platform += ['1j' + h + ' 1j2' for h in hex_parts]
    platform += ['0X10ad90','_0X1ad90','_0X.8ad90',
                 '_0X1P_1074ad90','_0X0ad90','0X0ad90','0Xad90','_0Xad90','0Xb1','_0Xb1','_0X0P0ad90']
    rows += [(w,'platform-reviewed') for w in sorted(set(platform))]
    rows += [(w,'precision-resource-construction') for w in ['2.1e_9223372036854775808fq']]
    rows += [(w,'platform-rounding-reviewed') for w in ['_0X1P_9999ad90','_0X1P_1075ad90']]
    mantissas = ['0', '1', '1.00000000000001', '1.8', '2', '2.0001', '3', '.8', '.80001',
                 '0001.0000000000', '1.00000000000000000000000000000000000001']
    rounding = {'_0X' + m + 'P_' + str(e) + angle
                for m, e, angle in itertools.product(mantissas, range(1073,1078), ['ad90', 'ar1'])}
    rows += [(w,'platform-rounding-reviewed') for w in sorted(rounding)]
    rows += [(w,'platform-exponent-reviewed') for w in
             ['_0X1P9223372036854775808ad90','_0X1P_9223372036854775809ad90',
              '_0X1P170141183460469231731687303715884105728ad90',
              '_0X1P_170141183460469231731687303715884105729ad90']]
    rows += [(w,'platform-reviewed') for w in ['1jINFINITY','1jinfinity','1j_nan','1jInfinityr2','1jnanr2']]
    rows += [(w,'platform-nan-formation') for w in ['1jNaN(1)','1jnan()','1jNAN(foo)','1j_nan(1)']]
    numerators = ['0X0','_0X0','0X1','_0X1','0X1.8','_0X1.8',
                  '0X1P_1074','_0X1P_1074','0X1P1023','_0X1P1023']
    denominators = ['0','_0','1','_1','2','_2','0X0','_0X0','0X2','_0X2',
                    '0X1P_1074','0X1P1023','INF','NAN']
    rows += [(n + 'r' + d + angle,'platform-ratio-reviewed')
             for n, d, angle in itertools.product(numerators, denominators, ['ad90','ar1'])]
    rows += [(w,'platform-ratio-unknown') for w in
             ['_0X1P_1075r1ad90', '0X1P9999r0X1P9999ad90']]
    rows += [(w,'precision-reviewed') for w in
             ['2.1e_9223372036854775808fq 2fqz', '2fqz 2.1e_9223372036854775808fq']]
    return rows


def run(args):
    if os.name != 'nt':
        raise RuntimeError('Run on native Windows with the pinned J DLL.')
    oracle, probe = Oracle(), Probe(args.binary)
    entries, failures, counts = [], [], Counter()
    try:
        for source, scope in numeric_cases():
            formed = oracle.words(source)
            observed = probe.inspect(source)
            c_result = oracle.run('vocabprobe=: ' + source)
            queue = observed.get('enqueue')
            actual_error = queue.get('error') if isinstance(queue,dict) else None
            reason_hex = observed.get('enqueue_unsupported_reason_hex')
            reason = bytes.fromhex(reason_hex).decode() if isinstance(reason_hex, str) else None
            local = []
            expected_words = [source.encode().hex()]
            if scope == 'platform-nan-formation':
                prefix, payload = source[:-1].split('(')
                expected_words = [prefix.encode().hex(), '28']
                if payload: expected_words.append(payload.encode().hex())
                expected_words.append('29')
            if formed != {'words_hex':expected_words} or observed.get('raw_words') != expected_words:
                local.append('word formation mismatch')
            if scope == 'platform-nan-formation':
                status = 'nan_word_formation_boundary'
                if (c_result != {'error':'syntax error'} or actual_error != 'unsupported'
                    or reason != 'validated numeric family payload construction'):
                    local.append('NaN parentheses must remain J tokens and a payload/parser coverage boundary')
            elif c_result is None:
                if actual_error == 'unsupported':
                    if reason and reason.startswith(('validated numeric family', 'validated real-family ratio')):
                        status = 'valid_payload_boundary'
                    elif reason == 'quad scale/exponent construction boundary':
                        status = 'quad_construction_boundary'
                    elif reason == 'overflowing integer literal conversion':
                        status = 'integer_conversion_boundary'
                    else:
                        status = 'unresolved_recognition_boundary'
                    if scope.endswith('-unknown') and status != 'unresolved_recognition_boundary':
                        local.append('unresolved numeric boundary was claimed validated')
                elif isinstance(queue,list) and len(queue)==1 and queue[0]['class']=='Noun':
                    status = 'accepted_noun_control'
                else:
                    status = 'failure'
                    local.append('valid C numeric word was rejected')
            elif c_result.get('error') in {'ill-formed number','spelling error'}:
                if queue == c_result:
                    status = 'error_class_verified'
                elif actual_error == 'unsupported' and scope.endswith('-unknown'):
                    status = 'unresolved_error_boundary'
                else:
                    status = 'failure'
                    local.append('numeric/spelling error class mismatch')
            elif (c_result == {'error':'J error 11'} and scope.startswith('precision')
                  and actual_error == 'unsupported' and reason == 'C reference precision conversion boundary'):
                status = 'reference_precision_boundary'
            else:
                status = 'failure'
                local.append('unexpected reference or probe result')
            counts[status]+=1
            entry = {'source':source,'scope':scope,'c_result':c_result,'rust_error':actual_error,'rust_unsupported_reason':reason,
                     'status':status,'failures':local}
            entries.append(entry)
            if local:
                failures.append(entry)
    finally:
        probe.close()
        oracle.close()
    digest=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
    report={'platform':os.name,'scope':'numeric word formation/error recognition; quad/Windows hex grammar and default round-to-even polar sign included; NaN parentheses checked as J tokens; no exact/complex/based/quad payload, full resource or changed rounding-environment conformance',
            'source_revision':args.source_revision,'reference_revision':args.reference_revision,
            'source_hashes':{rel:digest(args.source_directory/rel) for rel in ['jsrc/wn.c','jsrc/ws.c','jsrc/w.c','jsrc/jerr.h']},
            'reference_sha256':digest(Path(os.environ['J_LIBRARY'])),'binary_sha256':digest(args.binary),
            'cases':len(entries),'checks':dict(counts),'entries':entries,'failed':len(failures),'failures':failures}
    args.report.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:report[k] for k in ['cases','checks','failed']}))
    return bool(failures)


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--source-directory',type=Path,required=True)
    parser.add_argument('--source-revision',required=True)
    parser.add_argument('--reference-revision',required=True)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--report',type=Path,required=True)
    return run(parser.parse_args())


if __name__=='__main__':
    raise SystemExit(main())
