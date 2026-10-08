import json
from pathlib import Path
import subprocess
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import name_effect_audit as audit


class NameEffectAudit(unittest.TestCase):
    def test_only_marked_sentence_uses_the_plan_and_post_state_is_compared(self):
        class Oracle:
            def eval(self, source):
                return {"silent": True} if "=:" in source else {"error": "value error"}

            def close(self):
                pass

        output = '\n'.join(json.dumps(value) for value in
                           [{"silent": True}, {"silent": True}, {"error": "value error"}])
        fixture = [("one", ["a=:7"], "b=:a_:", ["a+0"])]
        with patch.object(audit, "FIXTURES", fixture), patch.object(audit, "Oracle", Oracle), \
                patch.object(audit.subprocess, "run", return_value=SimpleNamespace(stdout=output)) as run:
            records = audit.audit(Path("assets"), Path("probe.exe"))
        self.assertEqual([record["status"] for record in records], ["matched"] * 4)
        for call, mode in zip(run.call_args_list, ["effect", "array"] * 2):
            self.assertEqual(call.kwargs["input"], "eval\ta=:7\n" + mode + "\tb=:a_:\neval\ta+0\n")
            self.assertTrue(call.kwargs["check"])

    def test_failed_probe_is_not_retried_as_ordinary_execution(self):
        class Oracle:
            def eval(self, source):
                return {"silent": True}

            def close(self):
                pass

        with patch.object(audit, "FIXTURES", [("one", [], "a_:", [])]), \
                patch.object(audit, "Oracle", Oracle), \
                patch.object(audit.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "probe")) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                audit.audit(Path("assets"), Path("probe.exe"))
        self.assertEqual(run.call_count, 1)


if __name__ == "__main__":
    unittest.main()
