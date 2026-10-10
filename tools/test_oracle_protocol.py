import unittest

from oracle import Oracle, normalize_request


class OracleProtocolTests(unittest.TestCase):
    def test_string_request_is_backward_compatible_eval(self):
        self.assertEqual(
            normalize_request('1+2'),
            {'op': 'eval', 'source': '1+2'},
        )

    def test_sentence_request_normalizes_optional_lists(self):
        self.assertEqual(
            normalize_request({'op': 'sentence', 'source': 'f=:+/'}),
            {
                'op': 'sentence',
                'source': 'f=:+/',
                'names': [],
                'representations': [],
            },
        )

    def test_sentence_request_accepts_parser_observables(self):
        request = normalize_request({
            'op': 'sentence',
            'source': 'f=:+/',
            'names': ['f'],
            'representations': ['atomic', 'linear'],
        })
        self.assertEqual(request['names'], ['f'])
        self.assertEqual(request['representations'], ['atomic', 'linear'])

    def test_words_request_accepts_source_bytes_via_text(self):
        request = normalize_request({'op': 'words', 'source': "1 NB. x"})
        self.assertEqual(request, {'op': 'words', 'source': "1 NB. x"})

    def test_invalid_requests_fail_before_loading_j(self):
        bad = [
            3,
            {'op': 'unknown'},
            {'op': 'eval'},
            {'op': 'sentence', 'source': '1', 'names': 'x'},
            {'op': 'sentence', 'source': '1', 'representations': ['debug']},
            {'op': 'name_class', 'name': ''},
            {'op': 'representation', 'name': 'f', 'kind': 'tree'},
        ]
        for request in bad:
            with self.subTest(request=request):
                with self.assertRaises(ValueError):
                    normalize_request(request)


if __name__ == '__main__':
    unittest.main()

class AssignmentObservationTests(unittest.TestCase):
    def check(self, source, words, silent):
        oracle = Oracle.__new__(Oracle)
        calls = []
        oracle.words = lambda source: {'words_hex': [word.encode().hex() for word in words]}
        oracle.run = lambda source: calls.append(source)
        value = {'type': 4, 'shape': [], 'data': [4]}
        oracle.read_noun = lambda name: value
        self.assertEqual(oracle.eval(source), {'silent': True} if silent else value)
        self.assertEqual(calls, [source if silent else 'rustjresult =: ' + source])

    def test_only_outer_copula_is_silent(self):
        self.check('x=:2', ['x', '=:', '2'], True)
        self.check('x=.2', ['x', '=.', '2'], True)
        self.check('a=:b=:1', ['a', '=:', 'b', '=:', '1'], True)

    def test_inner_copula_literal_and_comment_keep_result(self):
        self.check('x+(x=:2)', ['x', '+', '(', 'x', '=:', '2', ')'], False)
        self.check('(x=:2)', ['(', 'x', '=:', '2', ')'], False)
        self.check("'=:'", ["'=:'"], False)
        self.check('1 NB. =:', ['1', 'NB. =:'], False)


class RawNounTransportTests(unittest.TestCase):
    def test_raw_quote_body_uses_prefix_for_outer_copula_without_double_execution(self):
        oracle=Oracle.__new__(Oracle)
        calls=[]
        source="raw=:{{)n'broken}}"
        oracle.words=lambda text: {'words_hex':[b'raw'.hex(),b'=:'.hex()]} if text=='raw=:' else {'error':'open quote'}
        oracle.run=lambda text:calls.append(text)
        self.assertEqual(oracle.eval(source),{'silent':True})
        self.assertEqual(calls,[source])

    def test_script_transport_quotes_once_and_rejects_nul(self):
        oracle=Oracle.__new__(Oracle)
        calls=[]
        oracle.run=lambda text:calls.append(text)
        oracle.run_script("raw=:{{)n\n'broken\n}}")
        self.assertEqual(calls,["0!:100 'raw=:{{)n\n''broken\n}}'"])
        with self.assertRaises(ValueError): oracle.run_script('a\x00b')


class OracleErrorTransportTests(unittest.TestCase):
    def test_jdo_errors_preserve_locale_class_and_unknown_code(self):
        from unittest.mock import Mock
        oracle = Oracle.__new__(Oracle)
        oracle.jt = 123
        oracle.lib = Mock()
        oracle.lib.JDo.side_effect = [30, 21, 999, 0]
        self.assertEqual(oracle.run('a__holder+0'), {'error': 'locale error'})
        self.assertEqual(oracle.run('missing+0'), {'error': 'value error'})
        self.assertEqual(oracle.run('1'), {'error': 'J error 999'})
        self.assertIsNone(oracle.run('1'))
        self.assertEqual(oracle.lib.JDo.call_args_list[0].args,
                         (123, b'a__holder+0'))
