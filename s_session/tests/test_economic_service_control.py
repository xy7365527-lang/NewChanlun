"""#1394：launcher 按冻结经济版本探测、输送并核验 Ready；不运行财务核。"""

from contextlib import redirect_stderr, redirect_stdout
import io
import json
from pathlib import Path
import socket
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import economic_service_control as control
from s_socket_client import canonical, decode_frame, digest, reply_envelope


SOURCE = {"source_namespace": "launcher-probe", "source_epoch": "1",
          "producer_id": "launcher", "producer_epoch": "1"}


class OwnerSocket:
    """仅提供具名版本的 ReadView，使用真实 Unix socket 与公共响应封装。"""

    def __init__(self, path, revision, domain, change=None):
        self.path, self.revision, self.domain, self.change = path, revision, domain, change
        self.requests, self.errors = [], []
        self.stopped = threading.Event()
        self.listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.listener.bind(str(path))
        self.listener.listen()
        self.listener.settimeout(0.05)
        self.thread = threading.Thread(target=self.serve)

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *args):
        self.stopped.set()
        self.thread.join(timeout=2)
        self.listener.close()
        self.path.unlink()
        if self.thread.is_alive() or self.errors:
            raise AssertionError("socket 测试服务未正常结束：" + str(self.errors))

    def serve(self):
        while not self.stopped.is_set():
            try:
                connection, _ = self.listener.accept()
            except socket.timeout:
                continue
            try:
                with connection:
                    connection.settimeout(1)
                    raw = bytearray()
                    while chunk := connection.recv(4096):
                        raw.extend(chunk)
                    request = decode_frame(raw)
                    self.requests.append(request)
                    if request["schema_revision"] != self.revision:
                        response = {"kind": "Error", "error": "SchemaUnsupported"}
                    else:
                        image = {"domain_id": self.domain}
                        payload = {"kind": "EconomicView", "domain_id": self.domain,
                                   "writer_epoch": "11", "commit_ns": "1", "image": image,
                                   "image_hash": digest(image),
                                   "cut": {"domain_id": self.domain, "commit_seq": "0", "root_hash": "0" * 64},
                                   "runtime": {"control_instance_id": "nonce-fixture", "pid": "12345"}}
                        response = reply_envelope(
                            request, payload, session_id="session-fixture", session_generation="1",
                            namespace="economic-session/replies", producer_id=self.domain + ":session-fixture",
                            producer_epoch="11")
                        if self.change:
                            self.change(response)
                    connection.sendall(canonical(response) + b"\n")
            except BaseException as exc:
                self.errors.append(exc)
                return


