import unittest
from conformance import equal, generated, runtime_coverage_boundary, cases, validate_cli_corpus

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

    def test_nested_boxes_preserve_type_shape_and_float_contract(self):
        def box(v): return {'type':32, 'shape':[], 'data':[v]}
        def number(x): return {'type':8, 'shape':[], 'data':[x]}
        self.assertTrue(equal(box(box(number(1.0))),box(box(number(1.0+1e-15)))))
        self.assertFalse(equal(box(number(0.0)),box(number(-0.0))))
        self.assertFalse(equal(box(number(1.0)),box({'type':4,'shape':[],'data':[1]})))
        self.assertFalse(equal(box(number(1.0)),box({'type':8,'shape':[1],'data':[1.0]})))


class CoverageBoundaryTests(unittest.TestCase):
    def test_only_exact_unsupported_runtime_forms_are_classified(self):
        reference = {'type': 8, 'shape': [], 'data': [2.0]}
        source = '(+ ("-)) i.4'
        self.assertIsNotNone(runtime_coverage_boundary(source, reference, {'error': 'unsupported'}))
        self.assertIsNone(runtime_coverage_boundary(source, reference, {'error': 'domain error'}))
        self.assertIsNone(runtime_coverage_boundary('+/entrynoun', reference, {'error': 'unsupported'}))
        for supported in ['(+ (@:-)) i.4', '(entryverb/ % #) entrynoun', '(entryverb/ % #) entrycopy']:
            self.assertIsNone(runtime_coverage_boundary(supported, reference, {'error': 'unsupported'}))
        self.assertIsNone(runtime_coverage_boundary(source, {'error': 'domain error'}, {'error': 'unsupported'}))
        self.assertIsNone(runtime_coverage_boundary(source, reference, reference))


class CliTransportTests(unittest.TestCase):
    def test_corpus_has_one_physical_sentence_per_cli_request(self):
        validate_cli_corpus(cases())
        for body in ["f=:3 : 'goto_a.\nlabel_a.'", "1\r2"]:
            with self.assertRaisesRegex(ValueError, 'physical line break'):
                validate_cli_corpus([body])
