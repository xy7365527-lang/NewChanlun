"""#1406 TB-03-C：冻结夹具与独立账簿 oracle 的自检（不导入任何生产 reducer）。

手算期望写在 HAND-ORACLE.json；oracle 只读原件字节按 ADR 0024/0031/0032/0034/0036 自行核账。
两者逐步相等、两次运行字节相同，才允许生产测试以它们为对拍基准。
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parent
FIXTURES = ROOT / "fixtures" / "tb03c"
sys.path.insert(0, str(ROOT))

import tb03c_oracle  # noqa: E402


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class FrozenFixtureTests(unittest.TestCase):
    def test_manifest_pins_every_fixture_byte(self):
        manifest = json.loads((FIXTURES / "MANIFEST.json").read_text("utf-8"))
        self.assertEqual(manifest["schema_revision"], "tb03c-fixtures/1")
        self.assertEqual(manifest["book_revision"], "tb03c-book/1")
        listed = sorted(manifest["files"])
        on_disk = sorted(str(p.relative_to(FIXTURES)) for p in FIXTURES.rglob("*")
                         if p.is_file() and p.name != "MANIFEST.json" and "__pycache__" not in p.parts)
        self.assertEqual(listed, on_disk)
        for name, digest in manifest["files"].items():
            self.assertEqual(sha256(FIXTURES / name), digest, name)

    def test_generator_reproduces_frozen_raw_documents(self):
        out = subprocess.run([sys.executable, "-B", str(FIXTURES / "generate_fixtures.py"), "--check"],
                             capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)


class OracleAgreesWithHandCalculationTests(unittest.TestCase):
    def test_every_hand_step_matches_independent_oracle(self):
        hand = json.loads((FIXTURES / "HAND-ORACLE.json").read_text("utf-8"))
        result = tb03c_oracle.run(FIXTURES)
        self.assertEqual(result["mismatches"], [])
        self.assertEqual(result["steps"], sum(len(case["steps"]) for case in hand["book_cases"]))
        self.assertGreaterEqual(result["steps"], 30)

    def test_apportion_sequences_match_hand_tables(self):
        hand = json.loads((FIXTURES / "HAND-ORACLE.json").read_text("utf-8"))
        for case in hand["apportion_cases"]:
            got = [tb03c_oracle.apportion(f, case["weights"], case["tie_rank"])
                   for f in range(case["total"] + 1)]
            self.assertEqual(got, case["sequence"], case["name"])

    def test_frozen_apportion_table_equals_oracle(self):
        cmd = [sys.executable, "-B", str(ROOT / "tb03c_oracle.py"), "--apportion-table"]
        fresh = subprocess.run(cmd, capture_output=True, check=True).stdout
        self.assertEqual((FIXTURES / "APPORTION-ORACLE.json").read_bytes(), fresh)
        rows = json.loads(fresh)["rows"]
        self.assertEqual(len(rows), 2 * (6 ** 2 + 5 ** 3 + 3 ** 4))
        for row in rows:
            q, seq = row["weights"], row["sequence"]
            self.assertEqual(seq[-1], q)
            for f, (prev, cur) in enumerate(zip(seq, seq[1:]), start=1):
                self.assertEqual(sum(cur), f)
                self.assertTrue(all(c >= p for p, c in zip(prev, cur)))
                self.assertTrue(all(f * w // sum(q) <= c <= -((-f * w) // sum(q)) for w, c in zip(q, cur)))

    def test_largest_remainder_counterexample_is_reproduced(self):
        lr = tb03c_oracle.largest_remainder
        self.assertEqual(lr(3, [1, 3, 3], [0, 1, 2]), [1, 1, 1])
        self.assertEqual(lr(4, [1, 3, 3], [0, 1, 2]), [0, 2, 2])

    def test_two_fresh_processes_print_identical_bytes(self):
        cmd = [sys.executable, "-B", str(ROOT / "tb03c_oracle.py"), str(FIXTURES)]
        first = subprocess.run(cmd, capture_output=True, check=True).stdout
        second = subprocess.run(cmd, capture_output=True, check=True).stdout
        self.assertEqual(first, second)
        self.assertEqual(json.loads(first)["mismatches"], [])


if __name__ == "__main__":
    unittest.main()
