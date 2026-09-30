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
          13: 'open quote', 14: 'rank error', 19: 'syntax error', 21: 'value error'}

PARSER_OBSERVE_OPS = {'eval', 'sentence', 'name_class', 'representation', 'words'}

def normalize_request(request):
    """Normalize one JSON-lines oracle request.

    Backward-compatible strings mean {"op":"eval","source":...}.  Parser
    conformance uses object requests so it can observe J name class and
    representations without coupling RustJ to jsource internals.
    """
    if isinstance(request, str):
        return {'op': 'eval', 'source': request}
    if not isinstance(request, dict):
        raise ValueError('oracle request must be a string or object')
    op = request.get('op')
    if op not in PARSER_OBSERVE_OPS:
        raise ValueError(f'unsupported oracle op: {op!r}')
    out = dict(request)
    if op in {'eval', 'sentence', 'words'}:
        if not isinstance(out.get('source'), str):
            raise ValueError(f'{op} requires string source')
    if op in {'name_class', 'representation'}:
        if not isinstance(out.get('name'), str) or not out['name']:
            raise ValueError(f'{op} requires nonempty string name')
    if op == 'sentence':
        names = out.get('names', [])
        if not isinstance(names, list) or not all(isinstance(n, str) and n for n in names):
            raise ValueError('sentence names must be a list of nonempty strings')
        out['names'] = names
        reps = out.get('representations', [])
        if not isinstance(reps, list) or not all(r in {'atomic', 'linear'} for r in reps):
            raise ValueError('representations must contain only atomic/linear')
        out['representations'] = reps
    if op == 'representation' and out.get('kind') not in {'atomic', 'linear'}:
        raise ValueError('representation kind must be atomic or linear')
    return out

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
        return self.read_noun('rustjresult')

    @staticmethod
    def _quote_name(name):
        return "'" + name.replace("'", "''") + "'"

    def _eval_probe_noun(self, expression):
        error = self.run('rustjprobe =: ' + expression)
        if error:
            return error
        return self.read_noun('rustjprobe')

    def name_class(self, name):
        observed = self._eval_probe_noun('4!:0 <' + self._quote_name(name))
        if 'error' in observed:
            return observed
        if observed.get('type') != 4 or observed.get('shape') != [] or len(observed.get('data', [])) != 1:
            raise RuntimeError(f'unexpected 4!:0 result: {observed!r}')
        return {'class': observed['data'][0]}

    def representation(self, name, kind):
        foreign = '5!:1' if kind == 'atomic' else '5!:5'
        observed = self._eval_probe_noun(foreign + ' <' + self._quote_name(name))
        if 'error' in observed:
            return observed
        return {'kind': kind, 'value': observed}

    def words(self, source):
        # Reconstruct the exact UTF-8 byte string as a J literal noun instead
        # of interpolating source text into J code. This keeps quotes, LF and
        # other word-forming bytes observable by ;: exactly as supplied.
        raw = source.encode('utf-8')
        indexes = ' '.join(str(b) for b in raw)
        expression = "''" if not raw else f'({indexes}) {{ a.'
        error = self.run('rustjsource =: ' + expression)
        if error:
            return error
        error = self.run('rustjwords =: ;: rustjsource')
        if error:
            return error
        observed = self.read_noun('rustjwords')
        if observed.get('type') != 32:
            raise RuntimeError(f'unexpected ;: result: {observed!r}')
        words_hex = []
        for item in observed['data']:
            if item.get('type') != 2 or len(item.get('shape', [])) != 1:
                raise RuntimeError(f'unexpected ;: word: {item!r}')
            words_hex.append(bytes(item['data']).hex())
        return {'words_hex': words_hex}

    def observe(self, request):
        request = normalize_request(request)
        op = request['op']
        if op == 'eval':
            return self.eval(request['source'])
        if op == 'name_class':
            return self.name_class(request['name'])
        if op == 'representation':
            return self.representation(request['name'], request['kind'])
        if op == 'words':
            return self.words(request['source'])

        outcome = self.eval(request['source'])
        names = {}
        for name in request['names']:
            info = self.name_class(name)
            entry = {'class': info.get('class')} if 'class' in info else {'class_error': info['error']}
            if 'class' in info and info['class'] in (1, 2, 3):
                for kind in request['representations']:
                    rep = self.representation(name, kind)
                    entry[kind] = rep.get('value') if 'value' in rep else {'error': rep['error']}
            names[name] = entry
        return {'outcome': outcome, 'names': names}

    def read_noun(self, name, depth=0):
        if depth > 128:
            raise RuntimeError('Oracle boxed nesting limit')
        t, rank, shape_ptr, data_ptr = (C.c_int64() for _ in range(4))
        err = self.lib.JGetM(self.jt, name.encode('ascii'), C.byref(t), C.byref(rank), C.byref(shape_ptr), C.byref(data_ptr))
        if err:
            raise RuntimeError(f'JGetM: {err}, noun={name!r}')
        shape = list((C.c_int64 * rank.value).from_address(shape_ptr.value))
        n = math.prod(shape)
        if t.value == 32:
            data = []
            child = f'rustjboxread{depth}'
            for i in range(n):
                error = self.run(f'{child} =: > {i} {{ , {name}')
                if error:
                    raise RuntimeError(f'Oracle box extraction: {error}')
                data.append(self.read_noun(child, depth + 1))
            return {'type': 32, 'shape': shape, 'data': data}
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
            try:
                request = json.loads(line)
                response = oracle.observe(request)
            except (ValueError, TypeError) as error:
                response = {'harness_error': str(error)}
            print(json.dumps(response), flush=True)
    finally:
        oracle.close()
