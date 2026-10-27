"""#1716 正式消费驱动的停机汇总：业务与清理两类失败都进入 RESULT 与退出码。业务只用替身，不起服务。"""
import io
import json
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tb1716_formal_consumption as driver


class FakeRun(driver.Run):
    failing_stops = set()
    business_error = None

    def __init__(self, case, binaries, output):
        self.case, self.output = case, output
        output.mkdir()
        self.log = io.StringIO()
        self.started, self.stops = [], []

    def run(self):
        self.started.extend(driver.DOMAINS)
        if self.business_error:
            raise self.business_error
        return {"business": "test double"}

    def control(self, action, domain, *extra):
        self.stops.append(domain)
        if domain in self.failing_stops:
            raise RuntimeError("stop failed " + domain)
        return {"result": {"state": "stopped"}}


class CleanupOutcome(unittest.TestCase):
    def run_main(self, failing_stops=(), business_error=None):
        runs = []

        class Probe(FakeRun):
            def __init__(self, *args):
                super().__init__(*args)
                runs.append(self)

        Probe.failing_stops, Probe.business_error = set(failing_stops), business_error
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "out"
            (Path(tmp) / "case").write_text("{}")
            argv = ["tb1716", "--case", str(Path(tmp) / "case"), "--binaries", tmp, "--output", str(out)]
            stdout, stderr = io.StringIO(), io.StringIO()
            with patch.object(driver, "Run", Probe), \
                    patch.object(driver, "decode_frame", return_value={"cap": "90", "fault": None}), \
                    patch.object(sys, "argv", argv), redirect_stdout(stdout), redirect_stderr(stderr):
                code = driver.main()
            result = json.loads((out / "RESULT.json").read_text())
        return code, result, json.loads(stdout.getvalue()), runs[0].stops, stderr.getvalue()

    def test_clean_stop_passes(self):
        code, result, printed, stops, _ = self.run_main()
        self.assertEqual((code, result["status"], printed["status"]), (0, "passed", "passed"))
        self.assertEqual(stops, list(reversed(driver.DOMAINS)))
        self.assertNotIn("cleanup_errors", result)

    def test_one_stop_failure_fails_and_other_owners_still_stop(self):
        code, result, printed, stops, stderr = self.run_main(failing_stops={"B:alpha"})
        self.assertEqual((code, result["status"], printed["status"]), (1, "failed", "failed"))
        self.assertEqual(stops, list(reversed(driver.DOMAINS)))
        self.assertEqual([e["domain"] for e in result["cleanup_errors"]], ["B:alpha"])
        self.assertIsNone(result.get("error"))
        self.assertIn("B:alpha", stderr)

    def test_business_and_cleanup_failures_both_kept(self):
        code, result, printed, stops, _ = self.run_main(
            failing_stops={"E", "B:beta"}, business_error=AssertionError("business broke"))
        self.assertEqual((code, result["status"]), (1, "failed"))
        self.assertIn("business broke", result["error"])
        self.assertEqual([e["domain"] for e in result["cleanup_errors"]], ["B:beta", "E"])
        self.assertEqual(printed["cleanup_errors"], result["cleanup_errors"])
        self.assertEqual(stops, list(reversed(driver.DOMAINS)))


if __name__ == "__main__":
    unittest.main()
