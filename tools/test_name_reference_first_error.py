"""Contract tests for isolated, pinned J-C NAME first-error collection.

No native C oracle or RustJ binary is called by these unit tests.
"""
import os
import unittest
from pathlib import Path

from name_reference_first_error import (
    TARGET_CASES, assess_reference_probe_consensus, focused_probe_step,
    observe_reference_cases, raw_reference_trace, require_complete_reference_pair,
    select_probe_index,
)
from name_compatibility_audit import SCOPE_FIXTURES, select_named_fixtures


class ReferenceOnlyProbe(unittest.TestCase):
    def test_required_first_error_cases_exist_in_scope_fixtures(self):
        selected = select_named_fixtures(SCOPE_FIXTURES, TARGET_CASES)
        self.assertEqual([name for name, _ in selected], list(TARGET_CASES))
        self.assertIn("call 4 + nounify 0", dict(selected)[
            "named_function_pos_rebinding_first_error"])

    def test_selected_probe_follows_the_pos_rebinding_not_an_earlier_call(self):
        sources_by_case = dict(SCOPE_FIXTURES)
        expected_indexes = {
            "named_locative_alias_pos_mismatch": 4,
            "named_direct_definition_reparse_after_pos_restore": 6,
            "named_direct_body_noun_before_call": 4,
            "named_direct_body_rhs_nounifies_before_call": 4,
            "named_function_pos_rebinding_first_error": 5,
        }
        self.assertEqual(set(TARGET_CASES), set(expected_indexes))
        for case, index in expected_indexes.items():
            with self.subTest(case=case):
                self.assertEqual(select_probe_index(case, sources_by_case[case]),
                                 index)

    def test_focused_probe_does_not_confuse_setup_failure_with_target_error(self):
        sources = ["setup", "call 4 + nounify 0", "op_probe_"]
        wrapped = [{"error": "value error"},
                   {"error": "syntax error"}, {"data": [7]}]
        raw = [{"error": "value error"},
               {"error": "domain error"}, {"silent": True}]
        probe = focused_probe_step(
            "named_function_pos_rebinding_first_error", sources, wrapped, raw)
        self.assertEqual(probe["index"], 1)
        self.assertEqual(probe["wrapped"], {"error": "syntax error"})
        self.assertEqual(probe["raw"], {"error": "domain error"})
        self.assertEqual(probe["wrapped_errors_before"], [
            {"index": 0, "source": "setup", "error": "value error"}])
        self.assertEqual(probe["raw_errors_before"], [
            {"index": 0, "source": "setup", "error": "value error"}])

    def test_missing_probe_occurrence_is_not_silently_replaced(self):
        with self.assertRaisesRegex(ValueError, "Probe source occurrence missing"):
            select_probe_index("named_direct_body_noun_before_call", ["call 4"])
        self.assertIsNone(select_probe_index("unselected_scope_case", ["call 4"]))

    def test_complete_reference_pair_is_required(self):
        libraries = [("j64", Path("j64.so")), ("j64avx2", Path("j64avx2.so"))]
        require_complete_reference_pair(libraries, {"unrun_reference_variants": []})
        with self.assertRaisesRegex(ValueError, "both J-C variants"):
            require_complete_reference_pair(libraries[:1], {
                "unrun_reference_variants": ["j64avx2"]})
        with self.assertRaisesRegex(ValueError, "both J-C variants"):
            require_complete_reference_pair(libraries[:1] * 2, {
                "unrun_reference_variants": []})
        with self.assertRaisesRegex(ValueError, "both J-C variants"):
            require_complete_reference_pair(
                [("wrong", Path("wrong.so")), ("other", Path("other.so"))],
                {"unrun_reference_variants": []})
        require_complete_reference_pair([
            ("j.dll", Path("j.dll")), ("javx2.dll", Path("javx2.dll"))
        ], {"unrun_reference_variants": []})

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
        self.assertIsNone(record["focused_probe"])
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


