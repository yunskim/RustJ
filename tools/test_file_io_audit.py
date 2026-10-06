"""Offline shape/fixture guardrails for the pinned file foreign diagnostic."""
import tempfile
import unittest
from pathlib import Path

from file_io_audit import INITIAL, Probe, _probe_one, cases, j_file_argument


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
        self.assertTrue(all("{file}" in p.j for p in a))

    def test_source_critical_cases_are_present(self):
        names = {p.name for p in cases()}
        self.assertTrue({"negative_start_read", "read_past_eof", "read_negative_length",
                         "write_beyond_eof", "missing_file_bad_range",
                         "discarded_missing_read", "zero_length_missing_file"} <= names)

    def test_file_name_quoting_escapes_apostrophes(self):
        self.assertEqual(j_file_argument(Path("/tmp/a'b.dat")), "<'/tmp/a''b.dat'>")

    def test_never_uses_preexisting_paths(self):
        with tempfile.TemporaryDirectory() as d:
            outside = Path(d) / "outside.dat"
            outside.write_bytes(b"untouched")
            location = Path(d) / "cases"
            location.mkdir()
            p = Probe("one", "1!:11 ({file};2 3)", after=INITIAL)
            o = FakeOracle()
            observed = _probe_one(o, p, location)
            self.assertEqual(observed["status"], "matches_source_expectation")
            self.assertEqual(outside.read_bytes(), b"untouched")
            self.assertIn("one.dat", o.calls[0])
            self.assertNotIn(str(location), observed["j"])

    def test_errors_are_observable_even_when_result_is_discarded(self):
        with tempfile.TemporaryDirectory() as d:
            p = Probe("absent", "0 [ (1!:11 ({file};0 1))",
                      initial=None, after=None, expect_error=True)
            result = _probe_one(FakeOracle({"error": "file not found"}), p, Path(d))
            self.assertEqual(result["status"], "matches_source_expectation")
            result2 = _probe_one(FakeOracle(None), p, Path(d))
            self.assertEqual(result2["status"], "requires_review")

    def test_write_failure_does_not_mark_mutation_success(self):
        with tempfile.TemporaryDirectory() as d:
            p = Probe("invalid", "'XY' 1!:12 ({file};_9)",
                      expect_error=True, after=INITIAL)
            result = _probe_one(FakeOracle({"error": "index error"}), p, Path(d))
            self.assertEqual(result["status"], "matches_source_expectation")
            self.assertEqual(result["file_after_hex"], INITIAL.hex())

    def test_nonmatching_expected_value_is_reported(self):
        with tempfile.TemporaryDirectory() as d:
            p = Probe("read", "1!:11 ({file};2 3)", inspect="eval",
                      noun={"type": 2, "shape": [3], "data": list(b"CDE")})
            result = _probe_one(FakeOracle({"type": 2, "shape": [3], "data": [0, 0, 0]}),
                                p, Path(d))
            self.assertFalse(result["checks"]["noun_value"])
            self.assertEqual(result["status"], "requires_review")


if __name__ == "__main__":
    unittest.main()
