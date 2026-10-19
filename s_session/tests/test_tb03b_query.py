"""#1394：查询协议单位负控；合成传输不计正式会话证据。"""
import copy
import hashlib
import io
import json
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import test_tb03a_query as baseline
import economic_http
from economic_query import EconomicQuery, EconomicQueryError
from s_socket_client import canonical, digest, validate_reply


def canonical_reply(request, payload, owner, namespace, epoch="11"):
    """独立标准库配方构造向量；不代表真实 E/B 会话运行。"""
    def native_hash(value):
        raw = json.dumps(value, ensure_ascii=False, sort_keys=True,
                         separators=(",", ":"), allow_nan=False).encode("utf-8")
        return hashlib.sha256(raw).hexdigest()

    ph = native_hash(payload)
    return {**request, "producer_id": owner + ":" + request["session_id"],
            "producer_epoch": epoch, "source_epoch": epoch, "source_namespace": namespace,
            "message_id": "reply-" + native_hash([
                request["source_namespace"], request["source_epoch"], request["message_id"], ph]),
            "payload": payload, "payload_hash": ph,
            "causal_refs": [{k: request[k] for k in (
                "source_namespace", "source_epoch", "message_id", "payload_hash")}]}


class EconomicV2ReplyTests(unittest.TestCase):
    def request(self, revision="economic-session/2"):
        payload = {"op": "ReadView", "cut": None}
        return {"schema_revision": revision, "session_id": "test", "session_generation": "1",
                "source_namespace": "readonly-query", "source_epoch": "1", "producer_id": "reader",
                "producer_epoch": "1", "message_id": "query-1", "payload": payload,
                "payload_hash": digest(payload), "causal_refs": []}

    def test_v2_e_book_and_q_replies_keep_native_binding(self):
        request = self.request()
        for owner in ("E", "B:opaque", "Q"):
            namespace = "economic-observe/replies" if owner == "Q" else "economic-session/replies"
            payload = {"kind": "EconomicView", "units": "9007199254740993"}
            reply = canonical_reply(request, payload, owner, namespace)
            with self.subTest(owner=owner):
                self.assertEqual(validate_reply(request, reply, producer=owner, producer_epoch="11"), payload)

    def test_unknown_revision_cannot_be_relabelled_as_structure_reply(self):
        request = self.request("economic-session/99")
        reply = canonical_reply(request, {"kind": "EconomicView"}, "S", "s-session/replies")
        with self.assertRaises(ValueError):
            validate_reply(request, reply, producer="S", producer_epoch="11")

    def test_v2_reply_rejects_old_domain_and_resigned_binding_tampering(self):
        request = self.request()
        reply = canonical_reply(request, {"kind": "EconomicView"}, "B:opaque", "economic-session/replies")
        for key, value in (("schema_revision", "economic-session/1"),
                           ("source_namespace", "s-session/replies"),
                           ("producer_id", "B:other:test"), ("producer_epoch", "10"),
                           ("causal_refs", []), ("payload_hash", "0" * 64)):
            changed = {**reply, key: value}
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate_reply(request, changed, producer="B:opaque", producer_epoch="11")


