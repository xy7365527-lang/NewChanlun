"""C6a 纯数据/替身进程合同：不启动产品、网络、SQLite 或发送真实信号。"""
import copy
import json
from pathlib import Path
import signal
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import tb03c_query_runtime as runtime
from s_socket_client import canonical, digest


class BudgetTests(unittest.TestCase):
    def test_case_deadline_cannot_extend_global_work_window(self):
        now = [100.0]
        b = runtime.Budget(100.0, clock=lambda: now[0])
        b.begin_case()
        self.assertEqual(b.case_end, 400)
        now[0] = 399
        self.assertEqual(b.remaining(30), 1)
        now[0] = 500
        b.case_end = b.work_end
        b.begin_case()
        self.assertEqual(b.case_end, 700)
        now[0] = 700
        with self.assertRaises(TimeoutError):
            b.check()
        self.assertEqual(b.check(final=True), 30)
        now[0] = 730
        with self.assertRaises(TimeoutError):
            b.check(final=True)

    def test_cancel_stops_work_but_does_not_disable_final_cleanup(self):
        b = runtime.Budget(0, clock=lambda: 10)
        b.cancel = signal.SIGTERM
        with self.assertRaises(InterruptedError):
            b.check()
        self.assertEqual(b.remaining(3, final=True), 3)


