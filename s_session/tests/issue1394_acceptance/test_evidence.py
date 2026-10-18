import copy
from dataclasses import replace
from pathlib import Path
import sqlite3
import tempfile
import time
import unittest
import driver as d
import scheduler as s
import evidence as e


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(dir="/tmp", prefix="e1394-")
        self.root = Path(self.tmp.name)
        self.budget = d.ReadBudget(time.monotonic()+4, 1000, 1000000)
        self.snapshots, self.exchanges = {}, []
        for domain, ids in (("B:alpha", ("a1", "a2")), ("B:beta", ("b1",))):
            db = self.root / (domain.replace(":", "_") + ".sqlite")
            requests, replies = {}, {}
            for ident in ids:
                payload = {"op": "PrepareDecision", "decision_id": ident}
                request = {"schema_revision": "economic-session/2", "session_id": "UNIT_TEST_ONLY",
                           "session_generation": "1", "payload": payload, "payload_hash": d.sha(d.canonical(payload))}
                cut = {"domain_id": domain, "commit_seq": str(len(requests)+1), "root": "toy"}
                reply = {"kind": "PreparedReceipt", "domain_id": domain, "receipt_key": ident, "cut": cut}
                req, rep = self.original(ident+"-request", request), self.original(ident+"-reply", reply)
                binding = s.Binding("UNIT_TEST_ONLY", "a"*64, "b"*64, "c"*64, "d"*64, "e"*64,
                    "f"*64, str(db), domain, domain, 1, s.digest(cut), req.sha256, ident, "PrepareDecision")
                self.exchanges.append(e.Exchange(s.Request(binding, "PreparedReceipt"), req, rep))
                requests[ident], replies[ident] = request, reply
            with sqlite3.connect(db) as connection:
                connection.executescript("CREATE TABLE meta(k TEXT PRIMARY KEY,v TEXT); CREATE TABLE commits(seq INTEGER,root TEXT,commit_ns TEXT,image TEXT); CREATE TABLE receipts(k TEXT PRIMARY KEY,json TEXT); CREATE TABLE messages(k TEXT PRIMARY KEY,request_json TEXT);")
                connection.execute("INSERT INTO commits VALUES(1,'toy','1','{}')")
                connection.executemany("INSERT INTO receipts VALUES(?,?)", [(k,d.canonical(v).decode()) for k,v in replies.items()])
                connection.executemany("INSERT INTO messages VALUES(?,?)", [(k,d.canonical(v).decode()) for k,v in requests.items()])
            # 此处是原件边界的显式 toy 规范化输入，不声称已实现真实 message-key 映射。
            connection.close()
            raw = d.read_domain(db, budget=self.budget)
            self.snapshots[domain] = {**raw, "db": str(db), "owner": domain, "epoch": 1}
        self.expected = tuple(x.request for x in self.exchanges)

    def tearDown(self):
        self.tmp.cleanup()

    def original(self, name, obj):
        path = self.root / name
        path.write_bytes(d.canonical(obj))
        return e.Original(path, d.sha(path.read_bytes()))

    def verify(self, exchanges=None):
        return e.verify_exchanges(self.expected, self.exchanges if exchanges is None else exchanges, self.budget,
                session_id="UNIT_TEST_ONLY", session_generation="1", domain_snapshots=self.snapshots)

    def test_originals_from_temporary_sqlite_multi_owner_decision(self):
        rows = self.verify()
        self.assertEqual(len(rows), 3)
        self.assertTrue(all(r.independent_verdict == "ORIGINALS_VERIFIED" for r in rows))

    def test_missing_extra_reordered_and_wrong_hash(self):
        for rows in (self.exchanges[:-1], self.exchanges+self.exchanges[:1], self.exchanges[::-1]):
            with self.assertRaises(d.Rejected): self.verify(rows)
        self.exchanges[0].request_original.path.write_bytes(b"{}")
        with self.assertRaises(d.Rejected): self.verify()

    def test_received_wrong_cut_and_wrong_owner(self):
        for field, value in (("kind", "Received"), ("cut", {}), ("domain_id", "B:other")):
            first = self.exchanges[0]
            reply = d.strict(first.reply_original.path.read_bytes())
            reply[field] = value
            bad = replace(first, reply_original=self.original("bad-"+field, reply))
            with self.assertRaises(d.Rejected): self.verify([bad, *self.exchanges[1:]])

    def test_extra_persistent_original_rejected(self):
        self.snapshots["B:alpha"]["requests"]["extra"] = {}
        with self.assertRaises(d.Rejected): self.verify()

    def test_idempotent_occurrence_order_and_original_bytes(self):
        duplicate = replace(self.exchanges[0], request=replace(self.expected[0], ordinal=1))
        self.exchanges.append(duplicate)
        self.expected += (duplicate.request,)
        self.verify()
        self.exchanges[-1] = replace(duplicate, request=replace(duplicate.request, ordinal=3))
        self.expected = tuple(x.request for x in self.exchanges)
        with self.assertRaises(d.Rejected): self.verify()

    def test_same_cut_queries_use_originals(self):
        q = self.original("query", {"cut": {"seq": "1"}, "image": {"q": "3"}})
        h = self.original("history", {"cut": {"seq": "2"}, "image": {"q": "3"}})
        with self.assertRaises(d.Rejected): e.same_cut(q,h,{"seq":"1"},self.budget)

    def test_recovery_missing_extra_duplicate_and_changed_history(self):
        before = {"requests": {"a": {}}, "receipts": {"a": {"ok": 1}}, "commits": [{"seq":1}]}
        after = {"requests": {"a": {}, "b": {"new":1}}, "receipts": {"a":{"ok":1},"b":{"ok":2}},
                 "commits": [{"seq":1},{"seq":2,"receipt_key":"b"}]}
        e.recovery_exact(before,after,seam="before_commit",pending_key="b",request={"new":1},receipt={"ok":2})
        e.recovery_exact(after,after,seam="after_commit",pending_key="b",request={"new":1},receipt={"ok":2})
        for case in ("missing", "extra", "duplicate", "changed"):
            bad = copy.deepcopy(after)
            if case == "missing": bad["requests"].pop("a")
            if case == "extra": bad["receipts"]["other"] = {}
            if case == "duplicate": bad["commits"].append(bad["commits"][-1])
            if case == "changed": bad["commits"][0]["seq"] = 0
            with self.subTest(case=case), self.assertRaises(d.Rejected):
                e.recovery_exact(before,bad,seam="before_commit",pending_key="b",request={"new":1},receipt={"ok":2})
