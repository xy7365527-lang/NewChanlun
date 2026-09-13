"""#1372：会误报双跑确定的反例必须由比较器拒绝。"""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from tb01c_compare import compare_traces, first_difference


class TraceComparisonTests(unittest.TestCase):
    def compare(self, a, b, expected=1):
        with tempfile.TemporaryDirectory(prefix="tb01c-compare-") as directory:
            left, right = Path(directory) / "left.jsonl", Path(directory) / "right.jsonl"
            left.write_text(a, encoding="utf-8")
            right.write_text(b, encoding="utf-8")
            return compare_traces(left, right, expected)

    def test_exact_values_keep_large_integer_and_object_key_order(self):
        self.assertEqual(self.compare('{"v":"9223372036854775807","a":true}\n',
                                      '{"a":true,"v":"9223372036854775807"}\n')["status"], "PASS")

    def test_first_known_difference_is_not_excluded(self):
        diff = first_difference({"objects": [{"first_known": "4"}]},
                                {"objects": [{"first_known": "5"}]})
        self.assertEqual(diff[:2], ("/objects/0/first_known", "value"))

    def test_relation_order_and_duplicate_members_are_preserved(self):
        a = {"relations": [{"subject": "A"}, {"subject": "B"}]}
        b = {"relations": list(reversed(a["relations"]))}
        self.assertIsNotNone(first_difference(a, b))
        self.assertIsNotNone(first_difference(a, {"relations": a["relations"] + a["relations"][:1]}))

    def test_bool_integer_and_text_are_not_equal(self):
        for a, b in ((True, 1), (1, "1"), (None, "null")):
            with self.subTest(a=a, b=b):
                self.assertEqual(first_difference(a, b)[1], "type")

    def test_missing_field_and_empty_are_distinct(self):
        self.assertEqual(first_difference({"raw_history": []}, {})[1], "missing_right")

    def test_duplicate_keys_and_floats_are_not_verified_even_when_both_equal(self):
        for invalid in ('{"cut":"1","cut":"2"}\n', '{"v":1.0}\n', '{"v":NaN}\n',
                        '{"v":1e999}\n', '\n', '[]\n', '{}\n'):
            with self.subTest(invalid=invalid):
                self.assertEqual(self.compare(invalid, invalid)["status"], "NOT_VERIFIED")

    def test_only_final_state_cannot_satisfy_full_trace(self):
        record = '{"final":{"generation":"160"}}\n'
        result = self.compare(record, record, expected=160)
        self.assertEqual(result["status"], "NOT_VERIFIED")

    def test_missing_record_and_extra_common_suffix(self):
        record = '{"cut":"1"}\n'
        self.assertEqual(self.compare(record, record + record, 2)["status"], "FAIL")
        self.assertEqual(self.compare(record + record, record + record)["status"], "NOT_VERIFIED")

    def test_earlier_difference_wins_even_when_final_states_equal(self):
        a = json.dumps({"accepted": "1"}) + '\n{"final":"same"}\n'
        b = json.dumps({"accepted": "2"}) + '\n{"final":"same"}\n'
        result = self.compare(a, b, 2)
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(result["first_difference"]["record"], 0)
        self.assertEqual(result["first_difference"]["path"], "/accepted")

    def test_json_pointer_is_unambiguous(self):
        self.assertEqual(first_difference({"a/b~c": "1"}, {"a/b~c": "2"})[0], "/a~1b~0c")

    def test_surrogates_always_emit_valid_result_and_correct_exit_code(self):
        cases = (
            ('{"identity":"\\ud800"}', '{"identity":"valid"}', 1, "FAIL"),
            ('{"\\ud800":"1"}', '{"valid":"1"}', 1, "FAIL"),
            ('{"\\ud800":"1","\\ud800":"2"}', '{"valid":"1"}', 2, "NOT_VERIFIED"),
        )
        with tempfile.TemporaryDirectory(prefix="tb01c-unicode-") as directory:
            left, right = Path(directory) / "left.jsonl", Path(directory) / "right.jsonl"
            script = Path(__file__).with_name("tb01c_compare.py")
            for a, b, code, status in cases:
                with self.subTest(a=a):
                    left.write_text(a + "\n", encoding="utf-8")
                    right.write_text(b + "\n", encoding="utf-8")
                    run = subprocess.run([sys.executable, str(script), str(left), str(right),
                                          "--expected-records", "1"], capture_output=True, check=False)
                    self.assertEqual(run.returncode, code, run.stderr)
                    self.assertEqual(json.loads(run.stdout.decode("utf-8"))["status"], status)


if __name__ == "__main__":
    unittest.main()