class InputTests(unittest.TestCase):
    def test_frozen_oracles_and_we5_final_view(self):
        seed, plan, oracle = runtime.load_inputs()
        self.assertEqual(len(plan["steps"]), 20)
        expected = runtime.We5Expected()
        for step in plan["steps"]:
            expected.advance(step)
        view = expected.book.view()
        self.assertEqual((view["q"], view["K"], view["Pi"]), (3, 305, 19))
        self.assertIsNone(view["P"])
        self.assertEqual(expected.resolved, ["r1", "r6"])
        self.assertEqual((len(expected.facts), len(expected.applied)), (6, 5))
        self.assertEqual(oracle.HAND["reports"][-1]["K"], [204, 506, 506])
        self.assertEqual((seed["startup_timeout_ms"], seed["exchange_timeout_ms"], seed["process_timeout_ms"]),
                         (10000, 30000, 30000))

    def test_each_frozen_input_hash_failure_stops_before_oracle_import(self):
        original = runtime.sha
        for name in runtime.INPUT_NAMES:
            def changed(path):
                return "0" * 64 if Path(path) == runtime.HERE / name else original(path)
            with self.subTest(name=name), patch.object(runtime, "sha", side_effect=changed), \
                    patch.object(runtime.importlib.util, "spec_from_file_location") as importer:
                with self.assertRaises(ValueError):
                    runtime.load_inputs()
                importer.assert_not_called()

    def test_we7_manifest_uses_exact_21_write_slots_and_frozen_money_inputs(self):
        seed, _, oracle = runtime.load_inputs()
        original = copy.deepcopy(seed)
        manifest = runtime.we7_manifest(seed, oracle)
        self.assertEqual(seed, original)
        messages = manifest["clock"]["plan"]["messages"]
        self.assertEqual(set(messages), {f"we7-{i:03d}" for i in range(21)})
        floor = 1
        for value in messages.values():
            values = [int(value[k]) for k in ("received_ns", "first_known_ns", "commit_ns")]
            self.assertLess(floor, values[0])
            self.assertEqual(values, sorted(values))
            floor = values[-1]
        self.assertEqual(manifest["fill_manifests"], [oracle.IndependentLedger().manifest])
        self.assertEqual(manifest["fill_allocation_policy"]["fill_allocation_policy_id"], oracle.POLICY)
        self.assertEqual(manifest["resources"], seed["manifest"]["resources"])

    def test_build_evidence_rejects_every_identity_mismatch(self):
        sources = {p: "1" * 64 for p in runtime.SOURCE_FILES}
        binaries = {p: "2" * 64 for p in ("e_economic_session", "b_chong_session")}
        build = {"source_sha": "a" * 40, "source_tree": "b" * 40, "command": ["cargo", "build"],
                 "exit_code": 0, "source_files": sources, "binaries": binaries}
        runtime.verify_build(build, "a" * 40, "b" * 40, sources, binaries)
        changes = [("source_sha", "c" * 40), ("source_tree", "d" * 40), ("exit_code", False),
                   ("exit_code", 1), ("command", []), ("source_files", {}), ("binaries", {})]
        for key, value in changes:
            bad = copy.deepcopy(build)
            bad[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                runtime.verify_build(bad, "a" * 40, "b" * 40, sources, binaries)

    def test_we5_payload_no_unbound_fact_fallback(self):
        _, plan, _ = runtime.load_inputs()
        apply = next(s for s in plan["steps"] if s["op"] == "apply")
        with self.assertRaises(KeyError):
            runtime.we5_payload(apply, {})


class SupervisorTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="c6a-supervisor-contract-")
        self.addCleanup(self.tmp.cleanup)
        self.now = [0.0]
        self.budget = runtime.Budget(0, clock=lambda: self.now[0])
        self.supervisor = runtime.Supervisor(Path(self.tmp.name), self.budget)
        self.addCleanup(self.supervisor.log.close)
        self.signals = patch.object(runtime.os, "kill")
        self.kill = self.signals.start()
        self.addCleanup(self.signals.stop)
        self.child_signal = patch.object(runtime.signal, "getsignal", return_value=signal.SIG_DFL)
        self.child_signal.start()
        self.addCleanup(self.child_signal.stop)

    def proc(self, returncode=None):
        p = SimpleNamespace(pid=41001, returncode=returncode, wait=Mock())
        self.supervisor.children[id(p)] = {"proc": p, "label": "fake", "command": ["fake"],
                                          "stdout": Path(self.tmp.name) / "fake.stdout",
                                          "stderr": Path(self.tmp.name) / "fake.stderr"}
        return p

    def test_reaped_child_is_evidence_only_no_signal_or_wait(self):
        p = self.proc(0)
        self.supervisor.stop(p)
        self.kill.assert_not_called()
        p.wait.assert_not_called()
        self.assertFalse(self.supervisor.children)
        with self.assertRaises(ValueError):
            self.supervisor.stop(p)
        self.kill.assert_not_called()

    def test_term_wait_before_optional_kill_and_never_after_reap(self):
        p = self.proc()
        order = []
        self.kill.side_effect = lambda pid, sig: order.append(("signal", pid, sig))
        def wait(timeout):
            order.append(("wait", timeout))
            if len(order) == 2:
                raise subprocess.TimeoutExpired("fake", timeout)
            p.returncode = -9
            return -9
        p.wait.side_effect = wait
        self.supervisor.stop(p)
        self.assertEqual(order, [("signal", p.pid, signal.SIGTERM), ("wait", 3),
                                 ("signal", p.pid, signal.SIGKILL), ("wait", 3)])
        self.assertFalse(self.supervisor.children)

    def test_term_reaped_stops_without_kill(self):
        p = self.proc()
        def wait(timeout):
            p.returncode = -15
            return -15
        p.wait.side_effect = wait
        self.supervisor.stop(p)
        self.kill.assert_called_once_with(p.pid, signal.SIGTERM)

    def test_expired_final_deadline_never_sends_signal(self):
        p = self.proc()
        self.now[0] = 630
        with self.assertRaises(TimeoutError):
            self.supervisor.stop(p)
        self.kill.assert_not_called()
        self.assertIn(id(p), self.supervisor.children)

    def test_default_sigchld_and_owner_thread_required(self):
        p = self.proc()
        with patch.object(runtime.signal, "getsignal", return_value=signal.SIG_IGN), self.assertRaises(ValueError):
            self.supervisor.stop(p)
        with patch.object(runtime.threading, "get_ident", return_value=-1), self.assertRaises(ValueError):
            self.supervisor.stop(p)
        self.kill.assert_not_called()

    def test_unknown_child_and_registry_replacement_cannot_be_signalled(self):
        p = self.proc()
        self.supervisor.children[id(p)]["proc"] = object()
        with self.assertRaises(ValueError):
            self.supervisor.stop(p)
        self.kill.assert_not_called()

    def test_cancellation_during_spawn_retains_child_for_cleanup(self):
        p = SimpleNamespace(pid=41002, returncode=None, wait=Mock())
        def spawn(*args, **kwargs):
            self.budget.cancel = signal.SIGTERM
            return p
        with patch.object(runtime.subprocess, "Popen", side_effect=spawn), self.assertRaises(InterruptedError):
            self.supervisor.spawn(["fake"], "cancel")
        self.assertIs(self.supervisor.children[id(p)]["proc"], p)
        self.assertTrue(self.supervisor.events[0]["direct_child"])
        self.kill.assert_not_called()

    def test_cli_timeout_retains_exact_handle_and_logs(self):
        p = SimpleNamespace(pid=41003, returncode=None, wait=Mock(side_effect=subprocess.TimeoutExpired("fake", 30)))
        with patch.object(runtime.subprocess, "Popen", return_value=p), self.assertRaises(subprocess.TimeoutExpired):
            self.supervisor.run(["fake"], "timeout")
        self.assertIs(self.supervisor.children[id(p)]["proc"], p)
        self.assertTrue(self.supervisor.children[id(p)]["stdout"].exists())
        self.kill.assert_not_called()

    def test_failed_reap_preserves_registry_and_original_streams(self):
        p = self.proc()
        p.wait.side_effect = subprocess.TimeoutExpired("fake", 3)
        with self.assertRaises(RuntimeError):
            self.supervisor.close()
        self.assertIn(id(p), self.supervisor.children)
        self.assertEqual(self.kill.call_count, 2)
        self.assertFalse(self.supervisor.log.closed)

    def test_failed_cleanup_is_not_retried_with_new_signal_window(self):
        p = self.proc()
        p.wait.side_effect = subprocess.TimeoutExpired("fake", 3)
        with self.assertRaises(TimeoutError):
            self.supervisor.stop(p)
        self.assertEqual(self.kill.call_count, 2)
        with self.assertRaises(RuntimeError):
            self.supervisor.close()
        self.assertEqual(self.kill.call_count, 2)
        self.assertIn(id(p), self.supervisor.children)

    def test_esrch_still_waits_only_the_same_direct_child(self):
        p = self.proc()
        self.kill.side_effect = ProcessLookupError
        def wait(timeout):
            p.returncode = 0
            return 0
        p.wait.side_effect = wait
        self.supervisor.stop(p)
        p.wait.assert_called_once_with(timeout=3)
        self.kill.assert_called_once_with(p.pid, signal.SIGTERM)


