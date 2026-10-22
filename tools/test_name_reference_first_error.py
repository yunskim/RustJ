"""Contract tests for isolated, pinned J-C NAME first-error collection.

No native C oracle or RustJ binary is called by these unit tests.
"""
import os
import unittest
from pathlib import Path

from name_reference_first_error import (
    TARGET_CASES, observe_reference_cases, raw_reference_trace,
    require_complete_reference_pair,
)
from name_compatibility_audit import SCOPE_FIXTURES, select_named_fixtures


class ReferenceOnlyProbe(unittest.TestCase):
    def test_required_first_error_cases_exist_in_scope_fixtures(self):
        selected = select_named_fixtures(SCOPE_FIXTURES, TARGET_CASES)
        self.assertEqual([name for name, _ in selected], list(TARGET_CASES))
        self.assertIn("call 4 + nounify 0", dict(selected)[
            "named_function_pos_rebinding_first_error"])

    def test_complete_reference_pair_is_required(self):
        libraries = [("j64", Path("j64.so")), ("j64avx2", Path("j64avx2.so"))]
        require_complete_reference_pair(libraries, {"unrun_reference_variants": []})
        with self.assertRaisesRegex(ValueError, "both J-C variants"):
            require_complete_reference_pair(libraries[:1], {
                "unrun_reference_variants": ["j64avx2"]})
        with self.assertRaisesRegex(ValueError, "both J-C variants"):
            require_complete_reference_pair(libraries[:1] * 2, {
                "unrun_reference_variants": []})

    def test_reference_only_keeps_error_order_and_post_error_reads(self):
        observed = []
        class FakeOracle:
            def eval(self, source):
                observed.append(("eval", source))
                return ({"error": "syntax error"} if source == "call 4 + nounify 0"
                        else {"data": [7]})

            def run_script(self, source):
                observed.append(("script", source))
                return None

            def run(self, source):
                observed.append(("raw", source))
                return ({"error": "domain error"} if source == "call 4 + nounify 0"
                        else None)

            def close(self):
                observed.append(("close", None))

        fixtures = [("first-error", ["setup", "call 4 + nounify 0", "op_probe_"])]
        previous = os.environ.get("J_LIBRARY")
        records = observe_reference_cases(
            [("j64", Path("/tmp/example-pinned-j64.so"))],
            fixtures, oracle_factory=FakeOracle)
        self.assertEqual(os.environ.get("J_LIBRARY"), previous)
        self.assertEqual(len(records), 1)
        record = records[0]
        self.assertEqual(record["case"], "first-error")
        self.assertEqual(record["reference_transport"], ["sentence"] * 3)
        self.assertEqual(record["first_error"], {
            "index": 1, "source": "call 4 + nounify 0", "error": "syntax error"})
        self.assertEqual(record["error_steps"], [record["first_error"]])
        self.assertEqual(record["reference"][2], {"data": [7]})
        self.assertEqual(record["raw_first_error"], {
            "index": 1, "source": "call 4 + nounify 0", "error": "domain error"})
        self.assertEqual(record["transport_error_disagreements"], [{
            "index": 1, "source": "call 4 + nounify 0",
            "wrapped_error": "syntax error", "raw_error": "domain error"}])
        self.assertEqual(record["raw_reference_transport"], ["raw-sentence"] * 3)
        self.assertEqual(
            [entry for entry in observed if entry[0] == "eval"],
            [("eval", x) for x in fixtures[0][1]])
        self.assertEqual(
            [entry for entry in observed if entry[0] == "raw"],
            [("raw", x) for x in fixtures[0][1]])
        self.assertEqual(len([x for x in observed if x[0] == "close"]), 2)

    def test_raw_transport_uses_true_script_for_existing_script_setups(self):
        from name_compatibility_audit import SCRIPT_SETUPS
        script = sorted(SCRIPT_SETUPS)[0]
        class FakeOracle:
            def __init__(self):
                self.calls = []

            def run(self, source):
                self.calls.append(("raw", source))

            def run_script(self, source):
                self.calls.append(("script", source))

        oracle = FakeOracle()
        results, transports = raw_reference_trace(oracle, [script, "a"])
        self.assertEqual(results, [{"silent": True}, {"silent": True}])
        self.assertEqual(transports, ["script", "raw-sentence"])
        self.assertEqual(oracle.calls, [("script", script), ("raw", "a")])

    def test_reference_only_always_closes_oracle_on_failure(self):
        closed = []
        class BrokenOracle:
            def eval(self, _source):
                raise RuntimeError("oracle failed")

            def close(self):
                closed.append(True)

        with self.assertRaisesRegex(RuntimeError, "oracle failed"):
            observe_reference_cases(
                [("j64", Path("j64.so"))], [("case", ["bad"])],
                oracle_factory=BrokenOracle)
        self.assertEqual(closed, [True])


if __name__ == "__main__":
    unittest.main()
