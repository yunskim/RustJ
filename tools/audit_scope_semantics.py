"""Pinned C scope audit. These checks do not assert RustJ support."""
from oracle import Oracle
import json

CASES = [
    (['a=:10', "f=:3 : 'a=.y'", 'r=:f 3'], [('r', [3]), ('a', [10])]),
    (['a=:10', "f=:3 : 'a=:y'", 'r=:f 3'], [('r', [3]), ('a', [3])]),
    (['outside=.7'], [('outside', [7])]),
    (["f=:3 : 'private=.y'", 'r=:f 3'], [("4!:0 <'private'", [-1])]),
    (['a_scopeprobe_=:5', "loc=:<'scopeprobe'"], [('a_scopeprobe_', [5]), ('a__loc', [5])]),
]

if __name__ == '__main__':
    passed = 0
    for setup, reads in CASES:
        oracle = Oracle()
        try:
            for sentence in setup:
                result = oracle.run(sentence)
                if result is not None:
                    raise RuntimeError((sentence, result))
            for sentence, expected in reads:
                result = oracle.eval(sentence)
                if result.get('shape') != [] or result.get('data') != expected:
                    raise RuntimeError((sentence, result, expected))
                passed += 1
        finally:
            oracle.close()
    print(json.dumps({'scope': 'C scope audit, not Rust conformance', 'passed': passed}))
