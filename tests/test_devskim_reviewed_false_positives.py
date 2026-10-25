"""#1488：精确误报豁免不得吞掉同三元键的新结果。"""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

original_spec = importlib.util.spec_from_file_location(
    "original_devskim_gate_tests", Path(__file__).with_name("test_devskim_sarif_gate.py")
)
original = importlib.util.module_from_spec(original_spec)
original_spec.loader.exec_module(original)

HELPER = Path(__file__).resolve().parents[1] / "scripts/devskim_reviewed_false_positives.py"
spec = importlib.util.spec_from_file_location("reviewed_fp", HELPER)
fp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fp)


class TestReviewedFalsePositives(unittest.TestCase):
    run_gate = original.TestDevskimSarifGate.run_gate
    write_scan = original.TestDevskimSarifGate.write_scan

    def setUp(self):
        original.TestDevskimSarifGate.setUp(self)
        self.source = self.root / "evidence.json"
        self.source.write_text('{"kind":"fixture"}\n')
        self.record = original.finding("evidence.json")
        self.record["locations"][0]["physicalLocation"]["region"].update({
            "startColumn": 2, "endLine": 1, "endColumn": 8,
            "snippet": {"text": "fixture"},
        })
        self.allow = self.root / "allow.json"
        self.manifest = {"schema": "devskim-reviewed-false-positives/v1", "issue": 1488,
                         "files": [{"path": "evidence.json",
                                    "sha256_bytes": list(hashlib.sha256(self.source.read_bytes()).digest()),
                                    "results": [{"rule": "DS000001", "identity_bytes": list(fp.identity(self.record))}]}]}
        self.write_allow()
        self.baseline.write_text(json.dumps({"schema":"devskim-baseline/v1", "findings":[]}))
        self.write_scan([self.record])

    def write_allow(self):
        self.allow.write_text(json.dumps(self.manifest))

    def reviewed_gate(self, *extra):
        return self.run_gate("--baseline", str(self.baseline), "--false-positives",
                             str(self.allow), "--source-root", str(self.root), *extra)

    def test_exact_result_passes_and_preserves_raw_scan(self):
        original = self.sarif.read_bytes()
        result = self.reviewed_gate()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(b"Reviewed false-positive results:** `1`", result.stdout)
        self.assertEqual(self.sarif.read_bytes(), original)

    def test_same_old_key_new_position_or_snippet_stays_red(self):
        for field, value in [("startColumn", 3), ("endColumn", 9), ("snippet", {"text":"new finding"})]:
            with self.subTest(field=field):
                changed = copy.deepcopy(self.record)
                changed["locations"][0]["physicalLocation"]["region"][field] = value
                self.write_scan([self.record, changed])
                result = self.reviewed_gate()
                self.assertEqual(result.returncode, 1)
                self.assertIn(b"DS000001|evidence.json|1", result.stdout)

    def test_new_rule_stays_red(self):
        changed = copy.deepcopy(self.record)
        changed["ruleId"] = "DS000002"
        self.write_scan([self.record, changed])
        self.assertEqual(self.reviewed_gate().returncode, 1)

    def test_file_drift_and_missing_file_fail(self):
        self.source.write_text("changed")
        self.assertEqual(self.reviewed_gate().returncode, 1)
        self.source.unlink()
        self.assertEqual(self.reviewed_gate().returncode, 1)

    def test_escape_paths_fail(self):
        for name in ["../evidence.json", str(self.source), "./evidence.json"]:
            with self.subTest(name=name):
                self.manifest["files"][0]["path"] = name
                self.write_allow()
                self.assertEqual(self.reviewed_gate().returncode, 1)

    def test_symlink_escape_fails(self):
        self.source.unlink()
        self.source.symlink_to(HELPER)
        self.assertEqual(self.reviewed_gate().returncode, 1)

    def test_bad_schema_and_identity_fail(self):
        self.manifest["schema"] = "other"
        self.write_allow()
        self.assertEqual(self.reviewed_gate().returncode, 1)
        self.manifest["schema"] = "devskim-reviewed-false-positives/v1"
        self.manifest["files"][0]["sha256_bytes"] = [0]
        self.write_allow()
        self.assertEqual(self.reviewed_gate().returncode, 1)
        self.allow.write_text("{")
        self.assertEqual(self.reviewed_gate().returncode, 1)

    def test_scan_upload_and_tool_failures_are_not_exempt(self):
        for option, value in [("--scanner-outcome", "failure"),
                              ("--artifact-outcome", "failure"),
                              ("--expected-tool-version", "other")]:
            with self.subTest(option=option):
                self.assertEqual(self.reviewed_gate(option, value).returncode, 1)

    def test_unconfigured_keeps_original_red(self):
        result = self.run_gate("--baseline", str(self.baseline))
        self.assertEqual(result.returncode, 1)
        self.assertNotIn(b"Reviewed false-positive results", result.stdout)

    def test_source_root_without_manifest_fails(self):
        self.write_scan([])
        self.assertEqual(self.run_gate("--source-root", str(self.root)).returncode, 1)

    def test_manifest_without_source_root_fails(self):
        self.assertEqual(self.run_gate("--false-positives", str(self.allow)).returncode, 1)


if __name__ == "__main__":
    unittest.main()
