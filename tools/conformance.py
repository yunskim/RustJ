#!/usr/bin/env python3
"""Deterministic differential checks of the supported M1/M2 subset."""
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
        '0', '1', '2', '_3', '1 0 1', '1 2 3', '1.5 2 3', '_', '__', '_.',
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
    return fixed

def equal(a, b):
    if a.keys() != b.keys():
        return False
    if 'data' not in a:
        return a == b
    if a['type'] != b['type'] or a['shape'] != b['shape'] or len(a['data']) != len(b['data']):
        return False
    return all(x == y or (a['type'] == 8 and isinstance(x, (int,float)) and isinstance(y, (int,float)) and math.isclose(x, y, rel_tol=1e-14, abs_tol=0)) for x,y in zip(a['data'], b['data']))

def main():
    corpus = cases()
    oracle = subprocess.run([sys.executable, str(ROOT / 'tools/oracle.py')], input=''.join(json.dumps(s)+'\n' for s in corpus), text=True, capture_output=True, timeout=120)
    rust = subprocess.run([str(ROOT / 'target/release/rustj'), '--json'], input='\n'.join(corpus)+'\n', text=True, capture_output=True, timeout=120)
    if oracle.returncode != 0 or rust.returncode not in (0,1):
        raise RuntimeError(f'process failure: oracle={oracle.returncode}, rust={rust.returncode}\n{oracle.stderr}\n{rust.stderr}')
    expected = [json.loads(s) for s in oracle.stdout.splitlines()]
    actual = [json.loads(s) for s in rust.stdout.splitlines()]
    if len(expected) != len(corpus) or len(actual) != len(corpus):
        raise RuntimeError(f'incomplete run: cases={len(corpus)} reference={len(expected)} rust={len(actual)}')
    failures = [{'source': s, 'reference': a, 'rust': b} for s,a,b in zip(corpus, expected, actual) if not equal(a,b)]
    report = {'reference_library': os.environ.get('J_LIBRARY', str(ROOT / '.reference/bin/linux/j64/libj.so')), 'cases': len(corpus), 'passed': len(corpus)-len(failures), 'failed': len(failures), 'skipped': 0, 'seed': 20260926, 'scope': 'M1/M2 curated + generated cases; not the upstream suite', 'failures': failures}
    (ROOT / 'reports').mkdir(exist_ok=True)
    (ROOT / 'reports/conformance.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return bool(failures)

if __name__ == '__main__':
    sys.exit(main())
