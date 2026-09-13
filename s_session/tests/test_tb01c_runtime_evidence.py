"""#1372：窗口外进度与未完成标记不能产生错误验收结论。"""

import json
import tempfile
import threading
import time
import unittest
from unittest import mock
from types import SimpleNamespace
from pathlib import Path

from tb01c_load_report import report
from tb01c_runtime import Run, NotVerified, require_kill_receipt, wait_for_marker


class RuntimeEvidenceTests(unittest.TestCase):
    def test_real_cli_single_service_stop_result_is_unwrapped(self):
        run = Run.__new__(Run)
        run.config = {'startup_timeout_ms': '10000'}
        run.config_path, run.state_dir = Path('/tmp/config'), Path('/tmp/state')
        run.emit = mock.Mock()
        stopped = {'service': 's', 'state': 'stopped', 'pid': 123, 'signal': 'SIGKILL'}
        for value in ([stopped], [], [stopped, stopped], stopped):
            completed = SimpleNamespace(returncode=0, stdout=json.dumps({'ok': True, 'result': value}).encode(), stderr=b'')
            with mock.patch('tb01c_runtime.subprocess.run', return_value=completed):
                if value == [stopped]:
                    receipt = run.control('stop-s', '--signal', 'KILL')
                    self.assertEqual(receipt, stopped)
                    require_kill_receipt(receipt, 's', {'pid': 123})
                else:
                    with self.assertRaises(NotVerified):
                        run.control('stop-s', '--signal', 'KILL')

    def load_report(self, offsets, *, stalled=False):
        events = [{"kind": "phase_begin", "first_operation": 96, "phase_anchor_ns": "100000000000"}]
        for index in range(160):
            when = 100000000000 + (index - 96) * 500000000
            events.append({"kind": "ingest", "operation": index, "planned_ns": str(when),
                           "sent_ns": str(when), "received_ns": str(when + 1000000),
                           "result": {"transport": "DeliveryUnknown" if index == 95 else "Received"}})
        for offset in offsets:
            frontier = 96 if stalled else 96 + int(offset / 500)
            events.append({"kind": "frontier", "phase_start": 96,
                           "phase_offset_ns": str(offset * 1000000),
                           "accepted": str(frontier), "committed": str(frontier)})
        events.append({"kind": "phase_end", "first_operation": 96, "accepted": "160", "committed": "160"})
        with tempfile.TemporaryDirectory(prefix="tb01c-window-") as directory:
            path = Path(directory) / "events.jsonl"
            path.write_text("".join(json.dumps(event) + "\n" for event in events))
            return report(path)

    def test_late_samples_cannot_prove_declared_windows(self):
        result = self.load_report([0, 15000, 25000, 35000])
        self.assertEqual(result["status"], "NOT_VERIFIED")
        self.assertTrue(all(window["evidence_status"].startswith("NOT_VERIFIED") for window in result["windows"]))

    def test_interior_progress_reports_actual_span(self):
        result = self.load_report([50, 9900, 10100, 19900, 20100, 29900, 35100])
        self.assertTrue(result["status"].startswith("OBSERVED_"))
        self.assertEqual(result["windows"][0]["actual_sample_span_ms"], [50, 9900])
        self.assertEqual(result["windows"][0]["end_unobserved_ms"], 100)
        for window in result["windows"]:
            begin, end = window["declared_window_ms"]
            a, b = window["actual_sample_span_ms"]
            self.assertLessEqual(begin, a)
            self.assertLess(a, b)
            self.assertLessEqual(b, end)

    def test_no_progress_is_fail_and_missing_evidence_is_not_verified(self):
        self.assertEqual(self.load_report([0, 10000, 20000, 30000], stalled=True)["status"], "FAIL")
        self.assertEqual(self.load_report([0, 10000, 20000])["status"], "NOT_VERIFIED")

    def test_marker_empty_and_partial_then_complete(self):
        expected = {"pid": "123", "stage": "after_begin", "db": "/tmp/db", "exe": "/tmp/s"}
        full = "".join(key + "=" + value + "\n" for key, value in expected.items())
        with tempfile.TemporaryDirectory(prefix="tb01c-marker-") as directory:
            path = Path(directory) / "marker"
            path.touch()
            def publish():
                time.sleep(0.03)
                path.write_text(full[:-3])
                time.sleep(0.03)
                path.write_text(full)
            writer = threading.Thread(target=publish)
            writer.start()
            try:
                wait_for_marker(path, expected, time.monotonic() + 1)
            finally:
                writer.join(timeout=1)
            self.assertFalse(writer.is_alive())

    def test_marker_complete_wrong_or_duplicate_identity_is_rejected(self):
        expected = {"pid": "123", "stage": "after_begin", "db": "/tmp/db", "exe": "/tmp/s"}
        cases = ["pid=999\nstage=after_begin\ndb=/tmp/db\nexe=/tmp/s\n",
                 "pid=123\nstage=after_begin\ndb=/tmp/db\npid=123\n"]
        with tempfile.TemporaryDirectory(prefix="tb01c-marker-") as directory:
            path = Path(directory) / "marker"
            for text in cases:
                path.write_text(text)
                with self.assertRaises(ValueError):
                    wait_for_marker(path, expected, time.monotonic() + 1)

    def test_marker_incomplete_deadline_and_size_are_bounded(self):
        expected = {"pid": "123", "stage": "after_begin", "db": "/tmp/db", "exe": "/tmp/s"}
        with tempfile.TemporaryDirectory(prefix="tb01c-marker-") as directory:
            path = Path(directory) / "marker"
            path.write_text("pid=123\n")
            started = time.monotonic()
            with self.assertRaises(TimeoutError):
                wait_for_marker(path, expected, started + 0.03)
            self.assertLess(time.monotonic() - started, 0.3)
            path.write_bytes(b"x" * 65537)
            with self.assertRaises(ValueError):
                wait_for_marker(path, expected, time.monotonic() + 1)


if __name__ == "__main__":
    unittest.main()
