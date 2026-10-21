#!/usr/bin/env python3
"""#1588 ci-time：test-shard.py 的自测（桩 cargo，不编译 Rust）。

运行：python3 .github/scripts/test_test_shard.py
"""

from __future__ import annotations

import importlib.util
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "test-shard.py"
spec = importlib.util.spec_from_file_location("test_shard", SCRIPT)
shard = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(shard)

NAMES = [f"mod{i % 3}::tests::case_{i:02d}" for i in range(20)] + ["tests::slow_a", "tests::slow_b"]

# 桩 cargo：--list 打全集（STUB_SKIP 指定的名按子串剔除）；--exact 逐个回显结果，
# STUB_DROP 指定的名不回显（模拟漏跑），STUB_FAIL 指定的名回显 FAILED 且退出码 101。
STUB_CARGO = textwrap.dedent(
    """\
    #!/usr/bin/env python3
    import os, sys
    names = os.environ["STUB_NAMES"].split(",")
    argv = sys.argv[1:]
    rest = argv[argv.index("--") + 1:]
    if "--list" in rest:
        skips = [rest[i + 1] for i, a in enumerate(rest) if a == "--skip"]
        for n in names:
            if not any(s in n for s in skips):
                print(f"{n}: test")
        sys.exit(0)
    assert rest[0] == "--exact", rest
    status = 0
    print(f"running {len(rest) - 1} tests")
    for n in rest[1:]:
        if n == os.environ.get("STUB_DROP"):
            continue
        if n == os.environ.get("STUB_FAIL"):
            print(f"test {n} ... FAILED")
            status = 101
        elif n.endswith("_00"):
            print(f"test {n} ... ignored, needs data")
        else:
            print(f"test {n} ... ok")
    sys.exit(status)
    """
)


class PartitionTests(unittest.TestCase):
    def test_union_is_full_set_and_disjoint_for_any_count(self) -> None:
        weights = {"tests::slow_a": 4000.0, "tests::slow_b": 3000.0}
        for count in range(1, 8):
            shards = shard.partition(NAMES, weights, count)
            flat = [n for s in shards for n in s]
            self.assertEqual(sorted(flat), sorted(NAMES))
            self.assertEqual(len(flat), len(set(flat)))
            shard.check_partition(NAMES, shards)

    def test_plan_is_independent_of_listing_order(self) -> None:
        weights = {"tests::slow_a": 4000.0}
        self.assertEqual(
            shard.partition(NAMES, weights, 3),
            shard.partition(list(reversed(NAMES)), weights, 3),
        )

    def test_heavy_tests_land_in_different_shards(self) -> None:
        weights = {n: 1.0 for n in NAMES} | {"tests::slow_a": 4000.0, "tests::slow_b": 3000.0}
        shards = shard.partition(NAMES, weights, 3)
        homes = [i for i, s in enumerate(shards) for n in s if n.startswith("tests::slow_")]
        self.assertEqual(len(set(homes)), 2)

    def test_empty_weight_table_still_covers_everything(self) -> None:
        shard.check_partition(NAMES, shard.partition(NAMES, {}, 4))

    def test_check_partition_rejects_missing_and_overlap(self) -> None:
        with self.assertRaises(SystemExit):
            shard.check_partition(NAMES, [NAMES[:-1]])
        with self.assertRaises(SystemExit):
            shard.check_partition(NAMES, [NAMES, NAMES[:1]])


class RunAndVerifyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        bin_dir = root / "bin"
        bin_dir.mkdir()
        cargo = bin_dir / "cargo"
        cargo.write_text(STUB_CARGO, encoding="utf-8")
        cargo.chmod(0o755)
        self.weights = root / "weights.tsv"
        self.weights.write_text("# seconds\tname\n4000\ttests::slow_a\n3000\ttests::slow_b\n", encoding="utf-8")
        self.root = root
        self.env = os.environ | {
            "PATH": f"{bin_dir}{os.pathsep}{os.environ['PATH']}",
            "STUB_NAMES": ",".join(NAMES + ["v3::financial_session::tests::skipped"]),
        }

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def run_shard(self, index: int, count: int, **env: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "run", "--index", str(index), "--count", str(count),
             "--weights", str(self.weights), "--out", str(self.root / f"shard-{index}"),
             "--skip", "v3::financial_session", "--", "--features", "x", "--bin", "y"],
            env=self.env | env, capture_output=True, text=True,
        )

    def verify(self, count: int, dirs: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "verify", "--count", str(count), *dirs],
            capture_output=True, text=True,
        )

    def test_all_shards_together_execute_the_full_set_once(self) -> None:
        for i in range(3):
            done = self.run_shard(i, 3)
            self.assertEqual(done.returncode, 0, done.stderr)
        done = self.verify(3, [str(self.root / f"shard-{i}") for i in range(3)])
        self.assertEqual(done.returncode, 0, done.stderr)
        ignored = sum(n.endswith("_00") for n in NAMES)
        self.assertIn(
            f"shard_union total={len(NAMES)} ok={len(NAMES) - ignored} ignored={ignored} failed=0",
            done.stdout,
        )
        all_list = (self.root / "shard-0" / "all.txt").read_text().splitlines()
        self.assertNotIn("v3::financial_session::tests::skipped", all_list)

    def test_dropped_test_turns_the_shard_red(self) -> None:
        plan = shard.partition(NAMES, shard.load_weights(self.weights), 3)
        done = self.run_shard(1, 3, STUB_DROP=plan[1][0])
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("Shard executed != assigned", done.stderr)

    def test_failing_test_keeps_cargo_status(self) -> None:
        plan = shard.partition(NAMES, shard.load_weights(self.weights), 2)
        done = self.run_shard(0, 2, STUB_FAIL=plan[0][-1])
        self.assertEqual(done.returncode, 101)

    def test_verify_rejects_missing_shard(self) -> None:
        for i in range(3):
            self.assertEqual(self.run_shard(i, 3).returncode, 0)
        done = self.verify(3, [str(self.root / f"shard-{i}") for i in range(2)])
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("Shard artifacts missing", done.stderr)

    def test_verify_rejects_shard_without_execution_record(self) -> None:
        for i in range(3):
            self.assertEqual(self.run_shard(i, 3).returncode, 0)
        (self.root / "shard-1" / "executed.tsv").unlink()
        done = self.verify(3, [str(self.root / f"shard-{i}") for i in range(3)])
        self.assertEqual(done.returncode, 1)
        self.assertIn("::error title=Shard executed.tsv missing::", done.stderr)
        self.assertNotIn("Traceback", done.stderr)

    def test_invalid_index_is_rejected(self) -> None:
        done = self.run_shard(3, 3)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("Invalid shard index", done.stderr)


if __name__ == "__main__":
    unittest.main()
