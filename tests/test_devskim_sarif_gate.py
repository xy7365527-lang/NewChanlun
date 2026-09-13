"""#1372：DevSkim 门禁须完整报告大差集，并保留已有退出及追加语义。"""

import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "scripts/devskim_sarif_gate.sh"
BASH = shutil.which("bash")


def finding(uri: str, line: int = 1) -> dict:
    return {
        "ruleId": "DS000001",
        "locations": [{"physicalLocation": {
            "artifactLocation": {"uri": uri},
            "region": {"startLine": line},
        }}],
    }


@unittest.skipUnless(BASH and shutil.which("jq"), "门禁测试需要 Bash 与 jq")
class TestDevskimSarifGate(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.sarif = self.root / "results.json"
        self.baseline = self.root / "baseline.json"
        self.summary = self.root / "summary.md"

    def write_scan(self, results: list[dict]) -> None:
        self.sarif.write_text(json.dumps({
            "version": "2.1.0",
            "runs": [{
                "tool": {"driver": {"name": "devskim", "version": "1.0.90"}},
                "results": results,
            }],
        }, ensure_ascii=False), encoding="utf-8")

    def run_gate(self, *extra: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [BASH, str(SCRIPT), "--sarif", str(self.sarif),
             "--scanner-outcome", "success", "--artifact-name", "test-sarif",
             "--artifact-outcome", "success", "--retention-days", "14", *extra],
            capture_output=True, timeout=30, check=False,
        )

    def test_no_findings_succeeds_without_summary_file(self) -> None:
        self.write_scan([])
        result = self.run_gate()
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stderr, b"")
        self.assertIn(b"Gate conclusion:** PASS", result.stdout)
        self.assertFalse(self.summary.exists())

    def test_known_baseline_and_removed_key_are_not_new_findings(self) -> None:
        self.write_scan([finding("file://known.json", 7)])
        self.baseline.write_text(json.dumps({
            "schema": "devskim-baseline/v1",
            "findings": ["DS000001|known.json|7", "DS000001|removed.json|1"],
        }), encoding="utf-8")
        result = self.run_gate("--baseline", str(self.baseline))
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stderr, b"")
        self.assertIn(b"Fixed since baseline: 1 finding(s).", result.stdout)
        self.assertNotIn(b"### Failure details", result.stdout)

    def test_large_difference_is_complete_ordered_and_appended(self) -> None:
        # 数量和长路径复现本次报告入仓后触发的累计拼接耗时；不放宽门禁。
        prefix = "evidence/汇总/" + "document-" * 12 + "/$literal/`quoted`/"
        uris = [f"{prefix}{index:05d}.json" for index in range(12_000)]
        self.write_scan([finding(uri) for uri in reversed(uris)])
        self.baseline.write_text(json.dumps({
            "schema": "devskim-baseline/v1", "findings": [],
        }), encoding="utf-8")
        prior = "已有摘要\n".encode("utf-8")
        self.summary.write_bytes(prior)
        result = self.run_gate("--baseline", str(self.baseline),
                               "--summary", str(self.summary))
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr, b"")
        self.assertEqual(self.summary.read_bytes(), prior + result.stdout)
        rows = [row for row in result.stdout.decode("utf-8").splitlines()
                if row.startswith("-   - `")]
        self.assertEqual(rows, [f"-   - `DS000001|{uri}|1`" for uri in uris])
        self.assertIn(b"12000 finding(s) not in the baseline (total 12000)",
                      result.stdout)
        self.assertTrue(result.stdout.endswith(b"\n"))

    def test_multiple_errors_and_summary_write_failure_remain_visible(self) -> None:
        self.write_scan([finding("unexpected.json")])
        result = self.run_gate(
            "--scanner-outcome", "failure", "--artifact-outcome", "failure",
            "--expected-tool-name", "other", "--expected-tool-version", "other",
            "--summary", str(self.root),
        )
        self.assertEqual(result.returncode, 1)
        messages = [
            b"DevSkim scanner outcome was `failure`",
            b"Ordinary artifact upload outcome was `failure`",
            b"SARIF DevSkim tool name was `devskim`",
            b"SARIF DevSkim tool version was `1.0.90`",
            b"SARIF contains 1 finding(s); this gate requires zero.",
        ]
        positions = [result.stdout.index(message) for message in messages]
        self.assertEqual(positions, sorted(positions))
        self.assertIn(b"Failed to append the DevSkim gate summary", result.stderr)

    def test_summary_write_failure_overrides_an_otherwise_passing_gate(self) -> None:
        self.write_scan([])
        result = self.run_gate("--summary", str(self.root))
        self.assertEqual(result.returncode, 1)
        self.assertIn(b"Gate conclusion:** PASS", result.stdout)
        self.assertIn(b"Failed to append the DevSkim gate summary", result.stderr)


if __name__ == "__main__":
    unittest.main()
