"""#1394 工作草稿。只有内存 fake/临时普通文件，无真实产品/网络/DB/子进程。"""
import copy
import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

import captured_cut as x
import driver as d
import product_capture as p
from test_capture_bridge import fixture as original_fixture


def http(body, status=200):
    reason = {200: "OK", 400: "Bad Request", 503: "Service Unavailable"}[status]
    return (f"HTTP/1.0 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\n"
            f"Content-Length: {len(body)}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n").encode() + body


def fixture():
    contract, captures, nodes, _ = original_fixture()
    resources = contract["resources"]
    plan = {k: copy.deepcopy(contract[k]) for k in ("source", "fixed_cut", "anchor", "page_size", "cut_projection_digest", "resources", "stage")}
    identity = lambda domain, pid: {"pid": pid, "start_identity": "UNIT_TEST_ONLY", "process_group": pid,
                                    "domain_id": domain, "writer_epoch": "1"}
    plan.update(schema_revision="issue1394-product-capture/1", lifecycle_id="UNIT_TEST_ONLY-LIFE", capture_id="capture-1",
                reader_binary="/UNIT_TEST_ONLY/producer", session={"owner_key": ["session", "S", "1"],
                    "identity": identity("S", 101), "binary_sha256": plan["source"]["binary_sha256"],
                    "db": "/UNIT_TEST_ONLY/db", "socket": "/UNIT_TEST_ONLY/session"},
                q={"owner_key": ["q-owner", "Q", "1"], "identity": identity("Q", 102), "binary_sha256": "d" * 64,
                   "endpoint": {"host": "127.0.0.1", "port": 4567, "path": "/api/v3/snapshot"}, "producer_epoch": "1",
                   "request_header": {"source_namespace": "toy", "source_epoch": "1", "producer_id": "toy", "producer_epoch": "1"}},
                slots=[], initial_usage={name: {key: 0 for key in p.USAGE} for name in resources["jobs"]},
                initial_session_requests=0, verification={k: "4000000" for k in
                    ("read_max_records", "read_max_bytes", "closure_max_nodes", "closure_max_bytes", "closure_max_visits")})
    replies = []
    wanted = []
    for i, capture in enumerate(captures):
        route = capture.route
        slot = {"id": "request-" + str(i), "route": route, "stage": "read-" + str(i),
                "deadline": resources["jobs"][capture.binding["job"]]["deadline"], "binding": capture.binding,
                "argv": [] if route == "snapshot" else p.financial_argv(plan["reader_binary"], route, plan["session"]["db"], plan["session"]["socket"]),
                "closure_max_reply_bytes": "65536" if route == "financial-closure" else None}
        plan["resources"]["deadlines"]["stages"][slot["stage"]] = slot["deadline"]
        plan["slots"].append(slot)
        if route == "snapshot":
            reply = d.strict(capture.reply)
            payload = reply["payload"]
            wanted.extend(row["key"][0] for row in payload["rows"] if row["family"] == "objects")
            request = d.strict(capture.request)
            request["message_id"] = slot["id"]
            reply["message_id"] = "reply-" + x.fingerprint([request["source_namespace"], request["source_epoch"], request["message_id"], reply["payload_hash"]])
            reply["causal_refs"][0]["message_id"] = slot["id"]
            reply["source_namespace"] = "s-observe/replies"
            reply["producer_id"] = "Q:" + plan["fixed_cut"]["session_id"]
            raw = d.canonical(reply)
            exchange = p.Exchange(reply=raw, transport_reply=http(raw), complete=True,
                identity={"owner": copy.deepcopy(plan["q"]["identity"]), "endpoint": copy.deepcopy(plan["q"]["endpoint"]), "http_status": 200})
        else:
            raw = capture.reply
            if route == "financial-basis":
                reply = d.strict(raw)
                by_id = {item["object_id"]: item for item in reply["selected_objects"]}
                reply["selected_objects"] = [by_id[oid] for oid in wanted]
                raw = d.canonical(reply) + b"\n"
            exchange = p.Exchange(reply=raw, transport_reply=raw, returncode=0, complete=True,
                identity={"owner": identity("S", 103 + i), "argv": slot["argv"], "stage": slot["stage"],
                          "binary_sha256_before": plan["source"]["binary_sha256"],
                          "binary_sha256_after": plan["source"]["binary_sha256"], "reaped": True})
        replies.append(exchange)
    return plan, replies


def run(plan, replies, ledger=None):
    transport = p.MemoryTransport(replies)
    machine = p.CaptureMachine(plan, ledger or p.Ledger(plan), transport)
    return machine.run(), machine, transport


def mutate_q(replies, index, fn):
    reply = d.strict(replies[index].reply)
    fn(reply)
    reply["payload_hash"] = x.fingerprint(reply["payload"])
    cause = reply["causal_refs"][0]
    reply["message_id"] = "reply-" + x.fingerprint([cause["source_namespace"], cause["source_epoch"], cause["message_id"], reply["payload_hash"]])
    replies[index].reply = d.canonical(reply)
    replies[index].transport_reply = http(replies[index].reply)


def mutate_cli(replies, index, fn):
    reply = d.strict(replies[index].reply)
    fn(reply)
    replies[index].reply = replies[index].transport_reply = d.canonical(reply) + b"\n"


