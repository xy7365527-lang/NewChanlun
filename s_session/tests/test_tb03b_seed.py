"""#1394：冻结请求文件的序列形状与精确 JSON 约束。"""
import hashlib
from pathlib import Path
import tempfile
import unittest

from s_socket_client import canonical, decode_frame
from tb03b_runtime import pinned


class PinnedRequestTests(unittest.TestCase):
    def load(self, raw, *, sequence=True, expected_hash=None):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "requests.json"
            path.write_bytes(raw)
            return pinned({}, {"path": str(path), "sha256": expected_hash or hashlib.sha256(raw).hexdigest()},
                          sequence=sequence)

    def test_pinned_business_collections_are_enumerated_as_objects(self):
        frames = [{"message_id": "s-1", "payload": {"op": "ingest", "seq": "0"}}]
        setup = [{"domain": "E", "request": {"payload": {"op": "CaptureExternal"}}},
                 {"domain": "B:alpha", "request": {"payload": {"op": "ApplyAllocation"}}}]
        self.assertEqual(self.load(canonical(frames))[0]["payload"]["op"], "ingest")
        self.assertEqual([x["domain"] for x in self.load(canonical(setup))], ["E", "B:alpha"])

    def test_manifest_and_wire_frame_still_require_nonempty_object(self):
        self.assertEqual(self.load(b'{"schema_revision":"test/1"}', sequence=False),
                         {"schema_revision": "test/1"})
        with self.assertRaises(ValueError):
            self.load(b'[{"a":"1"}]', sequence=False)
        with self.assertRaises(ValueError):
            decode_frame(b'[{"a":"1"}]')

    def test_empty_scalar_and_named_object_containers_are_rejected(self):
        for raw in [b'[]', b'{}', b'{"requests":[{"a":"1"}]}', b'null', b'[{}]', b'[1]', b'[[]]']:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                self.load(raw)

    def test_duplicate_keys_and_inexact_numbers_in_nested_frames_are_rejected(self):
        for raw in [b'[{"payload":{"a":1,"a":2}}]', b'[{"a":1.0}]',
                    b'[{"a":NaN}]', b'[{"a":Infinity}]', b'[{"a":1e2}]']:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                self.load(raw)

    def test_hash_mismatch_is_rejected_before_container_use(self):
        with self.assertRaisesRegex(ValueError, "hash"):
            self.load(b'[{"a":"1"}]', expected_hash="0" * 64)


if __name__ == "__main__":
    unittest.main()
