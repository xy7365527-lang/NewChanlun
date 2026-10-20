"""#1372：生命周期边界；仅本测试创建的空闲进程，不计为正式 S/Q 故障验收。"""

import json
import os
import signal
import socket
import threading
import time
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import s_service_control as control


class LifecycleTests(unittest.TestCase):
    @staticmethod
    def ps_reply(command, *, uid=None, started="Sun Sep 13 16:17:43 2026"):
        uid = control.os.getuid() if uid is None else uid
        return control.subprocess.CompletedProcess(
            [], 0, f"  {uid} Rs   {started}     {command}\n", "")

    def test_initial_abbreviated_command_waits_for_complete_identity(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-initial-argv-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned"),
                                             "startup_timeout_ms": "2000"}, Path(directory))
            command = ["/owned/python3.14", "-c", "import time; time.sleep(60)", "q"]
            child = Mock(pid=123456, **{"poll.return_value": None})
            calls = []
            def query(*args, **kwargs):
                self.assertFalse(controller.record_path("q").exists())
                calls.append(kwargs["timeout"])
                return self.ps_reply("(python3.14)" if len(calls) == 1 else " ".join(command))
            try:
                with patch.object(control.subprocess, "Popen", return_value=child), \
                     patch.object(control.subprocess, "run", side_effect=query), \
                     patch.object(control.time, "sleep") as sleep:
                    result = controller.start_process("q", command, deadline=time.monotonic() + 0.5)
                record = json.loads(controller.record_path("q").read_text())
                self.assertIs(result, child)
                self.assertEqual(len(calls), 2)
                self.assertTrue(all(0 < timeout <= 0.5 for timeout in calls))
                self.assertTrue(record["process_identity"].endswith(" ".join(command)))
                self.assertNotIn("(python3.14)", record["process_identity"])
                sleep.assert_called_once()
                child.terminate.assert_not_called()
            finally:
                controller.close()

    def test_persistent_abbreviation_expires_in_original_budget_and_cleans_only_child(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-argv-timeout-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned"),
                                             "startup_timeout_ms": "2000"}, Path(directory))
            original = b'{"untouched":"other service record"}\n'
            controller.record_path("s").write_bytes(original)
            child = Mock(pid=123456, **{"poll.return_value": None})
            clock = [0.0]
            def advance(duration):
                clock[0] += duration
            try:
                with patch.object(control.subprocess, "Popen", return_value=child), \
                     patch.object(control.subprocess, "run", return_value=self.ps_reply("(python3.14)")) as query, \
                     patch.object(control.time, "monotonic", side_effect=lambda: clock[0]), \
                     patch.object(control.time, "sleep", side_effect=advance), patch.object(control.os, "kill") as kill:
                    with self.assertRaises(TimeoutError):
                        controller.start_process("q", ["/owned/python3.14", "owned.py"], deadline=0.025)
                self.assertAlmostEqual(clock[0], 0.025)
                self.assertEqual(query.call_count, 3)
                self.assertTrue(all(0 < call.kwargs["timeout"] <= 0.025 for call in query.call_args_list))
                self.assertFalse(controller.record_path("q").exists())
                self.assertEqual(controller.record_path("s").read_bytes(), original)
                child.terminate.assert_called_once_with()
                child.wait.assert_called_once_with(timeout=2.0)
                kill.assert_not_called()
            finally:
                controller.close()

    def test_complete_wrong_arguments_or_uid_are_rejected_without_retry(self):
        for wrong_uid in (False, True):
            with self.subTest(wrong_uid=wrong_uid), tempfile.TemporaryDirectory(prefix="tb01c-argv-mismatch-") as directory:
                controller = control.Controller({"db": str(Path(directory) / "owned"),
                                                 "startup_timeout_ms": "2000"}, Path(directory))
                command = ["/owned/python3.14", "right.py"]
                response = self.ps_reply(" ".join(command) if wrong_uid else "/owned/python3.14 wrong.py",
                                         uid=control.os.getuid() + 1 if wrong_uid else None)
                child = Mock(pid=123456, **{"poll.return_value": None})
                try:
                    with patch.object(control.subprocess, "Popen", return_value=child), \
                         patch.object(control.subprocess, "run", return_value=response) as query, \
                         patch.object(control.time, "sleep") as sleep:
                        with self.assertRaisesRegex(ValueError, "不属于本用户" if wrong_uid else "参数与本次启动命令不符"):
                            controller.start_process("q", command)
                    query.assert_called_once()
                    sleep.assert_not_called()
                    child.terminate.assert_called_once_with()
                    child.wait.assert_called_once()
                    self.assertFalse(controller.record_path("q").exists())
                finally:
                    controller.close()

    def test_start_s_and_q_share_action_deadline_with_initial_registration(self):
        for service in ("s", "q"):
            with self.subTest(service=service), tempfile.TemporaryDirectory(prefix="tb01c-start-budget-") as directory:
                db = Path(directory) / "owned"
                db.touch()
                resource_config = Path(directory) / "query-resources.json"
                resource_config.write_bytes((Path(__file__).parent / "fixtures/tb01c/query-resources.json").read_bytes())
                config = {"db": str(db), "startup_timeout_ms": "25", "binary": "/owned/python3.14",
                          "python": "/owned/python3.14", "socket": "unused", "port": "12345",
                          "profile": "unused", "clock_plan": "unused", "browser": "unused",
                          "query_resource_config": str(resource_config), "s_resources": {field: "1" for field in (
                              "max_frame_bytes", "read_timeout_ms", "write_timeout_ms", "queue_capacity",
                              "max_connections", "response_timeout_ms", "audit_cache_source_bytes")}}
                controller = control.Controller(config, Path(directory))
                clock = [0.0]
                child = Mock(pid=123456, **{"poll.return_value": None})
                def spawn(*args, **kwargs):
                    clock[0] = 0.020  # 前置动作和 spawn 已用掉原预算中的 20ms。
                    return child
                def advance(duration):
                    clock[0] += duration
                try:
                    with patch.object(controller, "runtime_environment", return_value={}), \
                         patch.object(controller, "query_resource_revision", return_value="legacy/2"), \
                         patch.object(controller, "prepare_socket", return_value=None), \
                         patch.object(controller, "wait_ready") as ready, \
                         patch.object(control.subprocess, "Popen", side_effect=spawn), \
                         patch.object(control.subprocess, "run", return_value=self.ps_reply("(python3.14)")) as query, \
                         patch.object(control.time, "monotonic", side_effect=lambda: clock[0]), \
                         patch.object(control.time, "sleep", side_effect=advance):
                        with self.assertRaises(TimeoutError):
                            getattr(controller, "start_" + service)("1")
                    self.assertAlmostEqual(clock[0], 0.025)
                    query.assert_called_once()
                    self.assertAlmostEqual(query.call_args.kwargs["timeout"], 0.005)
                    ready.assert_not_called()
                    self.assertFalse(controller.record_path(service).exists())
                    child.terminate.assert_called_once_with()
                    child.wait.assert_called_once()
                finally:
                    controller.close()

    def test_start_q_selects_explicit_v3_resources_and_preserves_original_deadline(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-q3-entry-") as directory:
            resources = Path(directory) / "resources.json"
            resources.write_text(json.dumps({"schema_revision": "s-query-resources/3", "limits": {}}))
            config = {"db": str(Path(directory) / "owned"), "startup_timeout_ms": "25",
                      "binary": "/owned/s_structure_session", "python": "/owned/python3.12",
                      "browser": "/owned/browser", "port": "12345", "query_resource_config": str(resources)}
            controller = control.Controller(config, Path(directory))
            clock = [0.0]
            def environment(deadline):
                self.assertEqual(deadline, 0.025)
                clock[0] = 0.020
                return {"PYO3_PYTHON": config["python"]}
            expected = {"service": "q", "state": "ready"}
            try:
                with patch.object(control.time, "monotonic", side_effect=lambda: clock[0]), \
                     patch.object(controller, "query_resource_revision", return_value=json.loads(resources.read_text()).get("schema_revision", "legacy/2")), \
                     patch.object(controller, "runtime_environment", side_effect=environment) as env, \
                     patch.object(controller, "start_process") as start, \
                     patch.object(controller, "wait_ready", return_value=expected) as ready:
                    self.assertEqual(controller.start_q("7"), expected)
                env.assert_called_once_with(0.025)
                start.assert_called_once_with("q", [config["binary"], "query-serve", "--protocol-revision",
                    "s-session/3", "--db", config["db"], "--resource-config", str(resources),
                    "--producer-epoch", "7", "--host", "127.0.0.1", "--port", "12345"],
                    {"PYO3_PYTHON": config["python"]}, deadline=0.025)
                ready.assert_called_once_with("q", query_epoch="7", deadline=0.025)
            finally:
                controller.close()

    def test_start_q_legacy_resources_keep_python_browser_and_economic_entry(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-q2-entry-") as directory:
            resources = Path(directory) / "resources.json"
            resources.write_bytes((Path(__file__).parent / "fixtures/tb01c/query-resources.json").read_bytes())
            config = {"db": str(Path(directory) / "owned"), "startup_timeout_ms": "25",
                      "binary": "/owned/s_structure_session", "python": "/owned/python3.12",
                      "browser": "/owned/browser", "port": "12345", "query_resource_config": str(resources),
                      "economic_query_config": "/owned/economic.json"}
            controller = control.Controller(config, Path(directory))
            try:
                with patch.object(controller, "runtime_environment") as env, \
                     patch.object(controller, "query_resource_revision", return_value=json.loads(resources.read_text()).get("schema_revision", "legacy/2")), \
                     patch.object(controller, "start_process") as start, \
                     patch.object(controller, "wait_ready", return_value={"state": "ready"}):
                    controller.start_q("7")
                env.assert_not_called()
                command = start.call_args.args[1]
                self.assertEqual(command[:2], [config["python"], str(Path(control.__file__).with_name("s_readonly_server.py"))])
                self.assertEqual(command[-2:], ["--economic-config", config["economic_query_config"]])
                self.assertIn("--browser", command)
                self.assertNotIn("query-serve", command)
            finally:
                controller.close()

    def test_start_q_rejects_unknown_schema_and_v3_economic_configuration_before_spawn(self):
        cases = [("s-query-resources/4", False), ("s-query-resources/2", False),
                 ("s-query-resources/3", True)]
        for schema, economic in cases:
            with self.subTest(schema=schema, economic=economic), tempfile.TemporaryDirectory(prefix="tb01c-q-reject-") as directory:
                resources = Path(directory) / "resources.json"
                resources.write_text(json.dumps({"schema_revision": schema, "limits": {}}))
                config = {"db": str(Path(directory) / "owned"), "startup_timeout_ms": "25",
                          "binary": "/owned/s_structure_session", "python": "/owned/python3.12",
                          "browser": "/owned/browser", "port": "12345", "query_resource_config": str(resources)}
                if economic:
                    config["economic_query_config"] = "/owned/economic.json"
                controller = control.Controller(config, Path(directory))
                original = b'{"preserved":"earlier service receipt"}\n'
                controller.record_path("q").write_bytes(original)
                try:
                    with patch.object(controller, "runtime_environment") as env, \
                         patch.object(controller, "query_resource_revision", return_value=json.loads(resources.read_text()).get("schema_revision", "legacy/2")), \
                         patch.object(controller, "start_process") as start, \
                         patch.object(controller, "wait_ready") as ready:
                        with self.assertRaises(ValueError):
                            controller.start_q("7")
                    env.assert_not_called()
                    start.assert_not_called()
                    ready.assert_not_called()
                    self.assertEqual(controller.record_path("q").read_bytes(), original)
                finally:
                    controller.close()

    def test_start_q_v3_environment_cannot_reset_or_outlive_startup_budget(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-q3-budget-") as directory:
            resources = Path(directory) / "resources.json"
            resources.write_text(json.dumps({"schema_revision": "s-query-resources/3", "limits": {}}))
            config = {"db": str(Path(directory) / "owned"), "startup_timeout_ms": "25",
                      "binary": "/owned/s_structure_session", "python": "/owned/python3.12",
                      "browser": "/owned/browser", "port": "12345", "query_resource_config": str(resources)}
            controller = control.Controller(config, Path(directory))
            clock = [0.0]
            def environment(deadline):
                self.assertEqual(deadline, 0.025)
                clock[0] = deadline
                return {}
            try:
                with patch.object(control.time, "monotonic", side_effect=lambda: clock[0]), \
                     patch.object(controller, "query_resource_revision", return_value=json.loads(resources.read_text()).get("schema_revision", "legacy/2")), \
                     patch.object(controller, "runtime_environment", side_effect=environment), \
                     patch.object(controller, "start_process") as start, \
                     patch.object(controller, "wait_ready") as ready:
                    with self.assertRaises(TimeoutError):
                        controller.start_q("7")
                start.assert_not_called()
                ready.assert_not_called()
            finally:
                controller.close()

    def test_query_resource_fifo_is_rejected_without_waiting_for_a_writer(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-resource-fifo-") as directory:
            resources = Path(directory) / "resources.fifo"
            os.mkfifo(resources)
            controller = control.Controller({"db": str(Path(directory) / "owned"),
                "startup_timeout_ms": "200", "query_resource_config": str(resources)}, Path(directory))
            children = []
            original_popen = control.subprocess.Popen
            def observe(*args, **kwargs):
                child = original_popen(*args, **kwargs)
                children.append(child)
                return child
            try:
                started = time.monotonic()
                with patch.object(control.subprocess, "Popen", side_effect=observe):
                    with self.assertRaisesRegex(ValueError, "普通文件"):
                        controller.query_resource_revision(started + 0.2)
                self.assertLess(time.monotonic() - started, 0.3)
                self.assertEqual(len(children), 1)
                self.assertIsNotNone(children[0].poll())
            finally:
                controller.close()

    def test_slow_resource_reader_is_killed_and_reaped_at_original_deadline(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-resource-deadline-") as directory:
            resources = Path(directory) / "resources.json"
            resources.write_text('{"schema_revision":"s-query-resources/3","limits":{}}')
            controller = control.Controller({"db": str(Path(directory) / "owned"),
                "startup_timeout_ms": "200", "query_resource_config": str(resources)}, Path(directory))
            children = []
            original_popen = control.subprocess.Popen
            def slow(*args, **kwargs):
                command = list(args[0])
                command[2] = "import time; time.sleep(1); " + command[2]
                child = original_popen(command, **kwargs)
                children.append(child)
                return child
            try:
                started = time.monotonic()
                with patch.object(control.subprocess, "Popen", side_effect=slow), \
                     patch.object(controller, "start_process") as start, \
                     patch.object(controller, "runtime_environment") as env:
                    with self.assertRaises(control.subprocess.TimeoutExpired):
                        controller.start_q("1")
                self.assertLess(time.monotonic() - started, 0.4)
                self.assertEqual(len(children), 1)
                self.assertEqual(children[0].returncode, -signal.SIGKILL)
                start.assert_not_called()
                env.assert_not_called()
            finally:
                controller.close()

    def test_initial_identity_cannot_outlive_child_or_change_start_time(self):
        for failure in ("exited", "changed_start"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory(prefix="tb01c-argv-life-") as directory:
                controller = control.Controller({"db": str(Path(directory) / "owned"),
                                                 "startup_timeout_ms": "2000"}, Path(directory))
                command = ["/owned/python3.14", "owned.py"]
                child = Mock(pid=123456, **{"poll.return_value": None})
                responses = [self.ps_reply("(python3.14)"), self.ps_reply(
                    " ".join(command), started="Sun Sep 13 16:17:44 2026" if failure == "changed_start" else "Sun Sep 13 16:17:43 2026")]
                if failure == "exited":
                    # 缩略后仍存活；第二次完整命令查询期间，本次直接子进程退出。
                    child.poll.side_effect = [None, None, None, 0, 0]
                try:
                    with patch.object(control.subprocess, "Popen", return_value=child), \
                         patch.object(control.subprocess, "run", side_effect=responses), \
                         patch.object(control.time, "sleep"):
                        with self.assertRaisesRegex(ValueError, "立即退出" if failure == "exited" else "启动身份改变"):
                            controller.start_process("q", command)
                    self.assertFalse(controller.record_path("q").exists())
                    child.wait.assert_called_once()
                    if failure == "exited":
                        child.terminate.assert_not_called()
                finally:
                    controller.close()

    def test_complete_identity_arriving_after_deadline_is_not_recorded(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-argv-late-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned"),
                                             "startup_timeout_ms": "2000"}, Path(directory))
            clock = [0.0]
            child = Mock(pid=123456, **{"poll.return_value": None})
            command = ["/owned/python3.14", "owned.py"]
            def late_query(*args, **kwargs):
                self.assertEqual(kwargs["timeout"], 0.025)
                clock[0] = 0.026
                return self.ps_reply(" ".join(command))
            try:
                with patch.object(control.subprocess, "Popen", return_value=child), \
                     patch.object(control.subprocess, "run", side_effect=late_query), \
                     patch.object(control.time, "monotonic", side_effect=lambda: clock[0]):
                    with self.assertRaises(TimeoutError):
                        controller.start_process("q", command, deadline=0.025)
                self.assertFalse(controller.record_path("q").exists())
                child.terminate.assert_called_once_with()
                child.wait.assert_called_once()
            finally:
                controller.close()

    def test_abbreviation_does_not_relax_status_ready_or_stop(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-argv-existing-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned"),
                                             "startup_timeout_ms": "2000"}, Path(directory))
            command = ["/owned/python3.14", "owned.py"]
            record = {"service": "q", "pid": 123456, "db": controller.config["db"], "command": command,
                      "process_identity": "previous complete identity", "control_instance_id": "old"}
            controller.record_path("q").write_text(json.dumps(record))
            original = controller.record_path("q").read_bytes()
            try:
                with patch.object(control.subprocess, "run", return_value=self.ps_reply("(python3.14)")) as query, \
                     patch.object(control.time, "sleep") as sleep, patch.object(control.os, "kill") as kill:
                    for operation in (lambda: controller.status("q"), lambda: controller.wait_ready("q"),
                                      lambda: controller.stop("q", signal.SIGTERM)):
                        with self.assertRaises(control.ProcessObservationUnavailable):
                            operation()
                self.assertEqual(query.call_count, 3)
                self.assertEqual(controller.record_path("q").read_bytes(), original)
                sleep.assert_not_called()
                kill.assert_not_called()
            finally:
                controller.close()

    def test_after_signal_unknown_observation_is_not_adopted_or_signalled_again(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-exit-observation-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned"), "startup_timeout_ms": "500"}, Path(directory))
            record = {"state": "running", "pid": 123456, "command": ["owned"], "process_identity": "verified"}
            try:
                with patch.object(controller, "status", side_effect=[record, ValueError("退出时命令暂变"), {"state": "stopped"}]), \
                     patch.object(control, "process_identity", return_value="verified"), patch.object(control.os, "kill") as kill:
                    result = controller.stop("s", signal.SIGTERM)
                    self.assertEqual(result["state"], "stopped")
                    self.assertEqual(result["exit_observation_warnings"], ["退出时命令暂变"])
                    kill.assert_called_once_with(123456, signal.SIGTERM)
            finally:
                controller.close()

    def test_cleanup_requires_known_stopped_socket_identity(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-sockowner-", dir="/tmp") as directory:
            path = Path(directory) / "s.sock"
            controller = control.Controller({"db": str(Path(directory) / "owned"), "socket": str(path)}, Path(directory))
            listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            try:
                listener.bind(str(path))
                listener.listen(1)
                identity = controller.socket_identity()
                stopped = {"state": "stopped", "socket_identity": identity, "pid": 123456}
                with patch.object(controller, "status", return_value=stopped):
                    with self.assertRaisesRegex(ValueError, "活监听者"):
                        controller.prepare_socket(time.monotonic()+1)
                    self.assertTrue(path.exists())
                    listener.close()
                    result = controller.prepare_socket(time.monotonic()+1)
                    self.assertEqual(result["removed_stopped_service_socket"], identity)
                    self.assertFalse(path.exists())
            finally:
                listener.close()
                controller.close()

    def test_unknown_stale_socket_is_never_adopted(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-sockunknown-", dir="/tmp") as directory:
            path = Path(directory) / "s.sock"
            controller = control.Controller({"db": str(Path(directory) / "owned"), "socket": str(path)}, Path(directory))
            try:
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                    listener.bind(str(path))
                with self.assertRaisesRegex(ValueError, "不接管"):
                    controller.prepare_socket(time.monotonic()+1)
                self.assertTrue(path.exists())
            finally:
                controller.close()

    def test_incomplete_record_does_not_prove_stopped(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-incomplete-record-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned")}, Path(directory))
            try:
                controller.record_path("s").write_text(json.dumps({"db": controller.config["db"], "service": "s"}))
                with patch.object(control.subprocess, "run") as run:
                    with self.assertRaisesRegex(ValueError, "状态未知"):
                        controller.recover("2", "recover-001")
                    run.assert_not_called()
            finally:
                controller.close()

    def test_process_query_failure_does_not_prove_stopped(self):
        result = control.subprocess.CompletedProcess([], 2, "", "ps failed")
        with patch.object(control.subprocess, "run", return_value=result):
            with self.assertRaisesRegex(ValueError, "查询失败"):
                control.process_identity(123456)

    def test_recovery_decode_is_inside_action_deadline(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-decode-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned"), "startup_timeout_ms": "5",
                                             "binary": "test", "clock_plan": "test"}, Path(directory))
            try:
                with patch.object(controller, "status", return_value={"state": "stopped"}), \
                     patch.object(controller, "runtime_environment", return_value={}), \
                     patch.object(control.subprocess, "run", return_value=control.subprocess.CompletedProcess([], 0, b'{"ok":true}')), \
                     patch.object(control.time, "monotonic", side_effect=[0, 0.001, 0.1]):
                    with self.assertRaises(TimeoutError):
                        controller.recover("2", "recover-001")
            finally:
                controller.close()

    def test_database_owner_cannot_be_replaced_with_another_state_directory(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-dbowner-") as directory:
            config = {"db": str(Path(directory) / "owned")}
            first = control.Controller(config, Path(directory) / "first")
            first.close()
            with self.assertRaisesRegex(ValueError, "其他控制目录"):
                control.Controller(config, Path(directory) / "second")

    def test_unknown_process_record_does_not_authorize_recovery(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-recover-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned")}, Path(directory))
            try:
                with patch.object(control.subprocess, "run") as run:
                    with self.assertRaisesRegex(ValueError, "已确认"):
                        controller.recover("2", "recover-001")
                    run.assert_not_called()
            finally:
                controller.close()

    def test_failed_record_terminates_and_reaps_only_new_child(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-record-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned"),
                                             "startup_timeout_ms": "2000"}, Path(directory))
            children = []
            popen = control.subprocess.Popen
            def capture(*args, **kwargs):
                child = popen(*args, **kwargs)
                children.append(child)
                return child
            try:
                with patch.object(controller, "record", side_effect=ValueError("record failed")), \
                     patch.object(control.subprocess, "Popen", side_effect=capture):
                    with self.assertRaisesRegex(ValueError, "record failed"):
                        controller.start_process("q", [sys.executable, "-c", "import time; time.sleep(60)"])
                self.assertEqual(len(children), 1)
                self.assertIsNotNone(children[0].poll())
            finally:
                for child in children:
                    if child.poll() is None:
                        child.terminate()
                    child.wait(timeout=3)
                controller.close()

    def test_old_query_nonce_is_not_accepted_as_new_ready(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-oldq-") as directory:
            config = {"db": str(Path(directory) / "owned"), "startup_timeout_ms": "30",
                      "session_id": "test", "session_generation": "1", "port": "12345"}
            controller = control.Controller(config, Path(directory))
            try:
                record = {"state": "running", "pid": 123456, "process_identity": "current",
                          "control_instance_id": "new"}
                body = {"ok": True, "cut": {"session_id": "test", "session_generation": "1"},
                        "producer_epoch": "1", "control_instance_id": "old"}
                with patch.object(controller, "status", return_value=record), \
                     patch.object(control, "read_health", return_value=body):
                    with self.assertRaises(TimeoutError):
                        controller.wait_ready("q", query_epoch="1")
            finally:
                controller.close()

    def test_ready_rechecks_process_after_health_read(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-exitq-") as directory:
            config = {"db": str(Path(directory) / "owned"), "startup_timeout_ms": "30",
                      "session_id": "test", "session_generation": "1", "port": "12345"}
            controller = control.Controller(config, Path(directory))
            try:
                record = {"state": "running", "pid": 123456, "process_identity": "current",
                          "control_instance_id": "new"}
                body = {"ok": True, "cut": {"session_id": "test", "session_generation": "1"},
                        "producer_epoch": "1", "control_instance_id": "new"}
                with patch.object(controller, "status", side_effect=[record, record, {"state": "stopped"}]), \
                     patch.object(control, "read_health", return_value=body):
                    with self.assertRaisesRegex(ValueError, "已失效"):
                        controller.wait_ready("q", query_epoch="1")
            finally:
                controller.close()

    def test_slow_drip_header_cannot_extend_total_health_deadline(self):
        listener = socket.socket()
        listener.bind(("127.0.0.1", 0))
        listener.listen(1)
        port = listener.getsockname()[1]
        def drip():
            with listener:
                connection, _ = listener.accept()
                with connection:
                    connection.recv(4096)
                    for byte in b"HTTP/1.0 200 OK\r\nContent-Length: 2\r\n\r\n{}":
                        try:
                            connection.sendall(bytes([byte]))
                        except OSError:
                            break
                        time.sleep(0.01)
        thread = threading.Thread(target=drip)
        thread.start()
        start = time.monotonic()
        try:
            with self.assertRaises(TimeoutError):
                control.read_health(port, start + 0.04)
            self.assertLess(time.monotonic() - start, 0.25)
        finally:
            thread.join(timeout=2)
        self.assertFalse(thread.is_alive())

    def test_stopping_q_leaves_s_alive_and_preserves_process_record(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-control-") as directory:
            config = {"db": str(Path(directory) / "owned.sqlite"), "startup_timeout_ms": "2000"}
            controller = control.Controller(config, Path(directory))
            processes = []
            try:
                for service in ("s", "q"):
                    processes.append(controller.start_process(
                        service, [sys.executable, "-c", "import time; time.sleep(60)", service]))
                before = controller.record_path("s").read_bytes()
                result = controller.stop("q", signal.SIGTERM)
                self.assertEqual(result["state"], "stopped")
                self.assertEqual(controller.status("s")["state"], "running")
                self.assertEqual(controller.record_path("s").read_bytes(), before)
                self.assertIsNone(processes[0].poll())
                controller.stop("s", signal.SIGTERM)
            finally:
                for process in processes:
                    if process.poll() is None:
                        process.terminate()
                    process.wait(timeout=3)
                controller.close()

    def test_concurrent_control_cannot_overwrite_same_state_directory(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-lock-") as directory:
            first = control.Controller({"db": str(Path(directory) / "owned")}, Path(directory))
            try:
                with self.assertRaisesRegex(ValueError, "并发"):
                    control.Controller({"db": str(Path(directory) / "owned")}, Path(directory))
            finally:
                first.close()

    def test_reused_pid_is_rejected_without_signal(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-pid-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned"), "startup_timeout_ms": "100"}, Path(directory))
            try:
                controller.record_path("s").write_text(json.dumps({
                    "db": controller.config["db"], "service": "s", "pid": 123456,
                    "process_identity": "original start and command", "command": ["owned"], "control_instance_id": "test"}))
                with patch.object(control, "process_identity", return_value="different process"), \
                     patch.object(control.os, "kill") as kill:
                    with self.assertRaisesRegex(ValueError, "PID"):
                        controller.stop("s", signal.SIGKILL)
                    kill.assert_not_called()
            finally:
                controller.close()

    def test_identity_changed_between_read_and_signal_is_rejected(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-signal-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned"), "startup_timeout_ms": "100"}, Path(directory))
            try:
                controller.record_path("s").write_text(json.dumps({
                    "db": controller.config["db"], "service": "s", "pid": 123456, "process_identity": "original", "command": ["owned"], "control_instance_id": "test"}))
                with patch.object(control, "process_identity", side_effect=["original", "new"]), \
                     patch.object(control.os, "kill") as kill:
                    with self.assertRaisesRegex(ValueError, "身份发生变化"):
                        controller.stop("s", signal.SIGTERM)
                    kill.assert_not_called()
            finally:
                controller.close()

    def test_record_from_another_database_cannot_be_controlled(self):
        with tempfile.TemporaryDirectory(prefix="tb01c-owner-") as directory:
            controller = control.Controller({"db": str(Path(directory) / "owned")}, Path(directory))
            try:
                controller.record_path("q").write_text(json.dumps({"db": "other", "service": "q", "pid": 123456, "process_identity": "other", "command": ["owned"], "control_instance_id": "test"}))
                with patch.object(control.os, "kill") as kill:
                    with self.assertRaisesRegex(ValueError, "不属于"):
                        controller.stop("q", signal.SIGTERM)
                    kill.assert_not_called()
            finally:
                controller.close()


if __name__ == "__main__":
    unittest.main()
