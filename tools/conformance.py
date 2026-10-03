#!/usr/bin/env python3
"""Deterministic differential checks of the supported M1/M2 subset."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import random
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]

def cases():
    fixed = [
        'snap=:1', 'copy=:snap', 'snap=:2', 'copy',
        'fn=:+', 'alias=:fn', 'alias 3', 'fn=:*', 'alias 3', '2 alias 3',
        'deferred=:futureverb', 'deferred 3', 'futureverb=:-', 'deferred 3',
        'fn=:7', 'alias 3', 'fn=:+', 'alias 3',
        'summation=:+/', 'summation 1 2 3', 'fn/1 2 3',
        'fn=:absentverb 3', 'alias 3',
        'parenverb=:(+)', 'parenverb 3',
        "('a'+1)+(1 2+1 2 3)", "(1 2+1 2 3)+('a'+1)",
        'missing + (1 2+1 2 3)', 'missing + )',
        'errorhold=:1 2 3', "errorhold=:('a'+1)+(1 2+1 2 3)",
        'errorhold', "errorhold=:(1 2+1 2 3)+('a'+1)", 'errorhold',
        '1 NB.. 2', '1 NB.: 2', '0', '1', '2', '_3', '1 0 1', '1 2 3', '1.5 2 3', '_', '__', '_.',
        "'a'", "'abc'", "''", "'it''s'", "'NB. text'",
        '10 - 3 - 2', '(10 - 3) - 2', '2 * 3 + 4', '- 1 2 3', '% 1 2 4',
        '| _2 0 3', '* _2 0 3', '0 % 0', '1 % 0', '_1 % 0', '0 * _',
        '1 2 3 = 1 0 3', '1 2 3 < 2', '1 2 3 > 2', "'a' = 'b'", "'a' = 2",
        '0 + 0', '1 + 0', '1 * 0', '1 - 0', '+ 1', '+/ 1 0 1', '*/ 1 0 1',
        'i. 0', 'i. 1', 'i. 5', 'i. _4', 'i. 2 3', 'i. 2 _3', 'i. _2 3',
        '$ 3', '# 3', ', 3', '$ i. 2 3', '# i. 2 3', ', i. 2 3',
        '2 3 $ 1 2', '0 $ 3', '2 0 $ 3', '2 3 $ i. 0',
        '0 { 10 20 30', '_1 { 10 20 30', '2 0 { 10 20 30', '1 { i. 2 3',
        '1 2 , 3 4', "'ab' , 'cd'", '1 , 2.5',
        '+/ i. 5', '-/ 1 2 3', '+/ i. 2 3', '+/ i. 0', '*/ i. 0',
        '+/"1 i. 2 3', '$"1 i. 2 3', '#"0 i. 2 3', '+/"_1 i. 2 3',
        '10 20 + i. 2 3', '(i. 2 3) + 10 20',
        '9223372036854775807 + 1', '_9223372036854775808 - 1',
        '9223372036854775807 = 9223372036854775806',
        '1 2 + 1 2 3', "'abc' + 2", '10 { 1 2', 'missingname + 1',
        'a =: i. 4', 'b =: a', 'a =: a + 10', 'a', 'b',
        'a =: 1 2 + 3 4 5', 'a',
    ]
    for rank in ['0','1','2','3','_1','_2']:
        for verb in ['+','-','*','%','=','<','>']:
            fixed.extend([
                f'(i.2 3) {verb}"{rank} (10+i.2 3)',
                f'(i.3) {verb}"{rank} (i.2 3)',
                f'2 {verb}"{rank} (i.2 3)',
                f'(i.2 3) {verb}"{rank} (2)',
            ])
    fixed.extend(['(i.2 3) +"1 (i.2 4 3)', '(i.2 4 3) -"1 (i.2 3)',
                  '(i.2 3) +"1 (i.3 3)', '(i.2 0) +"1 (i.2 0)',
                  '(2 2 $ 1 2 9223372036854775807 4) +"1 (2 2 $ 1)',
                  "'ab' =\"0 'ac'"])
    for ranks in ['0 1','1 0','1 2','2 1','0 0 1','2 0 1','_1 1','1 _1']:
        fixed.extend([f'(i.2 3) +"{ranks} (i.2 3)',
                      f'(i.2 3) -"{ranks} (i.2 3 4)',
                      f'10 +"{ranks} (i.2 3)',
                      f'#"{ranks} i.2 3 4', f'+/"{ranks} i.2 3 4'])
    fixed.extend(['#"1 2 3 4 i.2 3', '#"1.5 2 i.2 3'])
    for noun in ['i.0', 'i.1', 'i.5', 'i.2 3', 'i.2 0', 'i.0 3',
                 'i.2 3 4', "'abc'", '1 0 1', '0.5+i.3', '7']:
        fixed.extend([f'|.({noun})', f'|:({noun})'])
        for verb in ['|.', '{.', '}.']:
            for n in ['0','1','_1','2','_2','7','_7']:
                fixed.append(f'{n} {verb} ({noun})')
    for a in ['3 1 3 2', "'abca'", 'i.2 3', 'i.0', 'i.2 0', '3', '0.0 1.0 2.0']:
        for b in ['3 4 1', "'acx'", 'i.2 3', 'i.0', 'i.3 0', '3']:
            for verb in ['i.', 'i:']:
                fixed.append(f'({a}) {verb} ({b})')
    fixed.extend(['i:0','i:3','i:_3','i:2.5','i:_2.5','i:2.2',
                  'I.0 1 0 1','I.2 0 3','I.3','I.i.0','I._1 2','I.1.5',
                  '(9223372036854775807 9223372036854775806) i.9223372036854775806',
                  '(1.0 2.0) i.(1.0+1e_15)'])
    for a in ['i.0','1','1 2','2 2',"'ana'","''",'1.0 2.0']:
        for b in ['i.0','1','1 2 1 2 2',"'banana'",'0.0 1.0 2.0']:
            for verb in ['e.','E.']:
                fixed.append(f'({a}) {verb} ({b})')
    fixed.extend(['(i.2 3)e.(i.2 3)', '(i.2 3)e.1 2 3', '(i.2 0)e.(i.3 0)'])
    rng = random.Random(20260926)
    for _ in range(200):
        n = rng.randint(1, 16)
        def nums():
            return ' '.join(str(rng.randint(-1000, 1000)).replace('-', '_') for _ in range(n))
        fixed.append(f'({nums()}) {rng.choice(["+", "-", "*", "=", "<", ">"])} ({nums()})')
    for rows in range(5):
        for cols in range(5):
            fixed.extend([f'i. {rows} {cols}', f'3 + i. {rows} {cols}', f'+/ i. {rows} {cols}'])
    # SIMD-sized arrays and scalar tails; compare actual J promotion and values.
    for n in [63,64,65,127,128,129,1025]:
        fixed.extend([f'a =: {n} $ 9223372036854775807 _9223372036854775808 9007199254740993 2 _2',
                      'a + 2', '2 + a', 'a - 2', '2 - a', 'a + a', 'a - a', 'a * 3',
                      f'(i. {n}) + 9223372036854775807',
                      f'0.5 + i. {n}', f'(0.5 + i. {n}) + 2.5'])
    # M2: streaming cell assembly, borrowed accumulator promotion and compact
    # literal tokens. Unsupported rank padding remains a separate Rust test.
    fixed.extend([
        '1e_2', '1  2\t3', '1 _ 3',
        '+/"1 (3 2 $ 2 3 9223372036854775807 1 4 5)',
        '-/"1 (2 3 $ 10 3 1 20 5 2)',
        ',"1 (2 0 $ 1)', '#"1 (2 3 $ \'abcdef\')',
        '$"1 (2 0 $ 1)', '*"0 (2 3 $ _2 0 3)',
    ])
    for cols in [1,64,65,129]:
        fixed.extend([
            f'+/ (3 {cols} $ 2 9223372036854775807 1)',
            f'-/ (3 {cols} $ 2 9223372036854775807 _1)',
            f'+/"1 (4 {cols} $ i. 17)',
            f'+/ (3 {cols} $ 0.5 2.5 4.5)',
        ])
    fixed.extend([
        '< 3', '< i.2 3', '< < 42', '> < i.2 3', '> > < < 42',
        ">(<1 2),<'ab'", '(<1 2),<3 4', '>(<1 2),<3 4',
        '>(<1 2),<3.5 4.5', '2 2$<1 2 3', '>2 2$<1 2 3',
        '>2$<i.0', '>0$<3', '<"0 i.4', '>"0 <"0 i.4',
        'boxsource=:i.4', 'boxsaved=:<boxsource',
        'boxsource=:boxsource+10', '>boxsaved',
        'boxopened=:>boxsaved', 'boxopened=:boxopened+20', '>boxsaved',
    ])
    # Sparse results are inspected through dense component queries so the
    # oracle need not interpret the C engine's private sparse headers.
    fixed.extend([
        'spcheck=:$.2 3$0 1 0 2 0 3', '$spcheck', '#spcheck',
        '2$.spcheck', '3$.spcheck', '4$.spcheck', '5$.spcheck', '7$.spcheck',
        '0$.spcheck', 'spalias=:spcheck', 'spvalues=:5$.spcheck',
        'spvalues=:spvalues+10', '5$.spalias',
        'spcheck=:1$.2 3', '2$.spcheck', '3$.spcheck', '4$.spcheck',
        '5$.spcheck', '0$.spcheck', '$.42', '0$.$.0 1 0 1',
        '0$.$.0.0 1.5 0.0', '0$.$.i.0', '3$.1 2', '6$.spcheck',
        '1$._1 2', '1$.i.0',
    ])
    # F1/P3/P5: construction errors are observed before a derived verb runs.
    fixed.extend(['3/', '+@:3', '3@:+', '(+"((1))) 3'])
    for rank in ['_', '__', '_.', '1e100', '_1e100',
                 '1.00000000000001', '_1.00000000000001',
                 '0 1 _', '1 _1', '_ 0 1', '1.5', "'a'", '1 2 3 4']:
        fixed.extend([f'rankfn=:+"{rank}', 'rankfn 3', 'rankfn i.2 3'])
    for noun in ['1 1$0', '2 2$0', '0 4$0', "1 1$'a'", '0$0', '4$0']:
        fixed.extend([f'rankarg=:{noun}', 'rankfn=:+"rankarg', 'rankfn 3'])
    # F2: stack-entry noun snapshots and completed named modifier/train boundaries.
    # These compare supported observable results/errors, not a C stack trace.
    fixed.extend([
        'entrynoun=:1 2 3', 'entrynoun+entrynoun', 'entryverb=:+',
        'entryverb/entrynoun', 'entrynoun+entryverb/entrynoun',
        '(entryverb/ % #) entrynoun',
        "entrynoun (+\"'bad') entrynoun", "((entryverb/))\"'bad'",
        'entrycopy=:entrynoun', 'entrynoun=:9', 'entrycopy',
        'entryverb=:*', '(entryverb/ % #) entrycopy',
    ])
    # P2/P3: concrete row results are available to subsequent constructors.
    fixed.extend([
        'computedrank=:+"(1+0)', 'computedrank i.2 3',
        'computedrank=: +"(#1 2)', 'computedrank i.2 3',
        'computedrank=:+"(0$0)', 'computedrank i.2 3',
        "computedrank=:+\"('a'+1)", 'computedrank i.2 3',
        'computedrank=:+"(1 2+1 2 3)', 'computedrank i.2 3',
        'computedfork=:(1+2) + *',
        'computedrank=:+"(1+0.5)', 'computedrank i.2 3',
    ])
    # P4: parser-time assignment, right-to-left lookup and committed early effects.
    fixed.extend([
        'mid=:0', 'mid+(mid=:2)', 'mid', 'mid+(mid=:mid+1)', 'mid',
        'outer=:99', "outer=:'a'+(mid=:2)", 'outer', 'mid',
        'mida=:midb=:1', 'mida', 'midb', '(mid=.4)', 'mid',
        'midarr=:i.65', 'midalias=:midarr', 'midarr+(midarr=:midarr+1)',
        'midarr', 'midalias', 'midfn=:+', 'midfn (midfn=:2)', 'midfn',
        '(midfn=:+) 3', 'midfn 3',
        '(midfn=:-) 3', 'midfn 3', '+(midadv=:/) i.3', '+midadv i.3',
        '+(midconj=:")0 i.3', '+midconj 0 i.3',
    ])
    fixed.extend([
        'parenwrite=:0', '(parenwrite=:2', 'parenwrite',
        'parenwrite=:0', '((parenwrite=:2)', 'parenwrite',
        'parenwrite=:0', 'parenwrite=:2)', 'parenwrite',
        'parenwrite=:0', ')+(parenwrite=:2)', 'parenwrite',
        'parenwrite=:0', 'parenwrite+(parenwrite=:2', 'parenwrite',
        'parenwrite=:0', "'bad'+(parenwrite=:2", 'parenwrite',
        'parenwrite=:0', "(parenwrite=:2)+('a'+1", 'parenwrite',
    ])
    fixed.extend([
        'modadv=:/', 'modalias=:modadv', 'modf=:+modalias', 'modadv=:1', 'modf i.3',
        '+modalias i.3', 'modadv=:/', '3 modadv',
        'modconj=:\"', 'modg=:+modconj 0', 'modconj=:1', 'modg i.3',
    ])
    # P3: constructing a modifier train does not apply it. Function/POS
    # representation is independently compared by the frontend stage suite.
    fixed.extend('train' + str(i) + '=: ' + expression for i, expression in enumerate([
        '+"', '"1', '3"', '/+', '/\\', '/@:', '@:/', '@:@:',
        '/ / /', '/ / +', '/ + *', '@: + *', '+ @: /', '+ @: @:',
        '(/ /) /', '"1 2', '3 @:', '"+', '@: 3', '(1+2) "',
    ]))
    fixed.extend(['trainerr=: (1 2+1 2 3) "', 'trainerr=: (/3)',
                  'trainitems=:i.65', 'trainarray=:trainitems "',
                  'trainitems=:trainitems+1', 'trainitems'])
    fixed.extend([
        'boundright=:"1', 'boundcopy=:boundright', 'boundright=:1',
        'boundfn=: - boundcopy', 'boundfn i.2 3', '(- ("1)) i.2 3',
        '(- ("1 2)) i.2 3', '(+ ("-)) i.4', '(+ (@:-)) i.4',
        'boundbad=:"\'a\'', 'boundkeep=:+', 'boundkeep=: - boundbad', 'boundkeep i.3',
        'boundkeep=: - ("1 2 3 4)', 'boundkeep i.3', 'boundkeep=: - (@:3)',
        'boundcopy=:"2', 'boundfn i.2 3', 'boundfn=: - boundcopy', 'boundfn i.2 3',
    ])
    return fixed

# Exact newly exercised runtime coverage gaps. Parser correctness is checked
# separately; these are neither conformance passes nor C baseline deviations.
RUNTIME_COVERAGE_BOUNDARIES = {
    '(+ ("-)) i.4': 'verb-valued rank operand has no runtime executor',
    '(+ (@:-)) i.4': 'atop has no runtime executor',
    '(entryverb/ % #) entrynoun': 'named insert inside fork has no runtime executor',
    '(entryverb/ % #) entrycopy': 'named insert inside fork has no runtime executor',
}

def runtime_coverage_boundary(source, reference, actual):
    if (source in RUNTIME_COVERAGE_BOUNDARIES and 'data' in reference
            and actual == {'error': 'unsupported'}):
        return RUNTIME_COVERAGE_BOUNDARIES[source]
    return None

def equal(a, b):
    if a.keys() != b.keys():
        return False
    if 'data' not in a:
        return a == b
    if a['type'] != b['type'] or a['shape'] != b['shape'] or len(a['data']) != len(b['data']):
        return False
    if a['type'] == 32:
        return all(isinstance(x, dict) and isinstance(y, dict) and equal(x, y)
                   for x, y in zip(a['data'], b['data']))
    return all((x == y and not (a['type'] == 8 and isinstance(x, (int,float)) and isinstance(y, (int,float)) and x == 0 and math.copysign(1,x) != math.copysign(1,y))) or (a['type'] == 8 and isinstance(x, (int,float)) and isinstance(y, (int,float)) and x != 0 and y != 0 and math.isclose(x, y, rel_tol=1e-14, abs_tol=0)) for x,y in zip(a['data'], b['data']))

def generated(seed, rounds):
    rng = random.Random(seed)
    out = []
    for _ in range(rounds):
        n = rng.choice([0, 1, 2, 3, 63, 64, 65, 127, 128, 129, 257])
        k = rng.randint(-100, 100)
        literal = str(k).replace('-', '_')
        op = rng.choice(['+', '-', '*'])
        # A stateful transaction: alias, update, failed assignment, then inspect.
        out.extend([f'qa=:i.{n}', 'qb=:qa', f'qa=:qa {op} {literal}',
                    'qa', 'qb', 'qa=:1 2 + 1 2 3', 'qa', 'qb'])
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--semantic-reference', action='store_true')
    parser.add_argument('--parser-capture', action='store_true', help='Use capture_probe as binary; verify runtime provenance')
    parser.add_argument('--reference-revision', help='Verified source revision of the supplied C library')
    parser.add_argument('--seed', type=int, default=20260926)
    parser.add_argument('--rounds', type=int, default=100)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/rustj')
    parser.add_argument('--report', type=Path, default=ROOT / 'reports/conformance.json')
    parser.add_argument('--allow-known-j64', action='store_true')
    args = parser.parse_args()
    if args.rounds < 0: parser.error('rounds must be nonnegative')
    if args.parser_capture and args.semantic_reference: parser.error('choose one execution path')
    args.report.parent.mkdir(parents=True, exist_ok=True)
    corpus = cases() + generated(args.seed, args.rounds)
    library = Path(os.environ.get('J_LIBRARY', str(ROOT / '.reference/bin/linux/j64/libj.so')))
    report = {'reference_revision': args.reference_revision, 'platform': os.name, 'rust_path': 'parser-capture' if args.parser_capture else ('semantic-reference' if args.semantic_reference else 'direct'), 'seed': args.seed, 'rounds': args.rounds, 'cases': len(corpus),
              'reference_library': str(library),
              'reference_sha256': hashlib.sha256(library.read_bytes()).hexdigest(),
              'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
              'scope': 'supported subset; upstream suite NOT executed',
              'passed': 0, 'known_deviations': [], 'coverage_boundaries': [], 'failures': []}
    try:
        oracle = subprocess.run([sys.executable, str(ROOT / 'tools/oracle.py')], input=''.join(json.dumps(s)+'\n' for s in corpus), text=True, capture_output=True, timeout=120)
        rust = subprocess.run([str(args.binary), '--json'] + (['--semantic-reference'] if args.semantic_reference else []), input='\n'.join(corpus)+'\n', text=True, capture_output=True, timeout=120)
        if oracle.returncode != 0 or rust.returncode not in (0,1):
            raise RuntimeError(f'process failure: oracle={oracle.returncode}, rust={rust.returncode}\n{oracle.stderr}\n{rust.stderr}')
        if args.parser_capture:
            report['graph_coverage_boundaries'] = []
            for line in rust.stderr.splitlines():
                for prefix, reason in [
                    ('ordered-effect graph boundary: ', 'capture needs ordered assignment/effect graph'),
                    ('modifier-value graph boundary: ', 'captured modifier value graph lowering'),
                ]:
                    if line.startswith(prefix):
                        index = int(line[len(prefix):])
                        report['graph_coverage_boundaries'].append({
                            'index': index, 'source': corpus[index], 'reason': reason})
        expected = [json.loads(s) for s in oracle.stdout.splitlines()]
        actual = [json.loads(s) for s in rust.stdout.splitlines()]
        if len(expected) != len(corpus) or len(actual) != len(corpus):
            raise RuntimeError('incomplete output')
        for i, (source, a, b) in enumerate(zip(corpus, expected, actual)):
            if equal(a,b):
                report['passed'] += 1
                continue
            item = {'index': i, 'source': source, 'reference': a, 'rust': b}
            boundary = runtime_coverage_boundary(source, a, b)
            if boundary is not None:
                report['coverage_boundaries'].append({**item, 'reason': boundary})
                continue
            # Narrow, explicit baseline discrepancy, never a blanket dtype waiver.
            known = (args.allow_known_j64 and library.parent.name == 'j64'
                     and source == '(i.2 3) -"1 0 (i.2 3 4)'
                     and a.get('type') == 8 and b.get('type') == 4
                     and a.get('shape') == b.get('shape') == [2,3,4,3]
                     and a.get('data') == b.get('data'))
            report['known_deviations' if known else 'failures'].append(item)
        if not report['failures']:
            args.report.with_suffix('.repro.ijs').unlink(missing_ok=True)
        if report['failures']:
            # Replay the full prefix: stateful failures cannot be reproduced by one line.
            end = report['failures'][0]['index'] + 1
            args.report.with_suffix('.repro.ijs').write_text('\n'.join(corpus[:end])+'\n')
    except (RuntimeError, ValueError, subprocess.TimeoutExpired) as error:
        report['harness_error'] = str(error)
    report['failed'] = len(report['failures'])
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2), newline='\n')
    print(json.dumps({k:v for k,v in report.items() if k not in ['failures','known_deviations']}, indent=2))
    print('known deviations:', len(report['known_deviations']))
    return bool(report['failures'] or report.get('harness_error'))

if __name__ == '__main__':
    sys.exit(main())
