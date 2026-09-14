#!/usr/bin/env python3
"""#1373：验收驱动的半帧期限与失败清理行为锁。"""

import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from tb02a_runtime import Tb02aRun, helper_exchange
import s_service_control as control


class RuntimeLibraryTests(unittest.TestCase):
    def environment(self, root, metadata):
        controller = control.Controller({"db": str(root / "owned.sqlite"), "python": "/explicit/python3.12"},
                                        root / "control")
        try:
            reply = subprocess.CompletedProcess([], 0, json.dumps(metadata).encode(), b"")
            with patch.object(control.subprocess, "run", return_value=reply):
                return controller.runtime_environment(time.monotonic() + 2)
        finally:
            controller.close()

    def test_regular_shared_library_layout(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "libpython3.12.so").write_bytes(b"test layout only")
            env = self.environment(root, [str(root), "libpython3.12.so", "", None])
            self.assertEqual(env["PYO3_PYTHON"], "/explicit/python3.12")
            self.assertEqual(env["LD_LIBRARY_PATH"].split(control.os.pathsep)[0], str(root.resolve()))

    def test_framework_uses_its_declared_prefix(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            library = root / "Frameworks/Python.framework/Versions/3.12/Python"
            library.parent.mkdir(parents=True)
            library.write_bytes(b"test layout only")
            env = self.environment(root, [str(root / "different-libdir"), "Python.framework/Versions/3.12/Python",
                                          "Python", str(root / "Frameworks")])
            self.assertEqual(env["DYLD_LIBRARY_PATH"].split(control.os.pathsep)[0], str(library.resolve().parent))
            self.assertEqual(env["DYLD_FRAMEWORK_PATH"].split(control.os.pathsep)[0], str(root / "Frameworks"))

    def test_missing_declared_framework_cannot_fall_back_to_libdir(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            library = root / "libdir/Python.framework/Versions/3.12/Python"
            library.parent.mkdir(parents=True)
            library.write_bytes(b"unrelated file must not be accepted")
            with self.assertRaisesRegex(ValueError, "共享运行库"):
                self.environment(root, [str(root / "libdir"), "Python.framework/Versions/3.12/Python",
                                        "Python", str(root / "missing-frameworks")])


class HelperDeadlineTests(unittest.TestCase):
    def exchange(self, code, **limits):
        process = subprocess.Popen([sys.executable, "-u", "-c", code], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, bufsize=0)
        try:
            return helper_exchange(process, {"id": "one"}, **limits)
        finally:
            if process.poll() is None:
                process.kill()
            process.wait(timeout=3)
            process.stdin.close()
            process.stdout.close()

    def test_partial_frame_cannot_outlive_deadline(self):
        start = time.monotonic()
        with self.assertRaises(TimeoutError):
            self.exchange("import sys,time;sys.stdin.readline();sys.stdout.write('{');sys.stdout.flush();time.sleep(30)",
                          timeout=0.2)
        self.assertLess(time.monotonic() - start, 3)

    def test_complete_fragmented_response(self):
        response = self.exchange("import sys,time;sys.stdin.readline();sys.stdout.write('{');sys.stdout.flush();"
                                 "time.sleep(.02);sys.stdout.write('\"id\":\"one\"}\\n');sys.stdout.flush()",
                                 timeout=2)
        self.assertEqual(response, {"id": "one"})

    def test_response_size_is_bounded(self):
        with self.assertRaisesRegex(ValueError, "超过声明大小"):
            self.exchange("import sys;sys.stdin.readline();sys.stdout.write('x'*1024);sys.stdout.flush()",
                          timeout=2, max_bytes=128)

    def test_eof_before_newline_is_failure(self):
        with self.assertRaises(EOFError):
            self.exchange("import sys;sys.stdin.readline();sys.stdout.write('{}');sys.stdout.flush()", timeout=2)


class CleanupFailureTests(unittest.TestCase):
    def run_shell(self, directory, fail_q=False, helper=None):
        run = Tb02aRun.__new__(Tb02aRun)
        run.output, run.messages, run.source_hashes = Path(directory), [], {}
        run.helper, run.helper_stderr, run.events = helper, None, io.StringIO()
        run.collect_core = lambda: 0
        calls = []

        def control(action):
            calls.append(action)
            if action == "stop-q" and fail_q:
                raise RuntimeError("Q stop injected failure")
            return {"service": action.removeprefix("stop-"), "state": "stopped"}

        run.control = control
        return run, calls

    def test_q_cleanup_failure_still_stops_s_and_writes_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            run, calls = self.run_shell(directory, fail_q=True)
            result = run.execute()
            self.assertEqual(calls, ["start", "stop-q", "stop-s"])
            self.assertEqual(result["status"], "FAIL")
            self.assertTrue(run.events.closed)
            self.assertEqual(json.loads((Path(directory) / "RESULT.json").read_text()), result)

    def test_failed_helper_exit_cannot_be_capture_success(self):
        helper = subprocess.Popen([sys.executable, "-c", "raise SystemExit(7)"], stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, bufsize=0)
        try:
            with tempfile.TemporaryDirectory() as directory:
                run, calls = self.run_shell(directory, helper=helper)
                result = run.execute()
                self.assertEqual(result["status"], "FAIL")
                self.assertEqual(calls, ["start", "stop-q", "stop-s"])
                self.assertIn({"action": "helper-exit", "returncode": 7}, result["cleanup"]["errors"])
        finally:
            if helper.poll() is None:
                helper.kill()
            helper.wait(timeout=3)


if __name__ == "__main__":
    unittest.main()
