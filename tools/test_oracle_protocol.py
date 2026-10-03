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
