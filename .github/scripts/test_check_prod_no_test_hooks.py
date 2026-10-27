import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check-prod-no-test-hooks.py")
SPEC = importlib.util.spec_from_file_location("check_prod_no_test_hooks", SCRIPT)
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


def scan_source(source):
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "sample.rs"
        path.write_text(source, encoding="utf-8")
        return checker.scan(path, "rust/src/s_content/sample.rs")


class ProdHookCheckerTests(unittest.TestCase):
    def test_library_observe_for_test_under_s_incremental_is_reported(self):
        source = '#[cfg(feature = "s_incremental")]\npub fn observe_for_test() {}\n'
        self.assertTrue(any("observe_for_test" in v for v in scan_source(source)))

    def test_any_test_or_s_incremental_gate_is_reported(self):
        source = (
            '#[cfg(any(test, feature = "s_incremental"))]\n'
            "pub fn probe_hook_for_test() {}\n"
        )
        self.assertTrue(any("probe_hook_for_test" in v for v in scan_source(source)))

    def test_cfg_test_gate_is_not_reported(self):
        self.assertEqual(scan_source("#[cfg(test)]\nfn helper_for_test() {}\n"), [])

    def test_testonly_feature_gate_is_not_reported(self):
        source = '#[cfg(feature = "s_capture_native_testonly")]\npub fn seed_for_testonly() {}\n'
        self.assertEqual(scan_source(source), [])

    def test_cfg_test_module_is_not_reported(self):
        source = "#[cfg(test)]\nmod tests {\n    fn helper_for_test() {}\n}\n"
        self.assertEqual(scan_source(source), [])

    def test_production_feature_sets_evaluation(self):
        self.assertTrue(checker.gate_is_test("#[cfg(test)]"))
        self.assertFalse(checker.gate_is_test("#[cfg(not(test))]"))
        self.assertFalse(checker.gate_is_test('#[cfg(feature = "s_incremental")]'))
        self.assertFalse(checker.gate_is_test('#[cfg(any(test, feature = "s_incremental"))]'))
        self.assertTrue(checker.gate_is_test('#[cfg(all(test, feature = "s_incremental"))]'))


if __name__ == "__main__":
    unittest.main()
