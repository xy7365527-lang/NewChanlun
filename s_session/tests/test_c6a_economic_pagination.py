"""#1406：分页 Q 的纯内存合同锁，不启动 HTTP、socket 或产品进程。"""
from collections import OrderedDict
from pathlib import Path
import sys
import threading
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from economic_query import EconomicQuery, EconomicQueryError
from s_socket_client import canonical, digest


def query():
    q = EconomicQuery.__new__(EconomicQuery)
    q.resources = {"max_cached_bytes": 4194304, "max_snapshots": 4,
                   "max_reply_bytes": 1048576, "max_frame_bytes": 1048576,
                   "operation_timeout_ms": 30000, "timeout_ms": 30000}
    q.cache = OrderedDict()
    q.transfers = {}
    q.open_records = {}
    q.metadata_bytes = 0
    q.cached_bytes = 0
    q.lock = threading.Lock()
    q.budget = threading.local()
    q._start()
    return q


class CacheBudgetTests(unittest.TestCase):
    def test_full_body_accounting_and_original_token(self):
        q = query()
        body = {"kind": "EconomicSnapshot", "data": "x" * 1100000}
        result = q._store(body, paged=True)
        self.assertEqual(result["snapshot_token"], "economic-snapshot-" + digest(body))
        self.assertEqual(q.cached_bytes, len(canonical(result)))
        self.assertEqual(q.cache[result["snapshot_token"]], canonical(result))
        with self.assertRaises(EconomicQueryError):
            q._store(body)

    def test_shared_count_pins_and_failed_admission_atomic(self):
        q = query()
        for i in range(4):
            key = str(i)
            q._cache_put_locked(key, b"a")
            q.transfers[key] = {"cache_key": key, "temporary": bool(i % 2),
                                "deadline": time.monotonic() + 30}
        before = dict(q.cache)
        with self.assertRaises(EconomicQueryError):
            q._cache_put_locked("fifth", b"a")
        self.assertEqual(dict(q.cache), before)
        self.assertEqual(q.cached_bytes, 4)
        q._drop_transfer_locked("1")
        self.assertEqual(q.cached_bytes, 3)
        q._cache_put_locked("fifth", b"a")
        self.assertEqual(q.cached_bytes, 4)

    def test_byte_limit_and_no_partial_eviction_on_pinned_failure(self):
        q = query()
        q.resources["max_cached_bytes"] = 10
        q._cache_put_locked("unpinned", b"12")
        q._cache_put_locked("pinned", b"12345678")
        q.transfers["t"] = {"cache_key": "pinned", "temporary": False,
                             "deadline": time.monotonic() + 30}
        with self.assertRaises(EconomicQueryError):
            q._cache_put_locked("new", b"123")
        self.assertEqual(q.cached_bytes, 10)
        self.assertIn("unpinned", q.cache)
        q._drop_transfer_locked("t")
        q._cache_put_locked("new", b"123")
        self.assertEqual(q.cached_bytes, 3)

    def test_expiry_frees_watch_and_retains_snapshot(self):
        q = query()
        for key, temporary in (("watch", True), ("snapshot", False)):
            q._cache_put_locked(key, b"123")
            q.transfers[key] = {"cache_key": key, "temporary": temporary,
                                "deadline": time.monotonic() - 1}
        q._expire_transfers_locked()
        self.assertEqual(q.cached_bytes, 3)
        self.assertEqual(list(q.cache), ["snapshot"])
        self.assertEqual(q.transfers, {})

    def test_old_fixed_token_survives_watch_close(self):
        q = query()
        q.session, q.generation = "test", "1"
        fixed = q._store({"kind": "EconomicSnapshot", "mode": "AsKnown"}, paged=True)
        q._cache_put_locked("watch", canonical({"kind": "EconomicWatch"}))
        q.transfers["t"] = {"cache_key": "watch", "temporary": True,
                             "deadline": time.monotonic() + 30}
        q._drop_transfer_locked("t")
        self.assertEqual(q.snapshot({"op": "snapshot", "cuts": None, "as_known_ns": None,
                                     "snapshot_token": fixed["snapshot_token"]}, paged=True), fixed)
        self.assertEqual(q.cached_bytes, len(canonical(fixed)))

    def test_exact_aggregate_limit_and_one_byte_over(self):
        q = query()
        q.resources["max_cached_bytes"] = 8
        q._cache_put_locked("exact", b"12345678")
        with self.assertRaises(EconomicQueryError):
            q._cache_put_locked("large", b"123456789")
        self.assertEqual(q.cached_bytes, 8)

    def test_deadline_is_shared_and_not_reset_by_nested_start(self):
        q = query()
        with patch("economic_query.time.monotonic", return_value=10):
            q._start(15)
            self.assertEqual(q.budget.deadline, 15)
            self.assertEqual(q._remaining_ms(), 5000)
        with patch("economic_query.time.monotonic", return_value=15):
            with self.assertRaises(EconomicQueryError):
                q._start(15)


