"""#1406 Q内部证据采集合同锁；只用本测试的临时Unix socket，不启动产品。"""
import json
from pathlib import Path
import socket
import sys
import tempfile
import threading
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from s_socket_client import EconomicWireEvidence, canonical, digest, exchange
from economic_query import EconomicQuery


class OwnerWireTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="c6w-", dir="/tmp")
        self.root = Path(self.temp.name)
        self.evidence = EconomicWireEvidence(self.root / "wire")
        self.request = {"message_id": "test-only", "payload": {"op": "ReadView"}}
        self.deadline = time.monotonic() + 2

    def tearDown(self):
        self.temp.cleanup()

    def scope(self):
        return self.evidence.scope(self.request, self.deadline)

    def socket_exchange(self, response, scope):
        address = self.root / "owner.sock"
        accepted = []
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
            listener.bind(str(address))
            listener.listen(1)
            listener.settimeout(2)
            def serve():
                connection, _ = listener.accept()
                with connection:
                    connection.settimeout(2)
                    raw = bytearray()
                    while chunk := connection.recv(65536):
                        raw.extend(chunk)
                    accepted.append(bytes(raw))
                    connection.sendall(response)
            worker = threading.Thread(target=serve)
            worker.start()
            try:
                result = exchange(address, self.request, timeout_ms=1000, max_frame_bytes=1048576,
                                  evidence=scope, logical_deadline=self.deadline)
            finally:
                worker.join(2)
            self.assertFalse(worker.is_alive())
        self.assertEqual(accepted, [canonical(self.request) + b"\n"])
        return result

    def test_actual_socket_exact_bytes_and_closed_account(self):
        scope = self.scope()
        raw = b' { "answer" : "fixture" }\n'
        self.assertEqual(self.socket_exchange(raw, scope), {"transport": "Received", "response": {"answer": "fixture"}})
        scope.close(None)
        self.assertEqual((scope.root / "rpc-0001-request.bin").read_bytes(), canonical(self.request) + b"\n")
        self.assertEqual((scope.root / "rpc-0001-response.bin").read_bytes(), raw)
        end = json.loads((scope.root / "end.json").read_bytes())
        self.assertEqual(end["rpc_count"], 1)
        self.assertEqual(end["wire_bytes"], len(raw) + len(canonical(self.request)) + 1)
        self.assertLess(end["observed_at"], self.deadline)

    def test_truncated_response_retains_original_and_failure(self):
        scope = self.scope()
        self.assertEqual(self.socket_exchange(b'{"partial":true}', scope)["transport"], "DeliveryUnknown")
        scope.close(None)
        self.assertEqual((scope.root / "rpc-0001-response.bin").read_bytes(), b'{"partial":true}')
        end = json.loads((scope.root / "rpc-0001-end.json").read_bytes())
        self.assertEqual((end["transport"], end["error_class"], end["started_send"]),
                         ("DeliveryUnknown", "ValueError", True))

    def test_connect_failure_is_zero_response_with_original_request(self):
        scope = self.scope()
        result = exchange(self.root / "absent.sock", self.request, timeout_ms=1000,
                          max_frame_bytes=1048576, evidence=scope, logical_deadline=self.deadline)
        self.assertEqual(result["transport"], "TransportUnavailable")
        scope.close(None)
        self.assertEqual((scope.root / "rpc-0001-response.bin").read_bytes(), b"")
        end = json.loads((scope.root / "rpc-0001-end.json").read_bytes())
        self.assertFalse(end["started_send"])

    def test_repeated_public_request_has_distinct_immutable_attempts(self):
        first = self.scope()
        first.close(None)
        before = {p.name: p.read_bytes() for p in first.root.iterdir()}
        second = self.scope()
        second.close(None)
        self.assertNotEqual(first.root, second.root)
        self.assertEqual({p.name: p.read_bytes() for p in first.root.iterdir()}, before)
        self.assertEqual((first.attempt, second.attempt), (1, 2))
        self.assertEqual((first.root / "public-request.json").read_bytes(),
                         (second.root / "public-request.json").read_bytes())

    def test_capture_enabled_open_replay_keeps_descriptor_and_original_deadline(self):
        from test_c6a_economic_pagination import PaginationTests
        fixture = PaginationTests()
        fixture.setUp()
        q = fixture.q
        q.wire_evidence = self.evidence
        calls = []
        q.view = lambda *args: (calls.append(args), fixture.view)[1]
        payload = {"schema_revision": "economic-read-pages/1", "op": "EconomicReadOpen",
                   "query": {"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None},
                   "requested_remaining_ms": "30000"}
        request = {"schema_revision": "economic-session/2", "session_id": q.session,
            "session_generation": q.generation, "source_namespace": "client", "source_epoch": "1",
            "producer_id": "reader", "producer_epoch": "1", "message_id": "same-open",
            "payload_hash": digest(payload), "causal_refs": [], "payload": payload}
        with q.capture(request, self.deadline):
            first = q.read(payload, request, deadline=self.deadline)
        self.assertEqual(first["kind"], "EconomicReadOpened")
        original_deadline = q.transfers[first["descriptor"]["transfer_id"]]["deadline"]
        first_calls = len(calls)
        with q.capture(request, self.deadline):
            second = q.read(payload, request, deadline=self.deadline)
        self.assertEqual(second, first)
        self.assertEqual(q.transfers[first["descriptor"]["transfer_id"]]["deadline"], original_deadline)
        self.assertEqual(len(calls), first_calls)
        ends = [json.loads((p / "end.json").read_bytes()) for p in sorted(self.evidence.root.iterdir())]
        self.assertEqual([r["attempt"] for r in ends], [1, 2])
        self.assertEqual(ends[1]["rpc_count"], 0)
        self.assertTrue(all(r["error_class"] is None for r in ends))

    def test_concurrent_identical_requests_do_not_collide(self):
        barrier = threading.Barrier(3)
        scopes, errors = [], []
        def run():
            try:
                barrier.wait()
                scope = self.scope()
                scope.close(None)
                scopes.append(scope)
            except BaseException as exc:
                errors.append(exc)
        workers = [threading.Thread(target=run) for _ in range(2)]
        for worker in workers:
            worker.start()
        barrier.wait()
        for worker in workers:
            worker.join(2)
        self.assertEqual(errors, [])
        self.assertEqual(sorted(s.attempt for s in scopes), [1, 2])
        self.assertEqual(len({s.root for s in scopes}), 2)

    def test_parallel_domains_inherit_scope_and_have_unique_sequence(self):
        q = EconomicQuery.__new__(EconomicQuery)
        q.budget = threading.local()
        q.budget.deadline = self.deadline
        q.nodes = {"E": {}, "B:fixture": {}}
        q.wire_evidence = self.evidence
        with q.capture(self.request, self.deadline):
            scope = q.budget.wire_scope
            def run(domain):
                self.assertIs(q.budget.wire_scope, scope)
                return exchange(self.root / (domain.replace(":", "-") + ".sock"), self.request,
                    timeout_ms=1000, max_frame_bytes=1048576, evidence=q.budget.wire_scope,
                    logical_deadline=q.budget.deadline)["transport"]
            self.assertEqual(q._parallel_domains(run), dict.fromkeys(q.nodes, "TransportUnavailable"))
        end = json.loads((scope.root / "end.json").read_bytes())
        self.assertEqual(end["rpc_count"], 2)
        self.assertEqual(len(list(scope.root.glob("rpc-*-begin.json"))), 2)
        self.assertIsNone(q.budget.wire_scope)

    def test_recording_failure_is_not_silently_ignored(self):
        scope = self.scope()
        (scope.root / "rpc-0001-request.bin").write_bytes(b"collision")
        with self.assertRaises(FileExistsError):
            exchange(self.root / "absent.sock", self.request, timeout_ms=1000,
                     max_frame_bytes=1048576, evidence=scope, logical_deadline=self.deadline)

    def test_evidence_failure_preserves_original_business_exception(self):
        from unittest.mock import patch
        q = EconomicQuery.__new__(EconomicQuery)
        q.budget = threading.local()
        q.wire_evidence = self.evidence
        with self.assertRaisesRegex(ValueError, "original business") as caught:
            with q.capture(self.request, self.deadline):
                with patch.object(q.budget.wire_scope, "_json", side_effect=OSError("fixture disk full")):
                    try:
                        raise ValueError("original business")
                    finally:
                        # 故意让证据结束路径失败；原业务异常仍须是主异常。
                        q.budget.wire_scope.close = lambda error: (_ for _ in ()).throw(OSError("fixture disk full"))
        self.assertTrue(any("fixture disk full" in note for note in caught.exception.__notes__))

    def test_public_failure_has_explicit_end_marker(self):
        q = EconomicQuery.__new__(EconomicQuery)
        q.budget = threading.local()
        q.wire_evidence = self.evidence
        with self.assertRaisesRegex(ValueError, "fixture"):
            with q.capture(self.request, self.deadline):
                raise ValueError("fixture")
        end = json.loads((self.evidence.root / "attempt-000001" / "end.json").read_bytes())
        self.assertEqual((end["rpc_count"], end["error_class"]), (0, "ValueError"))


if __name__ == "__main__":
    unittest.main()
