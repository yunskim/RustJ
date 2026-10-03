import unittest
from frontend_stage_conformance import atomic_function, atomic_node, expected_row, source_rows, equivalent


def chars(s):
    return {'type': 2, 'shape': [len(s)], 'data': list(s.encode())}


def box(shape, data):
    return {'type': 32, 'shape': shape, 'data': data}


class FrontendProjectionTests(unittest.TestCase):
    def test_completed_modifier_is_single_fork_child(self):
        modifier = box([2], [chars('/'), box([1], [chars('+')])])
        fork = box([], [box([2], [chars('3'), box([3], [modifier, chars('%'), chars('#')])])])
        expected = {'head_hex': '33', 'operands': [
            {'head_hex':'2f', 'operands':[{'head_hex':'2b', 'operands':[]}]},
            {'head_hex':'25', 'operands':[]}, {'head_hex':'23', 'operands':[]}]}
        self.assertEqual(atomic_function(fork), expected)

    def test_noun_shape_type_and_boxing_survive(self):
        noun = box([], [{'type':4, 'shape':[], 'data':[7]}])
        self.assertEqual(atomic_node(box([2], [chars('0'), noun])), {'noun':noun})

    def test_unknown_atomic_encoding_fails_closed(self):
        for value in [chars('+'), box([1], [chars('+')]), box([], [])]:
            with self.assertRaises(ValueError):
                atomic_function(value)

    def test_operand_comparison_preserves_float_contract(self):
        def operand(x):
            return {'noun': {'type':8, 'shape':[], 'data':[x]}}
        self.assertFalse(equivalent(operand(0.0), operand(-0.0)))
        self.assertTrue(equivalent(operand(1.0), operand(1.0+1e-15)))

    def test_row_priority_is_first_match(self):
        rows = [[set(['Verb'])]*4, [set(['Verb','Noun'])]*4]
        self.assertEqual(expected_row(rows, ['Verb']*4), 0)
        self.assertEqual(expected_row(rows, ['Noun']*4), 1)
        self.assertIsNone(expected_row(rows, ['Mark']*4))

    def test_missing_source_contract_is_not_a_pass(self):
        with self.assertRaises(ValueError):
            source_rows('PT cases[] = {};')
