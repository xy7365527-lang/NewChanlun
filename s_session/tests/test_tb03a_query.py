"""#1374：固定经济向量与域隔离的查询合同；真实 owner 轨迹另由运行验收覆盖。"""

import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from economic_query import EconomicQuery, EconomicQueryError
from s_socket_client import canonical, digest


class EconomicQueryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="tb03q-")
        self.root = Path(self.tmp.name)
        source = {"source_namespace": "test-query", "source_epoch": "fixture-1",
                  "producer_id": "reader", "producer_epoch": "1"}
        self.manifest = {"schema_revision": "economic-manifest/1", "session_id": "session-test",
                         "session_generation": "2", "authorized_sources": [{**source, "operations": ["ReadView", "Watch"]}],
                         "e": {"socket": str(self.root / "e.sock")},
                         "chongs": [{"chong_id": "a", "socket": str(self.root / "a.sock")},
                                    {"chong_id": "b", "socket": str(self.root / "b.sock")}]}
        (self.root / "manifest.json").write_bytes(canonical(self.manifest))
        self.config = {"schema_revision": "economic-query/1", "manifest": "manifest.json",
                       "manifest_hash": digest(self.manifest), "query_source": source,
                       "producer_epoch": "1", "expected_epochs": {"E": "1", "B:a": "1", "B:b": "1"},
                       "resources": {"max_frame_bytes": "100000", "timeout_ms": "1000",
                                     "operation_timeout_ms": "4000", "max_reply_bytes": "100000",
                                     "max_cached_bytes": "100000", "max_snapshots": "4", "max_domains": "3",
                                     "max_changes": "10"}}
        self.write_config()
        self.q = EconomicQuery(self.root / "query.json")
        self.states = {d: self.view(d) for d in ("E", "B:a", "B:b")}
        self.calls = []
        self.q.owner = self.owner

    def tearDown(self):
        self.tmp.cleanup()

    def write_config(self):
        (self.root / "query.json").write_bytes(canonical(self.config))

    def view(self, domain, seq="0"):
        image = {"book": None, "recorded": [], "pending": [], "applied": [], "unmatched": [], "history": []}
        return {"kind": "EconomicView", "domain_id": domain,
                "commit_ns": str(int(seq) * 10),
                "cut": {"domain_id": domain, "commit_seq": seq, "root_hash": digest([domain, seq])},
                "image": image, "image_hash": digest(image)}

    def owner(self, domain, payload):
        self.calls.append((domain, copy.deepcopy(payload)))
        value = self.states[domain]
        if isinstance(value, Exception):
            raise value
        return copy.deepcopy(value)

    def snapshot(self, **fields):
        return self.q.snapshot({"op": "snapshot", "cuts": None, "as_known_ns": None,
                                "snapshot_token": None, **fields})

    def test_bad_book_keeps_healthy_domains_readable(self):
        self.states["B:a"]["image_hash"] = "0" * 64
        result = self.snapshot()
        self.assertEqual(result["completeness"], "PartialVector")
        self.assertEqual(result["domains"]["B:a"]["status"], "Unavailable")
        self.assertEqual(result["domains"]["E"]["status"], "Available")
        self.assertEqual(result["domains"]["B:b"]["status"], "Available")

    def test_cached_vector_keeps_exact_old_bytes_after_new_read(self):
        old = self.snapshot()
        self.states["B:a"] = self.view("B:a", "1")
        self.snapshot()
        self.assertEqual(self.snapshot(snapshot_token=old["snapshot_token"]), old)
        self.assertEqual(len(self.calls), 6)

    def test_missing_historical_cut_does_not_fall_back_to_latest(self):
        old = self.snapshot()
        cuts = {d: v["cut"] for d, v in old["domains"].items()}
        self.states["B:b"] = self.view("B:b", "3")
        result = self.snapshot(cuts=cuts)
        self.assertEqual(result["mode"], "AsKnown")
        self.assertEqual(result["domains"]["B:b"]["status"], "Unavailable")
        self.assertNotIn("image", result["domains"]["B:b"])

    def test_complete_vector_and_exact_integer_required_before_owner_calls(self):
        for fields in ({"cuts": {"E": self.states["E"]["cut"]}},
                       {"as_known_ns": "01"}, {"as_known_ns": "9223372036854775808"},
                       {"as_known_ns": 1}, {"as_known_ns": True}):
            with self.subTest(fields=fields), self.assertRaises(EconomicQueryError):
                self.snapshot(**fields)
        self.assertEqual(self.calls, [])

    def test_as_known_is_forwarded_without_backfill(self):
        self.snapshot(as_known_ns="9007199254740993")
        self.assertTrue(all(p["as_known_ns"] == "9007199254740993" and p["cut"] is None for _, p in self.calls))

    def test_evicted_token_is_gap_not_current(self):
        self.q.resources["max_snapshots"] = 1
        old = self.snapshot()
        self.states["B:a"] = self.view("B:a", "1")
        self.snapshot()
        self.assertEqual(self.snapshot(snapshot_token=old["snapshot_token"])["kind"], "Gap")
        self.assertEqual(len(self.calls), 6)

    def test_manifest_hash_and_owner_epoch_are_independent_configuration(self):
        for change in ({"manifest_hash": "0" * 64}, {"expected_epochs": {"E": "1"}},
                       {"query_source": {**self.config["query_source"], "producer_id": "unapproved"}}):
            config = copy.deepcopy(self.config)
            self.config.update(change)
            self.write_config()
            with self.assertRaises(EconomicQueryError):
                EconomicQuery(self.root / "query.json")
            self.config = config

    def test_watch_gap_is_domain_unavailable(self):
        old = self.snapshot()
        cuts = {d: v["cut"] for d, v in old["domains"].items()}
        original = self.owner

        def changes(domain, payload):
            if payload["op"] == "Watch":
                end = self.view(domain, "2")
                return {"kind": "EconomicChanges", "domain_id": domain, "cut": end["cut"],
                        "changes": [{"cut": end["cut"], "image": end["image"], "commit_ns": "20"}]}
            return original(domain, payload)

        self.q.owner = changes
        result = self.q.watch({"op": "watch", "cuts": cuts})
        self.assertTrue(all(v["status"] == "Unavailable" for v in result["domains"].values()))


if __name__ == "__main__":
    unittest.main()
