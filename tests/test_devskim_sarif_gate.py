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
        # stdout（job log）保持全量、有序。
        rows = [row for row in result.stdout.decode("utf-8").splitlines()
                if row.startswith("-   - `")]
        self.assertEqual(rows, [f"-   - `DS000001|{uri}|1`" for uri in uris])
        self.assertIn(b"12000 finding(s) not in the baseline (total 12000)",
                      result.stdout)
        self.assertTrue(result.stdout.endswith(b"\n"))
        # #1588：摘要文件追加在既有内容后，只列默认前 200 条并指向全量出处。
        written = self.summary.read_bytes()
        self.assertTrue(written.startswith(prior))
        appended = written[len(prior):].decode("utf-8")
        summary_rows = [row for row in appended.splitlines()
                        if row.startswith("-   - `")]
        self.assertEqual(summary_rows, rows[:200])
        self.assertIn("listed the first 200 of 12000 new finding(s); 11800 more omitted",
                      appended)
        self.assertIn("`test-sarif` artifact", appended)
        self.assertIn("12000 finding(s) not in the baseline (total 12000)", appended)
        self.assertIn("Gate conclusion:** FAIL", appended)
        self.assertTrue(appended.endswith("\n"))
        self.assertLess(len(written), 1024 * 1024)

    def new_findings_run(self, count: int, cap: str) -> subprocess.CompletedProcess:
        self.write_scan([finding(f"n{index:03d}.json") for index in range(count)])
        self.baseline.write_text(json.dumps({
            "schema": "devskim-baseline/v1", "findings": [],
        }), encoding="utf-8")
        return self.run_gate("--baseline", str(self.baseline),
                             "--summary", str(self.summary),
                             "--summary-max-new", cap)

    def test_summary_cap_boundary_keeps_identical_report(self) -> None:
        result = self.new_findings_run(3, "3")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.summary.read_bytes(), result.stdout)
        self.assertNotIn(b"Step summary truncated", result.stdout)

    def test_summary_cap_exceeded_by_one_truncates_summary_only(self) -> None:
        result = self.new_findings_run(4, "3")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr, b"")
        self.assertNotIn(b"Step summary truncated", result.stdout)
        self.assertIn(b"n003.json", result.stdout)
        written = self.summary.read_bytes().decode("utf-8")
        self.assertIn("listed the first 3 of 4 new finding(s); 1 more omitted", written)
        self.assertIn("n002.json", written)
        self.assertNotIn("n003.json", written)

    def test_summary_cap_zero_still_fails_and_lists_nothing(self) -> None:
        result = self.new_findings_run(2, "0")
        self.assertEqual(result.returncode, 1)
        written = self.summary.read_bytes().decode("utf-8")
        self.assertIn("listed the first 0 of 2 new finding(s); 2 more omitted", written)
        self.assertNotIn("-   - `", written)
        self.assertIn("Gate conclusion:** FAIL", written)

    def test_summary_cap_leading_zero_is_decimal_not_octal(self) -> None:
        # 08 在 (( )) 里按八进制会报错；须按十进制 8 截断。
        result = self.new_findings_run(9, "08")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr, b"")
        written = self.summary.read_bytes().decode("utf-8")
        self.assertIn("listed the first 8 of 9 new finding(s); 1 more omitted", written)
        self.assertIn("n007.json", written)
        self.assertNotIn("n008.json", written)
        self.assertIn(b"n008.json", result.stdout)

    def test_summary_cap_0200_means_two_hundred(self) -> None:
        # 0200 按八进制是 128；须按十进制 200。
        result = self.new_findings_run(201, "0200")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stderr, b"")
        written = self.summary.read_bytes().decode("utf-8")
        self.assertIn("listed the first 200 of 201 new finding(s); 1 more omitted", written)
        self.assertIn("n199.json", written)
        self.assertNotIn("n200.json", written)

    def test_invalid_summary_cap_is_a_usage_error(self) -> None:
        self.write_scan([])
        result = self.run_gate("--summary-max-new", "many")
        self.assertEqual(result.returncode, 2)
        self.assertIn(b"--summary-max-new must be a non-negative integer", result.stderr)

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