class EconomicV2QueryTests(unittest.TestCase):
    tearDown = baseline.EconomicQueryTests.tearDown
    write_config = baseline.EconomicQueryTests.write_config
    view = baseline.EconomicQueryTests.view
    owner = baseline.EconomicQueryTests.owner

    def setUp(self):
        baseline.EconomicQueryTests.setUp(self)
        self.manifest["protocol_revision"] = "economic-session/2"
        self.manifest["finance"] = {"schema_revision": "tb03b-finance/2"}
        self.q = self.load(self.manifest)
        for view in self.states.values():
            view["image"]["finance"] = {"schema_revision": "tb03b-finance/2", "rights": [],
                                      "decisions": [], "slots": [], "responsibilities": []}
            view["image_hash"] = digest(view["image"])
        self.requests = []

    def load(self, manifest):
        (self.root / "manifest.json").write_bytes(canonical(manifest))
        self.config["manifest_hash"] = digest(manifest)
        self.write_config()
        return EconomicQuery(self.root / "query.json")

    def peer(self, socket, request, **bounds):
        self.requests.append(copy.deepcopy(request))
        domain = next(domain for domain, path in self.q.nodes.items() if path == socket)
        return {"transport": "Received", "response": canonical_reply(
            request, copy.deepcopy(self.states[domain]), domain, "economic-session/replies", epoch="1")}

    def test_explicit_v2_manifest_reads_v2_images_and_keeps_snapshot_binding(self):
        self.assertEqual(self.q.request({"op": "ReadView"})["schema_revision"], "economic-session/2")
        with patch("economic_query.exchange", side_effect=self.peer):
            result = self.q.snapshot({"op": "snapshot", "cuts": None, "as_known_ns": None,
                                      "snapshot_token": None})
        self.assertEqual(result["completeness"], "CompleteVector")
        self.assertEqual(result["manifest_hash"], digest(self.manifest))
        self.assertEqual({request["schema_revision"] for request in self.requests}, {"economic-session/2"})
        self.assertEqual(result["domains"]["E"]["image"]["finance"]["schema_revision"], "tb03b-finance/2")

    def test_unknown_or_mixed_manifest_versions_reject_before_owner_io(self):
        candidates = []
        for protocol, finance in (("economic-session/1", "tb03b-finance/2"),
                                  ("economic-session/2", "tb03b-finance/1"),
                                  ("economic-session/99", "tb03b-finance/2"),
                                  (False, "tb03b-finance/2"),
                                  ("economic-session/2", "tb03b-finance/99")):
            manifest = copy.deepcopy(self.manifest)
            manifest["protocol_revision"] = protocol
            manifest["finance"]["schema_revision"] = finance
            candidates.append(manifest)
        for key in ("protocol_revision", "finance"):
            manifest = copy.deepcopy(self.manifest)
            del manifest[key]
            candidates.append(manifest)
        for manifest in candidates:
            with self.subTest(manifest=manifest), patch("economic_query.exchange") as transport:
                with self.assertRaises(EconomicQueryError):
                    self.load(manifest)
                transport.assert_not_called()

    def test_http_accepts_only_the_frozen_domain_revision(self):
        payload = {"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None}
        request = self.q.request(payload)
        self.assertEqual(economic_http.validate_request(request, self.q), payload)
        for revision in ("economic-session/1", "economic-session/99", "s-session/3"):
            bad = {**request, "schema_revision": revision}
            with self.subTest(revision=revision), self.assertRaises(EconomicQueryError):
                economic_http.validate_request(bad, self.q)

    def test_ready_keeps_discovery_shape_and_reports_selected_finance_revision(self):
        sent = []
        handler = SimpleNamespace(path="/api/economic/ready", server=SimpleNamespace(economic_query=self.q),
                                  _send_json=lambda value, status=200: sent.append((value, status)))
        self.assertTrue(economic_http.get(handler))
        value, status = sent[0]
        self.assertEqual(status, 200)
        self.assertEqual(value["schema_revision"], "economic-discovery/1")
        self.assertEqual(value["finance_revision"], "tb03b-finance/2")
        self.assertEqual(set(value), {"schema_revision", "session_id", "session_generation", "producer_epoch",
                                     "manifest_hash", "domains", "max_reply_bytes", "operation_timeout_ms",
                                     "scope", "configured", "finance_revision"})

    def test_http_v2_snapshot_uses_native_q_reply_binding(self):
        payload = {"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None}
        request = self.q.request(payload)
        raw = canonical(request)
        sent = []
        handler = SimpleNamespace(
            path="/api/economic/snapshot", rfile=io.BytesIO(raw),
            headers=SimpleNamespace(get_all=lambda name, default: [str(len(raw))], get=lambda name: None),
            connection=SimpleNamespace(settimeout=lambda seconds: None),
            server=SimpleNamespace(economic_query=self.q, resources={
                "max_frame_bytes": self.q.resources["max_frame_bytes"],
                "max_reply_bytes": self.q.resources["max_reply_bytes"],
                "socket_read_timeout_ms": self.q.resources["timeout_ms"]}),
            _send_json=lambda value, status=200: sent.append((value, status)),
            _send_bytes=lambda value, status=200: sent.append((json.loads(value), status)))
        with patch("economic_query.exchange", side_effect=self.peer):
            self.assertTrue(economic_http.post(handler))
        response, status = sent[0]
        self.assertEqual(status, 200)
        result = validate_reply(request, response, producer="Q", producer_epoch="1")
        self.assertEqual(result["completeness"], "CompleteVector")

    def test_old_manifest_selector_and_finance_keep_v1_behavior(self):
        for selector in ("absent", None, "economic-session/1"):
            manifest = copy.deepcopy(self.manifest)
            if selector == "absent":
                del manifest["protocol_revision"]
            else:
                manifest["protocol_revision"] = selector
            manifest["finance"]["schema_revision"] = "tb03b-finance/1"
            with self.subTest(selector=selector):
                query = self.load(manifest)
                request = query.request({"op": "snapshot"})
                self.assertEqual(request["schema_revision"], "economic-session/1")
                self.assertEqual(economic_http.validate_request(request, query), {"op": "snapshot"})

    def test_loaded_revision_is_frozen_when_manifest_file_changes(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["protocol_revision"] = "economic-session/1"
        manifest["finance"]["schema_revision"] = "tb03b-finance/1"
        fresh = self.load(manifest)
        self.assertEqual(self.q.request({"op": "ReadView"})["schema_revision"], "economic-session/2")
        self.assertEqual(fresh.request({"op": "ReadView"})["schema_revision"], "economic-session/1")

    def test_v2_rejects_correctly_hashed_v1_image_or_reply(self):
        self.states["E"]["image"]["finance"]["schema_revision"] = "tb03b-finance/1"
        self.states["E"]["image_hash"] = digest(self.states["E"]["image"])
        with patch("economic_query.exchange", side_effect=self.peer), self.assertRaises(EconomicQueryError):
            self.q.view("E")
        self.states["E"]["image"]["finance"]["schema_revision"] = "tb03b-finance/2"
        self.states["E"]["image_hash"] = digest(self.states["E"]["image"])

        def old_reply(socket, request, **bounds):
            result = self.peer(socket, request, **bounds)
            result["response"]["schema_revision"] = "economic-session/1"
            return result
        with patch("economic_query.exchange", side_effect=old_reply), self.assertRaises(ValueError):
            self.q.view("E")

    def test_v2_does_not_borrow_a_writer_identity_for_readonly_q(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["authorized_sources"][0]["operations"].append("PrepareDecision")
        with self.assertRaisesRegex(EconomicQueryError, "只读"):
            self.load(manifest)


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
