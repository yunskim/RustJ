#!/usr/bin/env python3
"""Separate-process adapter for the pinned J engine. JSON lines in/out."""
import ctypes as C
import json
import math
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
ERRORS = {16: 'spelling error', 3: 'domain error', 6: 'index error', 9: 'length error', 10: 'limit error',
          14: 'rank error', 19: 'syntax error', 21: 'value error'}

class Oracle:
    def __init__(self):
        self.lib = C.CDLL(os.environ.get('J_LIBRARY',str(ROOT / '.reference/bin/linux/j64/libj.so')))
        self.lib.JInit.restype = C.c_void_p
        self.lib.JDo.argtypes = [C.c_void_p, C.c_char_p]
        self.lib.JFree.argtypes = [C.c_void_p]
        self.lib.JGetM.argtypes = [C.c_void_p, C.c_char_p] + [C.POINTER(C.c_int64)] * 4
        self.jt = self.lib.JInit()
        if not self.jt:
            raise RuntimeError('JInit failed')

    def close(self):
        if self.jt:
            self.lib.JFree(self.jt)
            self.jt = None

    def run(self, source):
        code = self.lib.JDo(self.jt, source.encode('utf-8'))
        if code:
            return {'error': ERRORS.get(code, f'J error {code}')}
        return None

    def eval(self, source):
        # Corpus uses one top-level assignment per line; no assignment-in-string.
        if '=:' in source:
            return self.run(source) or {'silent': True}
        error = self.run('rustjresult =: ' + source)
        if error:
            return error
        t, rank, shape_ptr, data_ptr = (C.c_int64() for _ in range(4))
        err = self.lib.JGetM(self.jt, b'rustjresult', C.byref(t), C.byref(rank), C.byref(shape_ptr), C.byref(data_ptr))
        if err:
            raise RuntimeError(f'JGetM: {err}, source={source!r}')
        shape = list((C.c_int64 * rank.value).from_address(shape_ptr.value))
        n = math.prod(shape)
        elem = {1: C.c_uint8, 2: C.c_uint8, 4: C.c_int64, 8: C.c_double}.get(t.value)
        if elem is None:
            raise RuntimeError(f'Unexpected oracle type {t.value}')
        data = list((elem * n).from_address(data_ptr.value))
        data = [('nan' if math.isnan(x) else 'inf' if x > 0 else '-inf') if isinstance(x, float) and not math.isfinite(x) else x for x in data]
        return {'type': t.value, 'shape': shape, 'data': data}

if __name__ == '__main__':
    oracle = Oracle()
    try:
        for line in sys.stdin:
            print(json.dumps(oracle.eval(json.loads(line))), flush=True)
    finally:
        oracle.close()
