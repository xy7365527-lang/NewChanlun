import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check-prod-no-test-hooks.py")
SPEC = importlib.util.spec_from_file_location("check_prod_no_test_hooks", SCRIPT)
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


def scan_source(source, rel="rust/src/s_content/sample.rs"):
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "sample.rs"
        path.write_text(source, encoding="utf-8")
        return checker.scan(path, rel)


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


    def test_unknown_cfg_predicates_are_production_reachable(self):
        features = frozenset({"s_session", "s_incremental"})
        for text in ("unix", 'target_os = "macos"', "debug_assertions", "not(unix)",
                     'all(unix, not(target_os = "windows"))', "any(test, debug_assertions)"):
            with self.subTest(cfg=text):
                self.assertTrue(checker.evaluate_cfg(checker.parse_cfg(text), features))
                self.assertFalse(checker.gate_is_test(f"#[cfg({text})]"))
        self.assertFalse(checker.evaluate_cfg(checker.parse_cfg("all(test, unix)"), features))
        self.assertTrue(checker.gate_is_test("#[cfg(all(test, unix))]"))
        source = '#[cfg(target_os = "linux")]\nfn probe_hook_for_test() {}\n'
        self.assertTrue(any("probe_hook_for_test" in v for v in scan_source(source)))

    def test_default_build_only_exemption_requires_incremental_gate(self):
        rel = "rust/src/bin/s_structure_session.rs"
        gate = '#[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]\n'
        self.assertEqual(scan_source(gate + "fn testonly_pause(stage: &str) {}\n", rel), [])
        for source in ("fn testonly_pause(stage: &str) {}\n",
                       "#[cfg(not(test))]\nfn testonly_pause(stage: &str) {}\n",
                       '#[cfg(feature = "s_incremental")]\nfn testonly_pause(stage: &str) {}\n'):
            with self.subTest(source=source):
                self.assertTrue(any("testonly_pause" in v for v in scan_source(source, rel)))
        self.assertTrue(any("testonly_pause" in v
                            for v in scan_source(gate + "fn testonly_pause(stage: &str) {}\n")))

    def test_roots_cover_pr_write_set_and_exist(self):
        root = Path(__file__).resolve().parents[2]
        self.assertIn("rust/src/bin/s_structure_session.rs", checker.ROOTS)
        self.assertIn("rust/src/session_protocol.rs", checker.ROOTS)
        for base in checker.ROOTS:
            with self.subTest(root=base):
                self.assertTrue((root / base).exists())


if __name__ == "__main__":
    unittest.main()
