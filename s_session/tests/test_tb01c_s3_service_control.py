"""#1394：S3 生命周期选择、冻结资源和 Ready 身份；unit 不冒称真实服务验收。"""

import hashlib
import json
import os
import stat
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import s_service_control as control
from s_socket_client import reply_envelope


class S3LifecycleTests(unittest.TestCase):
    @staticmethod
    def config(directory, revision="s-launcher/3"):
        root = Path(directory)
        resources = {key: str(index + 1) for index, key in enumerate(sorted(control.S_RESOURCE_FIELDS))}
        resources["max_frame_bytes"] = "8192"
        return {"schema_revision": revision, "db": str(root / "owned.sqlite"),
                "socket": str(root / "s.sock"), "binary": "/owned/s_structure_session",
                "python": sys.executable, "catalog": str(root / "catalog.json"),
                "profile": str(root / "profile.json"), "clock_plan": str(root / "clock.json"),
                "browser": str(root / "browser"), "query_resource_config": str(root / "q.json"),
                "session_id": "owned-session", "session_generation": "1",
                "source_namespace": "owned.test", "source_epoch": "1", "producer_id": "owned-launcher",
                "producer_epoch": "1", "writer_epoch": "1", "query_epoch": "1", "port": "12345",
                "delivery_retain_generations": "1", "startup_timeout_ms": "25", "s_resources": resources}

    @staticmethod
    def response(request, nonce="current"):
        return reply_envelope(request, {"kind": "Ready", "control_instance_id": nonce},
                              session_id=request["session_id"], session_generation=request["session_generation"],
                              namespace="s-session/replies", producer_id="S:" + request["session_id"],
                              producer_epoch="1")

    def test_named_configuration_selects_exact_version_and_resources(self):
        with tempfile.TemporaryDirectory(prefix="s3-cfg-") as directory:
            path = Path(directory) / "launcher.json"
            for revision in ("s-launcher/2", "s-launcher/3"):
                config = self.config(directory, revision)
                path.write_text(json.dumps(config))
                self.assertEqual(control.read_config(path)["schema_revision"], revision)
            for revision in (None, "s-launcher/4", "s-launcher/03"):
                config = self.config(directory, revision)
                path.write_text(json.dumps(config))
                with self.assertRaises(ValueError):
                    control.read_config(path)
            for invalid in ({}, {**self.config(directory)["s_resources"], "extra": "1"},
                            {**self.config(directory)["s_resources"], "queue_capacity": "0"},
                            {**self.config(directory)["s_resources"], "max_connections": 1}):
                config = self.config(directory)
                config["s_resources"] = invalid
                path.write_text(json.dumps(config))
                with self.assertRaises(ValueError):
                    control.read_config(path)

    def test_start_s3_init_serve_and_ready_use_one_deadline_and_unchanged_seven_values(self):
        with tempfile.TemporaryDirectory(prefix="s3-select-") as directory:
            config = self.config(directory)
            original = control.canonical(config["s_resources"])
            controller = control.Controller(config, Path(directory) / "state")
            clock = [0.0]
            def environment(deadline):
                self.assertEqual(deadline, 0.025)
                clock[0] = 0.010
                return {"owned": "environment"}
            def initialize(command, **kwargs):
                self.assertEqual(command[-2:], ["--protocol-revision", "s-session/3"])
                self.assertAlmostEqual(kwargs["timeout"], 0.015)
                clock[0] = 0.020
                return control.subprocess.CompletedProcess(command, 0, b'{"initialized":true}', b"")
            try:
                with patch.object(control.time, "monotonic", side_effect=lambda: clock[0]), \
                     patch.object(controller, "runtime_environment", side_effect=environment), \
                     patch.object(controller, "prepare_socket", return_value=None), \
                     patch.object(control.subprocess, "run", side_effect=initialize), \
                     patch.object(controller, "start_process") as start, \
                     patch.object(controller, "wait_ready", return_value={"state": "ready"}) as ready:
                    result = controller.start_s("1")
                command = start.call_args.args[1]
                self.assertEqual(command[command.index("--protocol-revision") + 1], "s-session/3")
                self.assertNotIn("--audit-cache-source-bytes", command)
                identity = start.call_args.kwargs["resource_identity"]
                self.assertEqual(command[command.index("--resource-config") + 1], identity["path"])
                self.assertEqual(start.call_args.kwargs["deadline"], 0.025)
                frozen = Path(identity["path"]).read_bytes()
                self.assertEqual(json.loads(frozen), {"schema_revision": "s-structure-resources/1",
                                                     "s_resources": config["s_resources"]})
                self.assertEqual(hashlib.sha256(frozen).hexdigest(), identity["sha256"])
                self.assertEqual(stat.S_IMODE(Path(identity["path"]).stat().st_mode), 0o600)
                ready.assert_called_once_with("s", "1", deadline=0.025)
                self.assertEqual(result["initialization"], {"initialized": True})
                self.assertEqual(control.canonical(config["s_resources"]), original)
                request = controller.ready_request("1")
                self.assertEqual(request["schema_revision"], "s-session/3")
                self.assertEqual(request["payload"], {"op": "ready"})
                self.assertEqual(request["payload_hash"], hashlib.sha256(control.canonical({"op": "ready"})).hexdigest())
            finally:
                controller.close()

    def test_expired_environment_or_init_never_resets_budget_or_spawns(self):
        for stage in ("environment", "init"):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory(prefix="s3-budget-") as directory:
                controller = control.Controller(self.config(directory), Path(directory) / "state")
                clock = [0.0]
                def environment(deadline):
                    if stage == "environment":
                        clock[0] = deadline
                    return {}
                def initialize(*args, **kwargs):
                    clock[0] = 0.025
                    return control.subprocess.CompletedProcess([], 0, b'{"initialized":true}', b"")
                try:
                    with patch.object(control.time, "monotonic", side_effect=lambda: clock[0]), \
                         patch.object(controller, "runtime_environment", side_effect=environment), \
                         patch.object(controller, "prepare_socket", return_value=None), \
                         patch.object(control.subprocess, "run", side_effect=initialize) as init, \
                         patch.object(controller, "start_process") as start, \
                         patch.object(controller, "wait_ready") as ready:
                        with self.assertRaises(TimeoutError):
                            controller.start_s("1")
                    start.assert_not_called()
                    ready.assert_not_called()
                    if stage == "environment":
                        init.assert_not_called()
                finally:
                    controller.close()

    def test_frozen_resource_file_cannot_be_adopted_changed_or_unbound(self):
        with tempfile.TemporaryDirectory(prefix="s3-freeze-") as directory:
            controller = control.Controller(self.config(directory), Path(directory) / "state")
            deadline = time.monotonic() + 2
            try:
                identity = controller.freeze_s_resources(deadline)
                path = Path(identity["path"])
                command = ["/owned/s_structure_session", "serve", "--protocol-revision", "s-session/3",
                           "--resource-config", str(path)]
                controller.validate_resource_identity(identity, command, deadline)
                with self.assertRaises(ValueError):
                    controller.validate_resource_identity(identity, command[:-1] + [str(path) + "other"])
                original = path.read_bytes()
                path.write_bytes(original + b" ")
                with self.assertRaisesRegex(ValueError, "内容或 hash 改变"):
                    controller.validate_resource_identity(identity, command)
                path.write_bytes(original)
                path.chmod(0o644)
                with self.assertRaisesRegex(ValueError, "权限改变"):
                    controller.validate_resource_identity(identity, command)
                path.chmod(0o600)
                external = Path(directory) / "external.json"
                external.write_bytes(original)
                path.unlink()
                path.symlink_to(external)
                with self.assertRaises(OSError):
                    controller.validate_resource_identity(identity, command)
                self.assertEqual(external.read_bytes(), original)
                nonce = "00000000-0000-0000-0000-000000000001"
                occupied = controller.state_dir / ("s-resources-" + nonce + ".json")
                occupied.write_bytes(b"preserve")
                with patch.object(control.uuid, "uuid4", return_value=nonce):
                    with self.assertRaises(FileExistsError):
                        controller.freeze_s_resources(deadline)
                self.assertEqual(occupied.read_bytes(), b"preserve")
            finally:
                controller.close()

    def test_resource_identity_is_saved_with_pid_nonce_and_exact_argv(self):
        with tempfile.TemporaryDirectory(prefix="s3-record-") as directory:
            config = self.config(directory)
            controller = control.Controller(config, Path(directory) / "state")
            deadline = time.monotonic() + 2
            try:
                identity = controller.freeze_s_resources(deadline)
                command = [config["binary"], "serve", "--protocol-revision", "s-session/3",
                           "--resource-config", identity["path"]]
                with patch.object(control, "process_identity", return_value="owned exact argv"):
                    controller.record("s", 123456, command, "current", resource_identity=identity)
                    record = controller.status("s")
                self.assertEqual(record["resource_identity"], identity)
                self.assertEqual(record["command"], command)
                self.assertEqual(record["control_instance_id"], "current")
                Path(identity["path"]).write_bytes(b"{}")
                with patch.object(control, "process_identity") as process:
                    with self.assertRaises(ValueError):
                        controller.status("s")
                process.assert_not_called()
            finally:
                controller.close()

    def test_s3_ready_validates_payload_nonce_complete_reply_and_live_process(self):
        with tempfile.TemporaryDirectory(prefix="s3-ready-") as directory:
            config = self.config(directory)
            controller = control.Controller(config, Path(directory) / "state")
            request = controller.ready_request("1")
            record = {"state": "running", "service": "s", "pid": 123456,
                      "process_identity": "exact current", "control_instance_id": "current"}
            identity = {"path": config["socket"], "device": 1, "inode": 2, "uid": os.getuid()}
            try:
                response = self.response(request)
                with patch.object(controller, "status", return_value=record), \
                     patch.object(control, "exchange", return_value={"transport": "Received", "response": response}), \
                     patch.object(controller, "socket_identity", return_value=identity):
                    result = controller.wait_ready("s", "1")
                self.assertEqual(result["state"], "ready")
                self.assertEqual(json.loads(controller.record_path("s").read_text())["socket_identity"], identity)
                for field, value in (("schema_revision", "s-session/2"), ("producer_epoch", "2"),
                                     ("producer_id", "S:other"), ("payload_hash", "0" * 64),
                                     ("causal_refs", [])):
                    bad = {**response, field: value}
                    with self.subTest(field=field), \
                         patch.object(controller, "status", return_value=record), \
                         patch.object(control, "exchange", return_value={"transport": "Received", "response": bad}):
                        with self.assertRaises(ValueError):
                            controller.wait_ready("s", "1")
                bad = {**self.response(request, "old"), "control_instance_id": "current"}
                with patch.object(controller, "status", return_value=record), \
                     patch.object(control, "exchange", return_value={"transport": "Received", "response": bad}):
                    with self.assertRaises(TimeoutError):
                        controller.wait_ready("s", "1")
                with patch.object(controller, "status", side_effect=[record, record, {"state": "stopped"}]), \
                     patch.object(control, "exchange", return_value={"transport": "Received", "response": response}):
                    with self.assertRaisesRegex(ValueError, "已失效"):
                        controller.wait_ready("s", "1")
            finally:
                controller.close()

    def test_nonpython_argv_is_exact_and_process_probe_uses_remaining_deadline(self):
        command = ["/owned/s_structure_session", "serve", "--protocol-revision", "s-session/3"]
        for suffix in ("", " --other argument"):
            result = control.subprocess.CompletedProcess([], 0,
                f"{os.getuid()} S Sun Sep 13 16:17:43 2026 {' '.join(command)}{suffix}\n", "")
            with patch.object(control.subprocess, "run", return_value=result) as query:
                if suffix:
                    with self.assertRaisesRegex(ValueError, "完整参数"):
                        control.process_identity(123456, command, deadline=time.monotonic() + 0.025)
                else:
                    self.assertIn(" ".join(command), control.process_identity(123456, command,
                                                                               deadline=time.monotonic() + 0.025))
            self.assertGreater(query.call_args.kwargs["timeout"], 0)
            self.assertLessEqual(query.call_args.kwargs["timeout"], 0.025)

    def test_s3_recover_selects_new_domain_and_never_falls_back(self):
        with tempfile.TemporaryDirectory(prefix="s3-recover-") as directory:
            controller = control.Controller(self.config(directory), Path(directory) / "state")
            try:
                with patch.object(controller, "status", return_value={"state": "stopped"}), \
                     patch.object(controller, "runtime_environment", return_value={}), \
                     patch.object(control.subprocess, "run", return_value=control.subprocess.CompletedProcess(
                         [], 0, b'{"recovered":true}', b"")) as recover:
                    self.assertEqual(controller.recover("2", "recover-owned")["state"], "recovered")
                self.assertEqual(recover.call_args.args[0][-2:], ["--protocol-revision", "s-session/3"])
                with patch.object(controller, "status", return_value={"state": "stopped"}), \
                     patch.object(controller, "runtime_environment", return_value={}), \
                     patch.object(control.subprocess, "run", side_effect=control.subprocess.CalledProcessError(
                         1, ["recover"])) as recover:
                    with self.assertRaises(control.subprocess.CalledProcessError):
                        controller.recover("2", "recover-owned")
                recover.assert_called_once()
            finally:
                controller.close()

    def test_s3_q_rejects_old_resource_version_before_spawn(self):
        with tempfile.TemporaryDirectory(prefix="s3-q-mismatch-") as directory:
            controller = control.Controller(self.config(directory), Path(directory) / "state")
            try:
                with patch.object(controller, "query_resource_revision", return_value="legacy/2"), \
                     patch.object(controller, "runtime_environment") as environment, \
                     patch.object(controller, "start_process") as start:
                    with self.assertRaisesRegex(ValueError, "版本错配"):
                        controller.start_q("1")
                environment.assert_not_called()
                start.assert_not_called()
            finally:
                controller.close()


if __name__ == "__main__":
    unittest.main()