class PublicContractTests(unittest.TestCase):
    @staticmethod
    def observed(seq="0"):
        image = {"book": None, "recorded": [], "pending": [], "applied": [], "unmatched": [], "history": []}
        return {"E": {"cut": {"domain_id": "E", "commit_seq": seq, "root_hash": digest(seq)},
                      "commit_ns": seq, "image": image, "image_hash": digest(image)}}

    def test_public_snapshot_rejects_mutated_bytes_even_rehashed(self):
        owner = self.observed()
        snapshot = {"kind": "EconomicSnapshot", "completeness": "CompleteVector",
                    "domains": {"E": {"status": "Available", **copy.deepcopy(owner["E"])}}}
        runtime.check_snapshot(snapshot, owner)
        snapshot["domains"]["E"]["image"]["book"] = {"injected_money": "1"}
        snapshot["domains"]["E"]["image_hash"] = digest(snapshot["domains"]["E"]["image"])
        with self.assertRaises(ValueError):
            runtime.check_snapshot(snapshot, owner)

    def test_public_watch_missing_duplicate_wrong_cut_and_money_reject(self):
        before, after = self.observed(), self.observed("1")
        changes = [{k: after["E"][k] for k in ("cut", "commit_ns", "image", "image_hash")}]
        watch = {"kind": "EconomicWatch", "domains": {"E": {"status": "Available", "base_cut": before["E"]["cut"],
                   "next_cut": after["E"]["cut"], "changes": changes}}}
        runtime.check_watch(watch, before, after)
        for mutate in (lambda x: x["domains"]["E"].update(changes=[]),
                       lambda x: x["domains"]["E"].update(changes=changes * 2),
                       lambda x: x["domains"]["E"].update(next_cut=before["E"]["cut"]),
                       lambda x: x["domains"]["E"]["changes"][0]["image"].update(book={"money": "1"})):
            bad = copy.deepcopy(watch)
            mutate(bad)
            with self.assertRaises(ValueError):
                runtime.check_watch(bad, before, after)

    def test_raw_http_parser_requires_exact_content_length_and_status(self):
        body = canonical({"book_revision": "tb03c-book/1"})
        good = b"HTTP/1.0 200 OK\r\nContent-Length: " + str(len(body)).encode() + b"\r\n\r\n" + body
        self.assertEqual(runtime.parse_http(good), {"book_revision": "tb03c-book/1"})
        for bad in (good.replace(b"200", b"500", 1), good[:-1], good + b"x",
                    good.replace(b"\r\n\r\n", b"\r\nContent-Length: 1\r\n\r\n"),
                    good.replace(b"\r\n\r\n", b"\r\nTransfer-Encoding: chunked\r\n\r\n")):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                runtime.parse_http(bad)

    def test_finalization_cannot_pass_with_child_or_primary_failure(self):
        with tempfile.TemporaryDirectory(prefix="c6a-final-contract-") as temp:
            b = runtime.Budget(0, clock=lambda: 1)
            supervisor = SimpleNamespace(children={})
            self.assertEqual(runtime.finalize(Path(temp), b, supervisor, [{}, {}], {"error": "x"}, None), 1)
            self.assertEqual(json.loads((Path(temp) / "RESULT.json").read_bytes())["status"], "FAIL")

    def test_late_finalization_does_not_write_a_pass(self):
        with tempfile.TemporaryDirectory(prefix="c6a-late-contract-") as temp:
            b = runtime.Budget(0, clock=lambda: 630)
            with self.assertRaises(TimeoutError):
                runtime.finalize(Path(temp), b, SimpleNamespace(children={}), [{}, {}], None, None)
            self.assertFalse((Path(temp) / "RESULT.json").exists())

    def test_wire_failure_preserves_partial_raw_response(self):
        with tempfile.TemporaryDirectory(prefix="c6a-wire-contract-") as temp:
            c = runtime.Case.__new__(runtime.Case)
            c.sequence, c.output, c.budget = 0, Path(temp), runtime.Budget(0, clock=lambda: 1)
            fake = Mock()
            fake.__enter__ = Mock(return_value=fake)
            fake.__exit__ = Mock(return_value=False)
            fake.recv.side_effect = [b"partial-non-json\xff", TimeoutError("mock timeout")]
            with patch.object(runtime.socket, "socket", return_value=fake), self.assertRaises(TimeoutError):
                c.wire("/not-a-real-socket", b"request\n")
            self.assertEqual((Path(temp) / "0001.owner-response.bin").read_bytes(), b"partial-non-json\xff")
            self.assertTrue((Path(temp) / "0001-transport.json").exists())


