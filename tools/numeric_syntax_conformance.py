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
    rows += [(w,'precision-resource-unknown') for w in ['2.1e_9223372036854775808fq']]
    rows += [(w,'platform-rounding-unknown') for w in ['_0X1P_9999ad90','_0X1P_1075ad90']]
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
            if formed != {'words_hex':[source.encode().hex()]} or observed.get('raw_words') != formed.get('words_hex'):
                local.append('word formation mismatch')
            if c_result is None:
                if actual_error == 'unsupported':
                    if reason and reason.startswith(('validated numeric family', 'validated real-family ratio')):
                        status = 'valid_payload_boundary'
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
    report={'platform':os.name,'scope':'numeric word formation/error recognition; quad/Windows hex grammar included; no exact/complex/based/quad payload, full resource or rounding conformance',
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
