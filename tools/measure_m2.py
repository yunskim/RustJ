#!/usr/bin/env python3
"""Linux M2 comparison. Builds first; measures sequentially on one CPU.

The local baseline worktree must point to 7add070. It is intentionally excluded
from the repository. This script adds only the identical layout harness there.
"""
import datetime
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / '.m2-baseline'
REV = '7add0706c652cbc935931db40a257dd80f5727a4'

def run(args, cwd=ROOT, **kwargs):
    return subprocess.check_output(args, cwd=cwd, text=True, **kwargs)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--prefix', default='m2', choices=['m2','m2-review'])
    prefix = parser.parse_args().prefix
    if not BASE.exists():
        run(['git', 'worktree', 'add', '--detach', str(BASE), REV])
    if run(['git', 'rev-parse', 'HEAD'], cwd=BASE).strip() != REV:
        raise RuntimeError('Unexpected baseline revision')
    (BASE / 'benches/layout.rs').write_bytes((ROOT / 'benches/layout.rs').read_bytes())
    manifest = BASE / 'Cargo.toml'
    if 'name = "layout"' not in manifest.read_text():
        with manifest.open('a') as f:
            f.write('\n[[bench]]\nname = "layout"\nharness = false\n')
    env = dict(os.environ, J_LIBRARY=str(ROOT / '.reference/bin/linux/j64avx2/libj.so'))
    if not Path(env['J_LIBRARY']).exists():
        raise RuntimeError('Build the pinned j64avx2 reference first')
    executables = {}
    for label, cwd in [('before', BASE), ('after', ROOT)]:
        for bench in ['layout', 'baseline', 'comparison']:
            messages = run(['cargo', 'bench', '--bench', bench, '--no-run', '--message-format=json'], cwd=cwd)
            for line in messages.splitlines():
                m = json.loads(line)
                if m.get('executable') and m.get('target', {}).get('name') == bench:
                    executables[label, bench] = m['executable']
    cpu = min(os.sched_getaffinity(0))
    os.sched_setaffinity(0, {cpu})
    results = {}
    # Reverse the order in pass two to expose time/order-dependent differences.
    for trial, labels in [(1, ['before','after']), (2, ['after','before'])]:
        for bench in ['layout','baseline','comparison']:
            for label in labels:
                raw = run([executables[label, bench]], env=env)
                key = f'{label}-{bench}-{trial}'
                (ROOT / 'reports' / f'{prefix}-{key}.jsonl').write_text(raw)
                results[key] = [json.loads(s) for s in raw.splitlines() if s.startswith('{')]
                print(key, 'completed', flush=True)
    report = {
        'timestamp_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'baseline_commit': REV, 'platform': platform.platform(), 'cpu_affinity': cpu,
        'rustc': run(['rustc','--version']).strip(),
        'source_sha256': {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
                          for p in sorted((ROOT/'src').glob('*.rs'))},
        'harness_sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                           for p in sorted((ROOT/'benches').glob('*.rs'))},
        'reference_sha256': hashlib.sha256(Path(env['J_LIBRARY']).read_bytes()).hexdigest(),
        'notes': ['layout/comparison timings uninstrumented; baseline timings use an allocation wrapper',
                  'Two passes in opposite version order; WSL2 load/frequency not fully controlled',
                  'One CPU; no CUDA measurements'],
        'results': results,
    }
    (ROOT/'reports'/f'{prefix}-measurements.json').write_text(json.dumps(report,indent=2))

if __name__ == '__main__':
    main()
