#!/usr/bin/env python3
"""Same-process native calls, default allocator, fixed CPU; no concurrent builds."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess

ROOT=Path(__file__).resolve().parents[1]
def main():
    cpu=min(os.sched_getaffinity(0))
    os.sched_setaffinity(0,{cpu})
    env=dict(os.environ,J_LIBRARY=str(ROOT/'.reference/bin/linux/j64avx2/libj.so'))
    results={}
    for label,cwd in [('before',ROOT/'.baseline'),('after',ROOT)]:
        print(f'Measuring {label} on CPU {cpu} against C AVX2...',flush=True)
        out=subprocess.check_output(['cargo','bench','--bench','comparison'],cwd=cwd,env=env,text=True,timeout=300)
        results[label]=[json.loads(line) for line in out.splitlines() if line.startswith('{')]
        if len(results[label])!=25:raise RuntimeError('incomplete benchmark')
        (ROOT/f'reports/comparison-{label}.jsonl').write_text(out)
    report={
        'timestamp_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'platform':platform.platform(),'cpu_affinity':cpu,
        'cpu':next(s.split(':',1)[1].strip() for s in Path('/proc/cpuinfo').read_text().splitlines() if s.startswith('model name')),
        'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),
        'reference_commit':'e75016ca74b5e595dd323226e6a4990172f72ec6',
        'reference_build':'gcc 13.3.0; j64avx2; -O2; PYXES/SLEEF/SLEEFQUAD/EMU_AVX=0',
        'reference_sha256':hashlib.sha256(Path(env['J_LIBRARY']).read_bytes()).hexdigest(),
        'source_sha256':{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((ROOT/'src').glob('*.rs'))},
        'baseline_archive_sha256':hashlib.sha256((ROOT/'dist/rustj-m1-source.tar.gz').read_bytes()).hexdigest(),
        'method':'default allocator (no counting wrapper), same process Rust eval vs native JDo; identical assignments; 4 warmups; 7 rounds alternating engine order; median/min/max; setup/compilation/output excluded; pinned CPU; no other agent build/test running',
        'limitations':['WSL2 and uncontrolled system load/frequency; one CPU model.', 'End-to-end engine workloads, including parsing, allocation and assignment; not isolated arithmetic throughput.', 'C build has AVX2 but is not an exhaustive search of compiler/build settings.', 'Rust implements a much smaller language subset.', 'Large size allocation behavior can dominate; do not attribute speed ratios entirely to SIMD.'],
        **results,
    }
    (ROOT/'reports/comparison.json').write_text(json.dumps(report,indent=2))
    for a,b in zip(results['before'],results['after']):
        print(f"{b['workload']:20s} n={b['n']:7d} before={a['rust_us']:9.3f} after={b['rust_us']:9.3f} C={b['c_us']:9.3f} us; improvement={a['rust_us']/b['rust_us']:.2f}x; C/Rust={b['c_us']/b['rust_us']:.2f}x")

if __name__=='__main__':main()
