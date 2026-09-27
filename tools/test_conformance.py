import unittest
from conformance import equal, generated

class ComparatorTests(unittest.TestCase):
    def test_structure_and_types(self):
        a = {'type':4, 'shape':[1], 'data':[2]}
        self.assertTrue(equal(a, dict(a)))
        self.assertFalse(equal(a, {**a, 'type':8}))
        self.assertFalse(equal(a, {**a, 'shape':[]}))
        self.assertFalse(equal({'error':'length error'}, {'error':'domain error'}))
    def test_float_contract(self):
        def v(x): return {'type':8, 'shape':[], 'data':[x]}
        self.assertFalse(equal(v(0.0),v(-0.0)))
        self.assertTrue(equal(v(1.0),v(1.0+1e-15)))
        self.assertFalse(equal(v(1.0),v(1.01)))
        self.assertTrue(equal(v('nan'),v('nan')))
        self.assertFalse(equal(v('inf'),v('-inf')))
    def test_reproducibility(self):
        self.assertEqual(generated(17,10),generated(17,10))
        self.assertNotEqual(generated(17,10),generated(18,10))
        self.assertEqual(len(generated(17,10)),80)
