"""#1527：父登记先于源域放行的确定性时序锁。"""
import os
import select
import threading
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest.mock import Mock, patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import s_service_control as control
from r4_anchor_fixture import anchored_controller

class GateTests(unittest.TestCase):
    def test_registration_completes_before_release(self):
        with tempfile.TemporaryDirectory() as directory:
            c = anchored_controller({"db": str(Path(directory) / "db"),
                                     "startup_timeout_ms": "2000"}, Path(directory))
            pipe = Mock()
            pipe.write.return_value = 1
            child = Mock(pid=1234, stdin=pipe, **{"poll.return_value": None})
            def record(*args, **kwargs):
                pipe.write.assert_not_called()
                c.save_record("s", {"registered": True})
            try:
                with patch.object(c, "status", return_value={"state": "stopped"}), \
                     patch.object(c, "record", side_effect=record), \
                     patch.object(control.subprocess, "Popen", return_value=child) as spawn:
                    c.start_process("s", ["service", "serve"])
                self.assertTrue(c.record_path("s").exists())
                pipe.write.assert_called_once_with(b"R")
                pipe.close.assert_called_once()
                self.assertEqual(spawn.call_args.kwargs["stdin"], control.subprocess.PIPE)
                self.assertEqual(spawn.call_args.kwargs["env"]["S_CONTROL_START_GATE"], "stdin-v1")
            finally:
                c.close()

    def test_registration_failure_and_expiry_never_release(self):
        for failure in ("write", "deadline"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                c = anchored_controller({"db": str(Path(directory) / "db"),
                                         "startup_timeout_ms": "2000"}, Path(directory))
                pipe = Mock()
                child = Mock(pid=1234, stdin=pipe, **{"poll.return_value": None})
                def record(*args, **kwargs):
                    pipe.write.assert_not_called()
                    if failure == "write":
                        raise OSError("record write failed")
                try:
                    with patch.object(c, "status", return_value={"state": "stopped"}), \
                         patch.object(c, "record", side_effect=record), \
                         patch.object(control.subprocess, "Popen", return_value=child), \
                         patch.object(control, "remaining", side_effect=[1, 1, TimeoutError("deadline")]):
                        with self.assertRaises((OSError, TimeoutError)):
                            c.start_process("s", ["service", "serve"], deadline=time.monotonic()+1)
                    pipe.write.assert_not_called()
                    pipe.close.assert_called_once()
                    child.terminate.assert_called_once()
                    child.wait.assert_called_once()
                finally:
                    c.close()

    def test_q_does_not_inherit_managed_s_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            c = anchored_controller({"db": str(Path(directory) / "db"),
                                     "startup_timeout_ms": "2000"}, Path(directory))
            child = Mock(pid=1234)
            try:
                with patch.object(c, "status", return_value={"state": "stopped"}), \
                     patch.object(c, "record"), \
                     patch.object(control.subprocess, "Popen", return_value=child) as spawn:
                    c.start_process("q", ["q-service"],
                                    environment={"S_CONTROL_START_GATE": "stdin-v1"})
                self.assertNotIn("S_CONTROL_START_GATE", spawn.call_args.kwargs["env"])
                self.assertEqual(spawn.call_args.kwargs["stdin"], control.subprocess.DEVNULL)
                child.stdin.write.assert_not_called()
            finally:
                c.close()

    def test_real_pipe_stays_closed_until_record_latch_released(self):
        with tempfile.TemporaryDirectory() as directory:
            c = anchored_controller({"db": str(Path(directory) / "db"),
                                     "startup_timeout_ms": "2000"}, Path(directory))
            read_fd, write_fd = os.pipe()
            writer = os.fdopen(write_fd, "wb", buffering=0)
            child = Mock(pid=1234, stdin=writer, **{"poll.return_value": None})
            entered, release = threading.Event(), threading.Event()
            errors = []
            def record(*args, **kwargs):
                entered.set()
                if not release.wait(1):
                    raise TimeoutError("test latch")
                c.save_record("s", {"registered": True})
            def start():
                try:
                    c.start_process("s", ["service", "serve"])
                except Exception as error:
                    errors.append(error)
            try:
                with patch.object(c, "status", return_value={"state": "stopped"}), \
                     patch.object(c, "record", side_effect=record), \
                     patch.object(control.subprocess, "Popen", return_value=child):
                    thread = threading.Thread(target=start)
                    thread.start()
                    try:
                        self.assertTrue(entered.wait(1))
                        self.assertEqual(select.select([read_fd], [], [], 0)[0], [])
                        self.assertFalse(c.record_path("s").exists())
                    finally:
                        release.set()
                        thread.join(2)
                    self.assertFalse(thread.is_alive())
                    self.assertEqual(errors, [])
                    self.assertTrue(c.record_path("s").exists())
                    self.assertEqual(os.read(read_fd, 1), b"R")
                    self.assertEqual(os.read(read_fd, 1), b"")
            finally:
                writer.close()
                os.close(read_fd)
                c.close()
