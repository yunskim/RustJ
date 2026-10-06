"""Offline JMF smoke plan checks; these tests do not prove JMF runtime support."""
import unittest

from jmf_smoke import jmf_expressions


class JMFScriptsTests(unittest.TestCase):
    def test_each_mode_maps_checks_and_unmaps_before_the_next(self):
        steps = jmf_expressions("'/tmp/a''b.jmf'")
        self.assertEqual(len(steps), 11)
        self.assertEqual(steps[0], "rustj_jmf_file=: <'/tmp/a''b.jmf'")
        self.assertEqual(steps[1], "createjmf_jmf_ rustj_jmf_file,<4096")
        for i, (label, mode) in enumerate((("rw", 0), ("ro", 1), ("cow", 2))):
            start = 2 + i * 3
            self.assertEqual(steps[start],
                             f"map_jmf_ (<'rustj_jmf_{label}'),rustj_jmf_file,'';{mode}")
            self.assertEqual(steps[start + 1], f"'' -: rustj_jmf_{label}")
            self.assertEqual(steps[start + 2], f"unmap_jmf_ 'rustj_jmf_{label}'")

    def test_plan_does_not_claim_boxed_payloads_or_permanent_files(self):
        text = "\n".join(jmf_expressions("'/tmp/test.jmf'"))
        self.assertNotIn("additem", text)
        self.assertNotIn("settypeshape", text)
        self.assertNotIn("force", text)
        self.assertEqual(text.count("unmap_jmf_"), 3)


if __name__ == "__main__":
    unittest.main()
