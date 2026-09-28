"""Audit C name-part-of-speech behavior; NOT a Rust conformance pass.

Run separately for each pinned C library using J_LIBRARY. A successful audit
only confirms the C expectations below; Rust function values remain pending.
"""
import json
import os
from pathlib import Path
from oracle import Oracle, ROOT

CASES = [
    ('missing', None, 3),
    ('1+missing', None, 3),
    ('missing+1', 'value error', -1),
    ('(1 2+1 2 3)+missing', 'length error', -1),
    ('1 2+1 2 3+missing', None, 3),
    ('missing+(1 2+1 2 3)', 'length error', -1),
    ('(1+2)+missing', None, 3),
]

def audit():
    rows = []
    for source, expected_error, expected_class in CASES:
        oracle = Oracle()
        try:
            error = oracle.run('probe=:' + source)
            classification = oracle.eval("4!:0 <'probe'")['data'][0]
            observed_error = error['error'] if error else None
            rows.append(dict(source=source, error=observed_error,
                             name_class=classification,
                             matches_c_expectation=(observed_error == expected_error
                                                    and classification == expected_class)))
        finally:
            oracle.close()
    for setup, source, expected in [
        (['a=:1', 'b=:a', 'a=:2'], 'b', 1),
        (['f=:+', 'g=:f', 'f=:*'], 'g 3', 1),
        (['g=:unknownverb', 'unknownverb=:-'], 'g 3', -3),
    ]:
        oracle = Oracle()
        try:
            errors = [oracle.run(sentence) for sentence in setup]
            result = oracle.eval(source)
            rows.append(dict(setup=setup, source=source, result=result,
                             matches_c_expectation=(all(e is None for e in errors)
                                                    and result.get('shape') == []
                                                    and result.get('data') == [expected])))
        finally:
            oracle.close()
    return rows

if __name__ == '__main__':
    import hashlib
    library = Path(os.environ.get('J_LIBRARY', ROOT / '.reference/bin/linux/j64/libj.so'))
    rows = audit()
    print(json.dumps(dict(scope='C behavior audit, not Rust conformance',
                          library=str(library), sha256=hashlib.sha256(library.read_bytes()).hexdigest(),
                          cases=rows), indent=2))
    raise SystemExit(0 if all(r['matches_c_expectation'] for r in rows) else 1)