class PaginationTests(unittest.TestCase):
    def setUp(self):
        self.q = query()
        q = self.q
        q.session, q.generation, q.query_epoch = "test-session", "1", "2"
        q.manifest_hash = "a" * 64
        q.nodes, q.epochs = {"E": "unused"}, {"E": "1"}
        self.cut = {"domain_id": "E", "commit_seq": "1", "root_hash": "b" * 64}
        image = {"raw": "中文\\\"" * 60000}
        self.view = {"status": "Available", "cut": self.cut, "commit_ns": "10", "image": image, "image_hash": digest(image)}
        q.view = lambda *args: self.view
        self.counter = 0

    def call(self, payload, *, message=None, identity=None):
        self.counter += 1
        request = {"session_id": self.q.session, "session_generation": self.q.generation,
                   "source_namespace": "client", "source_epoch": "1", "producer_id": "reader", "producer_epoch": "1",
                   "message_id": message or str(self.counter), "payload_hash": digest(payload)}
        request.update(identity or {})
        return self.q.read(payload, request)

    def open(self, logical=None, **kw):
        return self.call({"schema_revision": "economic-read-pages/1", "op": "EconomicReadOpen",
                          "query": logical or {"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None},
                          "requested_remaining_ms": "30000"}, **kw)

    def page(self, opened, offset="0", **kw):
        return self.call({"schema_revision": "economic-read-pages/1", "op": "EconomicReadPage",
                          "transfer_id": opened["descriptor"]["transfer_id"],
                          "descriptor_sha256": opened["descriptor_sha256"], "offset": offset,
                          "requested_remaining_ms": "30000"}, **kw)

    def close(self, opened, **kw):
        return self.call({"schema_revision": "economic-read-pages/1", "op": "EconomicReadClose",
                          "transfer_id": opened["descriptor"]["transfer_id"],
                          "descriptor_sha256": opened["descriptor_sha256"]}, **kw)

    def test_multiframe_exact_canonical_and_same_page(self):
        import base64
        import hashlib
        opened = self.open()
        self.assertEqual(opened["kind"], "EconomicReadOpened")
        descriptor = opened["descriptor"]
        self.assertEqual(set(descriptor), {"schema_revision", "transfer_id", "session_id", "session_generation", "manifest_hash", "producer_id", "producer_epoch", "query_sha256", "document_kind", "fixed_cuts", "owner_epochs", "document_bytes", "document_sha256", "chunk_bytes", "page_count"})
        self.assertGreater(int(descriptor["document_bytes"]), 262144)
        chunks = []
        for offset in range(0, int(descriptor["document_bytes"]), 262144):
            page = self.page(opened, str(offset))
            self.assertEqual(page, self.page(opened, str(offset)))
            chunk = base64.b64decode(page["data_base64"], validate=True)
            self.assertEqual(hashlib.sha256(chunk).hexdigest(), page["chunk_sha256"])
            self.assertEqual(len(chunk), int(page["raw_byte_count"]))
            chunks.append(chunk)
        raw = b"".join(chunks)
        self.assertEqual(hashlib.sha256(raw).hexdigest(), descriptor["document_sha256"])
        self.assertIn(raw, self.q.cache.values())
        self.assertEqual(descriptor["fixed_cuts"], {"E": self.cut})
        self.assertEqual(self.close(opened)["kind"], "EconomicReadClosed")
        self.assertEqual(self.close(opened)["kind"], "EconomicReadClosed")
        self.assertEqual(self.page(opened)["code"], "TransferExpired")

    def test_replayed_open_no_new_identity_or_deadline_and_no_resurrection(self):
        opened = self.open(message="same")
        deadline = self.q.transfers[opened["descriptor"]["transfer_id"]]["deadline"]
        self.assertEqual(self.open(message="same"), opened)
        self.assertEqual(self.q.transfers[opened["descriptor"]["transfer_id"]]["deadline"], deadline)
        conflict = self.open({"op": "watch", "cuts": {"E": self.cut}}, message="same")
        self.assertEqual(conflict["code"], "IdentityConflict")
        self.close(opened)
        self.assertEqual(self.open(message="same")["code"], "TransferExpired")
        self.assertGreater(self.q.metadata_bytes, 0)
        self.assertEqual(self.q.metadata_bytes, sum(len(canonical(x)) for x in self.q.open_records.values()))

    def test_wrong_caller_bad_offset_and_expired_original_deadline(self):
        opened = self.open()
        self.assertEqual(self.page(opened, identity={"source_epoch": "2"})["code"], "IdentityConflict")
        self.assertEqual(self.close(opened, identity={"source_epoch": "2"})["code"], "IdentityConflict")
        for offset in ("01", "-1", "1", opened["descriptor"]["document_bytes"]):
            self.assertEqual(self.page(opened, offset)["code"], "InvalidQuery")
        self.q.transfers[opened["descriptor"]["transfer_id"]]["deadline"] = time.monotonic() - 1
        self.assertEqual(self.page(opened)["code"], "TransferExpired")
        self.assertFalse(self.q.transfers)

    def test_partial_vector_freezes_null_without_upgrading(self):
        self.q.view = lambda *args: {"status": "Unavailable", "error": "offline"}
        opened = self.open()
        self.assertEqual(opened["descriptor"]["fixed_cuts"], {"E": None})
        self.q.view = lambda *args: self.view
        import base64
        from s_socket_client import decode_frame
        body = decode_frame(base64.b64decode(self.page(opened)["data_base64"]))
        self.assertEqual(body["completeness"], "PartialVector")
        self.assertEqual(body["domains"]["E"]["status"], "Unavailable")

    def test_metadata_is_charged_and_cannot_be_evicted_for_reopen(self):
        self.q.resources["max_cached_bytes"] = 512
        # Opening metadata fits, descriptor does not; failed Open tombstone stays charged.
        result = self.open(message="failed")
        self.assertEqual(result["code"], "QueryBudgetExceeded")
        self.assertGreater(self.q.metadata_bytes, 0)
        self.assertLessEqual(self.q.metadata_bytes + self.q.cached_bytes, 512)
        self.assertEqual(self.open(message="failed")["code"], "TransferExpired")
        self.assertFalse(self.q.transfers)

    def test_watch_shares_budget_and_close_retains_old_fixed_token(self):
        old = self.open({"op": "snapshot", "cuts": {"E": self.cut}, "as_known_ns": None, "snapshot_token": None})
        import base64
        from s_socket_client import decode_frame
        # Use actual cache bytes: snapshot contains more than one page.
        old_body = decode_frame(next(iter(self.q.cache.values())))
        self.close(old)
        self.q.watch = lambda payload, **kw: {"kind": "EconomicWatch", "session_id": self.q.session,
            "session_generation": self.q.generation, "manifest_hash": self.q.manifest_hash,
            "domains": {"E": {"status": "Available", "base_cut": self.cut, "next_cut": self.cut, "changes": []}}}
        watch = self.open({"op": "watch", "cuts": {"E": self.cut}})
        self.assertEqual(len(self.q.cache), 2)
        self.close(watch)
        self.assertEqual(len(self.q.cache), 1)
        token_read = self.open({"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": old_body["snapshot_token"]})
        self.assertEqual(token_read["descriptor"]["document_sha256"], old["descriptor"]["document_sha256"])
        self.assertEqual(len(self.q.cache), 1)

    def test_open_and_page_delay_use_earliest_deadline(self):
        real_snapshot = self.q.snapshot
        def slow(*args, **kwargs):
            self.q.budget.deadline = time.monotonic() - 1
            return {"kind": "Gap"}
        self.q.snapshot = slow
        self.assertEqual(self.open()["code"], "QueryBudgetExceeded")
        self.assertFalse(self.q.transfers)
        self.q.snapshot = real_snapshot
        opened = self.open()
        transfer = self.q.transfers[opened["descriptor"]["transfer_id"]]
        old_deadline = transfer["deadline"]
        self.assertEqual(self.page(opened)["kind"], "EconomicReadPage")
        self.assertEqual(transfer["deadline"], old_deadline)

if __name__ == "__main__":
    unittest.main()
