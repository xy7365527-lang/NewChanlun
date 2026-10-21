"""#1406 C6a：纯数据 Q 合同，替身 owner/HTTP handler；不启动网络或数据库。"""
import copy
import hashlib
import io
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import test_tb03a_query as baseline
import test_tb03b_query as v2
from economic_query import EconomicQuery, EconomicQueryError
import economic_http
from s_socket_client import canonical, digest, validate_reply

HERE = Path(__file__).resolve().parent


def fixture_hash(value):
    # 独立按 Rust session_protocol::canonical_json 的对象排序/数组保序配方。
    encoded = json.dumps(value, ensure_ascii=False, sort_keys=True,
                         separators=(",", ":"), allow_nan=False).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def fixture_record(index, name="RAW.json", raw=None):
    """合成记录，身份内容字段逐项照 service.rs:140–145；不是 owner 回执。"""
    if raw is None:
        raw = (HERE / "fixtures/tb03c_allocation" / name).read_text()
    document = json.loads(raw)
    event = copy.deepcopy(document["events"][index])
    pointer = "/events/" + str(index)
    raw_hash = hashlib.sha256(raw.encode("utf-8")).hexdigest()
    key = fixture_hash([event["source"][k] for k in
                        ("source_namespace", "source_epoch", "external_fact_id")])
    content = fixture_hash({"raw_sha256": raw_hash, "json_pointer": pointer,
                            "event": event, "units": document["units"]})
    # 不补 owner Applied/Resolve 收据。附属 capture 元数据只标识纯数据测试。
    return {"fact_key": key, "fact_hash": content, "raw_sha256": raw_hash,
            "raw_utf8": raw, "json_pointer": pointer, "event": event,
            "token": "synthetic-query-contract", "target_domain": "E",
            "source_event_time": event["source"]["source_event_time"],
            "source_received_time": event["source"]["received_time"],
            "source_producer": {"producer_id": "synthetic", "producer_epoch": "1"},
            "origin_message": {"source_namespace": event["source"]["source_namespace"],
                               "source_epoch": event["source"]["source_epoch"],
                               "message_id": "synthetic-" + str(index), "payload_hash": "0" * 64},
            "origin_request_hash": "0" * 64, "received_ns": "1", "first_known_ns": "1",
            "commit_ns": "1", "recorded_seq": str(index + 1)}


def allocation_fixture(index=3, name="RAW.json", revision="fixture-1"):
    frozen = json.loads((HERE / "fixtures/tb03c_allocation/FROZEN-MANIFEST.json").read_bytes())
    hand = json.loads((HERE / "fixtures/tb03c_allocation/HAND-ORACLE.json").read_bytes())["reports"][index - 3]
    record = fixture_record(index, name)
    event, key = record["event"], record["fact_key"]
    policy = {"fill_allocation_policy_id": "frozen-manifest-proportional-cumulative/2", "policy_revision": revision}
    row = {"fact_key": key, "fact_hash": record["fact_hash"], "manifest_hash": frozen["manifest_hash"],
           "symbol": frozen["symbol"], **policy, "from": str(hand["from"]), "to": str(hand["to"]),
           "evidence_id": "resolve-" + event["event"], "allocations": []}
    # 冻结 HAND 的逐重增量，不在测试/查询里调用 allocator。
    cash = ((0, 200, 100), (0, 0, 101), (102, 102, 102))[index - 3]
    for p, quantity, amount, fee in zip(frozen["participants"], hand["quantity"], cash, hand["fees"]):
        row["allocations"].append({**{k: p[k] for k in ("chong_id", "command_id", "family_id", "revision")},
            "allocation_id": "netted:" + fixture_hash([key, event["allocation_id"], frozen["manifest_hash"], p]),
            "units": str(quantity), "acquisition_cash": str(amount), "commission": str(fee)})
    return frozen, event, policy, row


