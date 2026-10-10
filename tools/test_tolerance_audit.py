import math
import unittest

from conformance import validate_cli_corpus
from tolerance_audit import KNOWN_PINNED_CCT_GAPS, j_float, tolerance_cases


class ToleranceAuditFixtureTests(unittest.TestCase):
    def test_roundtrip_preserves_float_word_class(self):
        for x in (1.0, 0.0, -0.0, 2.0 ** -44):
            s = j_float(x)
            self.assertTrue("." in s or "e" in s, s)
            self.assertFalse(s.startswith("-"), s)
        self.assertEqual(j_float(math.inf), "_")
        self.assertEqual(j_float(-math.inf), "__")
        self.assertEqual(j_float(math.nan), "_.")

    def test_known_pinned_default_cct_gaps_are_not_unscoped_waivers(self):
        self.assertEqual(len(KNOWN_PINNED_CCT_GAPS), 8)
        self.assertEqual(
            KNOWN_PINNED_CCT_GAPS,
            frozenset(
                f"{boundary}:{operation}"
                for boundary in ("lower_edge", "upper_edge")
                for operation in ("eq", "first", "last", "member")
            ),
        )
        self.assertNotIn("outside_upper:eq", KNOWN_PINNED_CCT_GAPS)
        self.assertNotIn("nontransitive_end:first", KNOWN_PINNED_CCT_GAPS)

    def test_boundary_suite_is_stable_and_labeled(self):
        cases = tolerance_cases()
        self.assertEqual(cases, tolerance_cases())
        self.assertEqual(len(cases), 48)
        validate_cli_corpus([source for _, source in cases])
        names = {name for name, _ in cases}
        for label in ("lower_edge", "upper_edge", "outside_lower", "outside_upper",
                      "nontransitive_end", "zero_sign", "positive_inf", "nan"):
            self.assertIn(label + ":first", names)
            self.assertIn(label + ":last", names)
            self.assertIn(label + ":eq", names)
            self.assertIn(label + ":member", names)
        self.assertNotEqual(
            next(source for name, source in cases if name == "upper_edge:first"),
            next(source for name, source in cases if name == "outside_upper:first"),
        )
