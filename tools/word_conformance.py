#!/usr/bin/env python3
"""Local word-formation oracle; does not evaluate the tested program."""
import argparse, hashlib, json, os, random, subprocess
from pathlib import Path
from oracle import Oracle
ROOT=Path(__file__).resolve().parents[1]
def main():
    p=argparse.ArgumentParser()
    suffix='.exe' if os.name=='nt' else ''
    p.add_argument('--binary',type=Path,default=ROOT/f'target/release/examples/scan_words{suffix}')
    p.add_argument('--seed',type=int,default=20260927)
    p.add_argument('--rounds',type=int,default=2000)
    p.add_argument('--reference-revision', help='Verified source revision of the supplied C library')
    p.add_argument('--report',type=Path,default=Path('/tmp/rustj-words.json'))
    args=p.parse_args()
    if not args.binary.exists():
        build=subprocess.run(['cargo','build','--quiet','--release','--example','scan_words'],cwd=ROOT,text=True,capture_output=True)
        if build.returncode: raise RuntimeError('scan_words build failed:\n'+build.stdout+'\n'+build.stderr)
    prefixes=[b'',b' ',b'1 ',b'+',b'a',b'N',b'NB',b"''",b'1',b'1 2',b"'",b'NB.',b'NB. x',b'\n',b'{',b'}',b'{{',b'}}']
    samples=[prefix+bytes([c]) for prefix in prefixes for c in range(256)]
    samples += [b'1 NB.. 2',b'1 NB.: 2',b'1 2: 3',b'{{.',b'}}:',b'1\n2',b"'it''s'",b'foo_bar=:2',b'if. x do. y end.',b'1r2 2j3', b'[. ]. ]:', b'/.. $:: F. F.. F.: F: F:. F::', b'c. f: t. T. m. Z:', b'@ @: &. &.:', b'd. D. D: t: .. .:']
    rng=random.Random(args.seed)
    alphabet=b" NB09a_.'\t\n:{}+()"
    samples += [bytes(rng.choice(alphabet) for _ in range(rng.randrange(50))) for _ in range(args.rounds)]
    rust=subprocess.run([str(args.binary)],input=''.join(s.hex()+'\n' for s in samples),text=True,capture_output=True,timeout=120,check=True)
    actual=[json.loads(s) for s in rust.stdout.splitlines()]
    if len(actual)!=len(samples): raise RuntimeError('incomplete scanner output')
    o=Oracle();failures=[];opened=0
    try:
        for source,got in zip(samples,actual):
            chars=' '.join(map(str,source)) if source else 'i.0'
            # Byte construction avoids quoting, newline, NUL and =: detection traps.
            error=o.run('auditwords=: ;: a. {~ , ('+chars+')')
            if error:
                # C error 13 is open quote; do not swallow unrelated oracle errors.
                if error['error'] != 'open quote':
                    raise RuntimeError((source.hex(),error))
                opened+=1
                if 'error' not in got: failures.append({'hex':source.hex(),'expected':'open quote','actual':got})
                continue
            sizes=o.eval('#&> auditwords')['data']
            data=bytes(o.eval(';auditwords')['data'])
            words=[];pos=0
            for size in sizes:words.append(data[pos:pos+size]);pos+=size
            expected=[];offset=0
            for word in words:
                start=source.index(word,offset);expected.append([start,start+len(word)]);offset=start+len(word)
            if got.get('spans')!=expected: failures.append({'hex':source.hex(),'expected':expected,'actual':got})
    finally:o.close()
    library=Path(os.environ.get('J_LIBRARY', str(ROOT / '.reference/bin/linux/j64/libj.so')))
    report={'jsource_revision':args.reference_revision,'platform':os.name,'reference_sha256':hashlib.sha256(library.read_bytes()).hexdigest(),'binary_sha256':hashlib.sha256(args.binary.read_bytes()).hexdigest(),'cases':len(samples),'seed':args.seed,'open_quotes':opened,'failed':len(failures),'zero_mismatch':not failures,'failures':failures,'reference':os.environ.get('J_LIBRARY')}
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report,indent=2))
    print(json.dumps({k:v for k,v in report.items() if k!='failures'},indent=2))
    if failures:
        print(json.dumps({"first_failures": failures[:50]}, indent=2))
    return bool(failures)
if __name__=='__main__':raise SystemExit(main())
