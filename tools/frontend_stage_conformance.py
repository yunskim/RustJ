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
from conformance import equal as noun_equal, modifier_trident_cases, compound_gerund_cases, gerund_name_cases, gerund_snapshot_cases, constructor_call_cases, late_modifier_cases, modifier_inventory_cases, definition_code_cases, definition_flow_bodies, control_sequence_matrix, goto_position_matrix, multiple_definition_cases, noun_direct_cases, entity_boundary_cases

CLASSES = ['Noun', 'Verb', 'Adverb', 'Conjunction', 'Name', 'Assignment', 'LParen', 'RParen', 'Mark']
C_CLASSES = dict(zip(['NOUN', 'VERB', 'ADV', 'CONJ', 'NAME', 'ASGN', 'LPAR', 'RPAR', 'MARK'], CLASSES))
PENDING = {
    'enqueue': ['complex/extended/rational numeric notation', 'locatives and name _:', 'complete spellin/ds inventory', 'tacit translator env=0 and locative copula upgrade'],
    'parser': ['runtime ptcol reachable-state trace equivalence', 'full local/locale assignment and right-to-left effect coverage', 'full computed noun modifier operands', 'derived modifier application and named derived identities', 'complete explicit/direct definitions: nested/tagged input, controls and invocation', 'full row provenance and reinsertion trace'],
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


def source_constructors(source):
    """Read cf.c dispositions. Missing/duplicate/unknown source fails closed.

    Special V V hook dispatch precedes bidents[]; MARK selects a fork.
    A zero action invokes a semantic constructor/call, not a deferred train.
    """
    source = re.sub(r'/\*.*?\*/|//[^\n]*', '', source, flags=re.S)
    if 'if(AT(a)&AT(w)&VERB)' not in re.sub(r'\s+', '', source):
        raise ValueError('C V V hook dispatch missing')
    classes = {'NOUN': 'Noun', 'ADV': 'Adverb', 'CONJ': 'Conjunction', 'VERB': 'Verb'}
    tables = {}
    for arity, name, size in [(2, 'bidents', 16), (3, 'tridents', 64)]:
        match = re.search(name + r'\[' + str(size) + r'\]\s*=\s*\{(.*?)\};', source, re.S)
        if not match:
            raise ValueError('C constructor table missing: ' + name)
        pattern = r'\[TYPE' + str(arity) + r'\(([^)]+)\)\]\s*=\s*\{\s*(\w+)\s*,\s*(\w+)\s*\}\s*,?'
        entries = list(re.finditer(pattern, match[1]))
        if re.sub(pattern, '', match[1]).strip():
            raise ValueError('unrecognized C constructor table fields: ' + name)
        table = {}
        for entry in entries:
            tokens = [part.strip() for part in entry[1].split(',')]
            if len(tokens) != arity or any(part not in classes for part in tokens):
                raise ValueError('unknown C constructor operand class')
            key = tuple(classes[part] for part in tokens)
            if key in table:
                raise ValueError('duplicate C constructor entry')
            action, result = entry[2], entry[3]
            if result == 'MARK' and action == '0' and arity == 3:
                disposition = 'BuildFork'
            elif action == '0' and result in ('NOUN', 'VERB'):
                disposition = 'ImmediateSemanticApply'
            elif result in ('ADV', 'CONJ') and re.search(r'static\s+DF[12]\(' + action + r'\)', source):
                disposition = 'BuildDerivedModifier(' + classes[result] + ')'
            else:
                raise ValueError('unknown C constructor action/result: ' + action + '/' + result)
            table[key] = disposition
        tables.update({parts: table.get(parts, 'SyntaxError') for parts in itertools.product(classes.values(), repeat=arity)})
    tables[('Verb', 'Verb')] = 'BuildHook'
    return tables


def source_control_words(source):
    """Read conword fixed-word lengths; this is not a private C API export."""
    block = source.split('static I jtconword', 1)[1].split('// w is string', 1)[0]
    names = {'CDO':'Do','CIF':'If','CEND':'End','CELSE':'Else','CWHILE':'While',
        'CELSEIF':'ElseIf','CFOR':'For','CRETURN':'Return','CBREAK':'Break',
        'CCONT':'Continue','CSELECT':'Select','CCASE':'Case','CFCASE':'FCase',
        'CWHILST':'Whilst','CASSERT':'Assert','CTHROW':'Throw','CTRY':'Try',
        'CCATCH':'Catch','CCATCHD':'CatchD','CCATCHT':'CatchT'}
    out = {}
    for line in block.splitlines():
        if 'cwtlen=(' not in line:
            continue
        match = re.search(r"MATCHNAME8\((\d+),([^)]*)\).*?cwtlen=\((\w+)<<8\)\+(\d+)", line)
        if not match or match[3] not in names:
            raise ValueError('unrecognized conword fixed match: ' + line)
        prefix = ''.join(re.findall(r"'(.)'", match[2]))[:int(match[1])]
        length = int(match[4])
        word = prefix if len(prefix) == length else prefix + '.' if prefix == 'continue' and length == 9 else None
        if not word or word in out:
            raise ValueError('invalid/duplicate conword match')
        out[word] = names[match[3]]
    if not out:
        raise ValueError('conword fixed table empty')
    return out


def reference_control_parts(source, words_hex, fixed):
    """getsen partition algorithm with C ;: words and reviewed conword classes."""
    data = source.encode()
    parts = []
    cursor, start, end = 0, None, 0
    for encoded in words_hex:
        word = bytes.fromhex(encoded)
        if word.startswith(b'NB.'):
            break
        index = data.find(word, cursor)
        if index < 0:
            raise ValueError('C word absent from source')
        cursor = index + len(word)
        if start is None:
            start = index
        text = word.decode()
        kind = fixed.get(text)
        if text.endswith('.'):
            for prefix, cls in [('for_','For'), ('goto_','Goto'), ('label_','Label')]:
                if text.startswith(prefix):
                    kind = cls
        if kind:
            if start < index:
                parts.append({'span':[start,index], 'control':None, 'text_hex':data[start:index].hex()})
            parts.append({'span':[index,cursor], 'control':kind, 'text_hex':encoded})
            start = None
        end = cursor
    if start is not None:
        parts.append({'span':[start,end], 'control':None, 'text_hex':data[start:end].hex()})
    return {'parts':parts}


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
    def __init__(self, binary, analysis=False, definitions=False):
        self.analysis = analysis
        self.p = subprocess.Popen([str(binary)] + (['--analysis'] if analysis else ['--definitions'] if definitions else []), stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, encoding='utf-8')
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
              'source_hashes': {n: hashlib.sha256((source_dir / n).read_bytes()).hexdigest() for n in ['jsrc/w.c', 'jsrc/p.c', 'jsrc/cf.c', 'jsrc/sn.c', 'jsrc/wn.c', 'jsrc/cr.c', 'jsrc/ap.c', 'jsrc/cg.c', 'jsrc/r.c', 'jsrc/a.c', 'jsrc/sc.c', 'jsrc/s.c', 'jsrc/jtype.h', 'jsrc/cx.c', 'jsrc/wc.c', 'jsrc/io.c', 'jsrc/jerr.h', 'jsrc/j.h', 'jsrc/w.h', 'test/ggoto.ijs', 'test/g0x.ijs']},
              'reference_library_sha256': hashlib.sha256(Path(os.environ['J_LIBRARY']).read_bytes()).hexdigest(),
              'probe_sha256': hashlib.sha256(Path(args.binary).read_bytes()).hexdigest(),
              'checks': {}, 'mismatches': [], 'analysis_coverage_boundaries': [], 'pending': PENDING,
              'limitations': ['enqueue control/name flags use source-derived goldens, not a C queue export', 'cases[] enumeration does not prove runtime ptcol action equivalence', 'cf.c disposition enumeration does not prove all operand values, effects or callable implementations', 'control partition uses C ;: words and reviewed conword/getsen source, not an exported C preparse/control-flow trace']}
    def check(stage, source, expected, actual):
        report['checks'][stage] = report['checks'].get(stage, 0) + 1
        if not equivalent(expected, actual):
            report['mismatches'].append({'stage': stage, 'source': source, 'expected': expected, 'actual': actual})
    combinations = list(itertools.product(CLASSES, repeat=4))
    check('row_transport', 'coverage', [list(x) for x in combinations], [x['classes'] for x in observed_rows])
    for classes, observed in zip(combinations, observed_rows):
        check('declarative_rows', list(classes), expected_row(rows, classes), observed['row'])
    expected_constructors = source_constructors((source_dir / 'jsrc/cf.c').read_text())
    observed_constructors = [json.loads(line) for line in subprocess.check_output([args.binary, '--constructors'], text=True).splitlines()]
    keys = [tuple(entry['classes']) for entry in observed_constructors]
    check('constructor_transport', 'coverage', sorted(expected_constructors), sorted(keys))
    for entry in observed_constructors:
        parts = tuple(entry['classes'])
        check('constructor_disposition', list(parts), expected_constructors.get(parts), entry['disposition'])
    oracle, probe, static_probe, definition_probe = Oracle(), Probe(args.binary), Probe(args.binary, analysis=True), Probe(args.binary, definitions=True)
    try:
        fixed_controls = source_control_words((source_dir / 'jsrc/wc.c').read_text())
        check('control_inventory_size', 'wc.c conword fixed inventory', 20, len(fixed_controls))
        control_fixtures = list(fixed_controls) + ['for_item.', 'for_a_b.', 'goto_exit.', 'label_exit.', 'goto_.',
            '  if. y + 1   do. y else.  0 end. NB. if. ignored',
            "assert. 'if. do. end.'", 'a NB. if. do. end.', 'NB. if.', '',
            'if.x continuex. IF.', 'try. a catch. b catchd. c catcht. d end.', "'한글' if. x do. y end."]
        for source in control_fixtures:
            words = oracle.words(source)['words_hex']
            check('control_partition_source_projection', source,
                reference_control_parts(source, words, fixed_controls), static_probe.inspect(source, 'Q'))
        # Input framing is an execution-free source projection, not a callable
        # parser. Construct C fixtures only to validate their source/body words.
        for body in [' y+1 ', "\nNB. }} {{ opaque\ny+1\n", "\ninner=.{{y+1}}\ninner y\n", " 'it''s }}' [ y "]:
            source = 'f=:{{' + body + '}}'
            actual = definition_probe.inspect(source)
            expected = {'state': 'definition', 'form': 'direct', 'mode': 9,
                        'body_hex': body.encode().hex(), 'span': [3, len(source)],
                        'body_span': [5, 5+len(body)]}
            check('definition_input', source, expected, {key:actual.get(key) for key in expected})
            if oracle.run(source):
                raise RuntimeError('C rejected direct input fixture: ' + source)
            for line in body.splitlines():
                check('definition_body_words', line, oracle.words(line).get('words_hex'), probe.inspect(line).get('raw_words'))
        for mode, body in [(1, 'u y'), (2, 'u v y'), (3, "counter=:99\ny+1"), (4, 'x+y'), (3, "'it''s }}' [ y")]:
            quoted = "'" + body.replace("'", "''") + "'"
            for form, source, start, end in [
                ('string', f'f=:{mode} : ' + quoted, 7, 7+len(quoted)),
                ('block', f'f=:{mode} : 0\n' + body + '\n  )  ', 9, 9+len(body)+1),
            ]:
                actual = definition_probe.inspect(source)
                expected_body = body if form == 'string' else body + '\n'
                expected = {'state': 'definition', 'form': form, 'mode': mode,
                    'body_hex': expected_body.encode().hex(), 'span': [3, len(source)], 'body_span': [start, end]}
                check('definition_input', source, expected, {key:actual.get(key) for key in expected})
                if form == 'string':
                    literal = oracle.eval(quoted)
                    check('definition_body_literal', source, literal.get('data'), list(bytes.fromhex(actual.get('body_hex', ''))))
                oracle.run('counter=:0')
                error = oracle.run(f'f=:{mode} : ' + quoted)
                if error or oracle.name_class('f')['class'] != (mode if mode <= 2 else 3):
                    raise RuntimeError(f'C rejected explicit input fixture: {source}: {error}')
                if oracle.eval('counter')['data'] != [0]:
                    raise RuntimeError('C ran definition body during construction')
                for line in expected_body.splitlines():
                    check('definition_body_words', line, oracle.words(line).get('words_hex'), probe.inspect(line).get('raw_words'))
        for source, state in [('f=:{{', 'incomplete'), ('f=:3 : 0', 'incomplete'),
                              ('f=:3 : 0\ny+1\n) NB. not terminator', 'incomplete'),
                              ("'{{ }} : define'", 'sentence'), ('NB. {{', 'sentence')]:
            check('definition_input_boundary', source, {'state':state}, definition_probe.inspect(source))
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
        # Runtime capture can construct this gerund, but static analysis must
        # not execute boxing/ravel to obtain its constructor operand.
        source = "candidate=: (, <'+')\\"
        actual = static_probe.inspect(source)
        reference = oracle.run(source)
        if reference is None and actual == {'error': 'unsupported'}:
            report['analysis_coverage_boundaries'].append({'source': source,
                'reason': 'computed gerund noun needs concrete constructor data; analysis does not execute it',
                'reference_pos': oracle.name_class('candidate')['class'], 'rust': actual})
        else:
            check('static_gerund_value_boundary', source,
                {'C_success': True, 'rust': {'error': 'unsupported'}},
                {'C_success': reference is None, 'rust': actual})
        for expression in ['- ("1)', '- ("1 2)', '- ("+)', '+ (@:-)']:
            analyze_function('candidate=: ' + expression, [])
        for expression in ['- ("\'a\')', '- ("1 2 3 4)', '- (@:3)']:
            analyze_function('candidate=: ' + expression, [])
        for expression in ['1 (-")', '1 2 (-")', '+ (-")', '- (+@:)',
                           '+ (/ /)', '+ (/ +)', '+ (/ / /)', '+ ((/ /) /)',
                           '1 ((-") /)']:
            analyze_function('candidate=: ' + expression, [])
        for expression in ["'a' (-\")", '1 2 3 4 (-")', "'a' ((-\") /)", '3 (/@:)']:
            analyze_function('candidate=: ' + expression, [])
        setup('leftbind=:-"', 'leftbind')
        setup('leftalias=:leftbind', 'leftalias')
        analyze_function('candidate=: 1 leftalias', [('leftalias', 1)])
        setup('leftbind=:1', 'leftbind')
        analyze_function('candidate=: 1 leftalias', [('leftalias', 1)])
        for expression in ['+ (/@:)', '+ (@:/) -', '+ (" /) 1',
                           '+ (@:@:) -', '+ (" ") 1', '+ (/ / +) *',
                           '+ ((@:/) -)', '+ ((/@:) /)']:
            analyze_function('candidate=: ' + expression, [])
        for expression in ['+ (" @:) 1 2 3 4', '+ (@: ") 1 2 3 4',
                           '3 (/ / +) 3']:
            analyze_function('candidate=: ' + expression, [])
        setup('derivedconj=:@:/', 'derivedconj')
        setup('derivedalias=:derivedconj', 'derivedalias')
        analyze_function('candidate=: + derivedalias -', [('derivedalias', 2)])
        setup('derivedconj=:1', 'derivedconj')
        analyze_function('candidate=: + derivedalias -', [('derivedalias', 2)])
        for expression in ['+ (+ + @:) -', '+ (3 + @:) -', '+ (@: + @:) -',
                           '+ (@: / /) -', '+ (+ @: /)', '+ (+ @: @:) -',
                           '+ (/ + -)', '+ (/ @: -)', '+ (/ @: /) -',
                           '+ (/ @: @:) -', '+ (@: + -) *', '+ (@: @: -) *',
                           '+ (@: @: /) -']:
            analyze_function('candidate=: ' + expression, [])
        for expression in ['+ (" + @:) 1 2 3 4', '+ (@: + ") 1 2 3 4',
                           '3 (+ @: /)', '3 (/ @: /) +', '3 (/ @: @:) 1 2 3 4',
                           '+ (" @: /) 1 2 3 4']:
            analyze_function('candidate=: ' + expression, [])
        for expression in modifier_trident_cases():
            analyze_function('candidate=: ' + expression, [])
        setup('tridentconj=:@: + @:', 'tridentconj')
        setup('tridentalias=:tridentconj', 'tridentalias')
        analyze_function('candidate=: + tridentalias -', [('tridentalias', 2)])
        setup('tridentconj=:1', 'tridentconj')
        analyze_function('candidate=: + tridentalias -', [('tridentalias', 2)])
        for expression in ['3"0', '1 2 3"1 2', "'abc'\"_", '3"+',
                           '3 (" /) 1', '3 (+ ")', '3 (+ " /)']:
            analyze_function('candidate=: ' + expression, [])
        for expression in ["3\"'a'", '3"1 2 3 4']:
            analyze_function('candidate=: ' + expression, [])
        for expression in ['3\\', "'abc'\\", "''\\"]:
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
        # R runs shared runtime parser construction and projects the completed
        # function for comparison with C 5!:1/4!:0. A stays read-only.
        for source in compound_gerund_cases():
            if source.startswith(('compoundfn=:', 'compoundkeep=:')):
                error = oracle.run(source)
                actual = static_probe.inspect(source, 'R')
                if error:
                    check('runtime_ar_constructor_error', source, error, actual)
                else:
                    target = source.split('=:', 1)[0].strip()
                    expected = {'pos': oracle.name_class(target)['class'],
                        'function': atomic_function(oracle.representation(target, 'atomic')['value'])}
                    check('runtime_ar_constructor', source, expected, actual)
            else:
                expected = oracle.eval(source)
                if source.startswith('compoundar=:') and 'error' in expected:
                    raise RuntimeError(f'invalid AR fixture setup: {source}: {expected}')
                check('runtime_ar_setup_or_target', source, expected, static_probe.inspect(source, 'E'))
        for source in gerund_name_cases() + gerund_snapshot_cases():
            if source.startswith(('gerundnamefn=:', 'gerund_name_keep=:')):
                error = oracle.run(source)
                actual = static_probe.inspect(source, 'R')
                if error:
                    check('runtime_gerund_name_error', source, error, actual)
                else:
                    target = source.split('=:', 1)[0].strip()
                    expected = {'pos': oracle.name_class(target)['class'],
                        'function': atomic_function(oracle.representation(target, 'atomic')['value'])}
                    check('runtime_gerund_name_constructor', source, expected, actual)
            else:
                check('runtime_gerund_name_setup_or_target', source, oracle.eval(source), static_probe.inspect(source, 'E'))
        for source in constructor_call_cases():
            if source.startswith(('callfn=:', 'callkeep=:')):
                error = oracle.run(source)
                actual = static_probe.inspect(source, 'R')
                if error:
                    check('runtime_constructor_call_error', source, error, actual)
                else:
                    target = source.split('=:', 1)[0].strip()
                    expected = {'pos': oracle.name_class(target)['class'],
                        'function': atomic_function(oracle.representation(target, 'atomic')['value'])}
                    check('runtime_constructor_call_result', source, expected, actual)
            else:
                expected = oracle.eval(source)
                if 'error' in expected:
                    raise RuntimeError(f'invalid constructor call fixture: {source}: {expected}')
                check('runtime_constructor_call_setup', source, expected, static_probe.inspect(source, 'E'))
        for source in [s for s in constructor_call_cases() if s.startswith('callar=:')][:4]:
            for setup in [source, "callouterar=:(<'3'),<((<callar),(<'+'),<'-')"]:
                expected = oracle.eval(setup)
                if 'error' in expected:
                    raise RuntimeError(f'invalid computed snapshot fixture: {setup}: {expected}')
                check('constructor_decode_setup', setup, expected, static_probe.inspect(setup, 'E'))
            error = oracle.run('calldecoded=:(<callouterar)5!:0')
            if error:
                raise RuntimeError(f'C computed snapshot decoder failed: {error}')
            decoded = atomic_function(oracle.representation('calldecoded', 'atomic')['value'])
            source = 'callfn=:(,<callouterar)' + chr(92)
            check('constructor_decoded_snapshot', source, {'decoded': [decoded]}, static_probe.inspect(source, 'D'))
        for source in modifier_inventory_cases():
            if source.startswith('inventoryar=:'):
                expected = oracle.eval(source)
                if 'error' in expected:
                    raise RuntimeError(f'invalid inventory AR fixture: {source}: {expected}')
                check('constructor_inventory_setup', source, expected, static_probe.inspect(source, 'E'))
            else:
                error = oracle.run(source)
                actual = static_probe.inspect(source, 'R')
                if error:
                    check('constructor_inventory_error', source, error, actual)
                else:
                    expected = {'pos': oracle.name_class('inventoryfn')['class'],
                        'function': atomic_function(oracle.representation('inventoryfn', 'atomic')['value'])}
                    check('constructor_inventory_result', source, expected, actual)
                    error = oracle.run('inventorydecoded=:(<inventoryar)5!:0')
                    if error:
                        raise RuntimeError(f'C inventory decode failed: {error}')
                    decoded = atomic_function(oracle.representation('inventorydecoded', 'atomic')['value'])
                    check('constructor_inventory_decoded', source, {'decoded': [decoded]}, static_probe.inspect(source, 'D'))
        for source in late_modifier_cases():
            if source.startswith(('latefn=:', 'latekeep=:')):
                error = oracle.run(source)
                actual = static_probe.inspect(source, 'R')
                if error:
                    check('late_modifier_error', source, error, actual)
                else:
                    target = source.split('=:', 1)[0].strip()
                    expected = {'pos': oracle.name_class(target)['class'],
                        'function': atomic_function(oracle.representation(target, 'atomic')['value'])}
                    check('late_modifier_constructor', source, expected, actual)
            else:
                expected = oracle.eval(source)
                if 'error' in expected:
                    raise RuntimeError(f'invalid late modifier fixture: {source}: {expected}')
                check('late_modifier_setup_or_target', source, expected, static_probe.inspect(source, 'E'))
        for binding in ['/', chr(92)]:
            for source in ['lateadv=:' + binding, "latear=:(<((<'4'),<((<'lateadv'),<'/'))),<(,<'+')"]:
                check('late_modifier_decode_setup', source, oracle.eval(source), static_probe.inspect(source, 'E'))
            error = oracle.run('latedecoded=:(<latear)5!:0')
            if error:
                raise RuntimeError(f'C late modifier decoding failed: {error}')
            expected = atomic_function(oracle.representation('latedecoded', 'atomic')['value'])
            source = 'latefn=:(,<latear)' + chr(92)
            check('late_modifier_decoded_snapshot', source, {'decoded': [expected]}, static_probe.inspect(source, 'D'))
        for source in definition_code_cases():
            if source.startswith(('defcode=:', 'defalias=:')):
                error = oracle.run(source)
                actual = static_probe.inspect(source, 'R')
                if error:
                    check('definition_constructor_error', source, error, actual)
                else:
                    target = source.split('=:', 1)[0]
                    expected = {'pos': oracle.name_class(target)['class'],
                        'function': atomic_function(oracle.representation(target, 'atomic')['value'])}
                    check('definition_constructor', source, expected, actual)
            else:
                check('definition_no_execution_or_transaction', source,
                      oracle.eval(source), static_probe.inspect(source, 'E'))
        for source in ["defcode=:3 : 'y+1\n:\nx+y'", "defcode=:{{\ny+1}}",
                       "defcode=:3 : 'copy=.futuredef\ncopy+y'",
                       "defcode=:4 : 'y+1\n:\nx+y'",
                       "defcode=:1 : 'u y\n:\nu x+y'"]:
            error = oracle.run(source)
            if error:
                raise RuntimeError(f'invalid definition fixture: {source}: {error}')
            expected = {'pos': oracle.name_class('defcode')['class'],
                'function': atomic_function(oracle.representation('defcode', 'atomic')['value'])}
            check('definition_multiline_constructor', source, expected, static_probe.inspect(source, 'R'))
        for source in ["defcode=:1 : 'u\n:\nv'", "defcode=:{{u\n:\nu}}"]:
            check('definition_valence_error', source, oracle.eval(source), static_probe.inspect(source, 'E'))
        for source in entity_boundary_cases():
            # The C name class decides whether to observe AR or actual noun/error.
            # Execute each original once, including nested/intermediate writes.
            target = source.split('=:', 1)[0] if '=:' in source else None
            if target is None:
                check('entity_boundary_value', source, oracle.eval(source), static_probe.inspect(source, 'E'))
                continue
            error = oracle.run(source)
            info = oracle.name_class(target) if target and not error else {}
            if info.get('class') in (1, 2, 3):
                expected = {'pos': info['class'],
                    'function': atomic_function(oracle.representation(target, 'atomic')['value'])}
                check('entity_boundary_function', source, expected, static_probe.inspect(source, 'R'))
            elif error:
                check('entity_boundary_error', source, error, static_probe.inspect(source, 'E'))
            elif target:
                check('entity_boundary_noun_assignment', source, {'silent': True}, static_probe.inspect(source, 'E'))
        for source in noun_direct_cases():
            if source.startswith('nounmixed='):
                error=oracle.run(source)
                if error: raise RuntimeError(f'invalid mixed noun fixture: {error}')
                expected={'pos':3,'function':atomic_function(oracle.representation('nounmixed','atomic')['value'])}
                check('noun_direct_mixed_constructor',source,expected,static_probe.inspect(source,'R'))
            else:
                check('noun_direct_value_or_assignment',source,oracle.eval(source),static_probe.inspect(source,'E'))
        for source in ["nounraw=:{{)n\nabc\n}}", "nounraw=:{{)nfirst\n embedded }}\n}}",
                "nounraw=:{{)n\n'broken NB. {{\n}}", "nounraw=:{{)n \n)\n}}",
                "nounraw=:{{)n\n}}", "nounraw=:{{)n\r\nabc\r\n}}"]:
            error=oracle.run_script(source)
            if error: raise RuntimeError(f'invalid noun input script: {error}')
            check('noun_direct_script_assignment',source,{'silent':True},static_probe.inspect(source,'E'))
            check('noun_direct_script_value',source,oracle.read_noun('nounraw'),static_probe.inspect('nounraw','E'))
        for source,body,start in [('nounraw=:{{)nabc}}','abc',13),
                ("nounraw=:{{)n'broken}}","'broken",13),
                ('nounraw=:{{)n\nabc\n}}','abc\n',14)]:
            expected={'state':'definition','form':'noun_direct','mode':0,'body_hex':body.encode().hex(),
                'span':[9,len(source)],'body_span':[start,start+len(body)],'nested':[]}
            check('noun_direct_input',source,expected,definition_probe.inspect(source))
        for source in multiple_definition_cases():
            if '{{' not in source:
                check('multiple_definition_setup_or_transaction',source,oracle.eval(source),static_probe.inspect(source,'E'))
                continue
            error=oracle.run(source)
            actual=static_probe.inspect(source,'R')
            if error:
                check('multiple_definition_error',source,error,actual)
            else:
                name=source.split('=:',1)[0]
                expected={'pos':oracle.name_class(name)['class'],'function':atomic_function(oracle.representation(name,'atomic')['value'])}
                check('multiple_definition_constructor',source,expected,actual)
        for source in ['many=:{{y+1}} + {{y-1}}','many=:{{y}} + {{\ny+1}}']:
            actual=definition_probe.inspect(source)
            expected=[]
            for start in [source.index('{{'),source.rindex('{{')]:
                close=source.index('}}',start)
                expected.append({'state':'definition','form':'direct','mode':9,'body_hex':source[start+2:close].encode().hex(),'span':[start,close+2],'body_span':[start+2,close],'nested':[]})
            check('multiple_definition_input',source,{'state':'definitions','definitions':expected},actual)
        for source in ['many=:{{y}} + {{','many=:{{y}} + {{\ny+1']:
            check('multiple_definition_incomplete',source,{'state':'incomplete'},definition_probe.inspect(source))
        for body in definition_flow_bodies() + ['assert.\nNB. ignored\ny',
                'if. y do. 1 end.\n:\nwhile. x do. break. end.', 'if.\n:\n1 2e', '1 2e\n:\nif.']:
            source="flowfn=:3 : '" + body.replace("'", "''") + "'"
            error=oracle.run(source)
            actual=static_probe.inspect(source,'R')
            if error:
                check('definition_flow_error',source,error,actual)
            else:
                expected={'pos':oracle.name_class('flowfn')['class'],
                    'function':atomic_function(oracle.representation('flowfn','atomic')['value'])}
                check('definition_flow_constructor',source,expected,actual)
        source="flowfn=:4 : 'if.\n:\nx+y'"
        error=oracle.run(source)
        if error: raise RuntimeError(f'invalid ignored-valence fixture: {error}')
        expected={'pos':3,'function':atomic_function(oracle.representation('flowfn','atomic')['value'])}
        check('definition_mode4_discarded_valence',source,expected,static_probe.inspect(source,'R'))
        source="flowfn=:3 : '" + 'return. '*32766 + "'"
        check('definition_control_entry_limit',source,oracle.run(source),static_probe.inspect(source,'R'))
        for body in control_sequence_matrix():
            source="matrixflow=:3 : '"+body+"'"
            error=oracle.run(source)
            actual=static_probe.inspect(source,'R')
            if error:
                check('control_sequence_matrix_error',source,error,actual)
            else:
                expected={'pos':3,'function':atomic_function(oracle.representation('matrixflow','atomic')['value'])}
                check('control_sequence_matrix_constructor',source,expected,actual)
        for body in goto_position_matrix():
            source="gotomatrix=:3 : '"+body.replace("'","''")+"'"
            error=oracle.run(source)
            actual=static_probe.inspect(source,'R')
            if error:
                check('goto_position_matrix_error',source,error,actual)
            else:
                expected={'pos':3,'function':atomic_function(oracle.representation('gotomatrix','atomic')['value'])}
                check('goto_position_matrix_constructor',source,expected,actual)
        # C's fix adverb decodes the same AR independently. It is reference-only;
        # Rust D observes the constructor-owned decode, never executes 5!:0.
        for noun in ['7', 'i.4', '<1 2']:
            for source in ['gssnapshot=:' + noun, "gsar=:(<'3'),<((<'gssnapshot'),(<'+'),<'-')"]:
                expected = oracle.eval(source)
                if 'error' in expected:
                    raise RuntimeError(f'invalid snapshot fixture: {source}: {expected}')
                check('snapshot_setup', source, expected, static_probe.inspect(source, 'E'))
            error = oracle.run('gsdecoded=:(<gsar)5!:0')
            if error:
                raise RuntimeError(f'C snapshot decoder failed: {error}')
            decoded = atomic_function(oracle.representation('gsdecoded', 'atomic')['value'])
            source = 'gerundnamefn=:(,<gsar)\\'
            check('decoded_noun_snapshot', source, {'decoded': [decoded]}, static_probe.inspect(source, 'D'))
            source = 'gssnapshot=:9'
            check('snapshot_rebind', source, oracle.eval(source), static_probe.inspect(source, 'E'))
            check('reference_snapshot_retention', noun, decoded,
                  atomic_function(oracle.representation('gsdecoded', 'atomic')['value']))
            error = oracle.run('gsdecodednew=:(<gsar)5!:0')
            if error:
                raise RuntimeError(f'C snapshot redecoder failed: {error}')
            current = atomic_function(oracle.representation('gsdecodednew', 'atomic')['value'])
            source = 'gerundnamefn=:(,<gsar)"0'
            check('decoded_noun_snapshot', source, {'decoded': [current]}, static_probe.inspect(source, 'D'))
        for source in ['rustjstagelocal=.1 2 3', 'rustjstagelocal=:1 2 3']:
            error = oracle.run(source)
            check('copula_reference', source, None, error)
            check('copula_reference', source, 0, oracle.name_class('rustjstagelocal')['class'])
    finally:
        probe.close()
        static_probe.close()
        definition_probe.close()
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