class FinalSealTests(unittest.TestCase):
    def test_source_hash_cancellation_and_late_return_stop_before_finalization(self):
        for cause in ("cancel", "deadline"):
            now = [10.0]
            budget = runtime.Budget(0, clock=lambda: now[0])
            def sha(path):
                if cause == "cancel":
                    budget.receive_cancel(signal.SIGTERM)
                else:
                    now[0] = 600
                return "expected"
            with self.subTest(cause=cause), patch.object(runtime, "sha", side_effect=sha), \
                    self.assertRaises(InterruptedError if cause == "cancel" else TimeoutError):
                runtime.verify_unchanged(budget, {"source": "expected"})

    def test_cancel_at_each_finalization_boundary_invalidates_candidate(self):
        for stage in ("entry", "source_hash", "artifact_write", "manifest_hash", "result_write", "sealed"):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory(prefix="c6a-seal-") as temp:
                output = Path(temp)
                (output / "source.bin").write_bytes(b"original")
                budget = runtime.Budget(0, clock=lambda: 10)
                if stage == "entry":
                    budget.receive_cancel(signal.SIGTERM)
                original_save, original_sha = runtime.save, runtime.sha
                def save(path, value):
                    original_save(path, value)
                    if (stage == "artifact_write" and Path(path).name == "ARTIFACTS.json") or (stage == "result_write" and Path(path).name == "RESULT.json"):
                        budget.receive_cancel(signal.SIGTERM)
                def sha(path):
                    value = original_sha(path)
                    if (stage == "source_hash" and Path(path).name == "source.bin") or (stage == "manifest_hash" and Path(path).name == "ARTIFACTS.json"):
                        budget.receive_cancel(signal.SIGINT)
                    return value
                with patch.object(runtime, "save", side_effect=save), patch.object(runtime, "sha", side_effect=sha):
                    result = runtime.finalize(output, budget, SimpleNamespace(children={}), [{}, {}], None, None)
                receipt_bytes = (output / "RESULT.json").read_bytes()
                receipt = json.loads(receipt_bytes)
                if stage == "sealed":
                    self.assertEqual(result, 0)
                    self.assertEqual(receipt["status"], "CANDIDATE_C6A_NORMAL_ONLY")
                    with self.assertRaises(SystemExit) as error:
                        budget.receive_cancel(signal.SIGTERM)
                    self.assertEqual(error.exception.code, 1)
                    self.assertEqual((output / "RESULT.json").read_bytes(), receipt_bytes)
                    self.assertEqual(json.loads((output / "CANCELLED-AFTER-SEAL.json").read_bytes())["status"], "FAIL")
                else:
                    self.assertEqual(result, 1)
                    self.assertTrue((output / "FINALIZATION-REJECTED.json").exists())
                    if stage != "result_write":
                        self.assertEqual(receipt["status"], "FAIL")
                        self.assertIsNotNone(receipt["cancellation"])

    def test_sealed_cancel_write_failure_still_exits_nonzero(self):
        budget = runtime.Budget(0, clock=lambda: 10)
        budget.sealed_output = Path("/not-opened-by-mock")
        with patch.object(runtime, "save", side_effect=OSError("mock denied")), self.assertRaises(SystemExit) as error:
            budget.receive_cancel(signal.SIGINT)
        self.assertEqual(error.exception.code, 1)

    def test_late_write_is_append_only_rejection(self):
        with tempfile.TemporaryDirectory(prefix="c6a-late-seal-") as temp:
            now = [100.0]
            budget = runtime.Budget(0, clock=lambda: now[0])
            original = runtime.save
            def save(path, value):
                original(path, value)
                if Path(path).name == "RESULT.json":
                    now[0] = 631
            with patch.object(runtime, "save", side_effect=save), self.assertRaises(TimeoutError):
                runtime.finalize(Path(temp), budget, SimpleNamespace(children={}), [{}, {}], None, None)
            self.assertEqual(json.loads((Path(temp) / "RESULT.json").read_bytes())["status"], "CANDIDATE_C6A_NORMAL_ONLY")
            self.assertEqual(json.loads((Path(temp) / "FINALIZATION-REJECTED.json").read_bytes())["status"], "FAIL")


