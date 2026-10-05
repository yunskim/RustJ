import unittest
from scan_contract_conformance import boolean_model, contract_failures, fixtures


class ScanContractTests(unittest.TestCase):
    def test_rejects_metadata_mismatch_and_unsupported_reference(self):
        c = {'type': 4, 'shape': [2], 'parallel_authorized': False, 'inject_identity': False}
        self.assertEqual(contract_failures(c, {'type': 4, 'shape': [2]}), [])
        self.assertEqual(contract_failures(c, {'error': 'unsupported'}), ['candidate for J error'])
        self.assertEqual(contract_failures(c, {'type': 1, 'shape': [2]}), ['type'])
        self.assertEqual(contract_failures(c, {'type': 4, 'shape': [3]}), ['shape'])
        self.assertIn('parallel permission', contract_failures({**c, 'parallel_authorized': True}, c))
        self.assertIn('identity injection', contract_failures({**c, 'inject_identity': True}, c))

    def test_scalar_empty_singleton_and_matrix_model(self):
        def value(shape, data): return {'type': 1, 'shape': shape, 'data': data}
        self.assertEqual(boolean_model(value([], [1]), '+'), value([1], [1]))
        self.assertEqual(boolean_model(value([2, 0], []), '+'), value([2, 0], []))
        self.assertEqual(boolean_model(value([1], [1]), '+'), value([1], [1]))
        self.assertEqual(boolean_model(value([2, 2], [0, 1, 1, 1]), '+'),
                         {'type': 4, 'shape': [2, 2], 'data': [0, 1, 1, 2]})

    def test_fixtures_cover_witness_and_numeric_boundaries(self):
        cases = fixtures()
        self.assertEqual(sum(accepted for _, _, accepted in cases), 274)
        self.assertTrue(any('9223372036854775807' in noun and not ok for noun, _, ok in cases))
        self.assertTrue(any('1e16' in noun and not ok for noun, _, ok in cases))
        self.assertTrue(any('"1' in verb and not ok for _, verb, ok in cases))
