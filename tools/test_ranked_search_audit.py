import unittest

from conformance import validate_cli_corpus
from ranked_search_audit import ranked_search_cases, rank_adversarial_cases


class RankSearchCorpusTests(unittest.TestCase):
    def test_deterministic_diagnostic_corpus(self):
        cases = ranked_search_cases()
        self.assertEqual(cases, ranked_search_cases())
        self.assertEqual(len(cases), 20)
        self.assertEqual(len({label for label, _ in cases}), len(cases))
        validate_cli_corpus([source for _, source in cases])

    def test_rank_probe_keeps_first_last_member_and_negative_cases(self):
        cases = dict(ranked_search_cases())
        self.assertIn("i.\"1 1", cases["row_first"])
        self.assertIn("i:\"1 1", cases["row_last"])
        self.assertIn("e.\"1 1", cases["row_member"])
        self.assertIn("i.\"0 0", cases["scalar_cells_first"])
        self.assertIn("i.\"1 1", cases["frame_mismatch"])
        self.assertIn("i.\"1 1", cases["empty_frame"])
        self.assertIn("i.\"1 1", cases["empty_frame_char"])
        self.assertIn("i.\"1 1", cases["empty_frame_float"])
        self.assertIn("+/\"1", cases["empty_frame_sum"])
        self.assertIn("+\"1 1", cases["empty_frame_add"])

    def test_rank_adversarial_corpus_keeps_zero_frame_distinct_from_empty_cells(self):
        cases = rank_adversarial_cases()
        self.assertEqual(len(cases), 24)
        self.assertEqual(len({label for label, _ in cases}), len(cases))
        validate_cli_corpus([source for _, source in cases])
        by_name = dict(cases)
        self.assertIn("i.0 3", by_name["zero_frame_front_reduce"])
        self.assertIn("i.2 0", by_name["empty_cells_nonzero_frame_reduce"])
        self.assertIn("i.2 0 3", by_name["zero_frame_middle_reduce"])
        self.assertIn("i.2 3 0", by_name["zero_frame_tail_rank0_add"])
        self.assertIn("i.0 3", by_name["one_side_empty_left_add"])
        self.assertIn("i.2 2 3", by_name["middle_frame_mismatch"])
        self.assertIn('"_1', by_name["negative_rank_reduce"])
        self.assertIn('"99', by_name["oversized_rank_reduce"])
        self.assertIn('+"0 0', by_name["nested_rank_empty"])
