"""Protect audit classification from silently counting missing coverage as passes."""
import unittest

from name_compatibility_audit import SCRIPT_SETUPS, compare_trace, reference_trace


class AuditClassification(unittest.TestCase):
    def test_block_setup_uses_script_and_preserves_original_source(self):
        calls = []
        class FakeOracle:
            def run_script(self, source):
                calls.append(("script", source))
            def eval(self, source):
                calls.append(("sentence", source))
                return {"data": [7]}
        block = sorted(SCRIPT_SETUPS)[0]
        sources = [block, "a", "adv=:{{ a=.u\na }}"]
        outcomes, transports = reference_trace(FakeOracle(), sources)
        self.assertEqual(transports, ["script", "sentence", "sentence"])
        self.assertEqual(calls, list(zip(transports, sources)))
        self.assertEqual(outcomes[0], {"silent": True})
        self.assertEqual(outcomes[1], {"data": [7]})

    def test_script_setup_error_is_preserved(self):
        class FakeOracle:
            def run_script(self, source):
                return {"error": "syntax error"}
        outcomes, _ = reference_trace(FakeOracle(), [sorted(SCRIPT_SETUPS)[0]])
        self.assertEqual(outcomes, [{"error": "syntax error"}])

    def test_complete_match(self):
        self.assertEqual(compare_trace(["a"], [{"data": [1]}], [{"data": [1]}]),
                         ("matched", []))

    def test_missing_output_is_never_a_match(self):
        self.assertEqual(compare_trace(["a"], [{"data": [1]}], [])[0], "observation_gap")

    def test_unsupported_setup_keeps_secondary_differences(self):
        status, differences = compare_trace(
            ["a=:7", "a"], [{"silent": True}, {"data": [7]}],
            [{"error": "unsupported"}, {"error": "value error"}])
        self.assertEqual(status, "unsupported_gap")
        self.assertEqual(len(differences), 2)

    def test_definition_stop_does_not_hide_unsupported(self):
        status, _ = compare_trace(["f=:definition", "f 0"],
                                  [{"silent": True}, {"data": [1]}],
                                  [{"error": "unsupported"}])
        self.assertEqual(status, "unsupported_gap")

    def test_error_precedence_difference_is_a_mismatch(self):
        status, _ = compare_trace(["bad"], [{"error": "value error"}],
                                  [{"error": "domain error"}])
        self.assertEqual(status, "semantic_mismatch")

    def test_inconsistent_reference_is_a_harness_error(self):
        with self.assertRaises(RuntimeError):
            compare_trace(["a"], [], [])


if __name__ == "__main__":
    unittest.main()