class BookQueryTests(unittest.TestCase):
    tearDown = baseline.EconomicQueryTests.tearDown
    write_config = baseline.EconomicQueryTests.write_config
    view = baseline.EconomicQueryTests.view
    owner = baseline.EconomicQueryTests.owner
    snapshot = baseline.EconomicQueryTests.snapshot

    def setUp(self):
        baseline.EconomicQueryTests.setUp(self)
        self.frozen, self.event, self.policy, self.row = allocation_fixture()
        self.manifest["book_revision"] = "tb03c-book/1"
        self.manifest["fill_allocation_policy"] = self.policy
        self.manifest["fill_manifests"] = [self.frozen]
        self.manifest["chongs"] = [{"chong_id": p["chong_id"], "symbol": "TEST-N",
                                   "socket": str(self.root / (p["chong_id"] + ".sock"))}
                                  for p in self.frozen["participants"]]
        self.config["resources"]["max_domains"] = "4"
        self.config["expected_epochs"] = {d: "1" for d in ["E", *["B:" + p["chong_id"] for p in self.frozen["participants"]]]}
        self.q = self.load(self.manifest)
        self.states = {d: self.view(d) for d in self.q.nodes}
        self.q.owner = self.owner
        self.install_row()

    def load(self, manifest):
        (self.root / "manifest.json").write_bytes(canonical(manifest))
        self.config["manifest_hash"] = digest(manifest)
        self.write_config()
        return EconomicQuery(self.root / "query.json")

    def install_row(self):
        image = self.states["E"]["image"]
        image["recorded"] = [fixture_record(3)]
        image["history"] = [{"kind": "ResolveEvidence", "evidence_id": self.row["evidence_id"], "covered": [self.row["fact_key"]]}]
        image["fill_allocations"] = [copy.deepcopy(self.row)]
        self.rehash()

    def rehash(self, domain="E"):
        self.states[domain]["image_hash"] = digest(self.states[domain]["image"])

    def install_corrections(self):
        self.policy["policy_revision"] = "we7-frozen-1"
        self.q = self.load(self.manifest)
        self.q.owner = self.owner
        image = self.states["E"]["image"]
        image["recorded"] = [fixture_record(i, "C4B-RAW.json") for i in range(3, 9)]
        image["fill_allocations"] = [allocation_fixture(i, "C4B-RAW.json", "we7-frozen-1")[3]
                                     for i in range(3, 6)]
        hand = json.loads((HERE / "fixtures/tb03c_allocation/C4B-HAND-ORACLE.json").read_bytes())
        effects = [("late_fee_N1", "fee_only", "fee_only", ("7", "7"), 3),
                   ("bust_N3", "acquisition_cash", "commission", ("7", "5"), 5),
                   ("refund_N3_commission", "commission", "commission", ("5", "5"), 5)]
        for index, (h, component, fee_field, interval, original) in enumerate(effects, 6):
            record = image["recorded"][index - 3]
            event, key = record["event"], record["fact_key"]
            original_key = image["recorded"][original - 3]["fact_key"]
            row = {"fact_key": key, "fact_hash": record["fact_hash"],
                   "manifest_hash": self.frozen["manifest_hash"], "symbol": "TEST-N", **self.policy,
                   "from": interval[0], "to": interval[1], "evidence_id": "resolve-" + event["event"],
                   "report_kind": event["netted_fill"]["report_kind"], "component": component,
                   "original_fact_key": original_key, "allocations": []}
            values = hand[h]
            fees = values["fees"] if fee_field == "fee_only" else values["commission_reversed"]
            for p, unit, cash, fee in zip(self.frozen["participants"], values["units"],
                                         values.get("cash_reversed", [0, 0, 0]), fees):
                row["allocations"].append({**{k: p[k] for k in ("chong_id", "command_id", "family_id", "revision")},
                    "allocation_id": "netted:" + fixture_hash([key, event["allocation_id"], self.frozen["manifest_hash"], p]),
                    "units": str(unit), "acquisition_cash": str(cash), fee_field: str(fee),
                    "component": component, "reversal": component != "fee_only", "original_fact_key": original_key})
            image["fill_allocations"].append(row)
        image["history"] = [{"kind": "ResolveEvidence", "evidence_id": row["evidence_id"],
                              "covered": [row["fact_key"]]} for row in image["fill_allocations"]]
        self.rehash()
        return image

    def rebind_event(self, record_index, change):
        """负控也保持原件、内容摘要和逐重 ID 自洽，避免被较早 hash 检查遮住。"""
        image = self.states["E"]["image"]
        record = image["recorded"][record_index]
        index = int(record["json_pointer"].split("/")[-1])
        document = json.loads(record["raw_utf8"])
        change(document["events"][index])
        raw = json.dumps(document, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        new = fixture_record(index, raw=raw)
        old_key = record["fact_key"]
        image["recorded"][record_index] = new
        for row in image["fill_allocations"]:
            if row["fact_key"] != old_key:
                continue
            row.update(fact_key=new["fact_key"], fact_hash=new["fact_hash"])
            for a, p in zip(row["allocations"], self.frozen["participants"]):
                a["allocation_id"] = "netted:" + fixture_hash([new["fact_key"], new["event"]["allocation_id"], row["manifest_hash"], p])
        for entry in image["history"]:
            entry["covered"] = [new["fact_key"] if k == old_key else k for k in entry["covered"]]

    def assert_rejected(self):
        self.rehash()
        with self.assertRaises(EconomicQueryError):
            self.q.view("E")

    def test_production_identity_recipe_fixed_vectors(self):
        record = fixture_record(3)
        self.assertEqual(record["fact_key"], "341aaa93482adf1556398be175f9250dcfbde5e9047cd568035c76a723eab0f8")
        self.assertEqual(record["raw_sha256"], "5b56870f8801ffc205683972158c286cfea25bc7672efd732300ac97d89c84e9")
        self.assertEqual(record["fact_hash"], "78a8ba036029611e270f27851dcb4abe599e92594001d1de52bc68bf3c1c7f41")
        expected = ["1b99bebb527e72f1562baf35c0d2ccc299fe25f5a14ec251151407b0a26c8fbd",
                    "363b338c83526712123352fc7a94704ee3e6c14a04a74cc1d0bf8c731879746c",
                    "6def39c299d7be31b33780447b395d8d09f9124a050cc5f09696ce9165ecc054"]
        for i, value in enumerate(expected, 6):
            self.assertEqual(fixture_record(i, "C4B-RAW.json")["fact_hash"], value)

    def test_three_legal_corrections_preserve_exact_image(self):
        image = self.install_corrections()
        before = canonical(image)
        self.assertEqual(canonical(self.q.view("E")["image"]), before)
        self.assertEqual([r["report_kind"] for r in image["fill_allocations"][3:]],
                         ["fee_only", "bust", "reverse_component"])

    def test_unknown_correction_kind_null_and_string_reject(self):
        for component in (None, "commission"):
            image = self.install_corrections()
            self.rebind_event(3, lambda e: e["netted_fill"].update(report_kind="future_unknown"))
            row = image["fill_allocations"][3]
            row.update(report_kind="future_unknown", component=component)
            for a in row["allocations"]:
                a["commission"] = a.pop("fee_only")
                a.update(component=component, reversal=True)
            with self.subTest(component=component):
                self.assert_rejected()

    def test_raw_event_pointer_hash_bindings_reject(self):
        original = copy.deepcopy(self.states["E"])
        changes = [lambda r: r.update(raw_utf8=r["raw_utf8"] + " "),
                   lambda r: r["event"]["netted_fill"].update(unit_price="999"),
                   lambda r: r.update(json_pointer="/events/4"),
                   lambda r: r.update(json_pointer="/events/03"),
                   lambda r: r.update(json_pointer="/events/99"),
                   lambda r: r.update(raw_sha256="0" * 64),
                   lambda r: r.pop("raw_utf8")]
        for i, change in enumerate(changes):
            self.states["E"] = copy.deepcopy(original)
            change(self.states["E"]["image"]["recorded"][0])
            with self.subTest(case=i):
                self.assert_rejected()

    def test_consistently_forged_fact_key_or_hash_reject(self):
        original = copy.deepcopy(self.states["E"])
        for field in ("fact_key", "fact_hash"):
            self.states["E"] = copy.deepcopy(original)
            image = self.states["E"]["image"]
            row, record = image["fill_allocations"][0], image["recorded"][0]
            row[field] = record[field] = "0" * 64
            if field == "fact_key":
                image["history"][0]["covered"] = [row[field]]
                for a, p in zip(row["allocations"], self.frozen["participants"]):
                    a["allocation_id"] = "netted:" + fixture_hash([row[field], record["event"]["allocation_id"], row["manifest_hash"], p])
            with self.subTest(field=field):
                self.assert_rejected()

    def test_correction_cannot_substitute_unrelated_record(self):
        image = self.install_corrections()
        row = image["fill_allocations"][3]
        row["original_fact_key"] = image["recorded"][1]["fact_key"]
        for a in row["allocations"]:
            a["original_fact_key"] = row["original_fact_key"]
        self.assert_rejected()

    def test_correction_original_reference_mutations_reject(self):
        changes = [lambda e: e["depends_on"].update(external_fact_id="net-fill-2"),
                   lambda e: e["netted_fill"]["fill_ref"].update(external_fact_id="net-fill-2"),
                   lambda e: e["netted_fill"]["fill_ref"].update(allocation_id="alloc-net-fill-2"),
                   lambda e: e["netted_fill"]["fill_ref"].update(manifest_hash="0" * 64),
                   lambda e: e["netted_fill"]["fill_ref"].update(fill_allocation_policy_id="other"),
                   lambda e: e["netted_fill"]["fill_ref"].update(policy_revision="other"),
                   lambda e: e.pop("depends_on"),
                   lambda e: e["netted_fill"].pop("fill_ref")]
        for i, change in enumerate(changes):
            self.install_corrections()
            self.rebind_event(3, change)
            with self.subTest(case=i):
                self.assert_rejected()
        image = self.install_corrections()
        image["fill_allocations"].pop(0)
        self.assert_rejected()

    def test_correction_component_matches_original(self):
        for record_index in (4, 5):
            self.install_corrections()
            self.rebind_event(record_index, lambda e: e["netted_fill"].update(component="other"))
            with self.subTest(record_index=record_index):
                self.assert_rejected()

    def test_row_range_matches_ordinary_and_bust_and_zero_quantity(self):
        for index in (0, 3, 4, 5):
            image = self.install_corrections()
            row = image["fill_allocations"][index]
            row["to"] = str(int(row["to"]) + 1)
            with self.subTest(index=index):
                self.assert_rejected()

    def test_units_sign_is_limited_by_variant(self):
        for index, unit in ((0, "-1"), (4, "1"), (3, "1"), (3, "-1"), (5, "1"), (5, "-1")):
            image = self.install_corrections()
            image["fill_allocations"][index]["allocations"][0]["units"] = unit
            with self.subTest(index=index, unit=unit):
                self.assert_rejected()

    def test_bound_cumulative_fill_is_accepted(self):
        self.rebind_event(0, lambda e: e["netted_fill"].update(report_kind="cumulative"))
        self.rehash()
        self.assertEqual(self.q.view("E")["status"], "Available")

    def test_valid_optional_allocation_is_returned_without_mutation(self):
        before = canonical(self.states["E"]["image"])
        self.assertEqual(canonical(self.snapshot()["domains"]["E"]["image"]), before)
        del self.states["E"]["image"]["fill_allocations"]
        self.rehash()
        result = self.snapshot()
        self.assertEqual(result["completeness"], "CompleteVector")
        self.assertNotIn("fill_allocations", result["domains"]["E"]["image"])

    def test_unknown_book_version_rejects_before_owner_io(self):
        for revision in (False, 1, [], {}, "tb03c-book/2", "economic-session/2"):
            with self.subTest(revision=revision), patch("economic_query.exchange") as owner:
                with self.assertRaises(EconomicQueryError):
                    self.load({**self.manifest, "book_revision": revision})
                owner.assert_not_called()

    def test_old_domain_rejects_new_top_level_and_mixed_book(self):
        for selector in (None, "missing"):
            old = copy.deepcopy(self.manifest)
            old.pop("fill_manifests")
            old.pop("fill_allocation_policy")
            old["book_revision"] = selector
            if selector == "missing":
                del old["book_revision"]
            q = self.load(old)
            q.owner = self.owner
            with self.subTest(selector=selector), self.assertRaises(EconomicQueryError):
                q.view("E")
            self.states["B:chong-N1"]["image"]["book"] = {"book_revision": "tb03c-book/1"}
            self.rehash("B:chong-N1")
            with self.assertRaises(EconomicQueryError):
                q.view("B:chong-N1")

    def test_new_domain_rejects_old_or_unknown_book(self):
        for book in ({}, {"book_revision": None}, {"book_revision": "tb03c-book/2"}):
            self.states["B:chong-N1"]["image"]["book"] = book
            self.rehash("B:chong-N1")
            with self.subTest(book=book), self.assertRaises(EconomicQueryError):
                self.q.view("B:chong-N1")

    def test_b_cannot_acquire_e_allocation_field(self):
        self.states["B:chong-N1"]["image"]["fill_allocations"] = []
        self.rehash("B:chong-N1")
        result = self.snapshot()
        self.assertEqual(result["domains"]["B:chong-N1"]["status"], "Unavailable")
        self.assertEqual(result["domains"]["E"]["status"], "Available")
        self.assertEqual(result["domains"]["B:chong-N2"]["status"], "Available")

    def test_allocation_exact_shapes_and_types_reject(self):
        valid = copy.deepcopy(self.states["E"])
        cases = [None, {}, "[]", [None], [{**self.row, "unknown": "1"}],
                 [{k: v for k, v in self.row.items() if k != "fact_hash"}],
                 [{**self.row, "allocations": []}], [{**self.row, "allocations": {}}]]
        for value in cases:
            self.states["E"] = copy.deepcopy(valid)
            self.states["E"]["image"]["fill_allocations"] = value
            self.rehash()
            with self.subTest(value=value), self.assertRaises(EconomicQueryError):
                self.q.view("E")

    def test_allocation_field_mutations_reject_with_correct_image_hash(self):
        original = copy.deepcopy(self.states["E"])
        mutations = [("fact_key", "0" * 64), ("fact_hash", "0" * 64), ("manifest_hash", "0" * 64),
                     ("symbol", "other"), ("fill_allocation_policy_id", "other"), ("policy_revision", "other"),
                     ("evidence_id", "missing"), ("from", 0), ("to", "03"), ("to", "9223372036854775808")]
        for field, value in mutations:
            self.states["E"] = copy.deepcopy(original)
            self.states["E"]["image"]["fill_allocations"][0][field] = value
            self.rehash()
            with self.subTest(field=field), self.assertRaises(EconomicQueryError):
                self.q.view("E")

    def test_participant_identity_and_numeric_text_are_strict(self):
        original = copy.deepcopy(self.states["E"])
        mutations = [("chong_id", "other"), ("command_id", "other"), ("family_id", "other"),
                     ("revision", "other"), ("allocation_id", "other"), ("units", 1.0), ("units", "-0"),
                     ("commission", -1), ("acquisition_cash", "1.2"), ("commission", "-1")]
        for field, value in mutations:
            self.states["E"] = copy.deepcopy(original)
            self.states["E"]["image"]["fill_allocations"][0]["allocations"][0][field] = value
            self.rehash()
            with self.subTest(field=field, value=value), self.assertRaises(EconomicQueryError):
                self.q.view("E")

    def test_missing_source_and_evidence_and_duplicates_reject(self):
        original = copy.deepcopy(self.states["E"])
        for field in ("recorded", "history"):
            self.states["E"] = copy.deepcopy(original)
            self.states["E"]["image"][field] = []
            self.rehash()
            with self.subTest(field=field), self.assertRaises(EconomicQueryError):
                self.q.view("E")
        self.states["E"] = copy.deepcopy(original)
        self.states["E"]["image"]["fill_allocations"] *= 2
        self.rehash()
        with self.assertRaises(EconomicQueryError):
            self.q.view("E")

    def test_integer_above_js_safe_range_is_preserved_and_hash_tamper_rejects(self):
        a = self.states["E"]["image"]["fill_allocations"][0]["allocations"][0]
        a["acquisition_cash"] = "9007199254740993"
        self.rehash()
        got = self.q.view("E")["image"]["fill_allocations"][0]["allocations"][0]
        self.assertEqual(got["acquisition_cash"], "9007199254740993")
        a["acquisition_cash"] = "9007199254740994"
        with self.assertRaises(EconomicQueryError):
            self.q.view("E")

    def test_source_policy_and_selected_revision_are_frozen(self):
        before = self.q.view("E")
        changed = copy.deepcopy(self.manifest)
        changed["fill_allocation_policy"]["policy_revision"] = "changed"
        (self.root / "manifest.json").write_bytes(canonical(changed))
        self.assertEqual(self.q.view("E"), before)
        self.q.manifest["book_revision"] = "tb03c-book/999"
        self.q.manifest["fill_allocation_policy"]["policy_revision"] = "changed"
        self.assertEqual(self.q.book_revision, "tb03c-book/1")
        self.assertEqual(self.q.view("E"), before)

    def test_config_source_mixing_and_unknown_fields_reject_before_io(self):
        changes = [lambda m: m.update(book_revision=None),
                   lambda m: m["fill_allocation_policy"].update(policy_revision=None),
                   lambda m: m["fill_allocation_policy"].update(extra="x"),
                   lambda m: m["fill_allocation_policy"].update(fill_allocation_policy_id="per-leg-source-attribution/1"),
                   lambda m: m["fill_manifests"][0].update(manifest_hash="0" * 64)]
        for change in changes:
            m = copy.deepcopy(self.manifest)
            change(m)
            with self.subTest(change=change), patch("economic_query.exchange") as peer:
                with self.assertRaises(EconomicQueryError):
                    self.load(m)
                peer.assert_not_called()

    def test_discovery_new_revision_and_old_canonical_bytes(self):
        def discovery(q):
            values = []
            handler = SimpleNamespace(path="/api/economic/ready", server=SimpleNamespace(economic_query=q),
                                      _send_json=lambda value, status=200: values.append(value))
            self.assertTrue(economic_http.get(handler))
            return values[0]
        self.assertEqual(discovery(self.q)["book_revision"], "tb03c-book/1")
        old = copy.deepcopy(self.manifest)
        for field in ("book_revision", "fill_manifests", "fill_allocation_policy"):
            del old[field]
        q = self.load(old)
        expected = {"schema_revision": "economic-discovery/1", "session_id": q.session,
                    "session_generation": q.generation, "producer_epoch": q.query_epoch,
                    "manifest_hash": q.manifest_hash, "domains": list(q.nodes),
                    "max_reply_bytes": str(q.resources["max_reply_bytes"]),
                    "operation_timeout_ms": str(q.resources["operation_timeout_ms"]),
                    "scope": "TestOnlyRecordedEconomics", "configured": True}
        self.assertEqual(canonical(discovery(q)), canonical(expected))

    def test_http_handler_snapshot_preserves_owner_bytes_and_reply_binding(self):
        request = self.q.request({"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None})
        raw, sent = canonical(request), []
        handler = SimpleNamespace(path="/api/economic/snapshot", rfile=io.BytesIO(raw),
            headers=SimpleNamespace(get_all=lambda name, default: [str(len(raw))], get=lambda name: None),
            connection=SimpleNamespace(settimeout=lambda seconds: None),
            server=SimpleNamespace(economic_query=self.q, resources={**self.q.resources, "socket_read_timeout_ms": 1000}),
            _send_json=lambda value, status=200: sent.append((canonical(value), status)),
            _send_bytes=lambda value, status=200: sent.append((value, status)))
        self.assertTrue(economic_http.post(handler))
        response, status = sent[0]
        self.assertEqual(status, 200)
        result = validate_reply(request, json.loads(response), producer="Q", producer_epoch="1")
        self.assertEqual(canonical(result["domains"]["E"]["image"]), canonical(self.states["E"]["image"]))

    def history_owner(self):
        old = {d: self.view(d) for d in self.q.nodes}
        new = copy.deepcopy(self.states)
        for d, v in new.items():
            v["cut"] = {"domain_id": d, "commit_seq": "1", "root_hash": digest([d, "1"])}
            v["commit_ns"] = "10"
        def owner(domain, payload):
            if payload["op"] == "Watch":
                v = new[domain]
                return {"kind": "EconomicChanges", "domain_id": domain, "cut": v["cut"],
                        "changes": [{k: v[k] for k in ("cut", "image", "image_hash", "commit_ns")}]}
            return copy.deepcopy(old[domain] if payload.get("cut") == old[domain]["cut"]
                                 or payload.get("as_known_ns") == "0" else new[domain])
        self.q.owner = owner
        return old, new, owner

    def test_same_cut_watch_snapshot_as_known_share_exact_images(self):
        old, new, _ = self.history_owner()
        watch = self.q.watch({"op": "watch", "cuts": {d: v["cut"] for d, v in old.items()}})
        snapshot = self.snapshot(cuts={d: v["cut"] for d, v in new.items()})
        for d in self.q.nodes:
            self.assertEqual(watch["domains"][d]["status"], "Available")
            self.assertEqual(canonical(watch["domains"][d]["changes"][0]["image"]),
                             canonical(snapshot["domains"][d]["image"]))
        past = self.snapshot(as_known_ns="0")
        self.assertNotIn("fill_allocations", past["domains"]["E"]["image"])
        self.assertEqual(self.snapshot(snapshot_token=past["snapshot_token"]), past)

    def test_watch_rejects_missing_duplicate_or_tampered_batches(self):
        old, _, original = self.history_owner()
        for mutation in ("drop", "duplicate", "tamper"):
            def owner(domain, payload):
                result = copy.deepcopy(original(domain, payload))
                if payload["op"] == "Watch":
                    if mutation == "drop":
                        result["changes"] = []
                    elif mutation == "duplicate":
                        result["changes"] *= 2
                    else:
                        result["changes"][0]["image_hash"] = "0" * 64
                return result
            self.q.owner = owner
            watch = self.q.watch({"op": "watch", "cuts": {d: v["cut"] for d, v in old.items()}})
            with self.subTest(mutation=mutation):
                self.assertTrue(all(v["status"] == "Unavailable" for v in watch["domains"].values()))

    def test_wrong_domain_cut_epoch_and_as_known_are_rejected(self):
        self.states["E"]["cut"]["domain_id"] = "B:chong-N1"
        with self.assertRaises(EconomicQueryError):
            self.q.view("E")
        self.states["E"]["cut"]["domain_id"] = "E"
        self.states["E"]["commit_ns"] = "100"
        with self.assertRaises(EconomicQueryError):
            self.q.view("E", as_known_ns="1")
        with self.assertRaises(EconomicQueryError):
            self.q.view("E", cut={"domain_id": "E", "commit_seq": "99", "root_hash": "f" * 64})
        q = self.load(self.manifest)
        def peer(socket, request, **kwargs):
            return {"transport": "Received", "response": v2.canonical_reply(request, self.states["E"], "E", "economic-session/replies", epoch="2")}
        with patch("economic_query.exchange", side_effect=peer), self.assertRaises(ValueError):
            q.view("E")


if __name__ == "__main__":
    unittest.main()
