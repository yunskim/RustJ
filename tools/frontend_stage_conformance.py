#!/usr/bin/env python3
"""Stage projections, not C pointer layouts. Run on native Windows with J_LIBRARY.

cases[] is the tacit-translator table: exhaustive matching is a declarative
eligibility check, NOT a proof of runtime ptcol dispatch or action sequencing.
"""
import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import platform
import random
import re
import subprocess

from oracle import Oracle
from conformance import equal as noun_equal

CLASSES = ['Noun', 'Verb', 'Adverb', 'Conjunction', 'Name', 'Assignment', 'LParen', 'RParen', 'Mark']
C_CLASSES = dict(zip(['NOUN', 'VERB', 'ADV', 'CONJ', 'NAME', 'ASGN', 'LPAR', 'RPAR', 'MARK'], CLASSES))
PENDING = {
    'enqueue': ['complex/extended/rational numeric notation', 'locatives and name _:', 'complete spellin/ds inventory', 'tacit translator env=0 and locative copula upgrade'],
    'parser': ['runtime ptcol reachable-state trace equivalence', 'full local/locale assignment and right-to-left effect coverage', 'full computed noun modifier operands', 'derived modifier application and named derived identities', 'explicit/direct definitions', 'full row provenance and reinsertion trace'],
}


def source_rows(source):
    """Read the actual C table, reject malformed/unrecognized expressions."""
    source = re.sub(r'/\*.*?\*/|//[^\n]*', '', source, flags=re.S)
    macros = {'AVN': 'ADV+VERB+NOUN', 'CAVN': 'CONJ+ADV+VERB+NOUN', 'EDGE': 'MARK+ASGN+LPAR'}
    for name, expression in macros.items():
        found = re.search(r'^\s*#define\s+' + name + r'\s+\(([^)]+)\)', source, re.M)
        if not found or re.sub(r'\s+', '', found[1]) != expression:
            raise ValueError('C macro contract changed: ' + name)
    table = re.search(r'PT\s+cases\[\]\s*=\s*\{(.*?)\};', source, re.S)
    if not table:
        raise ValueError('C cases table missing')
    def expand(expression):
        names = expression.strip().split('+')
        out = set()
        for name in names:
            name = name.strip()
            if name == 'ANY':
                out.update(CLASSES)
            elif name in macros:
                out.update(expand(macros[name]))
            elif name in C_CLASSES:
                out.add(C_CLASSES[name])
            else:
                raise ValueError('unknown C table token: ' + name)
        return out
    fields = [x.strip() for x in table[1].split(',') if x.strip()]
    if len(fields) != 81:
        raise ValueError('expected nine rows of nine fields')
    return [[expand(x) for x in fields[i:i+4]] for i in range(0, 81, 9)]


def expected_row(rows, classes):
    return next((i for i, row in enumerate(rows) if all(c in allowed for c, allowed in zip(classes, row))), None)


def atomic_node(value):
    """Decode only documented atomic function shapes; keep noun wrappers typed."""
    if value.get('type') == 2 and len(value.get('shape', [])) <= 1:
        return {'head_hex': bytes(value['data']).hex(), 'operands': []}
    if value.get('type') != 32 or value.get('shape') != [2] or len(value.get('data', [])) != 2:
        raise ValueError('unrecognized atomic function node')
    head, args = value['data']
    if head.get('type') != 2:
        raise ValueError('non-character atomic head')
    spelling = bytes(head['data'])
    if spelling == b'0':
        return {'noun': args}
    if args.get('type') != 32 or args.get('shape') != [len(args.get('data', []))]:
        raise ValueError('unrecognized atomic operand vector')
    return {'head_hex': spelling.hex(), 'operands': [atomic_node(x) for x in args['data']]}


def atomic_function(value):
    if value.get('type') != 32 or value.get('shape') != [] or len(value.get('data', [])) != 1:
        raise ValueError('atomic representation needs scalar box')
    return atomic_node(value['data'][0])


def equivalent(expected, actual):
    if isinstance(expected, dict) and isinstance(actual, dict):
        if 'type' in expected:
            return noun_equal(expected, actual)
        return expected.keys() == actual.keys() and all(equivalent(v, actual[k]) for k, v in expected.items())
    if isinstance(expected, list) and isinstance(actual, list):
        return len(expected) == len(actual) and all(equivalent(a, b) for a, b in zip(expected, actual))
    return expected == actual