class StartupTests(unittest.TestCase):
    def run_start(self, delay_target=None):
        with tempfile.TemporaryDirectory(prefix="c6a-startup-") as temp:
            now = [0.0]
            budget = runtime.Budget(0, clock=lambda: now[0])
            supervisor = SimpleNamespace(budget=budget, run=Mock(return_value=canonical(
                {"kind": "Initialized", "domain_id": "E", "writer_epoch": "1"})), spawn=Mock(return_value=object()))
            seed, _, _ = runtime.load_inputs()
            manifest = copy.deepcopy(seed["manifest"])
            manifest["chongs"] = []
            case = runtime.Case("probe", Path(temp) / "case", supervisor, Path(temp), manifest)
            deadlines = []
            def owner(*args, deadline=None):
                deadlines.append(("owner", deadline))
                if delay_target == "owner":
                    now[0] += 11
                return {"kind": "EconomicView"}
            def http(*args, deadline=None):
                deadlines.append(("Q", deadline))
                if delay_target == "Q":
                    now[0] += 11
                return {"manifest_hash": digest(case.manifest), "book_revision": "tb03c-book/1", "domains": ["E"],
                        "session_id": case.manifest["session_id"], "session_generation": case.manifest["session_generation"], "producer_epoch": "1"}
            case.owner, case.http = owner, http
            fake = Mock()
            fake.__enter__ = Mock(return_value=fake)
            fake.__exit__ = Mock(return_value=False)
            fake.getsockname.return_value = ("127.0.0.1", 19001)
            with patch.object(runtime.socket, "socket", return_value=fake), patch.object(Path, "is_socket", return_value=True):
                case.start()
            self.assertEqual(deadlines, [("owner", 10), ("Q", 10)])

    def test_ready_within_absolute_startup_deadlines(self):
        self.run_start()

    def test_owner_and_q_late_success_are_rejected(self):
        for target in ("owner", "Q"):
            with self.subTest(target=target), self.assertRaises(TimeoutError):
                self.run_start(target)

    def test_wire_uses_startup_deadline_for_connect_send_receive_and_return(self):
        for delay_at in ("connect", "send", "recv", None):
            with self.subTest(delay_at=delay_at), tempfile.TemporaryDirectory(prefix="c6a-wire-end-") as temp:
                now = [0.0]
                c = runtime.Case.__new__(runtime.Case)
                c.sequence, c.output = 0, Path(temp)
                c.budget = runtime.Budget(0, clock=lambda: now[0])
                fake = Mock()
                fake.__enter__ = Mock(return_value=fake)
                fake.__exit__ = Mock(return_value=False)
                def delay(stage):
                    if delay_at == stage:
                        now[0] = 11
                fake.connect.side_effect = lambda address: delay("connect")
                fake.sendall.side_effect = lambda data: delay("send")
                def recv(size):
                    delay("recv")
                    return b""
                fake.recv.side_effect = recv
                with patch.object(runtime.socket, "socket", return_value=fake):
                    if delay_at is None:
                        self.assertEqual(c.wire("fake", b"request", deadline=10), b"")
                    else:
                        with self.assertRaises(TimeoutError):
                            c.wire("fake", b"request", deadline=10)
                self.assertTrue(fake.settimeout.call_args_list)
                self.assertTrue(all(0 < call.args[0] <= 10 for call in fake.settimeout.call_args_list))

    def test_spawn_inherits_outer_driver_group(self):
        with tempfile.TemporaryDirectory(prefix="c6a-group-") as temp:
            supervisor = runtime.Supervisor(Path(temp), runtime.Budget(0, clock=lambda: 1))
            try:
                child = SimpleNamespace(pid=12345, returncode=None)
                with patch.object(runtime.subprocess, "Popen", return_value=child) as popen, \
                     patch.object(runtime.signal, "getsignal", return_value=signal.SIG_DFL):
                    supervisor.spawn(["not-executed"], "fake")
                self.assertFalse(popen.call_args.kwargs["start_new_session"])
                self.assertNotIn("preexec_fn", popen.call_args.kwargs)
                self.assertTrue(supervisor.events[0]["inherits_driver_group"])
            finally:
                supervisor.log.close()


