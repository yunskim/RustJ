"""Offline shape/fixture guardrails for the pinned file foreign diagnostic."""
import tempfile
import unittest
from pathlib import Path

from file_io_audit import (INITIAL, Probe, _probe_one, _run_ordered_case,
                           cases, ordered_cases, j_file_argument, j_file_name)


class FakeOracle:
    def __init__(self, outcome=None):
        self.outcome = outcome
        self.calls = []

    def run(self, expression):
        self.calls.append(expression)
        return self.outcome

    def eval(self, expression):
        self.calls.append(expression)
        return self.outcome


class FileIoAuditContractTests(unittest.TestCase):
    def test_cases_have_unique_names_and_reproducible_order(self):
        a = cases()
        self.assertEqual(a, cases())
        self.assertEqual(len(a), 15)
        self.assertEqual(len({p.name for p in a}), len(a))
        self.assertEqual({p.inspect for p in a}, {"run", "eval"})
        self.assertTrue(all("{file}" in p.j or "{name}" in p.j for p in a))
        self.assertTrue(all("{name}" in p.j for p in a if "1!:11" in p.j or "1!:12" in p.j))

    def test_source_critical_cases_are_present(self):
        names = {p.name for p in cases()}
        self.assertTrue({"negative_start_read", "read_past_eof", "read_negative_length",
                         "write_beyond_eof", "missing_file_bad_range",
                         "discarded_missing_read", "zero_length_missing_file"} <= names)

    def test_file_name_quoting_escapes_apostrophes(self):
        self.assertEqual(j_file_name(Path("/tmp/a'b.dat")), "'/tmp/a''b.dat'")
        self.assertEqual(j_file_argument(Path("/tmp/a'b.dat")), "<'/tmp/a''b.dat'")

    def test_never_uses_preexisting_paths(self):
        with tempfile.TemporaryDirectory() as d:
            outside = Path(d) / "outside.dat"
            outside.write_bytes(b"untouched")
            location = Path(d) / "cases"
            location.mkdir()
            p = Probe("one", "1!:11 ({name};2 3)", after=INITIAL)
            o = FakeOracle()
            observed = _probe_one(o, p, location)
            self.assertEqual(observed["status"], "matches_source_expectation")
            self.assertEqual(outside.read_bytes(), b"untouched")
            self.assertIn("one.dat", o.calls[0])
            self.assertNotIn(str(location), observed["j"])

    def test_errors_are_observable_even_when_result_is_discarded(self):
        with tempfile.TemporaryDirectory() as d:
            p = Probe("absent", "0 [ (1!:11 ({name};0 1))",
                      initial=None, after=None, expect_error=True)
            result = _probe_one(FakeOracle({"error": "file not found"}), p, Path(d))
            self.assertEqual(result["status"], "matches_source_expectation")
            result2 = _probe_one(FakeOracle(None), p, Path(d))
            self.assertEqual(result2["status"], "requires_review")

    def test_write_failure_does_not_mark_mutation_success(self):
        with tempfile.TemporaryDirectory() as d:
            p = Probe("invalid", "'XY' 1!:12 ({name};_9)",
                      expect_error=True, expected_error="index error", after=INITIAL)
            result = _probe_one(FakeOracle({"error": "index error"}), p, Path(d))
            self.assertEqual(result["status"], "matches_source_expectation")
            self.assertEqual(result["file_after_hex"], INITIAL.hex())

    def test_nonmatching_expected_value_is_reported(self):
        with tempfile.TemporaryDirectory() as d:
            p = Probe("read", "1!:11 ({name};2 3)", inspect="eval",
                      noun={"type": 2, "shape": [3], "data": list(b"CDE")})
            result = _probe_one(FakeOracle({"type": 2, "shape": [3], "data": [0, 0, 0]}),
                                p, Path(d))
            self.assertFalse(result["checks"]["noun_value"])
            self.assertEqual(result["status"], "requires_review")


    def test_syntax_error_is_not_accepted_as_observable_file_error(self):
        with tempfile.TemporaryDirectory() as d:
            p = Probe("missing", "1!:11 ({name};0 1)",
                      initial=None, after=None, expect_error=True)
            result = _probe_one(FakeOracle({"error": "length error"}), p, Path(d))
            self.assertEqual(result["status"], "requires_review")
            self.assertFalse(result["checks"]["not_argument_or_syntax_failure"])

    def test_out_of_range_requires_real_index_error(self):
        with tempfile.TemporaryDirectory() as d:
            p = Probe("bad", "1!:11 ({name};7 2)",
                      expect_error=True, expected_error="index error")
            result = _probe_one(FakeOracle({"error": "length error"}), p, Path(d))
            self.assertEqual(result["status"], "requires_review")
            self.assertFalse(result["checks"]["error_class"])



    def test_ordered_cases_are_independent_and_cover_effect_boundaries(self):
        cases_ = ordered_cases()
        self.assertEqual(cases_, ordered_cases())
        self.assertEqual(len(cases_), 6)
        self.assertEqual(len({p.name for p in cases_}), len(cases_))
        self.assertGreaterEqual(sum(len(p.steps) for p in cases_), 14)
        self.assertIn("completed_write_survives_later_error", {p.name for p in cases_})
        self.assertIn("right_error_prevents_left_write", {p.name for p in cases_})
        self.assertIn("discarded_write_has_effect", {p.name for p in cases_})
        self.assertIn("append_then_tail_read", {p.name for p in cases_})
        self.assertIn("truncate_then_full_read", {p.name for p in cases_})

    def test_ordered_case_error_prevents_left_write(self):
        class OrderedFake:
            def __init__(self):
                self.calls = []

            def run(self, expression):
                self.calls.append(expression)
                return {"error": "file not found"}

            def eval(self, expression):
                self.calls.append(expression)
                return {"type": 2, "shape": [1], "data": list(b"B")}

        case = next(x for x in ordered_cases() if x.name == "right_error_prevents_left_write")
        with tempfile.TemporaryDirectory() as d:
            fake = OrderedFake()
            result = _run_ordered_case(fake, case, Path(d))
            self.assertEqual(result["status"], "matches_source_expectation")
            self.assertEqual(len(fake.calls), 2)
            self.assertEqual((Path(d) / (case.name + ".dat")).read_bytes(), INITIAL)

    def test_ordered_case_fails_when_observable_preerror_write_occurred(self):
        class BuggyFake:
            def run(self, expression):
                # An incorrectly reordered implementation wrote before read failed.
                file = Path(expression.split("'")[1])
                file.write_bytes(b"Z" + INITIAL[1:])
                return {"error": "file not found"}

            def eval(self, expression):
                return {"type": 2, "shape": [1], "data": list(b"B")}

        case = next(x for x in ordered_cases() if x.name == "right_error_prevents_left_write")
        with tempfile.TemporaryDirectory() as d:
            result = _run_ordered_case(BuggyFake(), case, Path(d))
            self.assertEqual(result["status"], "requires_review")
            self.assertFalse(result["steps"][0]["checks"]["file_content"])

if __name__ == "__main__":
    unittest.main()
