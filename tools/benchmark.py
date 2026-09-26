#!/usr/bin/env python3
"""Reproducible baseline, not a claim of overall superiority over J."""
import hashlib
import json
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]

def reference():
    from oracle import Oracle
    j = Oracle()
    rows = []
    try:
        for n in (16, 100_000, 1_000_000):
            iterations = 10_000 if n < 100 else 20
            assert j.run(f'a =: i. {n}') is None
            for _ in range(5):
                assert j.run('r =: a + 2') is None
            start = time.perf_counter_ns()
            for _ in range(iterations):
                assert j.run('r =: a + 2') is None
            rows.append({'workload': 'bound_add', 'n': n, 'iterations': iterations,
                         'us': (time.perf_counter_ns()-start)/1000/iterations})
    finally:
        j.close()
    return rows

def main():
    if '--reference-worker' in sys.argv:
        print(json.dumps(reference()))
        return
    subprocess.run(['cargo', 'bench', '--bench', 'baseline', '--no-run'], cwd=ROOT, check=True)
    runs, references = [], []
    for _ in range(3):
        out = subprocess.check_output(['cargo', 'bench', '--bench', 'baseline'], cwd=ROOT, text=True)
        runs.append([json.loads(s) for s in out.splitlines() if s.startswith('{')])
        references.append(json.loads(subprocess.check_output([sys.executable, __file__, '--reference-worker'], text=True)))
    rust = []
    for i, row in enumerate(runs[0]):
        row = dict(row)
        row['us'] = statistics.median(run[i]['us'] for run in runs)
        rust.append(row)
    c = []
    for i, row in enumerate(references[0]):
        row = dict(row)
        row['us'] = statistics.median(run[i]['us'] for run in references)
        c.append(row)
    cpu = next((s.split(':',1)[1].strip() for s in Path('/proc/cpuinfo').read_text().splitlines() if s.startswith('model name')), 'unknown')
    report = {
        'platform': platform.platform(), 'cpu': cpu,
        'rustc': subprocess.check_output(['rustc','--version'],text=True).strip(),
        'reference': json.loads((ROOT/'.reference/manifest.json').read_text()),
        'source_sha256': {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((ROOT/'src').glob('*.rs'))},
        'method': 'median of 3 runs; warmed; process startup and output excluded; generic release build, no target-cpu=native',
        'limitations': ['C bound_add includes Python ctypes/JDo call overhead; Rust uses in-process Engine::eval.', 'C reference is scalar j64, without PYXES/SLEEF/EMU_AVX; not the fastest upstream configuration.', 'Owned_add measures consuming the Rust value directly; no equivalent C row.', 'Rust allocation counts are one extra warmed operation, not averaged timing; bytes include metadata. C allocation counts are not measured.', 'Only integer addition is benchmarked; results do not generalize to the complete language.'],
        'rust': rust, 'reference_c': c,
    }
    (ROOT/'reports').mkdir(exist_ok=True)
    (ROOT/'reports/benchmark.json').write_text(json.dumps(report,indent=2))
    print(json.dumps(report,indent=2))

if __name__ == '__main__':
    main()
