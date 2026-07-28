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


class ArmRTradesDigestTest(unittest.TestCase):
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
