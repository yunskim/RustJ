#!/usr/bin/env python3
"""Build a pinned, separate C oracle on Linux; never linked into RustJ."""
import argparse
import io
import json
import os
from pathlib import Path
import subprocess
import zipfile

PIN = 'e75016ca74b5e595dd323226e6a4990172f72ec6'
ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser()
p.add_argument('--source', type=Path, default=ROOT.parent / 'jsource-inspection')
p.add_argument('--variant', choices=['j64','j64avx2'], default='j64')
args = p.parse_args()
commit = subprocess.check_output(['git', '-C', str(args.source), 'rev-parse', 'HEAD'], text=True).strip()
if commit != PIN:
    raise SystemExit(f'Expected {PIN}, found {commit}')
dest = ROOT / '.reference'
dest.mkdir(exist_ok=True)
archive = subprocess.check_output(['git', '-C', str(args.source), 'archive', '--format=zip', PIN])
with zipfile.ZipFile(io.BytesIO(archive)) as z:
    z.extractall(dest)
for f in (dest / 'make2').glob('*.sh'):
    f.write_bytes(f.read_bytes().replace(b'\r\n', b'\n'))
    f.chmod(0o755)
(dest / 'jsrc/jversion.h').write_bytes((dest / 'jsrc/jversion-x.h').read_bytes())
env = dict(os.environ, jplatform='linux', j64x=args.variant, CC='gcc',
           USE_PYXES='0', USE_SLEEF='0', USE_SLEEFQUAD='0',
           USE_EMU_AVX='0', MAKEFLAGS='-j2')
with (dest / 'build.log').open('w') as log:
    subprocess.run(['bash', 'build_libj.sh'], cwd=dest / 'make2', env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
manifest=json.dumps({'commit': commit, 'platform': 'linux', 'variant': args.variant, 'compiler': subprocess.check_output(['gcc', '--version'], text=True).splitlines()[0], 'features': {k: env[k] for k in ['USE_PYXES', 'USE_SLEEF', 'USE_SLEEFQUAD', 'USE_EMU_AVX']}}, indent=2)
(dest / f'manifest-{args.variant}.json').write_text(manifest)
if args.variant=='j64': (dest / 'manifest.json').write_text(manifest)
print(dest / f'bin/linux/{args.variant}/libj.so')
