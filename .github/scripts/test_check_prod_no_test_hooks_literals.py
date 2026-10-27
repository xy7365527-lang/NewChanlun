import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).with_name("gen-prod-banned-literals.py")
SPEC = importlib.util.spec_from_file_location("gen_prod_banned_literals", SCRIPT)
gen = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gen)

LIB = '''
#[cfg(test)]
mod tests;
#[cfg(feature = "s_capture_native_testonly")]
pub fn arm() -> &'static str { "TestOnly 臂：只在对照构建出现的诊断" }
pub fn live() -> &'static str { "生产诊断：完整候选已持有并发布" }
#[cfg(any(not(feature = "s_incremental"), feature = "s_capture_native_testonly"))]
fn pause() { let _ = std::env::var("S_SESSION_PAUSE_DB"); }
#[cfg(feature = "backtest_bin")]
fn other_feature() -> &'static str { "其他 feature 的非测试字面量不收录" }
'''
TESTS = '''
#[test]
fn locks() {
    assert_eq!(super::live(), "生产诊断：完整候选已持有并发布");
    let _ = "完整候选已持有";
    let _ = "测试断言：须持有完整候选且不得降级";
    let _ = "S1394_PUBLICATION_PROBE_DB";
    let _ = "allocation_probe_total";
    let _ = r#"raw 测试串：injected write failure"#;
    let _ = format!("带占位 {} 的测试格式串不收", 1);
}
'''


class BannedLiteralGeneratorTests(unittest.TestCase):
    def generate(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "src").mkdir()
            (root / "src/lib.rs").write_text(LIB, encoding="utf-8")
            (root / "src/tests.rs").write_text(TESTS, encoding="utf-8")
            with patch.object(gen, "CRATE_ROOTS", ("src/lib.rs",)), \
                 patch.object(gen._checker, "ROOTS", ("src",)):
                return gen.banned_literals(root)

    def test_only_distinctive_test_gated_literals_are_banned(self):
        banned = self.generate()
        self.assertIn("测试断言：须持有完整候选且不得降级", banned)
        self.assertIn("TestOnly 臂：只在对照构建出现的诊断", banned)
        self.assertIn("S1394_PUBLICATION_PROBE_DB", banned)
        self.assertIn("raw 测试串：injected write failure", banned)
        for absent in ("生产诊断：完整候选已持有并发布", "完整候选已持有", "S_SESSION_PAUSE_DB",
                       "allocation_probe_total", "其他 feature 的非测试字面量不收录",
                       "带占位 {} 的测试格式串不收"):
            with self.subTest(absent=absent):
                self.assertNotIn(absent, banned)

    def test_checked_in_list_has_no_empty_or_header_like_patterns(self):
        listing = Path(__file__).resolve().parents[1] / "prod-banned-literals.txt"
        lines = listing.read_text(encoding="utf-8").splitlines()
        body = [line for line in lines if not line.startswith(gen.HEADER_PREFIX)]
        self.assertTrue(body)
        self.assertNotIn("", body)
        self.assertIn("须持有完整候选", body)


if __name__ == "__main__":
    unittest.main()