class Probe:
    def __init__(self, binary, analysis=False):
        self.analysis = analysis
        self.p = subprocess.Popen([str(binary)] + (['--analysis'] if analysis else []), stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, encoding='utf-8')
    def inspect(self, source, operation='A'):
        self.p.stdin.write(((operation + ' ') if self.analysis else '') + source.encode('utf-8').hex() + '\n')
        self.p.stdin.flush()
        response = self.p.stdout.readline()
        if not response:
            raise RuntimeError('frontend probe exited prematurely')
        result = json.loads(response)
        if 'transport_error' in result:
            raise RuntimeError(result)
        return result
    def close(self):
        self.p.stdin.close()
        self.p.wait(timeout=30)
        self.p.stdout.close()
        if self.p.returncode:
            raise RuntimeError('frontend probe failed')


def run(args):
    source_dir = Path(args.source_directory)
    source = (source_dir / 'jsrc/p.c').read_text()
    rows = source_rows(source)
    observed_rows = [json.loads(x) for x in subprocess.check_output([args.binary, '--rows'], text=True).splitlines()]
    report = {'platform': platform.platform(), 'reference_revision': args.reference_revision,
              'source_review_revision': args.source_revision,
              'source_hashes': {n: hashlib.sha256((source_dir / n).read_bytes()).hexdigest() for n in ['jsrc/w.c', 'jsrc/p.c', 'jsrc/cf.c', 'jsrc/sn.c', 'jsrc/wn.c']},
              'reference_library_sha256': hashlib.sha256(Path(os.environ['J_LIBRARY']).read_bytes()).hexdigest(),
              'probe_sha256': hashlib.sha256(Path(args.binary).read_bytes()).hexdigest(),
              'checks': {}, 'mismatches': [], 'analysis_coverage_boundaries': [], 'pending': PENDING,
              'limitations': ['enqueue control/name flags use source-derived goldens, not a C queue export', 'cases[] enumeration does not prove runtime ptcol action equivalence']}
    def check(stage, source, expected, actual):
        report['checks'][stage] = report['checks'].get(stage, 0) + 1
        if not equivalent(expected, actual):
            report['mismatches'].append({'stage': stage, 'source': source, 'expected': expected, 'actual': actual})
    combinations = list(itertools.product(CLASSES, repeat=4))
    check('row_transport', 'coverage', [list(x) for x in combinations], [x['classes'] for x in observed_rows])
    for classes, observed in zip(combinations, observed_rows):
        check('declarative_rows', list(classes), expected_row(rows, classes), observed['row'])
    oracle, probe, static_probe = Oracle(), Probe(args.binary), Probe(args.binary, analysis=True)
    try:
        literals = ['0', '1', '2', '_3', '0 1 1', '1 2 3', '1 2.5 _3', '_', '__', '_.', '1e_3', "''", "'a'", "'abc'", "'can''t'"]
        primitives = ['+', '-', '*', '%', '$', '$.', '#', ',', '=', '<', '>', '{', '|', 'i.', '|.', '|:', '{.', '}.', 'i:', 'I.', 'e.', 'E.', '/', '\\', '"', '@:']
        for word in literals + primitives:
            observation = probe.inspect(word)
            expected_words = oracle.words(word)
            check('tokenizer', word, expected_words.get('words_hex'), observation.get('raw_words'))
            error = oracle.run('rustjstagevalue =: ' + word)
            if error:
                raise RuntimeError(f'oracle rejected positive enqueue case {word}: {error}')
            pos = oracle.name_class('rustjstagevalue')['class']
            queue = observation['enqueue']
            if not isinstance(queue, list) or len(queue) != 1:
                check('enqueue_pos', word, 'one word', queue)
                continue
            check('enqueue_pos', word, {0:'Noun',1:'Adverb',2:'Conjunction',3:'Verb'}[pos], queue[0]['class'])
            if pos == 0:
                check('enqueue_noun', word, oracle.read_noun('rustjstagevalue'), queue[0]['noun'])
        for source in ['a=:b', 'a=.b', "'a'=:1", '(a)', 'a+b', 'a NB. ignored', '1 2 NB. ignored', 'NB. only', '', "'NB.'", 'a=:b=:1']:
            observation = probe.inspect(source)
            check('tokenizer', source, oracle.words(source).get('words_hex'), observation.get('raw_words'))
            visible = observation['raw_words']
            if visible and bytes.fromhex(visible[-1]).startswith(b'NB.'):
                visible = visible[:-1]
            check('comment_cutoff', source, visible, observation['visible_words'])
            queue = observation['enqueue']
            if not isinstance(queue, list):
                check('enqueue_flags', source, 'queue', queue)
                continue
            for i, word in enumerate(queue):
                spelling = bytes.fromhex(observation['visible_words'][i]).decode()
                # Independent goldens for this control/name corpus; do not
                # derive expected flags from Rust's reported classifications.
                cls = {'=:':'Assignment', '=.':'Assignment', '(':'LeftParen', ')':'RightParen', '+':'Verb'}.get(spelling)
                if cls is None:
                    cls = 'Name' if spelling in ('a', 'b') else 'Noun'
                check('enqueue_control_class', source + f' word {i}', cls, word['class'])
                is_name = cls == 'Name'
                is_assignment = cls == 'Assignment'
                expected = {'lookup': is_name and (i+1 == len(queue) or bytes.fromhex(observation['visible_words'][i+1]) not in (b'=:', b'=.')),
                            'global': is_assignment, 'local': False,
                            'to_name': is_assignment and i > 0 and bytes.fromhex(observation['visible_words'][i-1]) in (b'a', b'b')}
                check('enqueue_flags', source + f' word {i}', expected, {k:word[k] for k in expected})
                check('enqueue_provenance', source + f' word {i}', observation['visible_words'][i], source.encode()[slice(*word['span'])].hex())
                check('enqueue_index', source + f' word {i}', i, word['index'])
        functions = ['/', '\\', '"', '@:', '(/)', '(")', 'stageadverb=:/', 'stageconjunction=:@:', '+', '+/', '+\\', '+*', '+-*', '+-*%', '+-*%#', '+/ % #', '+"1', '+"0 1', '+"0 1 2', '+@:-', '+@:-@:*', '(+/) % #', '(+*) - %', '7 + *', '(7) + *', '+"((1))', '(+"1)/']
        # cf.c CADVF trains: retain arity, operands and actual ADV/CONJ POS.
        functions.extend(['+"', '"1', '3"', '/+', '/\\', '/@:', '@:/', '@:@:',
                          '/ / /', '/ / +', '/ + *', '@: + *', '+ @: /', '+ @: @:',
                          '(/ /) /', '("1)', '"1 2', '3 @:', '"+', '@: 3'])
        rng = random.Random(20261003)
        for length in range(2, 9):
            functions.extend(' '.join(rng.choice(['+', '-', '*', '%', '#', ',']) for _ in range(length)) for _ in range(12))
        for source in functions:
            observation = probe.inspect(source)
            check('tokenizer', source, oracle.words(source).get('words_hex'), observation.get('raw_words'))
            error = oracle.run('rustjstagefunction =: ' + source)
            if error:
                raise RuntimeError(f'oracle rejected positive function case {source}: {error}')
            expected = {'pos': oracle.name_class('rustjstagefunction')['class'], 'function': atomic_function(oracle.representation('rustjstagefunction', 'atomic')['value'])}
            check('parser_function', source, expected, observation['parse'])
        for source in ['(/3)', '(@:/3)', '3/', '+@:3', '3@:+', '+"1 2 3 4', "+\"'a'", '(', ')', '=:', 'NB..', "'unterminated", 'foo_', '1q', '1e', '1.2.3', '3..']:
            observation = probe.inspect(source)
            expected = oracle.run('rustjstagefunction =: ' + source)
            if not expected:
                raise RuntimeError('oracle unexpectedly accepted negative case: ' + source)
            actual = observation.get('error') or observation.get('parse', {}).get('error')
            check('frontend_error', source, expected['error'], actual)
        # A read-only parser observation is compared with C construction, not
        # C pointer layouts or backend specialization. Setup E is separate.
        versions = {}
        def setup(source, name):
            check('static_setup', source, oracle.eval(source), static_probe.inspect(source, 'E'))
            versions[name] = versions.get(name, 0) + 1
        def analyze_function(source, dependencies):
            observation = static_probe.inspect(source)
            error = oracle.run(source)
            if error:
                check('static_constructor_error', source, error, observation.get('parse', observation))
                return
            target = source.split('=:', 1)[0].strip()
            expected = {'pos': oracle.name_class(target)['class'],
                        'function': atomic_function(oracle.representation(target, 'atomic')['value'])}
            check('static_constructor', source, expected, observation.get('parse', observation))
            check('static_read_only', source, versions.get(target), observation.get('target_version', 'missing'))
            snapshots = [{'name_hex': name.encode().hex(), 'version': versions[name],
                          'expected_pos': pos, 'span': [source.index(name), source.index(name)+len(name)]}
                         for name, pos in dependencies]
            check('static_modifier_dependencies', source, snapshots, observation.get('snapshots'))
        for source, name in [('analysisadv=:/', 'analysisadv'),
                             ('analysisalias=:analysisadv', 'analysisalias'),
                             ('analysisrank=:\"', 'analysisrank'),
                             ('analysisatop=:@:', 'analysisatop'),
                             ('analysisverb=:-', 'analysisverb'),
                             ('protected=:+', 'protected')]:
            setup(source, name)
        for expression, deps in [('+ analysisadv', [('analysisadv', 1)]),
                                 ('+ analysisalias', [('analysisalias', 1)]),
                                 ('+ analysisalias % #', [('analysisalias', 1)]),
                                 ('- analysisrank 1', [('analysisrank', 2)]),
                                 ('analysisverb analysisrank 1', [('analysisrank', 2)]),
                                 ('+ analysisatop -', [('analysisatop', 2)])]:
            analyze_function('candidate=: ' + expression, deps)
        analyze_function('protected=: - analysisrank 1', [('analysisrank', 2)])
        for expression in ['3 analysisalias', "+ analysisrank 'a'", '+ analysisrank 1 2 3 4', '+ analysisatop 3']:
            analyze_function('candidate=: ' + expression, [])
        # C succeeds, but static analysis must not execute the rank operand.
        source = 'candidate=: + analysisrank (#1 2)'
        actual = static_probe.inspect(source)
        reference = oracle.run(source)
        if reference is None and actual == {'error': 'unsupported'}:
            report['analysis_coverage_boundaries'].append({'source': source, 'reason': 'rank operand needs a concrete noun; analysis does not execute it',
                                                          'reference_pos': oracle.name_class('candidate')['class'], 'rust': actual})
        else:
            check('static_value_boundary', source, {'C_success': True, 'rust': {'error': 'unsupported'}}, {'C_success': reference is None, 'rust': actual})
        for source in ['candidate=: + (/ /)', 'candidate=: + (@:/) -']:
            actual = static_probe.inspect(source)
            reference = oracle.run(source)
            if reference is None and actual == {'error': 'unsupported'}:
                report['analysis_coverage_boundaries'].append({'source': source,
                    'reason': 'derived modifier application semantics are not implemented',
                    'reference_pos': oracle.name_class('candidate')['class'], 'rust': actual})
            else:
                check('static_modifier_application_boundary', source,
                      {'C_success': True, 'rust': {'error': 'unsupported'}},
                      {'C_success': reference is None, 'rust': actual})
        for expression in ['- ("1)', '- ("1 2)', '- ("+)', '+ (@:-)']:
            analyze_function('candidate=: ' + expression, [])
        for expression in ['- ("\'a\')', '- ("1 2 3 4)', '- (@:3)']:
            analyze_function('candidate=: ' + expression, [])
        setup('boundrank=:"1', 'boundrank')
        setup('boundalias=:boundrank', 'boundalias')
        analyze_function('candidate=: - boundrank', [('boundrank', 1)])
        analyze_function('candidate=: - boundalias', [('boundalias', 1)])
        setup('boundrank=:"2', 'boundrank')
        analyze_function('candidate=: - boundalias', [('boundalias', 1)])
        analyze_function('candidate=: - boundrank', [('boundrank', 1)])
        setup('analysisadv=:\\', 'analysisadv')
        analyze_function('candidate=: + analysisalias', [('analysisalias', 1)])
        analyze_function('candidate=: + analysisadv', [('analysisadv', 1)])
        setup('analysisalias=:analysisadv', 'analysisalias')
        analyze_function('candidate=: + analysisalias', [('analysisalias', 1)])
        for source in ['rustjstagelocal=.1 2 3', 'rustjstagelocal=:1 2 3']:
            error = oracle.run(source)
            check('copula_reference', source, None, error)
            check('copula_reference', source, 0, oracle.name_class('rustjstagelocal')['class'])
    finally:
        probe.close()
        static_probe.close()
        oracle.close()
    report['failed'] = len(report['mismatches'])
    Path(args.report).parent.mkdir(parents=True, exist_ok=True)
    Path(args.report).write_text(json.dumps(report, indent=2) + '\n', newline='\n')
    print(json.dumps({'checks': report['checks'], 'failed': report['failed'], 'report': args.report}))
    return bool(report['failed'])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--source-directory', required=True)
    parser.add_argument('--source-revision', required=True)
    parser.add_argument('--reference-revision', required=True)
    parser.add_argument('--report', required=True)
    args = parser.parse_args()
    for revision in [args.source_revision, args.reference_revision]:
        if not re.fullmatch('[0-9a-f]{40}', revision):
            parser.error('revision must be a full commit hash')
    raise SystemExit(run(args))


if __name__ == '__main__':
    main()
