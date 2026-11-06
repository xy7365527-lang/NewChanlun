"""P57 变异回执：只有指定目标的实际断言失败才算负控成立。"""
import importlib.util
from pathlib import Path
import unittest

PATH = Path(__file__).with_name("run-p57-mutations.py")
SPEC = importlib.util.spec_from_file_location("p57_mutations", PATH)
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)


class MutationReceipts(unittest.TestCase):
    def test_target_assertion_failure_is_red(self):
        output = f"test {RUNNER.TRUTH} ... FAILED\npanicked at file.rs:10\ntest result: FAILED. 0 passed; 1 failed;"
        self.assertTrue(RUNNER.actual_assertion_red(101, output, RUNNER.TRUTH))
        self.assertFalse(RUNNER.actual_assertion_red(0, output, RUNNER.TRUTH))
        self.assertFalse(RUNNER.actual_assertion_red(101, output, RUNNER.SAME_TEXT))

    def test_compile_failure_or_zero_tests_is_not_red(self):
        self.assertFalse(RUNNER.actual_assertion_red(101, "error: could not compile", RUNNER.TRUTH))
        self.assertFalse(RUNNER.actual_assertion_red(0, "test result: ok. 0 passed;", RUNNER.TRUTH))

    def test_all_mutation_anchors_match_current_candidate(self):
        root = PATH.resolve().parents[2]
        sources = {p: (root / p).read_text() for p in (RUNNER.AUDIT, RUNNER.SCRATCH, RUNNER.CODEC)}
        for name in RUNNER.MUTANTS:
            with self.subTest(name=name):
                self.assertNotEqual(RUNNER.mutate(name, sources), sources)

    def test_ambiguous_or_missing_anchor_is_rejected(self):
        for source in ("absent", "old old"):
            with self.assertRaises(ValueError):
                RUNNER.replace_one(source, "old", "new")


if __name__ == "__main__":
    unittest.main()
