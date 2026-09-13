"""#1372：本地真实 socket 的丢回执/截断/期限测试，不冒充 S 恢复验收。"""

import importlib.util
import copy
import hashlib
import json
import socket
import tempfile
import threading
import time
import unittest
from unittest.mock import patch
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "s_socket_client", Path(__file__).resolve().parents[1] / "s_socket_client.py")
CLIENT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CLIENT)


class SocketClientTests(unittest.TestCase):
    def test_reply_binds_full_payload_and_original_causal_identity(self):
        digest = lambda value: hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        request = {"schema_revision": "s-session/2", "session_id": "test", "session_generation": "1",
                   "source_namespace": "source", "source_epoch": "1", "message_id": "original", "payload_hash": "original-hash"}
        payload = {"ok": True, "kind": "Committed", "accepted_seq": "9007199254740993"}
        reply = {**request, "source_namespace": "s-session/replies", "source_epoch": "2",
                 "producer_id": "S:test", "producer_epoch": "2", "payload": payload, "payload_hash": digest(payload),
                 "causal_refs": [{key: request[key] for key in ("source_namespace", "source_epoch", "message_id", "payload_hash")}]}
        reply["message_id"] = "reply-" + digest(["source", "1", "original", digest(payload)])
        self.assertEqual(CLIENT.validate_reply(request, reply, producer="S", producer_epoch="2"), payload)
        for path, value in (("session_generation", "2"), ("producer_epoch", "1"), ("payload_hash", "wrong"),
                            ("causal_refs", []), ("message_id", "wrong")):
            with self.subTest(path=path):
                changed = copy.deepcopy(reply)
                changed[path] = value
                with self.assertRaises(ValueError):
                    CLIENT.validate_reply(request, changed, producer="S", producer_epoch="2")
        changed = copy.deepcopy(reply)
        changed["payload"]["accepted_seq"] = "9007199254740992"
        with self.assertRaises(ValueError):
            CLIENT.validate_reply(request, changed, producer="S", producer_epoch="2")

    def roundtrip(self, send, *, timeout_ms=500, limit=4096):
        with tempfile.TemporaryDirectory(prefix="c-sock-", dir="/tmp") as directory:
            path = Path(directory) / "s.sock"
            errors = []
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
                listener.bind(str(path))
                listener.listen(1)

                def server():
                    try:
                        with listener.accept()[0] as peer:
                            peer.settimeout(1)
                            request = bytearray()
                            while True:
                                chunk = peer.recv(4096)
                                if not chunk:
                                    break
                                request.extend(chunk)
                            if request.count(b"\n") != 1 or not request.endswith(b"\n"):
                                raise ValueError("测试对端未收到完整单帧+EOF")
                            send(peer)
                    except (BrokenPipeError, ConnectionResetError):
                        pass  # 客户总期限结束后正常断开。
                    except Exception as exc:
                        errors.append(exc)

                thread = threading.Thread(target=server, daemon=True)
                thread.start()
                result = CLIENT.exchange(path, {"message_id": "original-id"},
                                         timeout_ms=timeout_ms, max_frame_bytes=limit)
                thread.join(2)
                self.assertFalse(thread.is_alive())
                self.assertEqual(errors, [])
                return result

    def test_complete_response_keeps_exact_fields(self):
        result = self.roundtrip(lambda peer: peer.sendall(b'{"seq":"9007199254740993"}\n'))
        self.assertEqual(result, {"transport": "Received", "response": {"seq": "9007199254740993"}})

    def test_closed_connection_after_send_is_unknown(self):
        result = self.roundtrip(lambda peer: None)
        self.assertEqual(result["transport"], "DeliveryUnknown")
        self.assertEqual(result["message_id"], "original-id")

    def test_truncated_duplicate_or_oversized_response_is_unknown(self):
        for raw in (b'{"seq":"1"}', b'{"seq":"1","seq":"2"}\n',
                    b'{"seq":1.0}\n', b'{"x":"' + b'x' * 4096 + b'"}\n'):
            with self.subTest(raw=raw[:80]):
                self.assertEqual(self.roundtrip(lambda peer: peer.sendall(raw))["transport"],
                                 "DeliveryUnknown")

    def test_timeout_is_total_not_per_chunk(self):
        def trickle(peer):
            for byte in b'{"ok":true}\n':
                peer.sendall(bytes([byte]))
                time.sleep(0.02)
        start = time.monotonic()
        result = self.roundtrip(trickle, timeout_ms=55)
        self.assertEqual(result["transport"], "DeliveryUnknown")
        self.assertLess(time.monotonic() - start, 0.5)

    def test_second_frame_in_later_packet_is_rejected(self):
        def two_frames(peer):
            peer.sendall(b'{"first":"1"}\n')
            time.sleep(0.02)
            peer.sendall(b'{"second":"2"}\n')
        self.assertEqual(self.roundtrip(two_frames)["transport"], "DeliveryUnknown")

    def test_response_parsing_is_inside_total_deadline(self):
        decode = CLIENT.decode_frame

        def slow_response(raw):
            if b'response' in raw:
                time.sleep(0.08)
            return decode(raw)

        with patch.object(CLIENT, "decode_frame", side_effect=slow_response):
            result = self.roundtrip(lambda peer: peer.sendall(b'{"response":"1"}\n'), timeout_ms=40)
        self.assertEqual(result["transport"], "DeliveryUnknown")

    def test_connect_failure_does_not_claim_input_was_sent(self):
        with tempfile.TemporaryDirectory(prefix="c-sock-", dir="/tmp") as directory:
            result = CLIENT.exchange(Path(directory) / "absent.sock", {"message_id": "same"},
                                     timeout_ms=100, max_frame_bytes=1024)
            self.assertEqual(result["transport"], "TransportUnavailable")

    def test_invalid_request_never_opens_socket(self):
        with self.assertRaises(ValueError):
            CLIENT.exchange("/no/socket", {"price": 1.2}, timeout_ms=100, max_frame_bytes=1024)


if __name__ == "__main__":
    unittest.main()
