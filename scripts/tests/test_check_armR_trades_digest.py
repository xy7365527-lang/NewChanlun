import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts import check_armR_trades_digest as gate


def _write_minimal_dump(root):
    trade = json.dumps(
        {
            "certificate": {"dir": "Long", "nest_depth": 0},
            "pnl_raw_unlevered": 1.25,
        }
    )
    for tag in gate.WINDOWS:
        window = root / tag
        window.mkdir(parents=True)
        (window / "trades.jsonl").write_text(trade + "\n")


def _valid_payload(dump_dir):
    windows, missing = gate.collect(dump_dir)
    if missing:
        raise AssertionError(f"test dump incomplete: {missing}")
    return {
        "_schema": gate.GOLDEN_SCHEMA,
        "provenance": {
            "anchors": [
                {
                    "source_base_head": "c126d4cf69613068404b6ad0a53acf2e4dd9957b",
                    "final_verification_head": "c126d4cf69613068404b6ad0a53acf2e4dd9957b",
                    "source_worktree": "test-worktree",
                    "run_date": "2026-07-28",
                    "regen_command": "test regen",
                    "check_command": "test check",
                }
            ]
        },
        "windows": windows,
    }


class ArmRTradesDigestTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        root = Path(self.tmp.name)
        self.dump_dir = root / "dump"
        self.golden_path = root / "golden.json"
        _write_minimal_dump(self.dump_dir)

    def _check_payload(self, payload):
        self.golden_path.write_text(json.dumps(payload))
        argv = ["check_armR_trades_digest.py", "--dump-dir", str(self.dump_dir)]
        stderr = io.StringIO()
        with (
            mock.patch.object(gate, "GOLDEN_PATH", self.golden_path),
            mock.patch.object(sys, "argv", argv),
            contextlib.redirect_stderr(stderr),
        ):
            exit_code = gate.main()
        return exit_code, stderr.getvalue()

    def test_check_rejects_non_hex_commit_head(self):
        payload = _valid_payload(self.dump_dir)
        payload["provenance"]["anchors"][0]["source_base_head"] = "forged-commit"

        exit_code, stderr = self._check_payload(payload)

        self.assertNotEqual(exit_code, gate.EXIT_OK)
        self.assertIn("source_base_head", stderr)

    def test_check_rejects_non_string_commit_head(self):
        for field, invalid in (
            ("final_verification_head", 123),
            ("source_lineage_start", None),
        ):
            with self.subTest(field=field, invalid=invalid):
                payload = _valid_payload(self.dump_dir)
                payload["provenance"]["anchors"][0][field] = invalid

                exit_code, stderr = self._check_payload(payload)

                self.assertNotEqual(exit_code, gate.EXIT_OK)
                self.assertIn(field, stderr)

    def test_check_rejects_hex_head_that_is_not_a_commit(self):
        payload = _valid_payload(self.dump_dir)
        payload["provenance"]["anchors"][0]["source_base_head"] = "0" * 40

        exit_code, stderr = self._check_payload(payload)

        self.assertNotEqual(exit_code, gate.EXIT_OK)
        self.assertIn("source_base_head", stderr)
        self.assertIn("仓内不可解析 commit", stderr)

    def test_check_rejects_invalid_digest_schema(self):
        payload = _valid_payload(self.dump_dir)
        payload["windows"]["wf7"]["digest_fnv1a64"] = "0xNOT-A-DIGEST!"

        exit_code, stderr = self._check_payload(payload)

        self.assertNotEqual(exit_code, gate.EXIT_OK)
        self.assertIn("16 位小写 hex", stderr)

    def test_check_rejects_integer_command(self):
        payload = _valid_payload(self.dump_dir)
        payload["provenance"]["anchors"][0]["regen_command"] = 123

        exit_code, stderr = self._check_payload(payload)

        self.assertNotEqual(exit_code, gate.EXIT_OK)
        self.assertIn("regen_command", stderr)
        self.assertIn("字符串", stderr)

    def test_check_rejects_reversed_lineage_ancestry(self):
        payload = _valid_payload(self.dump_dir)
        anchor = payload["provenance"]["anchors"][0]
        anchor["source_lineage_start"] = "6e15ceffeeb8259c065bf7c0ec9ec7c65935737c"
        anchor["final_verification_head"] = "a12a1022d9ddd8d1cae867a107a3a33c359358cf"

        exit_code, stderr = self._check_payload(payload)

        self.assertNotEqual(exit_code, gate.EXIT_OK)
        self.assertIn("source_lineage_start", stderr)
        self.assertIn("不是 source_base_head 的祖先", stderr)

    def test_check_rejects_reversed_base_ancestry_when_lineage_is_valid(self) -> None:
        valid_payload = _valid_payload(self.dump_dir)
        valid_anchors = valid_payload["provenance"]["anchors"]
        anchor = {
            **valid_anchors[0],
            "source_lineage_start": "a12a1022d9ddd8d1cae867a107a3a33c359358cf",
            "source_base_head": "6e15ceffeeb8259c065bf7c0ec9ec7c65935737c",
            "final_verification_head": "a12a1022d9ddd8d1cae867a107a3a33c359358cf",
        }
        payload = {
            **valid_payload,
            "provenance": {
                **valid_payload["provenance"],
                "anchors": [anchor, *valid_anchors[1:]],
            },
        }

        exit_code, stderr = self._check_payload(payload)

        self.assertNotEqual(exit_code, gate.EXIT_OK)
        self.assertIn("source_base_head", stderr)
        self.assertIn("不是 final_verification_head 的祖先", stderr)

    def test_check_rejects_non_positive_or_boolean_counts(self):
        for field, invalid in (("n_trades", 0), ("bytes", True)):
            with self.subTest(field=field, invalid=invalid):
                payload = _valid_payload(self.dump_dir)
                payload["windows"]["p3fold"][field] = invalid

                exit_code, stderr = self._check_payload(payload)

                self.assertNotEqual(exit_code, gate.EXIT_OK)
                self.assertIn(f"windows.p3fold.{field}", stderr)
                self.assertIn("正整数", stderr)

    def test_check_rejects_invalid_run_date(self):
        payload = _valid_payload(self.dump_dir)
        payload["provenance"]["anchors"][0]["run_date"] = "2026-02-30"

        exit_code, stderr = self._check_payload(payload)

        self.assertNotEqual(exit_code, gate.EXIT_OK)
        self.assertIn("run_date", stderr)
        self.assertIn("ISO 日期", stderr)

    def test_regen_appends_provenance_anchor_without_losing_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dump_dir = root / "dump"
            golden_path = root / "golden.json"
            historical_anchor = {
                "source_base_head": "historical-base",
                "final_verification_head": "historical-final",
                "source_worktree": "historical-worktree",
                "run_date": "2026-07-27",
                "regen_command": "historical regen",
                "check_command": "historical check",
            }
            golden_path.write_text(
                json.dumps(
                    {
                        "_schema": "armR-trades-digest/v2",
                        "provenance": {"anchors": [historical_anchor]},
                        "windows": {},
                    }
                )
            )
            _write_minimal_dump(dump_dir)
            argv = ["check_armR_trades_digest.py", "--dump-dir", str(dump_dir), "--regen"]

            with mock.patch.object(gate, "GOLDEN_PATH", golden_path), mock.patch.object(sys, "argv", argv):
                self.assertEqual(gate.main(), gate.EXIT_OK)

            regenerated = json.loads(golden_path.read_text())
            self.assertIn("provenance", regenerated)
            anchors = regenerated["provenance"]["anchors"]
            self.assertEqual(anchors[0], historical_anchor)
            self.assertEqual(len(anchors), 2)
            self.assertEqual(regenerated["_schema"], "armR-trades-digest/v2")

    def test_check_rejects_golden_without_required_provenance(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dump_dir = root / "dump"
            golden_path = root / "golden.json"
            _write_minimal_dump(dump_dir)
            windows, missing = gate.collect(dump_dir)
            self.assertEqual(missing, [])
            golden_path.write_text(json.dumps({"windows": windows}))
            argv = ["check_armR_trades_digest.py", "--dump-dir", str(dump_dir)]

            with mock.patch.object(gate, "GOLDEN_PATH", golden_path), mock.patch.object(sys, "argv", argv):
                self.assertEqual(gate.main(), gate.EXIT_DRIFT)

    def test_regen_migrates_legacy_anchor_without_final_verification_head(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dump_dir = root / "dump"
            golden_path = root / "golden.json"
            legacy = {
                "_source_base_head": "legacy-head",
                "_source_worktree": "legacy-worktree",
                "_run_date": "2026-07-27",
                "_regen_command": "legacy regen",
                "_check_command": "legacy check",
                "windows": {},
            }
            golden_path.write_text(json.dumps(legacy))
            _write_minimal_dump(dump_dir)
            argv = ["check_armR_trades_digest.py", "--dump-dir", str(dump_dir), "--regen"]

            with mock.patch.object(gate, "GOLDEN_PATH", golden_path), mock.patch.object(sys, "argv", argv):
                self.assertEqual(gate.main(), gate.EXIT_OK)

            anchors = json.loads(golden_path.read_text())["provenance"]["anchors"]
            self.assertEqual(anchors[0]["source_base_head"], "legacy-head")
            self.assertEqual(anchors[0]["final_verification_head"], "legacy-head")
            self.assertEqual(len(anchors), 2)


if __name__ == "__main__":
    unittest.main()