class ReferenceConsensusTests(unittest.TestCase):
    CASE = "named_direct_body_rhs_nounifies_before_call"
    VARIANTS = ("j64", "j64avx2")

    def build_records(self, raw_error="syntax error",
                      wrapped_error="syntax error"):
        sources = dict(SCOPE_FIXTURES)[self.CASE]
        index = select_probe_index(self.CASE, sources)
        records = []
        for variant in self.VARIANTS:
            wrapped = [{"silent": True} for _ in sources]
            raw = [{"silent": True} for _ in sources]
            if wrapped_error is not None:
                wrapped[index] = {"error": wrapped_error}
            if raw_error is not None:
                raw[index] = {"error": raw_error}
            records.append({
                "case": self.CASE, "variant": variant, "sources": sources,
                "reference": wrapped, "raw_reference": raw,
                "focused_probe": focused_probe_step(
                    self.CASE, sources, wrapped, raw),
            })
        return records

    def assess(self, records):
        return assess_reference_probe_consensus(
            records, [self.CASE], self.VARIANTS)

    def test_two_variants_and_both_transports_agree_on_error_kind(self):
        outcome = self.assess(self.build_records())
        self.assertTrue(outcome["complete"])
        self.assertEqual(outcome["cases"][0]["status"], "agreed_error")
        self.assertEqual(outcome["cases"][0]["raw_error_kind"], "syntax error")
        self.assertNotIn("RustJ passed", outcome["scope"])

    def test_consensus_on_no_error_is_not_misreported_as_an_error(self):
        outcome = self.assess(self.build_records(None, None))
        self.assertTrue(outcome["complete"])
        self.assertEqual(outcome["cases"][0]["status"], "agreed_no_error")
        self.assertIsNone(outcome["cases"][0]["raw_error_kind"])

    def test_reference_variant_disagreement_blocks_semantic_verdict(self):
        records = self.build_records()
        index = records[1]["focused_probe"]["index"]
        records[1]["raw_reference"][index] = {"error": "domain error"}
        records[1]["focused_probe"]["raw"] = {"error": "domain error"}
        records[1]["reference"][index] = {"error": "domain error"}
        records[1]["focused_probe"]["wrapped"] = {"error": "domain error"}
        outcome = self.assess(records)
        self.assertFalse(outcome["complete"])
        self.assertTrue(any("variants disagree" in reason
                            for reason in outcome["cases"][0]["reasons"]))

    def test_raw_and_wrapped_error_disagreement_blocks_semantic_verdict(self):
        outcome = self.assess(self.build_records(
            raw_error="syntax error", wrapped_error="domain error"))
        self.assertFalse(outcome["complete"])
        self.assertTrue(any("raw JDo and wrapped" in reason
                            for reason in outcome["cases"][0]["reasons"]))

    def test_setup_failure_blocks_target_even_if_target_variants_agree(self):
        records = self.build_records()
        for record in records:
            record["reference"][0] = {"error": "value error"}
            record["focused_probe"]["wrapped_errors_before"] = [
                {"index": 0, "source": record["sources"][0],
                 "error": "value error"}]
        outcome = self.assess(records)
        self.assertFalse(outcome["complete"])
        self.assertTrue(any("earlier setup sentence failed" in reason
                            for reason in outcome["cases"][0]["reasons"]))

    def test_missing_duplicate_or_unexpected_variant_record_blocks(self):
        records = self.build_records()
        self.assertFalse(self.assess(records[:1])["complete"])
        self.assertFalse(self.assess(records + [records[0]])["complete"])
        unexpected = dict(records[0], variant="not-pinned")
        outcome = self.assess(records + [unexpected])
        self.assertFalse(outcome["complete"])
        self.assertTrue(any("unexpected observation keys" in reason
                            for item in outcome["cases"]
                            for reason in item["reasons"]))

    def test_wrong_target_index_or_short_trace_cannot_be_accepted(self):
        records = self.build_records()
        records[0]["focused_probe"]["index"] = 0
        self.assertFalse(self.assess(records)["complete"])
        records = self.build_records()
        records[0]["reference"].pop()
        self.assertFalse(self.assess(records)["complete"])

    def test_consensus_rejects_falsely_empty_setup_error_provenance(self):
        records = self.build_records()
        records[0]["reference"][0] = {"error": "value error"}
        # The derived focused_probe still claims no earlier errors.
        outcome = self.assess(records)
        self.assertFalse(outcome["complete"])
        self.assertTrue(any("prerequisite errors disagree" in reason
                            for reason in outcome["cases"][0]["reasons"]))

    def test_consensus_rejects_empty_or_duplicated_dimensions(self):
        records = self.build_records()
        empty = assess_reference_probe_consensus(records, [self.CASE], [])
        self.assertFalse(empty["complete"])
        self.assertTrue(any("variant dimension is empty" in reason
                            for case in empty["cases"] for reason in case["reasons"]))
        duplicates = assess_reference_probe_consensus(
            records, [self.CASE, self.CASE], self.VARIANTS)
        self.assertFalse(duplicates["complete"])

    def test_target_without_a_defined_locator_fails_closed(self):
        records = self.build_records()
        for record in records:
            record["case"] = "unrelated_scope_case"
        outcome = assess_reference_probe_consensus(
            records, ["unrelated_scope_case"], self.VARIANTS)
        self.assertFalse(outcome["complete"])
        self.assertTrue(any("no defined probe locator" in reason
                            for reason in outcome["cases"][0]["reasons"]))



if __name__ == "__main__":
    unittest.main()