def gate_fixture(directory):
    """用真实 pin/单次 receipt 构造测试准入；owner 仅内存对象，从未创建进程。"""
    td = Path(directory)
    plan, replies = fixture()
    binary = td / "producer"
    binary.write_bytes(b"UNIT_TEST_ONLY inert bytes, not executable")
    binary_hash = d.sha(binary.read_bytes())
    plan["reader_binary"] = str(binary)
    plan["source"]["binary_sha256"] = binary_hash
    plan["session"]["binary_sha256"] = binary_hash
    plan["q"]["binary_sha256"] = binary_hash
    for slot in plan["slots"]:
        if slot["argv"]:
            slot["argv"][0] = str(binary)

    def pin(name, value):
        path = td / name
        path.write_bytes(d.canonical(value))
        return {"path": str(path), "sha256": d.sha(path.read_bytes())}

    source = pin("source.json", {"scope": "UNIT_TEST_ONLY"})
    adapter_sha = d.sha(Path(p.__file__).read_bytes())
    plan_pin = pin("plan.json", plan)
    compatibility = pin("compatibility.json", {"schema_revision": "issue1394-read-compatibility/1", "source": plan["source"],
        "historical_freeze_sha256": p.SOURCE_FREEZE, "adapter_sha256": adapter_sha, "verdict": "COMPATIBLE"})
    implementation = pin("supervisor-inert.json", {"scope": "UNIT_TEST_ONLY; no OS implementation"})
    supervision = pin("supervision.json", {"schema_revision": "issue1394-read-supervision/1", "implementation": implementation,
        "capabilities": ["process_tree", "hard_deadlines", "per_object_resources", "endpoint_owner", "read_guard/1"],
        "verdict": "APPROVE_READ_GUARD"})
    commands = []
    for role in ("session", "q"):
        item = plan[role]
        commands.append({"stage": item["owner_key"][0], "domain_id": item["identity"]["domain_id"], "writer_epoch": "1",
            "argv": [str(binary), "UNIT_TEST_ONLY-owner"], "binary_sha256": binary_hash, "resource_caps": plan["resources"],
            "read_endpoint": item["endpoint"] if role == "q" else {"db": item["db"], "socket": item["socket"]}})
    for slot in plan["slots"]:
        if slot["argv"]:
            commands.append({"stage": slot["stage"], "domain_id": "S", "writer_epoch": "1", "argv": slot["argv"],
                             "binary_sha256": binary_hash, "resource_caps": plan["resources"]})
    contract = {"schema_revision": "issue1394-driver-execution/1", "execution_authorized": True,
        "cwd": str(td), "environment": {}, "source_roles": source, "input_originals": [source],
        "artifacts": [{"path": str(binary), "sha256": binary_hash}, plan_pin, compatibility, supervision, implementation],
        "total_timeout_ms": "60000", "cleanup_reserve_ms": "10000", "seal_reserve_ms": "10000",
        "term_grace_ms": "100", "kill_wait_ms": "100", "single_use_receipt": str(td / "single-use.json"),
        "supervisor_socket": str(td / "no-real-socket"), "commands": commands,
        "product_capture": {"schema_revision": "issue1394-read-admission/1", "adapter_sha256": adapter_sha,
            "plan": plan_pin, "compatibility": compatibility, "supervision": supervision,
            "supervisor_limits": {"max_frame_bytes": "1000000", "deadline": plan["resources"]["deadlines"]["work"]},
            "output": str(td / "capture")}}
    cp = pin("contract.json", contract)
    gp = pin("grant.json", {"execution_authorized": True, "contract_sha256": cp["sha256"],
        "driver_sha256": d.sha(Path(d.__file__).read_bytes()), "nonce": "UNIT_TEST_ONLY-nonce"})
    admission = d.ExecutionAdmission(cp, gp)
    supervisor = d.Supervisor(contract["supervisor_socket"], admission.contract_sha, plan["resources"]["deadlines"]["work"], 1000000)
    for role in ("session", "q"):
        item = plan[role]
        owner = d.OwnedProcess.__new__(d.OwnedProcess)
        owner.admission, owner.supervisor, owner.released = admission, supervisor, True
        owner.identity, owner.binary_before = item["identity"], item["binary_sha256"]
        owner.argv = commands[0]["argv"]
        owner.stage, owner.domain, owner.epoch = item["owner_key"]
        owner.process = Mock()
        owner.process.poll.return_value = None
        owner.verify_identity = Mock(return_value=True)
        admission.active_owners[tuple(item["owner_key"])] = owner
    return p.ReadAdmission(admission, supervisor), replies


def owned_transport(test, gate):
    directory = d.OwnedDirectory(gate.extension["output"])
    test.addCleanup(directory.close)
    return p.ProductTransport(gate, directory.path, directory=directory)


