import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import frontend_contract_audit as audit


class FrontendAdmissionAudit(unittest.TestCase):
    def test_inspection_cannot_claim_execution_or_j_handler_authority(self):
        accepted = {"inspection_stage": "frontend", "execution_performed": False, "ok": True}
        self.assertEqual(audit.inspection_outcome("F", accepted), "accepted-representation")
        for changed in [{"execution_performed": True}, {"inspection_stage": "logical"}, {"ok": False}]:
            with self.assertRaises(ValueError):
                audit.inspection_outcome("F", accepted | changed)
        rejected = accepted | {"error": "syntax error", "category": "j-language", "j_handler_eligible": False}
        self.assertEqual(audit.inspection_outcome("F", rejected), "j-language")
        with self.assertRaises(ValueError):
            audit.inspection_outcome("F", rejected | {"j_handler_eligible": True})

    def test_internal_failures_are_not_capability_misses(self):
        for category in ["unsupported-capability", "verifier-defect", "backend-failure"]:
            failure = {"inspection_stage": "logical", "execution_performed": False,
                       "error": "failure", "category": category, "j_handler_eligible": False}
            self.assertEqual(audit.inspection_outcome("L", failure), category)
        with self.assertRaises(ValueError):
            audit.inspection_outcome("L", failure | {"category": "unknown"})

    def test_probe_keeps_frontend_handoff_binding_and_runtime_separate(self):
        values = [{"setup": True}] + [{"stage": s} for s in ["F", "H", "P", "G", "L", "R"]] + [{"after": True}]
        output = "\n".join(json.dumps(v) for v in values)
        with patch.object(audit.subprocess, "run", return_value=SimpleNamespace(stdout=output)) as run:
            setup, stages, after = audit.probe(Path("probe.exe"), ["a=:7"], "a_: + 1", ["a+0"])
        self.assertEqual(setup, values[:1])
        self.assertEqual(list(stages), ["F", "H", "P", "G", "L", "R"])
        self.assertEqual(after, values[-1:])
        operations = [line.split()[0] for line in run.call_args.kwargs["input"].splitlines()]
        self.assertEqual(operations, ["E", "F", "H", "P", "G", "L", "R", "E"])
        self.assertTrue(run.call_args.kwargs["check"])

    def test_machine_comparison_ignores_diagnostics_but_preserves_j_error_kind(self):
        self.assertEqual(audit.semantic_outcome({"error": "length error", "category": "j-language", "j_handler_eligible": True}), {"error": "length error"})


if __name__ == "__main__":
    unittest.main()
