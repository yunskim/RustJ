import unittest

from conformance import validate_cli_corpus
from search_three_way import classify, search_cases


def value(n):
    return {"type": 4, "shape": [1], "data": [n]}


class ThreeWayClassificationTests(unittest.TestCase):
    def test_all_five_outcome_categories_are_mutually_exclusive(self):
        a, b, c = value(1), value(2), value(3)
        self.assertEqual(classify(a, a, a), "pass")
        self.assertEqual(classify(a, a, b), "optimized_route_mismatch")
        self.assertEqual(classify(a, b, a), "rust_reference_mismatch")
        self.assertEqual(classify(a, b, b), "rust_semantic_mismatch")
        self.assertEqual(classify(a, b, c), "all_three_differ")

    def test_errors_are_compared_as_observable_outcomes(self):
        a = {"error": "domain error"}
        b = {"error": "length error"}
        self.assertEqual(classify(a, a, b), "optimized_route_mismatch")
        self.assertEqual(classify(a, b, a), "rust_reference_mismatch")

    def test_search_fixtures_are_stable_and_include_required_modes(self):
        cases = search_cases()
        self.assertEqual(cases, search_cases())
        self.assertEqual(len(cases), 45)
        validate_cli_corpus(cases)
        self.assertTrue(any(" i. " in x for x in cases))
        self.assertTrue(any(" i: " in x for x in cases))
        self.assertTrue(any(" e. " in x for x in cases))
        self.assertTrue(any("2 3 $" in x for x in cases))
        self.assertTrue(any("keys=:" in x for x in cases))
        self.assertTrue(any("widekeys=:" in x for x in cases))