class ProtocolTests(unittest.TestCase):
    def test_complete_real_shapes_raw_order_and_missing_dependency(self):
        plan, replies = fixture()
        with patch.object(x, "export_captured", side_effect=AssertionError("不得自算 root pin 调 export")):
            result, machine, transport = run(plan, replies)
        self.assertEqual(result["capture_status"], "UNAUTHENTICATED_RAW_CAPTURE", result["failure"])
        self.assertEqual(result["status"], "MISSING_DEPENDENCY")
        self.assertFalse(result["execution_authorized"])
        self.assertEqual(len(machine.captures), len(plan["slots"]))
        query = next(d.strict(item.request) for item in machine.captures if item.route == "financial-basis")
        wanted = [r["key"][0] for r in machine.rows if r["family"] == "objects"]
        self.assertEqual(query["object_ids"], wanted)
        self.assertEqual(wanted, ["carrier", "source", "terminal"])
        self.assertEqual([i["snapshot_index"] for i in machine.basis["selected_objects"]], ["2", "0", "1"])
        closure = [d.strict(item.request) for item in machine.captures if item.route == "financial-closure"]
        self.assertEqual(closure[0]["after"], "")
        self.assertTrue(all(item["path"] == [] and item["anchor"] == plan["anchor"] for item in closure))
        for slot, raw, wire, _ in transport.requests:
            if slot["route"] == "snapshot":
                self.assertEqual(wire, p.http_request(plan["q"]["endpoint"], raw))
                self.assertEqual(set(d.strict(raw)), p.ENVELOPE)
            else:
                self.assertEqual(raw, wire)
                self.assertEqual(slot["argv"][6:], ["--session", plan["session"]["socket"]])

    def test_q_cut_offset_metadata_token_and_identity_stop_before_next(self):
        mutations = [lambda r: r["payload"]["fixed_cut"].update(structure_cut="other"),
                     lambda r: r["payload"].update(offset="2"), lambda r: r["payload"].update(done=True),
                     lambda r: r["payload"].update(omissions=["objects"]),
                     lambda r: r["payload"]["next_token"].update(issued_capture_digest="a" * 64),
                     lambda r: r["payload"]["next_token"].update(extra="unknown"),
                     lambda r: r.update(source_epoch="2"), lambda r: r.update(producer_id="foreign"),
                     lambda r: r["payload"].update(ok=False), lambda r: r.update(extra=True)]
        for change in mutations:
            plan, replies = fixture()
            mutate_q(replies, 0, change)
            result, machine, transport = run(plan, replies)
            with self.subTest(change=change):
                self.assertEqual(result["capture_status"], "INCOMPLETE")
                self.assertEqual(len(transport.requests), 1)
                self.assertEqual(machine.captures[0].reply, replies[0].reply)

    def test_q_full_envelope_required_and_duplicate_json_rejected(self):
        for raw_kind in ("payload", "duplicate", "unknown", "truncated"):
            plan, replies = fixture()
            raw = replies[0].reply
            if raw_kind == "payload":
                raw = d.canonical(d.strict(raw)["payload"])
            elif raw_kind == "duplicate":
                raw = b'{"payload":{},' + raw[1:]
            elif raw_kind == "unknown":
                raw = b'{"error":"DeliveryUnknown"}'
            else:
                raw = raw[:-1]
            replies[0].reply, replies[0].transport_reply = raw, http(raw)
            result, _, transport = run(plan, replies)
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            self.assertEqual(len(transport.requests), 1)

    def test_q_duplicate_rows_family_order_and_counts_rejected(self):
        for change in (lambda r: r["payload"]["rows"].__setitem__(1, r["payload"]["rows"][0]),
                       lambda r: r["payload"]["counts"].update(total="999"),
                       lambda r: r["payload"]["rows"][0].update(family="unknown")):
            plan, replies = fixture()
            mutate_q(replies, 0, change)
            result, _, transport = run(plan, replies)
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            self.assertEqual(len(transport.requests), 1)

    def test_q_intrafamily_reorder_rejected_before_any_financial_request(self):
        plan, replies = fixture()
        mutate_q(replies, 0, lambda r: r["payload"]["rows"].reverse())
        result, _, transport = run(plan, replies)
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertEqual([slot["route"] for slot, _, _, _ in transport.requests], ["snapshot"] * 4)
        self.assertIn("投影根", result["failure"])

    def test_validated_digest_may_change_but_issued_identity_must_not(self):
        plan, replies = fixture()
        for i, slot in enumerate(plan["slots"]):
            if slot["route"] == "snapshot" and i:
                mutate_q(replies, i, lambda r: r["payload"].update(validated_capture_digest="e" * 64))
        result, _, _ = run(plan, replies)
        self.assertEqual(result["capture_status"], "UNAUTHENTICATED_RAW_CAPTURE", result["failure"])
        mutate_q(replies, 1, lambda r: r["payload"]["next_token"].update(issued_capture_digest="e" * 64))
        result, _, transport = run(plan, replies)
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertEqual(len(transport.requests), 2)

    def test_missing_q_page_or_closure_page_never_falls_through(self):
        for route in ("snapshot", "financial-closure"):
            plan, replies = fixture()
            index = next(i for i, slot in enumerate(plan["slots"]) if slot["route"] == route)
            replies.pop(index)
            result, _, transport = run(plan, replies)
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            self.assertEqual(len(transport.requests), index + 1)

    def test_unknown_transport_keeps_attempted_request(self):
        plan, _ = fixture()
        result, machine, transport = run(plan, [])
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertEqual(len(transport.requests), 1)
        self.assertEqual(len(machine.originals), 1)
        self.assertEqual(machine.captures[0].request, transport.requests[0][1])
        self.assertIn("DeliveryUnknown", machine.originals[0]["exchange"].error)

    def test_basis_all_objects_order_index_hash_and_identity_locked(self):
        for change in (lambda r: r["selected_objects"].reverse(),
                       lambda r: r["selected_objects"].pop(),
                       lambda r: r["selected_objects"][0].update(snapshot_index="99"),
                       lambda r: r["selected_objects"][0].update(object_hash="0" * 64),
                       lambda r: r.update(writer_epoch="99"), lambda r: r.update(pending_revision=True)):
            plan, replies = fixture()
            index = next(i for i, s in enumerate(plan["slots"]) if s["route"] == "financial-basis")
            mutate_cli(replies, index, change)
            result, _, transport = run(plan, replies)
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            self.assertEqual(len(transport.requests), index + 1)

    def test_closure_after_next_total_order_and_node_bytes_locked(self):
        for change in (lambda r: r.update(after=None), lambda r: r.update(next="sc1-" + "f" * 64),
                       lambda r: r.update(total="0"), lambda r: r.update(done=True),
                       lambda r: r["nodes"].reverse(), lambda r: r["nodes"][0].update(body_hex="ff"),
                       lambda r: r["anchor"].update(writer_epoch="99"), lambda r: r.update(path=[{"field": "snapshot"}])):
            plan, replies = fixture()
            index = next(i for i, s in enumerate(plan["slots"]) if s["route"] == "financial-closure")
            mutate_cli(replies, index, change)
            result, _, transport = run(plan, replies)
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            self.assertEqual(len(transport.requests), index + 1)

    def test_raw_closure_whitespace_and_lf_count_against_each_request(self):
        for padding, allowed in ((0, True), (100, False)):
            plan, replies = fixture()
            index = next(i for i, s in enumerate(plan["slots"]) if s["route"] == "financial-closure")
            cap = len(replies[index].reply)
            plan["slots"][index]["closure_max_reply_bytes"] = str(cap)
            replies[index].reply = replies[index].transport_reply = b" " * padding + replies[index].reply
            result, machine, _ = run(plan, replies)
            self.assertEqual(result["capture_status"], "UNAUTHENTICATED_RAW_CAPTURE" if allowed else "INCOMPLETE", result["failure"])
            self.assertEqual(len(machine.captures[index].reply), cap + padding)

    def test_errors_stderr_and_nonzero_originals_preserved_without_next(self):
        for kind in ("exit", "unknown", "stderr", "http-error"):
            plan, replies = fixture()
            index = 0 if kind == "http-error" else next(i for i, s in enumerate(plan["slots"]) if s["route"] == "financial-basis")
            if kind == "exit":
                replies[index].returncode = 7
                replies[index].stderr = b"original-error\x00\xff\n"
            elif kind == "unknown":
                replies[index].complete = False
                replies[index].error = "DeliveryUnknown"
            elif kind == "stderr":
                replies[index].stderr = b"x" * 6
                plan["resources"]["reader"]["max_stderr_bytes"] = "5"
            else:
                replies[index].reply = b'{"ok":false,"detail":"original failure"}'
                replies[index].transport_reply = http(replies[index].reply, 503)
            result, machine, transport = run(plan, replies)
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            self.assertEqual(len(transport.requests), index + 1)
            self.assertEqual(machine.originals[-1]["exchange"].stderr, replies[index].stderr)
            self.assertEqual(machine.captures[-1].reply, replies[index].reply)

    def test_object_request_and_reply_caps_independent(self):
        for role, key in (("q", "max_frame_bytes"), ("q", "max_reply_bytes"),
                          ("session", "max_request_bytes"), ("reader", "max_request_bytes"),
                          ("reader", "max_reply_bytes")):
            plan, replies = fixture()
            caps = plan["resources"]["q"]["limits"] if role == "q" else plan["resources"][role]
            caps[key] = "1"
            result, _, transport = run(plan, replies)
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            expected = 0 if role == "q" and key == "max_frame_bytes" else 1 if role == "q" else 4 + (key == "max_reply_bytes")
            self.assertEqual(len(transport.requests), expected)

    def test_job_wire_counts_http_headers_stderr_and_prior_usage(self):
        plan, replies = fixture()
        replies[4].stderr = b"diagnostic \xff\n"
        result, machine, _ = run(plan, replies)
        self.assertEqual(result["capture_status"], "UNAUTHENTICATED_RAW_CAPTURE", result["failure"])
        for job in plan["resources"]["jobs"]:
            total = sum(len(row["transport_request"]) + len(row["exchange"].transport_reply) + len(row["exchange"].stderr)
                        for row in machine.originals if row["slot"]["binding"]["job"] == job)
            self.assertEqual(result["observed_job_usage"][job]["wire"], total)
        plan["initial_usage"]["q-job"]["wire"] = 999999
        result, _, transport = run(plan, replies)
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertEqual(len(transport.requests), 0)

    def test_q_http_request_header_counts_against_original_frame_cap(self):
        plan, replies = fixture()
        machine = p.CaptureMachine(plan, p.Ledger(plan), p.MemoryTransport(replies))
        raw = machine.request(plan["slots"][0])
        wire = p.http_request(plan["q"]["endpoint"], raw)
        for maximum, sent in ((len(raw), 0), (len(wire) - 1, 0), (len(wire), 1)):
            altered = copy.deepcopy(plan)
            altered["resources"]["q"]["limits"]["max_frame_bytes"] = str(maximum)
            _, _, transport = run(altered, replies)
            self.assertEqual(min(len(transport.requests), 1), sent)

    def test_q_each_page_charges_whole_projection_not_small_returned_slice(self):
        plan, replies = fixture()
        total = int(d.strict(replies[0].reply)["payload"]["counts"]["total"])
        plan["resources"]["jobs"]["q-job"]["limits"]["max_total_records"] = str(total)
        result, _, transport = run(plan, replies)
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertEqual(len(transport.requests), 2)
        self.assertEqual(result["observed_job_usage"]["q-job"]["records"], 2 * total)

    def test_jobs_requests_nodes_records_and_session_cumulative(self):
        for dim in ("requests", "nodes", "node_bytes", "records", "session"):
            plan, replies = fixture()
            if dim == "session":
                plan["resources"]["session"]["max_requests"] = "1"
            elif dim == "records":
                plan["resources"]["jobs"]["q-job"]["limits"][p.USAGE[dim]] = "1"
            else:
                # 这两个字段本来就是同一 reader Job 的独立副本值，均须同步。
                plan["resources"]["reader"]["job_limits"][p.USAGE[dim]] = "1"
                plan["resources"]["jobs"]["financial-job"]["limits"][p.USAGE[dim]] = "1"
            result, _, _ = run(plan, replies)
            self.assertEqual(result["capture_status"], "INCOMPLETE", dim)

    def test_no_default_caps_or_owner_job_borrowing(self):
        mutations = [lambda p0: p0["resources"]["reader"].pop("max_stderr_bytes"),
                     lambda p0: p0["resources"]["jobs"].pop("q-job"),
                     lambda p0: p0["slots"][0]["binding"].update(job="financial-job"),
                     lambda p0: p0["resources"]["object_ids"].update(q=p0["resources"]["object_ids"]["reader"]),
                     lambda p0: p0["initial_usage"]["q-job"].pop("store_bytes"),
                     lambda p0: p0["resources"]["session"].update(retained_bytes="268435457")]
        for change in mutations:
            plan, replies = fixture()
            change(plan)
            with self.assertRaises((d.Rejected, KeyError)):
                run(plan, replies)

    def test_staged_store_usage_not_reset_and_limits_not_extended(self):
        for dim in ("staged_bytes", "store_bytes"):
            plan, _ = fixture()
            plan["initial_usage"]["q-job"][dim] = 1000001
            with self.assertRaises(d.Rejected):
                p.Ledger(plan)

    def test_lifecycle_cannot_retry_or_clear_counter_on_second_capture(self):
        plan, replies = fixture()
        ledger = p.Ledger(plan)
        result, _, _ = run(plan, replies, ledger)
        self.assertEqual(result["capture_status"], "UNAUTHENTICATED_RAW_CAPTURE", result["failure"])
        old = copy.deepcopy(ledger.used)
        with self.assertRaises(d.Rejected):
            run(plan, replies, ledger)
        plan["capture_id"] = "capture-2"
        result, _, transport = run(plan, replies, ledger)
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertEqual(len(transport.requests), 0)
        self.assertEqual(old, ledger.used)

    def test_rejection_halts_entire_lifecycle(self):
        plan, replies = fixture()
        replies[0].complete = False
        ledger = p.Ledger(plan)
        run(plan, replies, ledger)
        plan["capture_id"] = "other"
        with self.assertRaises(d.Rejected):
            run(plan, replies, ledger)

    def test_deadlines_before_after_and_request_timeout_no_renewal(self):
        plan, replies = fixture()
        plan["slots"][0]["deadline"] = time.monotonic() - 1
        result, _, transport = run(plan, replies)
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertEqual(len(transport.requests), 0)
        plan, replies = fixture()
        replies[0].ended = plan["slots"][0]["deadline"] + 1
        result, machine, transport = run(plan, replies)
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertEqual(len(transport.requests), 1)
        self.assertEqual(machine.originals[0]["exchange"].reply, replies[0].reply)
        plan, replies = fixture()
        plan["resources"]["q"]["limits"]["socket_read_timeout_ms"] = "3"
        result, _, transport = run(plan, replies)
        first = transport.requests[0]
        self.assertLessEqual(first[3], time.monotonic() + .003)

    def test_deadline_minimum_current_whole_job_and_historical_stage(self):
        plan, replies = fixture()
        plan["resources"]["deadlines"]["stages"]["completed-old"] = time.monotonic() - 100
        machine = p.CaptureMachine(plan, p.Ledger(plan), p.MemoryTransport(replies))
        first = plan["slots"][0]
        self.assertEqual(machine._deadline(first, time.monotonic()), first["deadline"])
        result = machine.run()
        self.assertEqual(result["capture_status"], "UNAUTHENTICATED_RAW_CAPTURE", result["failure"])

    def test_wrong_argv_cold_path_endpoint_and_path_rejected_pre_io(self):
        for change in (lambda a: a["slots"][4]["argv"].__delitem__(slice(6, None)),
                       lambda a: a["q"]["endpoint"].update(path="/api/v3/snapshot?x=1"),
                       lambda a: a["q"]["endpoint"].update(host="localhost"),
                       lambda a: a["slots"][0].update(id="../../escape")):
            plan, replies = fixture()
            change(plan)
            with self.assertRaises((d.Rejected, ValueError)):
                run(plan, replies)

    def test_exchange_identity_mismatch_stops(self):
        for index, key in ((0, "endpoint"), (4, "argv"), (4, "binary_sha256_after"), (4, "stage")):
            plan, replies = fixture()
            replies[index].identity[key] = "foreign"
            result, _, transport = run(plan, replies)
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            self.assertEqual(len(transport.requests), index + 1)

    def test_inventory_exact_originals_including_failed_stderr(self):
        plan, replies = fixture()
        replies[4].returncode, replies[4].stderr = 9, b"\x00original stderr\xff\n"
        result, machine, _ = run(plan, replies)
        with tempfile.TemporaryDirectory() as td:
            directory = d.OwnedDirectory(Path(td) / "raw")
            try:
                items = p.persist_originals(machine, result, directory, time.monotonic() + 10)
                for item in items:
                    raw = (directory.path / item["path"]).read_bytes()
                    self.assertEqual((len(raw), d.sha(raw)), (item["bytes"], item["sha256"]))
                self.assertEqual((directory.path / "capture-000004.stderr").read_bytes(), replies[4].stderr)
                self.assertNotIn("ORIGINALS.json", [item["path"] for item in items])
            finally:
                directory.close()


