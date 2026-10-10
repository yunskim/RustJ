import unittest
from alias_guard_audit import reference_probe


class ReferenceProbeTests(unittest.TestCase):
    def test_pos_and_missing_target_misses_keep_reference_j_errors_separate(self):
        for mode, error in [("noun-miss", "domain error"), ("missing-miss", "value error")]:
            with self.subTest(mode=mode):
                class FakeOracle:
                    def eval(self, source):
                        return {"error": error} if source == "h argument" else {"silent": True}
                setup, query, expected = reference_probe(FakeOracle(), ("+", mode, "", ""))
                self.assertEqual(query, "h argument")
                self.assertIn("argument=:_2", setup)
                self.assertEqual(expected, {"guard": "miss", "result": {"error": error}})

    def test_effect_probe_retains_argument_and_observes_counter_once(self):
        calls = []
        class FakeOracle:
            def eval(self, source):
                calls.append(source)
                return {"silent": True} if "=:" in source else {"data": [1]}
        setup, query, expected = reference_probe(FakeOracle(), ("+", "effect-miss", "", ""))
        self.assertEqual(calls, setup + [query, "count"])
        self.assertEqual(calls.count("argument=:_2 change"), 1)
        self.assertEqual(query, "h argument")
        self.assertEqual(expected["guard"], "miss")

    def test_reference_setup_errors_are_harness_errors(self):
        class FakeOracle:
            def eval(self, source):
                return {"error": "syntax error"}
        with self.assertRaises(RuntimeError):
            reference_probe(FakeOracle(), ("+", "monad", "", "1"))


if __name__ == "__main__":
    unittest.main()
