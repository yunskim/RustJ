"""Protect audit classification from silently counting missing coverage as passes."""
import unittest
import tempfile
import json
from pathlib import Path

from name_compatibility_audit import SCRIPT_SETUPS, compare_trace, reference_trace, reference_inputs, PIN


class ReferenceSelection(unittest.TestCase):
    def build(self, root, variant="j64", commit=PIN):
        library = root / "bin/linux" / variant / "libj.so"
        library.parent.mkdir(parents=True)
        library.write_bytes(b"test oracle placeholder")
        (root / f"manifest-{variant}.json").write_text(json.dumps({"commit": commit, "platform": "linux", "variant": variant}))
        return library

    def test_single_built_variant_does_not_claim_other_variant(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            library = self.build(root)
            libraries, metadata = reference_inputs(reference_root=root, platform="linux")
            self.assertEqual(libraries, [("j64", library)])
            self.assertEqual(list(metadata["reference_manifests"]), ["j64"])
            self.assertEqual(metadata["unrun_reference_variants"], ["j64avx2"])

    def test_wrong_revision_and_partial_build_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            library = self.build(root, commit="wrong")
            with self.assertRaisesRegex(ValueError, "Mismatched"):
                reference_inputs(reference_root=root, platform="linux")
            library.unlink()
            with self.assertRaisesRegex(ValueError, "Incomplete"):
                reference_inputs(reference_root=root, platform="linux")

    def test_missing_linux_reference_is_not_a_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, "No built"):
                reference_inputs(reference_root=Path(directory), platform="linux")

    def test_windows_selection_preserves_both_dll_variants(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            location = root / "target/cj-windows/j64"
            location.mkdir(parents=True)
            for name in ["j.dll", "javx2.dll"]:
                (location / name).write_bytes(b"test placeholder")
            libraries, _ = reference_inputs(assets_root=root, platform="win32")
            self.assertEqual([name for name, _ in libraries], ["j.dll", "javx2.dll"])
            with self.assertRaisesRegex(ValueError, "Linux/WSL"):
                reference_inputs(assets_root=root, platform="linux")


class AuditClassification(unittest.TestCase):
    def test_block_setup_uses_script_and_preserves_original_source(self):
        calls = []
        class FakeOracle:
            def run_script(self, source):
                calls.append(("script", source))
            def eval(self, source):
                calls.append(("sentence", source))
                return {"data": [7]}
        block = sorted(SCRIPT_SETUPS)[0]
        sources = [block, "a", "adv=:{{ a=.u\na }}"]
        outcomes, transports = reference_trace(FakeOracle(), sources)
        self.assertEqual(transports, ["script", "sentence", "sentence"])
        self.assertEqual(calls, list(zip(transports, sources)))
        self.assertEqual(outcomes[0], {"silent": True})
        self.assertEqual(outcomes[1], {"data": [7]})

    def test_base_local_write_uses_script_setup_but_observes_queries(self):
        from name_compatibility_audit import FIXTURES
        sources = dict(FIXTURES)["base_noun_local_write"]
        calls = []
        class FakeOracle:
            def run_script(self, source):
                calls.append(("script", source))
            def eval(self, source):
                calls.append(("sentence", source))
                return {"data": [7]}
        _, transport = reference_trace(FakeOracle(), sources)
        self.assertEqual(transport, ["sentence", "script", "sentence", "sentence"])
        self.assertEqual(calls, list(zip(transport, sources)))

    def test_script_setup_error_is_preserved(self):
        class FakeOracle:
            def run_script(self, source):
                return {"error": "syntax error"}
        outcomes, _ = reference_trace(FakeOracle(), [sorted(SCRIPT_SETUPS)[0]])
        self.assertEqual(outcomes, [{"error": "syntax error"}])

    def test_complete_match(self):
        self.assertEqual(compare_trace(["a"], [{"data": [1]}], [{"data": [1]}]),
                         ("matched", []))

    def test_missing_output_is_never_a_match(self):
        self.assertEqual(compare_trace(["a"], [{"data": [1]}], [])[0], "observation_gap")

    def test_unsupported_setup_keeps_secondary_differences(self):
        status, differences = compare_trace(
            ["a=:7", "a"], [{"silent": True}, {"data": [7]}],
            [{"error": "unsupported"}, {"error": "value error"}])
        self.assertEqual(status, "unsupported_gap")
        self.assertEqual(len(differences), 2)

    def test_definition_stop_does_not_hide_unsupported(self):
        status, _ = compare_trace(["f=:definition", "f 0"],
                                  [{"silent": True}, {"data": [1]}],
                                  [{"error": "unsupported"}])
        self.assertEqual(status, "unsupported_gap")

    def test_error_precedence_difference_is_a_mismatch(self):
        status, _ = compare_trace(["bad"], [{"error": "value error"}],
                                  [{"error": "domain error"}])
        self.assertEqual(status, "semantic_mismatch")

    def test_inconsistent_reference_is_a_harness_error(self):
        with self.assertRaises(RuntimeError):
            compare_trace(["a"], [], [])


if __name__ == "__main__":
    unittest.main()
