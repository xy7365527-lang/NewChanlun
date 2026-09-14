"""#1394：查询协议单位负控；合成传输不计正式会话证据。"""
import copy
import unittest
import test_tb03a_query as baseline
from economic_query import EconomicQueryError
from s_socket_client import digest


class FinancialQueryTests(unittest.TestCase):
    tearDown = baseline.EconomicQueryTests.tearDown
    write_config = baseline.EconomicQueryTests.write_config
    view = baseline.EconomicQueryTests.view
    owner = baseline.EconomicQueryTests.owner

    def setUp(self):
        baseline.EconomicQueryTests.setUp(self)
        self.q.manifest["finance"] = {"schema_revision": "tb03b-finance/1"}
        self.decision = {"decision_id": "one", "state": "Prepared"}
        for domain, view in self.states.items():
            view["image"]["finance"] = {"schema_revision": "tb03b-finance/1", "rights": [], "decisions": [], "slots": [], "responsibilities": []}
            if domain == "B:a":
                view["cut"]["commit_seq"] = "3"
                view["image"]["finance"]["decisions"] = [self.decision]
            view["image_hash"] = digest(view["image"])
        self.receipt_cut = self.states["B:a"]["cut"].copy()
        original_owner = self.q.owner

        def owner(domain, payload):
            if payload["op"] == "QueryDecision":
                return {"kind": "EconomicDecision", "domain_id": domain,
                        "decision": copy.deepcopy(self.decision), "receipts": [{"cut": self.receipt_cut}]}
            return original_owner(domain, payload)
        self.q.owner = owner

    def request(self):
        return {"op": "original", "kind": "decision", "domain_id": "B:a", "object_id": "one",
                "cuts": {d: v["cut"] for d, v in self.states.items()}}

    def test_original_is_bound_to_exact_complete_vector(self):
        result = self.q.original(self.request())
        self.assertEqual(result["original"]["decision"], self.decision)
        self.assertEqual(result["cuts"], self.request()["cuts"])

    def test_future_receipt_is_not_backfilled_into_old_cut(self):
        self.receipt_cut["commit_seq"] = "4"
        with self.assertRaisesRegex(EconomicQueryError, "未来回执"):
            self.q.original(self.request())

    def test_declared_finance_cannot_be_silently_dropped(self):
        del self.states["B:a"]["image"]["finance"]
        self.states["B:a"]["image_hash"] = digest(self.states["B:a"]["image"])
        with self.assertRaisesRegex(EconomicQueryError, "图像字段"):
            self.q.original(self.request())

    def test_cross_owner_or_incomplete_vector_is_rejected(self):
        request = self.request()
        del request["cuts"]["E"]
        with self.assertRaises(EconomicQueryError):
            self.q.original(request)
        request = self.request()
        request["kind"] = "grant"
        with self.assertRaises(EconomicQueryError):
            self.q.original(request)