class EconomicLauncherTests(unittest.TestCase):
    def setUp(self):
        # 使用短 socket 路径，避免 macOS 临时目录触及 Unix socket 名称上限。
        self.temporary = tempfile.TemporaryDirectory(prefix="ec-", dir="/tmp")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.manifest = {
            "schema_revision": "economic-manifest/1", "session_id": "session-fixture",
            "session_generation": "1", "e": {"db": str(self.root / "e.db"), "socket": str(self.root / "e.sock")},
            "chongs": [{"chong_id": "opaque", "db": str(self.root / "b.db"), "socket": str(self.root / "b.sock")}],
            "authorized_sources": [{**SOURCE, "operations": ["ReadView"]}],
            "resources": {"max_frame_bytes": "8192"},
        }
        self.config = {"schema_revision": "economic-launcher/1", "manifest": "manifest.json",
                       "manifest_hash": digest(self.manifest), "python": sys.executable,
                       "e_binary": "e-owner", "b_binary": "b-owner", "state_root": str(self.root / "state"),
                       "startup_timeout_ms": "180", "expected_epochs": {"E": "11", "B:opaque": "11"},
                       "probe_source": SOURCE}

    def write_config(self, manifest):
        (self.root / "manifest.json").write_bytes(canonical(manifest))
        self.config["manifest_hash"] = digest(manifest)
        path = self.root / "launcher.json"
        path.write_bytes(canonical(self.config))
        return path

    def v2_manifest(self):
        return {**self.manifest, "protocol_revision": "economic-session/2",
                "finance": {"schema_revision": "tb03b-finance/2"}}

    def controller(self, manifest, domain="E"):
        config, frozen, node = control.read_config(self.write_config(manifest), domain)
        controller = control.EconomicController(config, frozen, node, domain)
        self.addCleanup(controller.close)
        return controller

    def test_legacy_no_finance_and_finance_v1_probe_remain_v1(self):
        manifests = [self.manifest, {**self.manifest, "protocol_revision": None, "finance": None},
                     {**self.manifest, "finance": {"schema_revision": "tb03b-finance/1"}},
                     {**self.manifest, "protocol_revision": "economic-session/1",
                      "finance": {"schema_revision": "tb03b-finance/1"}}]
        for manifest in manifests:
            with self.subTest(manifest=manifest):
                controller = self.controller(manifest)
                request = controller.probe()
                self.assertEqual(request["schema_revision"], "economic-session/1")
                self.assertEqual(request["payload_hash"], digest(request["payload"]))
                controller.close()

    def test_v2_probe_uses_frozen_domain_version(self):
        controller = self.controller(self.v2_manifest())
        request = controller.probe()
        self.assertEqual(request["schema_revision"], "economic-session/2")
        self.assertEqual(request["payload"], {"op": "ReadView", "cut": None, "as_known_ns": None})
        self.assertEqual(request["payload_hash"], digest(request["payload"]))

    def test_unknown_malformed_or_mixed_versions_reject_before_controller_io(self):
        candidates = []
        for finance in ({}, False, [], "tb03b-finance/2", {"schema_revision": False},
                        {"schema_revision": "tb03b-finance/99"}, {"schema_revision": "tb03b-finance/1"}):
            candidates.append({**self.v2_manifest(), "finance": finance})
        for protocol in (False, {}, [], "economic-session/99", "economic-session/1", None):
            candidates.append({**self.v2_manifest(), "protocol_revision": protocol})
        candidates += [self.v2_manifest() | {"finance": None},
                       {k: v for k, v in self.v2_manifest().items() if k != "protocol_revision"}]
        for manifest in candidates:
            with self.subTest(manifest=manifest), self.assertRaises(ValueError):
                control.read_config(self.write_config(manifest), "E")
        self.assertFalse((self.root / "state").exists())

    def ready(self, manifest, domain, change=None):
        controller = self.controller(manifest, domain)
        Path(controller.config["db"]).touch()  # 生命周期测试不初始化财务数据库。
        expected = {"state": "running", "service": "owner", "pid": 12345,
                    "process_identity": "fixture-process", "control_instance_id": "nonce-fixture"}
        revision = manifest.get("protocol_revision") or "economic-session/1"
        try:
            with OwnerSocket(Path(controller.config["socket"]), revision, domain, change) as owner, \
                 patch.object(controller, "runtime_environment", return_value={}), \
                 patch.object(controller, "prepare_socket", return_value=None), \
                 patch.object(controller, "start_process"), \
                 patch.object(controller, "status", return_value=expected):
                result = controller.start_owner("11", "11")
                self.assertEqual(result["state"], "ready")
                self.assertEqual(result["reply"]["schema_revision"], revision)
                self.assertTrue(owner.requests)
                self.assertEqual({r["schema_revision"] for r in owner.requests}, {revision})
                record = decode_frame(controller.record_path("owner").read_bytes())
                self.assertEqual(record["socket_identity"], controller.socket_identity())
                return result
        finally:
            controller.close()

    def test_v1_and_v2_e_and_book_ready_use_real_socket_and_reply_validation(self):
        for manifest in (self.manifest, self.v2_manifest()):
            for domain in ("E", "B:opaque"):
                with self.subTest(revision=manifest.get("protocol_revision"), domain=domain):
                    try:
                        self.ready(manifest, domain)
                    except TimeoutError as exc:
                        self.fail("声明版本的 ReadView 应使 owner Ready：" + str(exc))

    def test_v2_ready_rejects_resigned_runtime_epoch_and_public_binding_changes(self):
        def altered_payload(key, value):
            def change(response):
                if key in ("pid", "control_instance_id"):
                    response["payload"]["runtime"][key] = value
                else:
                    response["payload"][key] = value
                response["payload_hash"] = digest(response["payload"])
                cause = response["causal_refs"][0]
                response["message_id"] = "reply-" + digest([
                    cause["source_namespace"], cause["source_epoch"], cause["message_id"], response["payload_hash"]])
            return change

        changes = [altered_payload("pid", "54321"), altered_payload("control_instance_id", "other-nonce"),
                   altered_payload("writer_epoch", "10"), altered_payload("domain_id", "B:other"),
                   lambda r: r.update(schema_revision="economic-session/1"),
                   lambda r: r.update(producer_epoch="10"), lambda r: r.update(causal_refs=[])]
        for change in changes:
            with self.subTest(change=change), self.assertRaises(TimeoutError):
                self.ready(self.v2_manifest(), "E", change)
            self.assertFalse((self.root / "state" / digest("E") / "owner.process.json").exists())

    def test_send_preserves_matching_original_request_and_rejects_version_mismatch_before_io(self):
        for manifest, expected in ((self.manifest, "economic-session/1"),
                                   (self.v2_manifest(), "economic-session/2")):
            config_path = self.write_config(manifest)
            for revision in ("economic-session/1", "economic-session/2", "economic-session/99"):
                request = {"schema_revision": revision, "session_id": "session-fixture", "session_generation": "1",
                           **SOURCE, "message_id": "original-message", "payload": {"op": "ReadView", "cut": None,
                                                                                        "as_known_ns": None},
                           "causal_refs": []}
                request["payload_hash"] = digest(request["payload"])
                request_path = self.root / "request.json"
                request_path.write_bytes(canonical(request))
                response = reply_envelope(request, {"kind": "EconomicView"}, session_id="session-fixture",
                                          session_generation="1", namespace="economic-session/replies",
                                          producer_id="E:session-fixture", producer_epoch="11")
                with self.subTest(expected=expected, actual=revision), \
                     patch.object(sys, "argv", ["economic_service_control.py", "send", "--config", str(config_path),
                                                "--domain", "E", "--request", str(request_path)]), \
                     patch.object(control, "exchange", return_value={"transport": "Received", "response": response}) as send, \
                     redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()) as errors:
                    status = control.main()
                if revision == expected:
                    self.assertEqual(status, 0)
                    self.assertEqual(send.call_args.args[1], request)
                else:
                    self.assertEqual(status, 1)
                    send.assert_not_called()
                    self.assertIn("版本", json.loads(errors.getvalue())["error"])


if __name__ == "__main__":
    unittest.main()