class AdmissionTests(unittest.TestCase):
    def test_unadmitted_booleans_strings_and_fake_zero_io(self):
        with patch.object(p.socket, "socket") as network, patch.object(p.subprocess, "Popen") as spawn, patch.object(d, "OwnedDirectory") as files:
            for candidate in (None, True, "PRODUCT", {"execution_authorized": True}, p.MemoryTransport([])):
                with self.assertRaises(d.Rejected):
                    p.capture_product(candidate, "/unused")
                with self.assertRaises(d.Rejected):
                    p.ProductTransport(candidate, "/unused")
            network.assert_not_called()
            spawn.assert_not_called()
            files.assert_not_called()

    def test_real_driver_pin_receipt_and_lifecycle_reused_without_reset(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            gate.ledger.add("q-job", wire=123)
            other = p.ReadAdmission(gate.admission, gate.supervisor)
            self.assertIs(other.ledger, gate.ledger)
            self.assertEqual(other.ledger.used["q-job"]["wire"], 123)
            self.assertTrue((Path(td) / "single-use.json").exists())
            cp, gp = (Path(td) / name for name in ("contract.json", "grant.json"))
            with self.assertRaises(FileExistsError):
                d.ExecutionAdmission({"path": str(cp), "sha256": d.sha(cp.read_bytes())}, {"path": str(gp), "sha256": d.sha(gp.read_bytes())})

    def test_mutated_binary_argv_endpoint_owner_deadline_zero_io(self):
        for kind in ("binary", "argv", "endpoint", "identity", "deadline", "caps"):
            with tempfile.TemporaryDirectory() as td:
                gate, _ = gate_fixture(td)
                if kind == "identity":
                    owner = gate.admission.active_owners[tuple(gate.plan["q"]["owner_key"])]
                    owner.identity = dict(owner.identity, start_identity="other")
                elif kind == "endpoint":
                    gate.plan["q"]["endpoint"]["port"] += 1
                elif kind == "deadline":
                    gate.plan["slots"][0]["deadline"] = time.monotonic() - 1
                else:
                    command = gate.admission.contract["commands"][-1]
                    command[{"binary": "binary_sha256", "argv": "argv", "caps": "resource_caps"}[kind]] = "other"
                with patch.object(p.socket, "socket") as network, patch.object(p.subprocess, "Popen") as spawn:
                    with self.assertRaises(d.Rejected):
                        gate.preflight()
                    network.assert_not_called()
                    spawn.assert_not_called()

    def test_binary_file_change_rejected_before_supervisor_or_product_connect(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            Path(gate.plan["reader_binary"]).write_bytes(b"changed actual binary bytes")
            slot = gate.plan["slots"][0]
            with patch.object(gate.supervisor, "ack") as ack, patch.object(p.socket, "socket") as network, patch.object(p.subprocess, "Popen") as spawn:
                with self.assertRaises(d.Rejected):
                    gate.guard(slot, b"{}", slot["deadline"], "before")
                ack.assert_not_called()
                network.assert_not_called()
                spawn.assert_not_called()

    def test_product_transport_requires_its_state_machine_and_no_retry(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            transport = owned_transport(self, gate)
            slot = gate.plan["slots"][0]
            with self.assertRaises(d.Rejected):
                transport.exchange(slot, b"{}", b"{}", slot["deadline"])
            altered = copy.deepcopy(gate.plan)
            altered["capture_id"] = "not-root"
            with self.assertRaises(d.Rejected):
                p.CaptureMachine(altered, gate.ledger, transport)
            machine = p.CaptureMachine(gate.plan, gate.ledger, transport)
            request = machine.request(slot)
            wire = p.http_request(gate.plan["q"]["endpoint"], request)
            gate.ledger.begin(gate.plan, slot, request, wire, time.monotonic())
            with patch.object(gate, "guard", side_effect=d.Rejected("missing OS")), patch.object(p.socket, "socket") as network:
                first = transport.exchange(slot, request, wire, slot["deadline"])
                self.assertFalse(first.complete)
                with self.assertRaises(d.Rejected):
                    transport.exchange(slot, request, wire, slot["deadline"])
                network.assert_not_called()

    def test_supervisor_ack_returning_late_never_reaches_product_or_read_guard(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            slot = gate.plan["slots"][0]
            now = [time.monotonic()]

            def delayed(*args):
                now[0] = slot["deadline"] + .1
                return {}

            with patch.object(p.time, "monotonic", side_effect=lambda: now[0]), \
                    patch.object(gate.supervisor, "ack", side_effect=delayed), patch.object(d, "exchange_frame") as control, \
                    patch.object(p.socket, "socket") as network, patch.object(p.subprocess, "Popen") as spawn:
                with self.assertRaises(d.Rejected):
                    gate.guard(slot, b"{}", slot["deadline"], "before")
                control.assert_not_called()
                network.assert_not_called()
                spawn.assert_not_called()

    def test_post_request_guard_refusal_keeps_full_reply_and_stops_next(self):
        with tempfile.TemporaryDirectory() as td:
            gate, replies = gate_fixture(td)
            transport = owned_transport(self, gate)
            machine = p.CaptureMachine(gate.plan, gate.ledger, transport)

            def guard(slot, request, deadline, phase):
                if phase == "after":
                    raise d.Rejected("原资源/截止的返回后采样失败")

            def exchange(slot, wire, deadline, original):
                reply = copy.deepcopy(replies[0])
                reply.started = original.started
                return reply

            with patch.object(gate, "guard", side_effect=guard) as observed, patch.object(transport, "_http", side_effect=exchange) as http_call:
                result = machine.run()
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            self.assertEqual(machine.captures[0].reply, replies[0].reply)
            self.assertEqual(http_call.call_count, 1)
            self.assertEqual([args.args[-1] for args in observed.call_args_list], ["before", "after"])

    def test_absent_or_legacy_read_guard_denies_product_io(self):
        for response in ({"transport": "DeliveryUnknown", "error": "not implemented"},
                         {"transport": "Received", "response": {"kind": "ACK"}}):
            with tempfile.TemporaryDirectory() as td:
                gate, _ = gate_fixture(td)
                slot = gate.plan["slots"][0]
                transport = owned_transport(self, gate)
                machine = p.CaptureMachine(gate.plan, gate.ledger, transport)
                request = machine.request(slot)
                wire = p.http_request(gate.plan["q"]["endpoint"], request)
                gate.ledger.begin(gate.plan, slot, request, wire, time.monotonic())
                with patch.object(gate.supervisor, "ack", return_value={}), patch.object(d, "exchange_frame", return_value=response), \
                        patch.object(p.socket, "socket") as network, patch.object(p.subprocess, "Popen") as spawn:
                    result = transport.exchange(slot, request, wire, slot["deadline"])
                    self.assertFalse(result.complete)
                    self.assertIn("Error", result.error.replace("Rejected", "Error"))
                    network.assert_not_called()
                    spawn.assert_not_called()

    def test_read_guard_exact_nonce_binding_times_and_usage(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            slot = gate.plan["slots"][0]

            def answer(endpoint, raw, timeout, frame, deadline):
                binding = d.strict(raw)["binding"]
                return {"transport": "Received", "response": {"kind": "READ_ACK", "binding": binding,
                    "checked_at": time.monotonic(), "job_usage": copy.deepcopy(gate.ledger.used),
                    "session_requests": gate.ledger.session_requests}}

            with patch.object(gate.supervisor, "ack", return_value={}), patch.object(d, "exchange_frame", side_effect=answer):
                gate.guard(slot, b"{}", slot["deadline"], "before")
            for kind in ("nonce", "late", "usage"):
                def bad(*args):
                    value = answer(*args)
                    if kind == "nonce":
                        value["response"]["binding"]["nonce"] = "different"
                    elif kind == "late":
                        value["response"]["checked_at"] = slot["deadline"] + 1
                    else:
                        value["response"]["job_usage"]["q-job"]["wire"] = -1
                    return value
                with patch.object(gate.supervisor, "ack", return_value={}), patch.object(d, "exchange_frame", side_effect=bad):
                    with self.assertRaises(d.Rejected):
                        gate.guard(slot, b"{}", slot["deadline"], "before")

    def test_stdin_gate_preserves_raw_bytes_after_ack_and_consumes_command_once(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            slot = next(s for s in gate.plan["slots"] if s["route"] == "financial-basis")
            request = b' { "raw": "\\u4e2d" } \n\x00\xff'
            # 只替身化独立 READ_ACK 这个边界；没有真实服务、进程或请求。
            gate.guarded = (slot["id"], d.sha(request), slot["deadline"], "before")
            gate.ledger.dispatched.add(slot["id"])
            written = bytearray()
            child = Mock()
            child.pid, child.stdin.fileno.return_value = 404, 999
            child.poll.return_value = 0
            child.wait.return_value = 0
            acknowledged = []

            def ack(*args):
                acknowledged.append(args)
                return {"kind": "ACK", "binding": {"UNIT_TEST_ONLY": True}}

            def write(fd, raw):
                self.assertTrue(acknowledged)
                written.extend(raw)
                return len(raw)

            with patch.object(p.subprocess, "Popen", return_value=child) as spawn, \
                    patch.object(d, "process_identity", return_value={"pid": 404, "start_identity": "fake", "process_group": 404}), \
                    patch.object(gate.supervisor, "ack", side_effect=ack), patch.object(p.os, "set_blocking"), patch.object(p.os, "write", side_effect=write):
                owner = p._StdinProcess(gate, slot, request, Path(td) / "client", slot["deadline"])
                control, actual = bytes(written).split(b"\n", 1)
                self.assertEqual(actual, request)
                self.assertEqual(d.strict(control)["argv"], slot["argv"])
                self.assertIn("os.read(0,1)", spawn.call_args.args[0][2])
                with self.assertRaises(d.Rejected):
                    p._StdinProcess(gate, slot, request, Path(td) / "client-again", slot["deadline"])
                self.assertEqual(spawn.call_count, 1)
                self.assertTrue(owner.cleanup("UNIT_TEST_ONLY")["reaped"])

    def test_stdin_ack_failure_never_writes_gate_or_request(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            slot = next(s for s in gate.plan["slots"] if s["route"] == "financial-basis")
            gate.guarded = (slot["id"], d.sha(b"secret-free raw"), slot["deadline"], "before")
            gate.ledger.dispatched.add(slot["id"])
            child = Mock()
            child.pid = 404
            child.poll.return_value = 0
            child.wait.return_value = 0
            with patch.object(p.subprocess, "Popen", return_value=child), \
                    patch.object(d, "process_identity", return_value={"pid": 404, "start_identity": "fake", "process_group": 404}), \
                    patch.object(gate.supervisor, "ack", side_effect=d.Rejected("拒绝 ACK")), patch.object(p.os, "write") as write:
                with self.assertRaises(d.Rejected):
                    p._StdinProcess(gate, slot, b"secret-free raw", Path(td) / "client", slot["deadline"])
                write.assert_not_called()

    def test_real_http_implementation_on_memory_socket_preserves_wire(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            slot = gate.plan["slots"][0]
            body = b' {"raw":"original bytes"} \n'
            incoming = bytearray(http(body))
            channel = Mock()
            channel.__enter__ = Mock(return_value=channel)
            channel.__exit__ = Mock(return_value=False)

            def recv(size):
                chunk = bytes(incoming[:size])
                del incoming[:size]
                return chunk

            channel.recv.side_effect = recv
            transport = owned_transport(self, gate)
            machine = p.CaptureMachine(gate.plan, gate.ledger, transport)
            request = machine.request(slot)
            wire = p.http_request(gate.plan["q"]["endpoint"], request)
            gate.ledger.begin(gate.plan, slot, request, wire, time.monotonic())
            with patch.object(gate, "guard"), patch.object(p.socket, "socket", return_value=channel), patch.object(p.subprocess, "Popen") as spawn:
                result = transport.exchange(slot, request, wire, slot["deadline"])
            self.assertTrue(result.complete, result.error)
            self.assertEqual(result.reply, body)
            self.assertEqual(result.transport_reply, http(body))
            channel.sendall.assert_called_once_with(wire)
            channel.connect.assert_called_once_with(("127.0.0.1", 4567))
            spawn.assert_not_called()


class RepairTests(unittest.TestCase):
    def test_a1_preexisting_cli_directory_never_supplies_originals(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            slot = next(s for s in gate.plan["slots"] if s["route"] == "financial-basis")
            output = Path(gate.extension["output"])
            output.mkdir()
            foreign = output / slot["id"]
            foreign.mkdir()
            for name in ("stdout", "stderr"):
                (foreign / name).write_bytes(b"FOREIGN_PREEXISTING_BYTES")
            request = b"{}"
            gate.guarded = (slot["id"], d.sha(request), slot["deadline"], "before")
            gate.ledger.dispatched.add(slot["id"])
            # 直接测实际 _StdinProcess 构造失败及 _cli finally，传输构造边界另由 A2 锁。
            transport = p.ProductTransport.__new__(p.ProductTransport)
            transport.gate, transport.output = gate, output
            exchange = p.Exchange()
            with patch.object(p.subprocess, "Popen") as spawn:
                with self.assertRaises(FileExistsError) as raised:
                    transport._cli(slot, request, slot["deadline"], exchange)
                spawn.assert_not_called()
            self.assertIsNone(raised.exception.read_owner.directory)
            self.assertFalse(exchange.identity["cleanup"]["reaped"])
            self.assertEqual(exchange.reply, b"")
            self.assertEqual(exchange.stderr, b"")
            self.assertEqual((foreign / "stdout").read_bytes(), b"FOREIGN_PREEXISTING_BYTES")

    def test_a2_direct_transport_rejects_unapproved_or_unowned_output(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            with patch.object(p.socket, "socket") as network, patch.object(p.subprocess, "Popen") as spawn:
                for output in (Path(td) / "unapproved-output", Path(gate.extension["output"])):
                    with self.subTest(output=output), self.assertRaises(d.Rejected):
                        p.ProductTransport(gate, output)
                network.assert_not_called()
                spawn.assert_not_called()
            self.assertFalse(Path(gate.extension["output"]).exists())

    def test_a3_binary_drift_rejected_before_output_creation(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            Path(gate.plan["reader_binary"]).write_bytes(b"changed after admission")
            with patch.object(p.socket, "socket") as network, patch.object(p.subprocess, "Popen") as spawn:
                try:
                    p.capture_product(gate, gate.extension["output"])
                except d.Rejected:
                    pass
                network.assert_not_called()
                spawn.assert_not_called()
            self.assertFalse(Path(gate.extension["output"]).exists())

    def test_a4_formal_low_tool_budget_rejects_raw_input(self):
        plan, replies = fixture()
        plan["verification"].update(read_max_bytes="852", read_max_records="7")
        result, machine, transport = run(plan, replies)
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertIn("读取帽", result["failure"])
        self.assertLessEqual(len(transport.requests), 1)
        if machine.captures:
            self.assertEqual(machine.captures[0].reply, replies[0].reply)


    def cli_originals(self, replacement=None, ack_error=False):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            transport = owned_transport(self, gate)
            slot = next(s for s in gate.plan["slots"] if s["route"] == "financial-basis")
            request = b"raw request\n"
            gate.guarded = (slot["id"], d.sha(request), slot["deadline"], "before")
            gate.ledger.dispatched.add(slot["id"])
            child = Mock()
            child.pid, child.stdin.fileno.return_value = 404, 999
            child.poll.return_value = 9
            child.wait.return_value = 9
            stdout, stderr = b' {"unknown":"actual owner"} \n', b'original failure \x00\xff\n'
            directory = transport.output / slot["id"]

            def spawn(*args, **kwargs):
                for name, raw in (("stdout", stdout), ("stderr", stderr)):
                    kwargs[name].write(raw)
                    kwargs[name].flush()
                return child

            def ack(*args):
                if replacement == "directory":
                    directory.rename(directory.with_name("held-original-directory"))
                    directory.mkdir()
                    for name in ("stdout", "stderr"):
                        (directory / name).write_bytes(b"FOREIGN_REPLACEMENT")
                if replacement == "file":
                    for name in ("stdout", "stderr"):
                        (directory / name).rename(directory / (name + ".original"))
                        (directory / name).write_bytes(b"FOREIGN_REPLACEMENT")
                if ack_error:
                    raise d.Rejected("actual ACK failure")
                return {"kind": "ACK"}

            exchange = p.Exchange()
            with patch.object(p.subprocess, "Popen", side_effect=spawn) as spawned, \
                    patch.object(d, "process_identity", return_value={"pid":404,"start_identity":"fake","process_group":404}), \
                    patch.object(gate.supervisor, "ack", side_effect=ack), patch.object(p.os, "set_blocking"), \
                    patch.object(p.os, "write", side_effect=lambda fd, raw: len(raw)):
                if ack_error:
                    with self.assertRaisesRegex(d.Rejected, "actual ACK failure"):
                        transport._cli(slot, request, slot["deadline"], exchange)
                else:
                    transport._cli(slot, request, slot["deadline"], exchange)
            self.assertEqual(spawned.call_count, 1)
            self.assertEqual(exchange.reply, stdout)
            self.assertEqual(exchange.stderr, stderr)
            self.assertEqual(exchange.returncode, 9)
            self.assertTrue(exchange.identity["cleanup"]["reaped"])
            owner = gate.admission.active_owners[(slot["stage"], "S", "1")]
            self.assertEqual(owner.output_readers, {})
            self.assertIsNone(owner.directory.fd)
            self.assertTrue(owner.stdout.closed and owner.stderr.closed)
            # 封存路径替换时只复制持有原件，不能把外来文件加进清单。
            machine = p.CaptureMachine(gate.plan, gate.ledger, transport)
            machine.originals.append(dict(slot=slot,deadline=slot["deadline"],request=request,
                                          transport_request=request,exchange=exchange))
            result = {"capture_status":"INCOMPLETE"}
            inventory = p.persist_originals(machine, result, transport.directory, gate.admission.seal_deadline)
            self.assertEqual((transport.output / "capture-000000.reply").read_bytes(), stdout)
            self.assertEqual((transport.output / "capture-000000.stderr").read_bytes(), stderr)
            for item in inventory:
                self.assertEqual(d.sha((transport.output / item["path"]).read_bytes()), item["sha256"])
            if replacement:
                self.assertTrue(result["original_path_errors"])
                self.assertFalse(any(i["path"].endswith("/stdout") or i["path"].endswith("/stderr") for i in inventory))
            else:
                self.assertTrue(any(i["path"].endswith("/stdout") for i in inventory))
            return exchange

    def test_a1_owned_error_originals_survive_cleanup(self):
        self.cli_originals(ack_error=True)

    def test_a1_owned_unknown_nonzero_originals_retained(self):
        self.cli_originals()

    def test_a1_replaced_directory_retains_only_held_originals(self):
        exchange = self.cli_originals(replacement="directory", ack_error=True)
        self.assertTrue(exchange.identity["cleanup"]["errors"])

    def test_a1_replaced_output_files_never_supply_foreign_bytes(self):
        self.cli_originals(replacement="file", ack_error=True)

    def test_a2_direct_owned_transport_keeps_legal_capture(self):
        with tempfile.TemporaryDirectory() as td:
            gate, replies = gate_fixture(td)
            transport = owned_transport(self, gate)
            machine = p.CaptureMachine(gate.plan, gate.ledger, transport)
            for slot, reply in zip(gate.plan["slots"], replies):
                if slot["argv"]:
                    reply.identity.update(argv=slot["argv"], binary_sha256_before=gate.plan["source"]["binary_sha256"],
                                          binary_sha256_after=gate.plan["source"]["binary_sha256"])
            pending = iter(replies)
            def receive(slot, raw, deadline, result):
                reply = copy.deepcopy(next(pending))
                reply.started = result.started
                return reply
            with patch.object(gate, "guard"), patch.object(transport,"_http",side_effect=receive) as http_call, \
                    patch.object(transport,"_cli",side_effect=receive) as cli_call, \
                    patch.object(p.socket,"socket") as network, patch.object(p.subprocess,"Popen") as spawn:
                result = machine.run()
            self.assertEqual(result["capture_status"], "UNAUTHENTICATED_RAW_CAPTURE", result["failure"])
            self.assertEqual(http_call.call_count + cli_call.call_count, len(gate.plan["slots"]))
            self.assertEqual([c.reply for c in machine.captures], [r.reply for r in replies])
            spawn.assert_not_called()
            network.assert_not_called()

    def test_a2_wrong_closed_and_replaced_directory_rejected(self):
        for kind in ("wrong", "closed", "replaced", "fake"):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as td:
                gate, _ = gate_fixture(td)
                output = Path(gate.extension["output"])
                directory = d.OwnedDirectory(output if kind != "wrong" else Path(td) / "other")
                try:
                    if kind == "closed": directory.close()
                    if kind == "replaced":
                        output.rename(Path(td) / "old")
                        output.mkdir()
                    with patch.object(p.socket,"socket") as network, patch.object(p.subprocess,"Popen") as spawn:
                        with self.assertRaises(d.Rejected):
                            p.ProductTransport(gate, output, directory=Mock() if kind == "fake" else directory)
                        spawn.assert_not_called()
                        network.assert_not_called()
                finally:
                    directory.close()

    def test_a2_directory_replaced_after_transport_before_send_zero_io(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            transport = owned_transport(self, gate)
            machine = p.CaptureMachine(gate.plan, gate.ledger, transport)
            transport.output.rename(Path(td) / "old")
            transport.output.mkdir()
            with patch.object(gate,"guard") as guard, patch.object(transport,"_http") as http_call:
                result = machine.run()
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            guard.assert_not_called()
            http_call.assert_not_called()

    def test_a3_live_owner_and_earliest_deadline_drift_zero_directory(self):
        for kind in ("session-owner", "q-owner", "work", "capture-stage", "q-job", "request-stage"):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as td:
                gate, _ = gate_fixture(td)
                deadline = None
                if kind.endswith("owner"):
                    role = kind.split("-")[0]
                    owner = gate.admission.active_owners[tuple(gate.plan[role]["owner_key"])]
                    owner.verify_identity.side_effect = d.Rejected("live owner disappeared")
                else:
                    r = gate.plan["resources"]
                    deadline = {"work": r["deadlines"]["work"], "capture-stage": r["deadlines"]["stages"][gate.plan["stage"]],
                        "q-job": r["jobs"]["q-job"]["deadline"], "request-stage": r["deadlines"]["stages"][gate.plan["slots"][0]["stage"]]}[kind]
                with patch.object(p.socket,"socket") as network, patch.object(p.subprocess,"Popen") as spawn, \
                        patch.object(gate.supervisor,"ack") as ack, patch.object(d,"exchange_frame") as control:
                    if deadline is None:
                        with self.assertRaises(d.Rejected): p.capture_product(gate, gate.extension["output"])
                    else:
                        with patch.object(p.time,"monotonic",return_value=deadline), self.assertRaises(d.Rejected):
                            p.capture_product(gate, gate.extension["output"])
                    for call in (network, spawn, ack, control): call.assert_not_called()
                self.assertFalse(Path(gate.extension["output"]).exists())

    def test_a3_send_guard_still_rechecks_binary_after_transport(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            transport = owned_transport(self, gate)
            machine = p.CaptureMachine(gate.plan, gate.ledger, transport)
            Path(gate.plan["reader_binary"]).write_bytes(b"late drift")
            with patch.object(gate.supervisor,"ack") as ack, patch.object(transport,"_http") as http_call:
                result = machine.run()
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            self.assertIn("binary", result["failure"])
            ack.assert_not_called()
            http_call.assert_not_called()

    def test_a4_raw_bytes_count_before_parse_and_keep_whitespace(self):
        plan, replies = fixture()
        raw = b" " * 900 + replies[0].reply + b"\n"
        replies[0].reply, replies[0].transport_reply = raw, http(raw)
        request = p.CaptureMachine(plan, p.Ledger(plan), p.MemoryTransport([])).request(plan["slots"][0])
        first_bytes = len(p.http_request(plan["q"]["endpoint"], request)) + len(http(raw))
        plan["verification"]["read_max_bytes"] = str(first_bytes - 1)
        with patch.object(p.CaptureMachine,"_snapshot",side_effect=AssertionError("未扣原bytes就解析")) as parse:
            result, machine, transport = run(plan, replies)
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertEqual(len(transport.requests), 1)
        self.assertEqual(machine.captures[0].reply, raw)
        self.assertEqual(machine.read_budget.bytes_read, first_bytes)
        parse.assert_not_called()

    def test_a4_byte_budget_accumulates_across_requests(self):
        plan, replies = fixture()
        first_request = p.CaptureMachine(plan,p.Ledger(plan),p.MemoryTransport([])).request(plan["slots"][0])
        first_bytes = len(p.http_request(plan["q"]["endpoint"],first_request)) + len(replies[0].transport_reply)
        plan["verification"]["read_max_bytes"] = str(first_bytes)
        result, machine, transport = run(plan,replies)
        self.assertEqual(result["capture_status"], "INCOMPLETE")
        self.assertEqual(len(transport.requests), 1)
        self.assertGreater(machine.read_budget.bytes_read, first_bytes)
        self.assertIs(machine.read_budget, machine.ledger.read_budget)

    def test_a4_record_budget_counts_nodes_and_projection_work(self):
        for limit in (7, 500):
            plan, replies = fixture()
            plan["verification"]["read_max_records"] = str(limit)
            result, machine, _ = run(plan,replies)
            self.assertEqual(result["capture_status"], "INCOMPLETE")
            self.assertEqual(machine.read_budget.records, limit + 1)
            self.assertIn("读取帽", result["failure"])
        plan, replies = fixture()
        result, machine, _ = run(plan,replies)
        self.assertEqual(result["capture_status"], "UNAUTHENTICATED_RAW_CAPTURE",result["failure"])
        nodes = len(machine.nodes)
        self.assertGreater(machine.read_budget.records, nodes + 2 * len(machine.captures) + len(machine.rows))
        self.assertEqual(machine.read_budget.bytes_read, sum(len(o["transport_request"]) +
            len(o["exchange"].transport_reply) + len(o["exchange"].stderr) for o in machine.originals))

    def test_a4_same_budget_spans_projection_closure_and_final_graph(self):
        plan, replies = fixture()
        references = []
        original_ref, original_decode, original_verify = x.content_ref, x.decode_verified, p.c.verify
        def content(value,budget,*args):
            references.append(budget.budget)
            return original_ref(value,budget,*args)
        def decode(target,nodes,budget):
            references.append(budget.budget)
            return original_decode(target,nodes,budget)
        def verify(target,nodes,source,budget):
            references.append(budget.read_budget)
            return original_verify(target,nodes,source,budget)
        with patch.object(x,"content_ref",side_effect=content), patch.object(x,"decode_verified",side_effect=decode), \
                patch.object(p.c,"verify",side_effect=verify):
            result, machine, _ = run(plan,replies)
        self.assertEqual(result["capture_status"], "UNAUTHENTICATED_RAW_CAPTURE", result["failure"])
        self.assertGreater(len(references), 5)
        self.assertTrue(all(b is machine.ledger.read_budget for b in references))

    def test_a4_budget_not_reset_by_admission_wrapper_or_second_machine(self):
        with tempfile.TemporaryDirectory() as td:
            gate, _ = gate_fixture(td)
            gate.ledger.read_budget.charge((b"prior bytes",))
            before = gate.ledger.read_budget
            other = p.ReadAdmission(gate.admission, gate.supervisor)
            machine = p.CaptureMachine(other.plan,other.ledger,p.MemoryTransport([]))
            self.assertIs(machine.read_budget, before)
            self.assertEqual((before.records,before.bytes_read),(1,11))


if __name__ == "__main__":
    unittest.main()