class We7FakeCase:
    """Rust 序列化字段形状的纯数据替身；不用被测 We7Expected 生成图像。

    金额来自固定 HAND，全部身份/证明来自该替身先前提交。不是 owner 运行证据。
    """
    def __init__(self, output, oracle, mutation=None, *, manifest=None):
        self.output, self.oracle, self.mutation = Path(output), oracle, mutation
        if manifest is None:
            seed = json.loads((runtime.HERE / "fixtures/tb03c_runtime/SEED.json").read_text())
            manifest = runtime.we7_manifest(seed, oracle)
            manifest["session_id"] = "tb1406-c6a-we7"
        self.manifest = copy.deepcopy(manifest)
        # 替身 E 配置和真实 Config 同取启动 manifest；不用 We7Expected 造实际原件。
        self.e_identity = {"domain_id": "E", "session_id": self.manifest["session_id"],
                           "session_generation": self.manifest["session_generation"]}
        self.reference = oracle.IndependentLedger()
        self.raw = (runtime.HERE / "fixtures/tb03c_allocation/RAW.json").read_text()
        self.raw_sha = runtime.hashlib.sha256(self.raw.encode()).hexdigest()
        self.responses, self.capture_proofs, self.resolution_proofs = {}, {}, {}
        self.images = {"E": self.empty()}
        for p in self.reference.manifest["participants"]:
            self.images["B:" + p["chong_id"]] = self.empty()
        self.seqs = {d: 0 for d in self.images}

    @staticmethod
    def empty():
        return {"book": None, "recorded": [], "pending": [], "applied": [], "unmatched": [], "history": []}

    def start(self):
        pass

    def finish(self):
        pass

    def cut(self, domain):
        return {"domain_id": domain, "commit_seq": str(self.seqs[domain]), "root_hash": digest([domain, self.seqs[domain]])}

    def owner(self, domain, payload, message):
        if message in self.responses:
            return copy.deepcopy(self.responses[message])
        previous_root = self.cut(domain)["root_hash"]
        self.seqs[domain] += 1
        ns, op = str(self.seqs[domain]), payload["op"]
        if op == "CaptureExternal":
            index = int(payload["json_pointer"].split("/")[-1])
            event = self.reference.raw["events"][index]
            key = runtime.fact_key(event)
            target = "B:" + event["book_identity"]["chong_id"] if index < 3 else "E:netted:" + self.reference.manifest["manifest_hash"]
            self.images["E"]["recorded"].append({"fact_key": key, "fact_hash": runtime.fact_hash(self.raw, payload["json_pointer"]),
                "raw_utf8": self.raw, "raw_sha256": self.raw_sha, "json_pointer": payload["json_pointer"], "event": event, "target_domain": target})
            self.images["E"]["pending"].append({"fact_key": key, "token": payload["token"], "target_domain": target,
                "scope": target, "status": "Pending", "reason": {"kind": "Unresolved"}})
            self.images["E"]["history"].append({"kind": "CaptureExternal", "fact_key": key})
            if index >= 3:
                self.reference.capture(index)
            reply = {"kind": "Recorded", "fact_key": key, "fact_hash": runtime.fact_hash(self.raw, payload["json_pointer"]), "cut": self.cut("E")}
            self.capture_proofs[key] = {"cut": self.cut("E"), "e_receipt": copy.deepcopy(reply),
                "e_commit_proof": {"previous_root": previous_root, "commit_ns": ns, "image": copy.deepcopy(self.images["E"])}}
        elif op == "ResolveEvidence":
            key = payload["covered"][0]["fact_key"]
            index, event = next((i, e) for i, e in enumerate(self.reference.raw["events"]) if runtime.fact_key(e) == key)
            self.reference.resolve(event["source"]["external_fact_id"])
            hand = self.oracle.HAND["reports"][index - 3]
            row = {"fact_key": key, "fact_hash": runtime.fact_hash(self.raw, f"/events/{index}"),
                "manifest_hash": self.reference.manifest["manifest_hash"], "symbol": self.reference.manifest["symbol"],
                "fill_allocation_policy_id": self.oracle.POLICY, "policy_revision": "we7-frozen-1",
                "from": str(hand["from"]), "to": str(hand["to"]), "evidence_id": payload["evidence_id"], "allocations": []}
            for i, participant in enumerate(self.reference.manifest["participants"]):
                row["allocations"].append({**{k: participant[k] for k in ("chong_id", "command_id", "family_id", "revision")},
                    "allocation_id": "netted:" + digest([key, event["allocation_id"], row["manifest_hash"], participant]),
                    "units": str(hand["quantity"][i]), "commission": str(hand["fees"][i]),
                    "acquisition_cash": str(hand["quantity"][i] * hand["price"])})
            self.images["E"].setdefault("fill_allocations", []).append(row)
            self.images["E"]["pending"] = [p for p in self.images["E"]["pending"] if p["fact_key"] != key]
            self.images["E"]["history"].append({"kind": "ResolveEvidence", "evidence_id": payload["evidence_id"], "covered": [key]})
            reply = {"kind": "Resolved", "cut": self.cut("E")}
            self.resolution_proofs[key] = {"image": copy.deepcopy(self.images["E"]), "receipt": copy.deepcopy(reply)}
        else:
            key = payload["fact_key"]
            index, event = next((i, e) for i, e in enumerate(self.reference.raw["events"]) if runtime.fact_key(e) == key)
            i = next(i for i, p in enumerate(self.reference.manifest["participants"]) if domain == "B:" + p["chong_id"])
            source = copy.deepcopy(next(r for r in self.images["E"]["recorded"] if r["fact_key"] == key))
            source.update(copy.deepcopy(self.capture_proofs[key]))
            source.update({"e_" + key: value for key, value in self.e_identity.items()})
            if index < 3:
                self.images[domain]["book"] = self.open_book(index)
            else:
                self.reference.apply(event["source"]["external_fact_id"], i)
                resolution = self.resolution_proofs[key]
                row = next(r for r in resolution["image"]["fill_allocations"] if r["fact_key"] == key)
                source.update(resolved_allocation=copy.deepcopy(row["allocations"][i]), e_resolution_proof=copy.deepcopy(resolution))
                book = self.images[domain]["book"]
                hand = self.oracle.HAND["reports"][index - 3]
                if hand["quantity"][i]:
                    for kind, amount in (("acquisition_cash", hand["quantity"][i] * hand["price"]), ("commission", hand["fees"][i])):
                        book["known_cost_components"].append({"kind": kind, "canonical_amount": str(amount), "fact_key": key,
                            "raw_sha256": self.raw_sha, "derivation": {"kind": "E_resolved_netted_allocation",
                            "allocation_id": payload["allocation_id"], "manifest_hash": row["manifest_hash"], "resolve_cut": resolution["receipt"]["cut"]}})
                        book["cost_totals"][kind] = str(int(book["cost_totals"][kind]) + amount)
                book["actual_quantity"] = str(int(book["actual_quantity"]) + hand["quantity"][i])
                cost = int(book["cost_totals"]["acquisition_cash"]) + int(book["cost_totals"]["commission"])
                fees = int(book["cost_totals"]["commission"])
                book["cost_basis"]["K"] = book["cost_remaining"]["value"] = book["unit_cost"]["K"] = str(cost)
                book["unit_cost"]["q"] = book["actual_quantity"]
                book["realized"].update(fees=str(fees), after_fees=str(-fees))
                book["stable_point"] = {"profile": "TestOnly", "seq": event["stable_point"]["seq"]}
            self.images[domain]["recorded"].append(source)
            self.images[domain]["applied"].append({"allocation_id": payload["allocation_id"], "fact_key": key,
                "fact_hash": payload["expected_fact_hash"], "target_domain": domain, "status": "Applied", "commit_seq": ns, "commit_ns": ns})
            reply = {"kind": "Applied"}
        self.responses[message] = copy.deepcopy(reply)
        return reply

    def open_book(self, index):
        event = self.reference.raw["events"][index]
        identity, o = event["book_identity"], event["observed_existing_book"]
        key = runtime.fact_key(event)
        cost = sum(int(c["canonical_amount"]) for c in event["known_cost_components"])
        return {"book_revision": "tb03c-book/1", "chong_id": identity["chong_id"], "symbol": identity["symbol"],
            "operation_level": identity["op_level"], "fixed_q": identity["fixed_q"], "side": "long", "units": self.reference.raw["units"],
            "family_id": event["family_id"], "opening_fact_ref": {"fact_key": key, "fact_hash": runtime.fact_hash(self.raw, f"/events/{index}"),
                "e_cut": self.capture_proofs[key]["cut"], "json_pointer": f"/events/{index}", "raw_sha256": self.raw_sha},
            "opening_quantity": o["units"], "actual_quantity": o["units"],
            "income_book": {"Pi": "0", "A": "0", "W": "0", "R": "0"},
            "known_cost_components": [{**c, "fact_key": key, "raw_sha256": self.raw_sha} for c in event["known_cost_components"]],
            "cost_totals": {"acquisition_cash": str(cost), "commission": "0", "disposal_cash": "0", "fee_only": "0", "dividend_cash": "0", "withholding_tax": "0"},
            "realized": {"gross": "0", "fees": "0", "after_fees": "0"},
            "principal": {"batches": [], "P_recovered": "0", "status": "NotEvaluable", "reason": "no authorized principal batch"},
            "phase": {"latch": "CostReduction", "events": [], "gate": None, "growth_eligible": {"status": "NotEvaluable"},
                      "growth_enabled": {"status": "NotEvaluable", "reason": "principal batches absent"}},
            "unknown": {"complete_historical_fees": {"status": "Unknown", "reason": "only named known components available"},
                        "target": {"status": "Unknown", "reason": "not supplied"}},
            "voice": {"status": "MissingDependency", "reason": "confirmed gamma binding absent", "identity": None},
            "admission": "NotEvaluated", "cost_remaining": {"status": "Known", "value": str(cost)},
            "cost_basis": {"status": "Known", "K": str(cost)}, "unit_cost": {"status": "Known", "K": str(cost), "q": o["units"]},
            "stable_point": {"profile": "TestOnly", "seq": event["stable_point"]["seq"]}}

    def observe(self):
        images = copy.deepcopy(self.images)
        if self.mutation:
            self.mutation(images)
        return {d: {"cut": self.cut(d), "commit_ns": str(self.seqs[d]), "image": image, "image_hash": digest(image)} for d, image in images.items()}

    def snapshot(self):
        return {"kind": "EconomicSnapshot", "completeness": "CompleteVector",
                "domains": {d: {"status": "Available", **v} for d, v in self.observe().items()}}

    def check_public(self, before, after, label):
        snapshot = self.snapshot()
        runtime.check_snapshot(snapshot, after)
        changes = {}
        for d, old in before.items():
            new = after[d]
            changes[d] = {"status": "Available", "base_cut": old["cut"], "next_cut": new["cut"],
                "changes": [] if old["cut"] == new["cut"] else [{k: new[k] for k in ("cut", "commit_ns", "image", "image_hash")}]}
        runtime.check_watch({"kind": "EconomicWatch", "domains": changes}, before, after)
        return snapshot


