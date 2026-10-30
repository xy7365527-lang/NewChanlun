import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

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

    def env_violations(self, lib):
        gen = checker.load_gen()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text(lib, encoding="utf-8")
            with patch.object(gen, "CRATE_ROOTS", ("src/lib.rs",)), \
                 patch.object(gen._checker, "ROOTS", ("src",)):
                return checker.env_violations(root)

    def test_env_axis_reports_ungated_sidecar_under_any_function_name(self):
        for name in ("record_connection_pragmas", "observe", "x"):
            lib = (f"fn {name}() {{\n"
                   '    if std::env::var("S_SESSION_PRAGMA_SIDECAR").ok().as_deref() != Some("1") { return; }\n'
                   "}\n")
            with self.subTest(name=name):
                self.assertTrue(any("S_SESSION_PRAGMA_SIDECAR" in v and "s_incremental" in v
                                    for v in self.env_violations(lib)))
        self.assertEqual(scan_source('fn record_connection_pragmas() {}\n',
                                     "rust/src/bin/s_structure_session.rs"), [])

    def test_env_axis_accepts_incremental_gate_and_production_env(self):
        gate = '#[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]\n'
        lib = (gate + 'fn sidecar() { let _ = std::env::var("S_SESSION_PRAGMA_SIDECAR_PATH"); }\n'
               'fn live() { let _ = std::env::var_os("S_CONTROL_INSTANCE_ID"); }\n'
               "#[cfg(test)]\nmod tests;\n")
        self.assertEqual(self.env_violations(lib), [])

    def test_env_axis_resolves_const_and_rejects_unresolved_arguments(self):
        lib = ('const CACHE: &str = "S_UNLISTED_CACHE_SWITCH";\n'
               "fn a() { let _ = std::env::var_os(CACHE); }\n"
               "fn b(key: &str) { let _ = std::env::var(key); }\n")
        found = self.env_violations(lib)
        self.assertTrue(any("S_UNLISTED_CACHE_SWITCH" in v for v in found))
        self.assertTrue(any("非字面量参数 key" in v for v in found))

    def test_env_axis_keeps_fn_gate_across_generic_return_comma(self):
        gate = '#[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]\n'
        lib = gate + 'fn r() -> Result<Option<String>, String> { Ok(std::env::var("S_SESSION_PAUSE").ok()) }\n'
        self.assertEqual(self.env_violations(lib), [])

    def test_env_axis_reports_ungated_native_sqlite_cache_read(self):
        # #1640 D-A（ADR 0042 D1）负控：缓存实验输入不在 PRODUCTION_ENV，未门控读取即判生产可达。
        self.assertNotIn("S_NATIVE_SQLITE_CACHE_KIB", checker.PRODUCTION_ENV)
        for lib in ('fn configure() { let _ = std::env::var_os("S_NATIVE_SQLITE_CACHE_KIB"); }\n',
                    'const CACHE_ENV: &str = "S_NATIVE_SQLITE_CACHE_KIB";\n'
                    "fn configure() { let _ = std::env::var_os(CACHE_ENV); }\n",
                    '#[cfg(feature = "s_incremental")]\n'
                    'fn configure() { let _ = std::env::var_os("S_NATIVE_SQLITE_CACHE_KIB"); }\n'):
            with self.subTest(lib=lib):
                self.assertTrue(any("S_NATIVE_SQLITE_CACHE_KIB" in v and "s_incremental" in v
                                    for v in self.env_violations(lib)))

    def test_env_axis_accepts_testonly_gated_native_sqlite_cache_read(self):
        # #1640 D-A 正控：同一读取只编入 s_capture_native_testonly 时生产不可达，不报。
        gate = '#[cfg(feature = "s_capture_native_testonly")]\n'
        for lib in (gate + 'fn configure() { let _ = std::env::var_os("S_NATIVE_SQLITE_CACHE_KIB"); }\n',
                    gate + 'const CACHE_ENV: &str = "S_NATIVE_SQLITE_CACHE_KIB";\n'
                    + gate + "fn apply(c: &C) -> Result<Option<i32>, String> {\n"
                    "    parse(std::env::var_os(CACHE_ENV).as_deref())\n}\n"
                    '#[cfg(not(feature = "s_capture_native_testonly"))]\n'
                    "fn apply(_c: &C) -> Result<Option<i32>, String> { Ok(None) }\n"):
            with self.subTest(lib=lib):
                self.assertEqual(self.env_violations(lib), [])

    def test_env_axis_default_only_requires_listing(self):
        gate = '#[cfg(not(feature = "s_incremental"))]\n'
        lib = gate + 'fn other() { let _ = std::env::var("S_UNLISTED_DIAG_SWITCH"); }\n'
        self.assertTrue(any("S_UNLISTED_DIAG_SWITCH" in v and "默认" in v for v in self.env_violations(lib)))

    def test_env_axis_ignores_test_module_files(self):
        gen = checker.load_gen()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text("#[cfg(test)]\nmod probe;\n", encoding="utf-8")
            (root / "src/probe.rs").write_text('fn f() { let _ = std::env::var("S1394_PROBE_DB"); }\n',
                                               encoding="utf-8")
            with patch.object(gen, "CRATE_ROOTS", ("src/lib.rs",)), \
                 patch.object(gen._checker, "ROOTS", ("src",)):
                self.assertEqual(checker.env_violations(root), [])

    def test_roots_cover_pr_write_set_and_exist(self):
        root = Path(__file__).resolve().parents[2]
        self.assertIn("rust/src/bin/s_structure_session.rs", checker.ROOTS)
        self.assertIn("rust/src/session_protocol.rs", checker.ROOTS)
        for base in checker.ROOTS:
            with self.subTest(root=base):
                self.assertTrue((root / base).exists())


if __name__ == "__main__":
    unittest.main()
