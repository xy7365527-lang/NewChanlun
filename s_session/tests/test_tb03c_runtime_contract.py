"""#1406 C7a 驱动契约负控；不启动 Rust/产品，不冒称真实恢复证据。"""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import tb03c_runtime as runtime


class RuntimeContract(unittest.TestCase):
    def setUp(self):
        self.seed, self.case = runtime.load_inputs()

    def changed_seed(self, change):
        seed = copy.deepcopy(self.seed)
        change(seed)
        with tempfile.TemporaryDirectory(prefix="tb03c-contract-") as temp:
            path = Path(temp) / "seed.json"
            path.write_text(json.dumps(seed))
            with self.assertRaises(ValueError):
                runtime.load_inputs(path)

    def test_five_source_bindings_are_mandatory_and_hash_checked(self):
        for i in range(len(self.seed["bindings"])):
            with self.subTest(binding=i):
                self.changed_seed(lambda s, i=i: s["bindings"][i].update(sha256="0" * 64))
        self.changed_seed(lambda s: s["bindings"].pop())
        self.changed_seed(lambda s: s["bindings"].append(s["bindings"][0]))

    def test_own_source_scope_and_timeouts_cannot_be_silently_dropped(self):
        self.changed_seed(lambda s: s.update(scope="production"))
        self.changed_seed(lambda s: s.update(e_case_index=1))
        for key in ("startup_timeout_ms", "exchange_timeout_ms", "marker_timeout_ms", "process_timeout_ms"):
            for value in (0, -1, 60001, True):
                with self.subTest(key=key, value=value):
                    self.changed_seed(lambda s, k=key, v=value: s.update({k: v}))

    def test_all_epoch_clock_slots_are_after_recovery_and_monotone(self):
        plan = self.seed["manifest"]["clock"]["plan"]
        for epoch in range(4):
            floor = int(plan["init_ns"] if epoch == 0 else plan["recover_ns"][epoch - 1])
            for message, _ in runtime.write_message_schedule(self.case):
                entry = plan["messages"][message]
                received, first, commit = [int(entry[k][epoch]) for k in ("received_ns", "first_known_ns", "commit_ns")]
                self.assertLess(floor, received)
                self.assertLessEqual(received, first)
                self.assertLessEqual(first, commit)
                floor = commit
            if epoch < 3:
                self.assertLess(floor, int(plan["recover_ns"][epoch]))

    def test_every_write_message_requires_all_epoch_clock_slots(self):
        schedule = runtime.write_message_schedule(self.case)
        self.assertEqual(len(schedule), 21)
        self.assertEqual(schedule[15], ("extra-local-fill-wait", "ApplyAllocation"))
        for message, _ in schedule:
            with self.subTest(message=message):
                self.changed_seed(lambda s, m=message: s["manifest"]["clock"]["plan"]["messages"].pop(m))
        for epoch in range(4):
            self.changed_seed(lambda s, e=epoch: s["manifest"]["clock"]["plan"]["messages"]["extra-local-fill-wait"]["received_ns"].__setitem__(e, "0"))
        self.changed_seed(lambda s: s["manifest"]["clock"]["plan"]["messages"]["extra-local-fill-wait"]["commit_ns"].pop())
        self.changed_seed(lambda s: s["manifest"]["clock"]["plan"]["messages"].update(extra=s["manifest"]["clock"]["plan"]["messages"]["step-000"]))

    def test_any_unregistered_driver_write_is_rejected_before_delivery(self):
        obj = runtime.Run.__new__(runtime.Run)
        obj.case, obj.manifest = self.case, self.seed["manifest"]
        for message, op in runtime.write_message_schedule(self.case):
            self.assertEqual(obj.envelope({"op": op}, message)["message_id"], message)
        for message, op in (("forgotten-write", "ApplyAllocation"), ("step-000", "ResolveEvidence")):
            with self.assertRaises(ValueError):
                obj.envelope({"op": op}, message)

    def test_we5_independent_all_fields_and_named_predecessor(self):
        expected = runtime.Expected()
        for step in self.case["steps"]:
            expected.advance(step)
        view = expected.book.view()
        self.assertEqual((view["q"], view["K"], view["Pi"]), (3, 305, 19))
        self.assertEqual(view["P"], None)
        self.assertEqual(view["growth_eligible"], "NotEvaluable")
        self.assertEqual(view["stable_point"], "403")
        self.assertEqual(len(expected.applied), 5)
        self.assertEqual(len(expected.facts), 6)
        self.assertEqual(list(expected.resolved), ["r1", "r6"])
        doc = json.loads((runtime.FIXTURES / "raw/chong-R1.json").read_text())
        fee = doc["events"][3]
        with self.assertRaises(runtime.oracle.Typed):
            runtime.oracle.verify_fill_predecessors(fee, {})
        wrong = copy.deepcopy(doc["events"][4])
        wrong["allocation_id"] = "other"
        with self.assertRaises(runtime.oracle.Typed):
            runtime.oracle.verify_fill_predecessors(fee, {runtime.oracle.fact_identity(wrong): wrong})

    @staticmethod
    def typed_observation(expected):
        """只供变异负控的合成观测；非产品输出、非运行验收。"""
        v = expected.book.view()
        raw, _, event = next(x for x in expected.applied.values() if "observed_existing_book" in x[2])
        return {"actual_quantity": str(v["q"]), "cost_basis": {"status": "Known", "K": str(v["K"])},
                "principal": {"status": "NotEvaluable", "batches": [], "P_recovered": "0", "reason": "no authorized principal batch"},
                "income_book": {k: str(v[k]) for k in ("Pi", "A", "W", "R")},
                "fixed_q": str(v["fixed_q"]),
                "realized": {"gross": str(v["realized_gross"]), "fees": str(v["fees"]),
                             "after_fees": str(v["realized_gross"] - v["fees"])},
                "cost_remaining": {"status": "Known", "value": str(v["cost_remaining"])},
                "phase": {"growth_eligible": {"status": v["growth_eligible"]}, "latch": v["latch"],
                          "gate": None, "events": [], "growth_enabled": {"status": "NotEvaluable", "reason": "principal batches absent"}},
                "unit_cost": {"status": "Known", "K": str(v["K"]), "q": str(v["q"])},
                "stable_point": {"profile": "TestOnly", "seq": v["stable_point"]},
                "cost_totals": {k: str(n) for k, n in expected.totals.items()},
                "known_cost_components": copy.deepcopy(expected.components),
                "book_revision": "tb03c-book/1", "family_id": "fam-R1", "chong_id": "chong-R1",
                "side": "long", "admission": "NotEvaluated",
                "voice": {"status": "MissingDependency", "reason": "confirmed gamma binding absent", "identity": None},
                "symbol": "TEST-R1", "operation_level": "1", "units": json.loads(raw)["units"],
                "opening_quantity": event["observed_existing_book"]["units"], "opening_fact_ref": {},
                "unknown": {"complete_historical_fees": {"status": "Unknown", "reason": "only named known components available"},
                            "target": {"status": "Unknown", "reason": "not supplied"}}}

    def test_every_money_field_and_causal_component_tampering_is_rejected(self):
        expected = runtime.Expected()
        for step in self.case["steps"]:
            expected.advance(step)
        observed = self.typed_observation(expected)
        expected.check_book(observed)
        paths = [("actual_quantity",), ("cost_basis", "K"), ("principal", "P_recovered"),
                 ("fixed_q",), ("realized", "gross"), ("realized", "fees"), ("realized", "after_fees"),
                 ("cost_remaining", "value"), ("unit_cost", "K"), ("unit_cost", "q"), ("stable_point", "seq")]
        paths += [("income_book", k) for k in ("Pi", "A", "W", "R")]
        paths += [("cost_totals", k) for k in expected.totals]
        for path in paths:
            with self.subTest(path=path):
                bad = copy.deepcopy(observed)
                parent = bad
                for key in path[:-1]:
                    parent = parent[key]
                parent[path[-1]] = str(int(parent[path[-1]]) + 1)
                with self.assertRaises(ValueError):
                    expected.check_book(bad)
        for key in ("fact_key", "raw_sha256", "source_ref", "canonical_amount"):
            bad = copy.deepcopy(observed)
            bad["known_cost_components"][-1][key] = "tampered"
            with self.assertRaises(ValueError):
                expected.check_book(bad)

    @staticmethod
    def recovery_pair():
        before = {table: {"columns": ["synthetic"], "rows": [["unchanged"]]} for table in runtime.TABLES}
        before["meta"]["rows"] = [["writer_epoch", "1"], ["manifest_hash", "original"]]
        before["epochs"]["rows"] = [[1, "init", "1"]]
        after = copy.deepcopy(before)
        after["meta"]["rows"][0][1] = "2"
        after["epochs"]["rows"].append([2, "recover", "1000000"])
        return before, after, {"writer_epoch": "2", "recover_ns": "1000000"}

    def test_recovery_rejects_any_non_epoch_persistent_change(self):
        before, after, receipt = self.recovery_pair()
        runtime.assert_recovery(before, after, receipt)
        for table in runtime.TABLES:
            bad = copy.deepcopy(after)
            bad[table]["rows"].append(["extra"])
            with self.subTest(table=table), self.assertRaises((ValueError, TypeError)):
                runtime.assert_recovery(before, bad, receipt)
        bad = copy.deepcopy(after)
        bad["meta"]["rows"][1][1] = "forged"
        with self.assertRaises(ValueError):
            runtime.assert_recovery(before, bad, receipt)

    def test_marker_pid_mismatch_never_signals_owned_process(self):
        class Fake:
            pid = 12345
            killed = False
            def poll(self):
                return None
            def kill(self):
                self.killed = True
            def wait(self, timeout):
                return -9
        obj = runtime.Run.__new__(runtime.Run)
        proc = Fake()
        obj.processes = {"E": proc}
        obj.seed = self.seed
        obj.phase = "before_commit"
        obj.record = lambda _: None
        marker = {"domain_id": "E", "message_id": "step-014", "phase": obj.phase, "pid": 12346, "test_only": True}
        with self.assertRaises(ValueError):
            obj.stop("E", kill=True, marker=marker)
        self.assertFalse(proc.killed)
        marker["pid"] = proc.pid
        self.assertEqual(obj.stop("E", kill=True, marker=marker)["exit_code"], -9)
        self.assertTrue(proc.killed)
        self.assertNotIn("E", obj.processes)

    def test_no_test_can_spawn_a_product_process(self):
        # 这套轻量契约测试本身以禁止 Popen 的条件运行纯 oracle；不将合成观测冒充产品。
        with patch.object(runtime.subprocess, "Popen", side_effect=AssertionError("本机禁产品")):
            runtime.load_inputs()
            expected = runtime.Expected()
            for step in self.case["steps"]:
                expected.advance(step)
            expected.check_book(self.typed_observation(expected))

    def cli_object(self, path):
        obj = runtime.Run.__new__(runtime.Run)
        obj.seed = self.seed
        obj.output, obj.number, obj.processes, obj.epochs = path, 0, {}, {}
        obj.command = lambda domain, op: ["never-executed-product", op, domain]
        records = []
        obj.record = records.append
        return obj, records

    def test_cli_normal_success_and_nonzero_preserve_raw_output(self):
        for op, kind in (("init", "Initialized"), ("recover", "Recovered")):
            for code in (0, 7):
                with self.subTest(op=op, code=code), tempfile.TemporaryDirectory(prefix="c7a-cli-mock-") as temp:
                    obj, records = self.cli_object(Path(temp))
                    stdout = runtime.canonical({"kind": kind, "domain_id": "E", "writer_epoch": "2"}) + b"\n"
                    stderr = b"diagnostic\xff\n"
                    class Fake:
                        pid = 12345
                        returncode = None
                        def __init__(self, args, stdout, stderr):
                            stdout.write(output_bytes)
                            stderr.write(error_bytes)
                        def wait(self, timeout):
                            self.returncode = code
                            return code
                        def poll(self):
                            return self.returncode
                    output_bytes, error_bytes = stdout, stderr
                    with patch.object(runtime.subprocess, "Popen", Fake):
                        if code:
                            with self.assertRaises(ValueError):
                                obj.cli("E", op)
                        else:
                            self.assertEqual(obj.cli("E", op)["kind"], kind)
                    receipt = records[-1]
                    self.assertEqual(receipt["status"], "returned")
                    self.assertEqual(receipt["exit_code"], code)
                    self.assertEqual((Path(temp) / receipt["stdout"]["path"]).read_bytes(), stdout)
                    self.assertEqual((Path(temp) / receipt["stderr"]["path"]).read_bytes(), stderr)
                    self.assertEqual(obj.processes, {})

    def test_cli_timeout_keeps_partial_bytes_and_kills_waits_exact_handle(self):
        for op in ("init", "recover"):
            with self.subTest(op=op), tempfile.TemporaryDirectory(prefix="c7a-timeout-mock-") as temp:
                obj, records = self.cli_object(Path(temp))
                calls = []
                timeout = subprocess.TimeoutExpired(obj.command("E", op), 30)
                class Fake:
                    pid = 12345
                    returncode = None
                    def __init__(self, args, stdout, stderr):
                        stdout.write(b"partial-out\x00\xff")
                        stderr.write(b"partial-error\xfe")
                    def wait(self, timeout):
                        calls.append("wait")
                        if self.returncode is None:
                            raise original_timeout
                        return self.returncode
                    def poll(self):
                        return self.returncode
                    def kill(self):
                        calls.append("kill")
                        self.returncode = -9
                original_timeout = timeout
                with patch.object(runtime.subprocess, "Popen", Fake), self.assertRaises(subprocess.TimeoutExpired) as caught:
                    obj.cli("E", op)
                self.assertIs(caught.exception, timeout)
                self.assertEqual(calls, ["wait", "kill", "wait"])
                self.assertEqual(obj.processes, {})
                receipt = records[-1]
                self.assertEqual(receipt["status"], "timeout")
                self.assertEqual(receipt["exit_code"], -9)
                self.assertEqual(receipt["failure_type"], "TimeoutExpired")
                self.assertEqual(receipt["timeout_seconds"], 30)
                self.assertEqual((Path(temp) / receipt["stdout"]["path"]).read_bytes(), b"partial-out\x00\xff")
                self.assertEqual((Path(temp) / receipt["stderr"]["path"]).read_bytes(), b"partial-error\xfe")

    def test_cli_spawn_failure_is_recorded_and_evidence_failure_does_not_hide_timeout(self):
        with tempfile.TemporaryDirectory(prefix="c7a-failed-cli-mock-") as temp:
            obj, records = self.cli_object(Path(temp))
            original = FileNotFoundError("mock absent binary")
            with patch.object(runtime.subprocess, "Popen", side_effect=original), self.assertRaises(FileNotFoundError) as caught:
                obj.cli("E", "init")
            self.assertIs(caught.exception, original)
            self.assertEqual(records[-1]["failure_type"], "FileNotFoundError")
            self.assertEqual(records[-1]["stdout"]["bytes"], 0)
            self.assertEqual(obj.processes, {})
        with tempfile.TemporaryDirectory(prefix="c7a-record-failure-mock-") as temp:
            obj, records = self.cli_object(Path(temp))
            original = subprocess.TimeoutExpired(["never-executed"], 30)
            class Fake:
                pid = 12345
                returncode = None
                def wait(self, timeout):
                    if self.returncode is None:
                        raise original
                    return self.returncode
                def poll(self):
                    return self.returncode
                def kill(self):
                    self.returncode = -9
            def broken_record(entry):
                records.append(entry)
                if entry["event"] == "cli-result":
                    raise OSError("mock evidence write failure")
            obj.record = broken_record
            with patch.object(runtime.subprocess, "Popen", return_value=Fake()), self.assertRaises(subprocess.TimeoutExpired) as caught:
                obj.cli("E", "recover")
            self.assertIs(caught.exception, original)
            self.assertIsInstance(caught.exception.__cause__, OSError)
            self.assertEqual(obj.processes, {})


if __name__ == "__main__":
    unittest.main()