class We7OracleTests(unittest.TestCase):
    def execute(self, mutation=None, *, manifest=None):
        _, _, oracle = runtime.load_inputs()
        with tempfile.TemporaryDirectory(prefix="c6a-we7-pure-") as temp:
            return runtime.execute_we7(We7FakeCase(temp, oracle, mutation, manifest=manifest), oracle)

    def test_complete_21_step_normal_control(self):
        result = self.execute()
        self.assertEqual(result["steps"], 21)
        self.assertEqual((result["q"], result["K"]), ([2, 5, 5], [204, 506, 506]))

    def test_b_source_identity_wrong_fields_are_rejected(self):
        for field, value in (("e_domain_id", "NOT-E"), ("e_session_id", "foreign-session"),
                             ("e_session_generation", "999")):
            def mutate(images):
                for domain, image in images.items():
                    if domain != "E":
                        for record in image["recorded"]:
                            record[field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "原 E 来源身份 " + field):
                self.execute(mutate)

    def test_b_source_identity_missing_fields_are_rejected(self):
        for field in ("e_domain_id", "e_session_id", "e_session_generation"):
            def mutate(images):
                for domain, image in images.items():
                    if domain != "E":
                        for record in image["recorded"]:
                            del record[field]
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, "原 E 来源身份缺 " + field):
                self.execute(mutate)

    def test_source_identity_tracks_nondefault_manifest_for_all_21_steps(self):
        seed, _, oracle = runtime.load_inputs()
        manifest = runtime.we7_manifest(seed, oracle)
        manifest.update(session_id="c6a-other-source-session", session_generation="7")
        with tempfile.TemporaryDirectory(prefix="c6a-we7-identity-") as temp:
            case = We7FakeCase(temp, oracle, manifest=manifest)
            result = runtime.execute_we7(case, oracle)
            self.assertEqual(result["steps"], 21)
            expected = {"e_domain_id": "E", "e_session_id": manifest["session_id"],
                        "e_session_generation": manifest["session_generation"]}
            records = [record for domain, image in case.images.items() if domain != "E" for record in image["recorded"]]
            self.assertEqual(len(records), 12)
            for record in records:
                self.assertEqual({key: record[key] for key in expected}, expected)
            artifacts = sorted(Path(temp).glob("we7-*-oracle.json"))
            self.assertEqual(len(artifacts), 21)
            for path in artifacts:
                self.assertEqual(json.loads(path.read_text())["e_source_identity"], expected)

    def test_expected_source_identity_is_frozen_before_first_response(self):
        seed, _, oracle = runtime.load_inputs()
        manifest = runtime.we7_manifest(seed, oracle)
        manifest.update(session_id="c6a-pinned-session", session_generation="7")
        with tempfile.TemporaryDirectory(prefix="c6a-we7-pinned-") as temp:
            case = We7FakeCase(temp, oracle, manifest=manifest)
            original_owner = case.owner
            def owner(domain, payload, message):
                # 响应抵达后更改可变容器，不能改写已选定的来源预期。
                case.manifest.update(session_id="foreign-session", session_generation="999")
                return original_owner(domain, payload, message)
            case.owner = owner
            result = runtime.execute_we7(case, oracle)
            self.assertEqual(result["steps"], 21)
            receipt = json.loads((Path(temp) / "we7-020-oracle.json").read_text())
            self.assertEqual(receipt["e_source_identity"], {"e_domain_id": "E", "e_session_id": "c6a-pinned-session", "e_session_generation": "7"})

    def test_missing_duplicate_wrong_allocation_and_policy_are_rejected(self):
        mutations = [lambda row: row.update(allocations=[]),
                     lambda row: row["allocations"].pop(),
                     lambda row: row["allocations"].append(copy.deepcopy(row["allocations"][0])),
                     lambda row: row["allocations"][0].update(allocation_id="forged"),
                     lambda row: row["allocations"][0].update(chong_id="other"),
                     lambda row: row.update(policy_revision="other")]
        for i, mutation in enumerate(mutations):
            def mutate(images):
                for row in images["E"].get("fill_allocations", []):
                    mutation(row)
            with self.subTest(case=i), self.assertRaises(ValueError):
                self.execute(mutate)

    def test_income_and_other_book_fields_reject_even_with_unchanged_q_k(self):
        paths = [("income_book", "Pi"), ("income_book", "A"), ("income_book", "W"), ("income_book", "R"),
                 ("realized", "gross"), ("realized", "after_fees"), ("principal", "P_recovered"),
                 ("phase", "latch"), ("cost_totals", "commission"), ("unit_cost", "q"),
                 ("stable_point", "seq"), ("opening_fact_ref", "fact_key")]
        for path in paths:
            def mutate(images):
                for domain, image in images.items():
                    if domain != "E" and image["book"] is not None:
                        image["book"][path[0]][path[1]] = "999"
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.execute(mutate)

    def test_e_and_b_fact_set_content_and_applied_binding_reject(self):
        for target in ("E", "B:chong-N1"):
            for mode in ("missing", "duplicate", "key", "hash", "event"):
                def mutate(images):
                    records = images[target]["recorded"]
                    if not records:
                        return
                    if mode == "missing": records.pop()
                    elif mode == "duplicate": records.append(copy.deepcopy(records[0]))
                    elif mode == "key": records[0]["fact_key"] = "0" * 64
                    elif mode == "hash": records[0]["fact_hash"] = "0" * 64
                    else: records[0]["event"] = {}
                with self.subTest(target=target, mode=mode), self.assertRaises(ValueError):
                    self.execute(mutate)
        for field in ("fact_key", "fact_hash", "allocation_id", "target_domain", "status"):
            def mutate(images):
                for image in images.values():
                    if image["applied"]:
                        image["applied"][0][field] = "forged"
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.execute(mutate)

    def test_b_original_and_resolved_policy_proofs_cannot_be_substituted(self):
        for mode in ("capture", "allocation", "policy", "receipt"):
            def mutate(images):
                for domain, image in images.items():
                    if domain == "E": continue
                    for record in image["recorded"]:
                        if mode == "capture": record["e_commit_proof"]["previous_root"] = "0" * 64
                        if "e_resolution_proof" not in record: continue
                        if mode == "allocation": record["resolved_allocation"]["allocation_id"] = "forged"
                        elif mode == "policy": record["e_resolution_proof"]["image"]["fill_allocations"][0]["policy_revision"] = "other"
                        elif mode == "receipt": record["e_resolution_proof"]["receipt"]["cut"]["root_hash"] = "0" * 64
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                self.execute(mutate)


if __name__ == "__main__":
    unittest.main()
