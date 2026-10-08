import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from verify_frontend_reports import checked_hash, verify_report


class FrontendReportEvidence(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.assets = self.root / "assets"
        self.record = {
            "rust_source_sha256": {"src/test.rs": self.file("src/test.rs", b"source")},
            "reference_sha256": {"j.dll": self.file("assets/target/cj-windows/j64/j.dll", b"dll")},
            "probe_sha256": self.file("target/debug/examples/frontend_contract_probe.exe", b"binary"),
            "counts": {"matched": 2, "runtime_gap": 2}, "observations": 4,
        }
        self.path = self.root / "report.json"

    def file(self, relative, data):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return hashlib.sha256(data).hexdigest()

    def verify(self, **changes):
        self.path.write_text(json.dumps(self.record | changes), encoding="utf-8")
        return verify_report(self.root, self.assets, self.path)

    def test_known_gaps_do_not_become_conformance_passes(self):
        self.assertEqual(self.verify(), 3)
        with self.assertRaises(ValueError):
            self.verify(fixture_set="numeric-overflow")

    def test_source_tampering_and_missing_binaries_fail(self):
        self.file("src/test.rs", b"changed")
        with self.assertRaises(ValueError):
            self.verify()
        self.file("src/test.rs", b"source")
        (self.root / "target/debug/examples/frontend_contract_probe.exe").unlink()
        with self.assertRaises(FileNotFoundError):
            self.verify()

    def test_stage_failures_and_missing_observations_fail(self):
        for changes in [{"failed": 1}, {"observations": 5},
                        {"counts": {"unknown": 4}},
                        {"stage_admission_counts": {"H": {"verifier-defect": 1}}},
                        {"stage_admission_counts": {"L": {"backend-failure": 1}}}]:
            with self.assertRaises(ValueError):
                self.verify(**changes)

    def test_recorded_source_paths_cannot_escape_the_root(self):
        with self.assertRaises(ValueError):
            checked_hash(self.assets, "../src/test.rs", self.record["rust_source_sha256"]["src/test.rs"])
