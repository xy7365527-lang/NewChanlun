#!/usr/bin/env python3
"""仅 driver 轻量单测；任何 toy 数据/ACK 都不计 #1394 产品验收证据。"""
import io
import sqlite3
import subprocess
from types import SimpleNamespace
from unittest.mock import MagicMock
import copy
import json
import os
from pathlib import Path
import signal
import socket
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch
import driver as d
import oracle


class AdapterTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(dir="/tmp", prefix="r1394-")
        self.root = Path(self.tmp.name)
        os.chmod(self.root, 0o700)

    def tearDown(self):
        self.tmp.cleanup()

    def protocol(self):
        reader = {key: "1" for key in d.READER}
        reader.update(schema_revision="s-financial-reader-resources/1",
                      job_limits={key: "1" for key in d.JOB})
        resources = {key: "1" for key in d.SESSION}
        resources.update(schema_revision="s-financial-session-resources/3",
                         retained_bytes="268435456")
        binary = self.root / "toy-binary"
        binary.write_bytes(b"not executable")
        inputs = self.root / "toy-input"
        inputs.write_bytes(d.canonical({"schema_revision": "tb03b-finance/2"}))
        structure = dict(binary=str(binary), binary_sha256=d.sha(binary.read_bytes()),
                         db=str(self.root / "s.db"), session_id="toy",
                         session_generation="1", writer_epoch="1",
                         reader_revision="s-financial-basis/3", content_format="s-content/1",
                         reader_resources=reader, reader_session=str(self.root / "s.sock"))
        manifest = dict(schema_revision="economic-manifest/1",
                        protocol_revision="economic-session/2", session_id="toy",
                        session_generation="1", clock={"kind": "TestOnly"},
                        voice_binding={"unit_test_only": True},
                        chongs=[{"chong_id": "alpha"}, {"chong_id": "beta"}],
                        finance=dict(schema_revision="tb03b-finance/2",
                                     input_path=str(inputs), input_sha256=d.sha(inputs.read_bytes()),
                                     coordinator_source={}, structure=structure))
        return manifest, resources

    def test_current_protocol(self):
        manifest, resources = self.protocol()
        self.assertEqual(d.validate_manifest(manifest, resources)["reading"], "SessionClosure")

    def test_null_cold_reading_refused(self):
        manifest, resources = self.protocol()
        manifest["finance"]["structure"]["reader_session"] = None
        with self.assertRaises(d.Rejected):
            d.validate_manifest(manifest, resources)

    def test_nine_field_old_v2_refused(self):
        manifest, resources = self.protocol()
        del manifest["finance"]["structure"]["reader_session"]
        with self.assertRaises(d.Rejected):
            d.validate_manifest(manifest, resources)

    def test_old_economic_domain_refused(self):
        manifest, resources = self.protocol()
        manifest["protocol_revision"] = "economic-session/1"
        with self.assertRaises(d.Rejected):
            d.validate_manifest(manifest, resources)

    def test_mixed_session_refused(self):
        manifest, resources = self.protocol()
        manifest["finance"]["structure"]["session_id"] = "other"
        with self.assertRaises(d.Rejected):
            d.validate_manifest(manifest, resources)

    def test_resource_null_and_256mib_overrun_refused(self):
        manifest, resources = self.protocol()
        for value in (None, "268435457"):
            resources["retained_bytes"] = value
            with self.assertRaises(d.Rejected):
                d.validate_manifest(manifest, resources)

    def test_reply_cap_relationship(self):
        manifest, resources = self.protocol()
        manifest["finance"]["structure"]["reader_resources"]["max_reply_bytes"] = "2"
        with self.assertRaises(d.Rejected):
            d.validate_manifest(manifest, resources)

    def test_socket_parent_permissions(self):
        manifest, resources = self.protocol()
        os.chmod(self.root, 0o755)
        with self.assertRaises(d.Rejected):
            d.validate_manifest(manifest, resources)

    def test_strict_json(self):
        for raw in (b'{"a":1,"a":2}', b'{"a":NaN}'):
            with self.assertRaises(d.Rejected):
                d.strict(raw)

    def test_exact_positive_numbers(self):
        for value in (None, 1, "01", "0", "-1", str(2**63)):
            with self.assertRaises(d.Rejected):
                d.positive(value)

    def test_pinned_hash(self):
        item = self.root / "original"
        item.write_bytes(b"actual")
        with self.assertRaises(d.Rejected):
            d.pinned({"path": str(item), "sha256": "0" * 64})

    def test_ab_fresh_independent(self):
        paths = d.ab_directories(self.root / "AB")
        self.assertNotEqual(paths["A"].stat().st_ino, paths["B"].stat().st_ino)
        self.assertEqual(list(paths["A"].iterdir()), [])
        self.assertEqual(list(paths["B"].iterdir()), [])
        with self.assertRaises(d.Rejected):
            d.ab_directories(self.root / "AB")

    def test_custody_flat_shape_without_database_insertion(self):
        # 普通字节文件用于形状单测，不冒充 SQLite 或产品原件。
        file = self.root / "toy-bytes"
        file.write_bytes(b"ordinary bytes")
        before = file.read_bytes()
        proof = d.custody(file, budget=d.ReadBudget(time.monotonic()+1, 100, 10000))
        self.assertEqual(set(proof["main"]), {"sha256", "st_dev", "st_ino",
                                              "st_size", "st_mtime_ns", "st_ctime_ns"})
        self.assertEqual(file.read_bytes(), before)
        self.assertIsNone(proof["wal"])

    def test_missing_database_never_created(self):
        missing = self.root / "absent.db"
        with self.assertRaises(d.Rejected):
            d.read_domain(missing)
        self.assertFalse(missing.exists())

    def test_recovery_detects_changed_originals(self):
        before = {"meta": {"writer_epoch": "1"}, "commits": [1],
                  "receipts": {"actual": 2}, "requests": {"original": 3}}
        after = copy.deepcopy(before)
        after["meta"]["writer_epoch"] = "2"
        self.assertTrue(d.recovery_comparison(before, after, "1", "2"))
        after["receipts"]["actual"] = 9
        with self.assertRaises(d.Rejected):
            d.recovery_comparison(before, after, "1", "2")

    def test_ab_compares_actuals_exactly(self):
        actual = {domain: {"commits": [1], "receipts": {"r": 2},
                           "requests": {"q": 3}} for domain in ("E", "B:alpha", "B:beta")}
        other = copy.deepcopy(actual)
        other["B:beta"]["requests"]["q"] = 4
        with self.assertRaises(d.Rejected):
            d.compare_ab(actual, other)

    def test_oracle_default_actual_database_read_refused(self):
        with self.assertRaises(d.Rejected):
            oracle.check(self.root / "must-not-open")

    def test_oracle_arithmetic_and_optimized_python_safety(self):
        self.assertEqual(oracle.amount("120"), 120)
        for value in ("01", "-1", 1, str(2**63)):
            with self.assertRaises(ValueError):
                oracle.amount(value)

    def test_ready_binds_current_startup_and_three_capture_versions(self):
        manifest, resources = self.protocol()
        structure = manifest["finance"]["structure"]
        startup = self.root / "TOY-STARTUP"
        receipt = {"schema": "s-financial-session-startup/2", "status": "READY",
                   "binary_sha256": structure["binary_sha256"], "pid": "123",
                   "db": structure["db"], "consistency": "a",
                   "anchor_mismatch_policy": "reject-request", "resources": resources,
                   "resources_sha256": d.sha(d.canonical(resources)), "anchor": {"toy": True},
                   "data_version": ["1", "1", "1"]}
        d.save_new(startup, receipt)
        ready = {"schema": "s-financial-session-ready/1", "startup_receipt": receipt,
                 "startup_receipt_sha256": d.sha(d.canonical(receipt)),
                 "startup_receipt_path": str(startup)}
        line = b"READY " + d.canonical(ready) + b"\n"
        self.assertEqual(d.verify_ready(line, startup, structure, resources, 123,
                                        {"toy": True}, budget=d.ReadBudget(time.monotonic()+1, 100, 10000)), receipt)
        with self.assertRaises(d.Rejected):
            d.verify_ready(line, startup, structure, resources, 124, {"toy": True}, budget=d.ReadBudget(time.monotonic()+1, 100, 10000))

    def test_default_execution_refuses_before_popen(self):
        with patch.object(d.subprocess, "Popen") as spawn:
            with self.assertRaises(d.Rejected):
                d.OwnedProcess(["/not-a-product"], self.root / "never", None, "x", {}, 1)
            spawn.assert_not_called()

    def test_preparation_false_does_not_become_admission(self):
        contract = self.root / "contract"
        grant = self.root / "grant"
        d.save_new(contract, {"schema_revision": "issue1394-driver-execution/1",
                              "execution_authorized": False})
        d.save_new(grant, {"execution_authorized": False})
        with self.assertRaises(d.Rejected):
            d.ExecutionAdmission({"path": str(contract), "sha256": d.sha(contract.read_bytes())},
                                 {"path": str(grant), "sha256": d.sha(grant.read_bytes())})

    def test_economic_envelope_is_current_domain(self):
        manifest, _ = self.protocol()
        source = {"source_namespace": "toy", "source_epoch": "1",
                  "producer_id": "toy", "producer_epoch": "1"}
        payload = {"op": "TOY-ONLY"}
        request = d.economic_envelope(manifest, source, "toy-message", payload, [])
        self.assertEqual(request["schema_revision"], "economic-session/2")
        self.assertEqual(request["payload_hash"], d.sha(d.canonical(payload)))

    def test_wire_unknown_keeps_original_and_no_reply_fabrication(self):
        request = b'{"toy":true}'
        result = d.exchange_frame(str(self.root / "absent.sock"), request, 100, 1024,
                                  time.monotonic() + 1)
        self.assertEqual(result["transport"], "DeliveryUnknown")
        self.assertEqual(result["request_sha256"], d.sha(request))
        self.assertNotIn("response", result)

    def test_wire_frame_cap_refuses_before_connect(self):
        with self.assertRaises(d.Rejected):
            d.exchange_frame(str(self.root / "absent.sock"), b'{"toy":true}',
                             100, 2, time.monotonic() + 1)

    def test_supervisor_real_unix_ack_and_mismatch(self):
        endpoint = str(self.root / "ack.sock")
        errors = []
        def serve(wrong):
            with socket.socket(socket.AF_UNIX) as listener:
                listener.bind(endpoint)
                listener.listen(1)
                with listener.accept()[0] as channel:
                    raw = bytearray()
                    while not raw.endswith(b"\n"):
                        raw.extend(channel.recv(4096))
                    binding = d.strict(raw)["binding"]
                    if wrong:
                        binding["nonce"] = "wrong"
                    channel.sendall(d.canonical({"kind": "ACK", "binding": binding}) + b"\n")
        for wrong in (False, True):
            thread = threading.Thread(target=serve, args=(wrong,), daemon=True)
            thread.start()
            limit = time.monotonic() + 2
            while not Path(endpoint).exists():
                self.assertLess(time.monotonic(), limit)
                time.sleep(.005)
            supervisor = d.Supervisor(endpoint, "a" * 64, limit, 8192)
            if wrong:
                with self.assertRaises(d.Rejected):
                    supervisor.ack("unit-only", {"pid": os.getpid()}, {"toy": True}, limit)
            else:
                self.assertEqual(supervisor.ack("unit-only", {"pid": os.getpid()},
                                                {"toy": True}, limit)["kind"], "ACK")
            thread.join(2)
            self.assertFalse(thread.is_alive())
            Path(endpoint).unlink()


    def toy_admission(self, argv, endpoint):
        # 本方法只构造 UNIT_TEST 控制协议夹具，不冻结任何产品、Voice 或资金原件。
        original = self.root / "TOY-ORIGINAL"
        original.write_bytes(b"UNIT_TEST_ONLY_NOT_PRODUCT_SOURCE")
        pointer = {"path": str(original), "sha256": d.sha(original.read_bytes())}
        program = Path(argv[0])
        contract = dict(schema_revision="issue1394-driver-execution/1",
                        execution_authorized=True, cwd=str(self.root), environment={},
                        source_roles=pointer, input_originals=[pointer],
                        artifacts=[{"path": str(program), "sha256": d.sha(program.read_bytes())}],
                        total_timeout_ms="20000", cleanup_reserve_ms="5000",
                        seal_reserve_ms="1000", term_grace_ms="50", kill_wait_ms="200",
                        single_use_receipt=str(self.root / "TOY-SINGLE-USE"),
                        supervisor_socket=endpoint,
                        commands=[dict(stage="toy-only", argv=argv, domain_id="E", writer_epoch="1",
                                       fault={"marker_path": str(self.root / "TOY-MARKER"),
                                              "domain_id": "E", "message_id": "toy-only", "phase": "after_commit"},
                                       binary_sha256=d.sha(program.read_bytes()),
                                       resource_caps={"scope": "UNIT_TEST_ONLY"})])
        cp, gp = self.root / "TOY-CONTRACT", self.root / "TOY-GRANT"
        d.save_new(cp, contract)
        d.save_new(gp, {"execution_authorized": True, "contract_sha256": d.sha(cp.read_bytes()),
                       "driver_sha256": d.sha(Path(d.__file__).read_bytes()), "nonce": "toy-unit"})
        pointers = [{"path": str(p), "sha256": d.sha(p.read_bytes())} for p in (cp, gp)]
        return d.ExecutionAdmission(*pointers), pointers

    def test_owned_toy_marker_sigkill_and_reap(self):
        endpoint = str(self.root / "supervisor.sock")
        marker_path = self.root / "TOY-MARKER"
        child_code = ("import json,os,time; from pathlib import Path; "
                      "Path(" + repr(str(marker_path)) + ").write_text(json.dumps("
                      "{'domain_id':'E','message_id':'toy-only','phase':'after_commit',"
                      "'pid':os.getpid(),'test_only':True})); time.sleep(20)")
        argv = [str(Path(sys.executable).resolve()), "-c", child_code]
        admission, pointers = self.toy_admission(argv, endpoint)
        with self.assertRaises(FileExistsError):
            d.ExecutionAdmission(*pointers)
        requests, errors = [], []
        stop = threading.Event()
        ready = threading.Event()
        def serve():
            try:
                with socket.socket(socket.AF_UNIX) as listener:
                    listener.bind(endpoint)
                    listener.listen(4)
                    listener.settimeout(.05)
                    ready.set()
                    while not stop.is_set():
                        try:
                            channel, _ = listener.accept()
                        except socket.timeout:
                            continue
                        with channel:
                            channel.settimeout(1)
                            raw = bytearray()
                            while not raw.endswith(b"\n"):
                                part = channel.recv(4096)
                                if not part:
                                    raise ValueError("toy ACK disconnected")
                                raw.extend(part)
                            request = d.strict(raw)
                            requests.append(request)
                            if request["binding"]["stage"] == "toy-only":
                                # 产品/child toy 到此尚未 exec；先监督 ACK 再释放。
                                self.assertFalse(marker_path.exists())
                            channel.sendall(d.canonical({"kind": "ACK",
                                                        "binding": request["binding"]}) + b"\n")
            except BaseException as error:
                errors.append(repr(error))
                ready.set()
        thread = threading.Thread(target=serve, daemon=True)
        thread.start()
        self.assertTrue(ready.wait(2))
        supervisor = d.Supervisor(endpoint, admission.contract_sha,
                                  admission.business_deadline, 16384)
        owner = None
        try:
            owner = d.OwnedProcess(argv, self.root / "TOY-OWNER", supervisor,
                                   "toy-only", {"scope": "UNIT_TEST_ONLY"},
                                   admission.business_deadline, admission=admission)
            marker = owner.wait_marker(marker_path, "E", "toy-only", "after_commit")
            proof = owner.kill_at_marker(marker)
            self.assertEqual(proof["returncode"], -signal.SIGKILL)
            self.assertTrue(proof["reaped"])
            self.assertEqual(proof["owner"]["pid"], marker["pid"])
        finally:
            if owner is not None:
                owner.cleanup()
            stop.set()
            thread.join(2)
        self.assertFalse(thread.is_alive())
        self.assertEqual(errors, [])
        self.assertIn("SIGKILL", [r["binding"]["stage"] for r in requests])


    def test_strict_overflow_nested_and_finite_preserved(self):
        for raw in (b'{"a":1e999}', b'{"a":-1e999}', b'{"a":[1e999]}',
                    b'{"a":Infinity}', b'{"a":1,"a":2}'):
            with self.subTest(raw=raw), self.assertRaises(d.Rejected):
                d.strict(raw)
        self.assertEqual(d.strict(b'{"a":1.5}'), {"a": 1.5})

    def test_ack_all_steps_share_absolute_deadline(self):
        for late in ("ps", "connect", "send", "recv", "parse"):
            now = [0.0]
            class Channel:
                def __enter__(self): return self
                def __exit__(self, *args): pass
                def settimeout(self, value):
                    self.last_timeout = value
                def connect(self, endpoint):
                    if late == "connect": now[0] = 11.0
                def sendall(self, payload):
                    self.binding = d.strict(payload)["binding"]
                    if late == "send": now[0] = 11.0
                def recv(self, size):
                    if late == "recv": now[0] = 11.0
                    return d.canonical({"kind": "ACK", "binding": self.binding}) + b"\n"
            def ps(*args, **kw):
                self.assertGreater(kw["timeout"], 0)
                if late == "ps": now[0] = 11.0
                return SimpleNamespace(stdout="same-start")
            original = d.strict
            def parse(raw):
                value = original(raw)
                if late == "parse" and value.get("kind") == "ACK": now[0] = 11.0
                return value
            with self.subTest(step=late), patch.object(d.time, "monotonic", lambda: now[0]), \
                 patch.object(d.subprocess, "run", ps), patch.object(d.socket, "socket", return_value=Channel()), \
                 patch.object(d, "strict", parse):
                with self.assertRaises(d.Rejected):
                    d.Supervisor("fake", "f" * 64, 10, 8192).ack("toy", {}, {}, 10)

    def test_save_checks_deadline_after_fsync(self):
        now = [0.0]
        with patch.object(d.time, "monotonic", lambda: now[0]), \
             patch.object(d.os, "fsync", side_effect=lambda fd: now.__setitem__(0, 11)):
            with self.assertRaises(d.Rejected):
                d.save_new(self.root / "LATE", {"toy": True}, deadline=10)
        self.assertTrue((self.root / "LATE").exists())  # 原件保留但不得放行

    def test_command_stage_epoch_consumed_even_on_hash_failure(self):
        argv = [str(Path(sys.executable).resolve()), "-c", "pass"]
        admission, _ = self.toy_admission(argv, "fake")
        caps = {"scope": "UNIT_TEST_ONLY"}
        admission.check_command(argv, "toy-only", caps, admission.business_deadline)
        with self.assertRaises(d.Rejected):
            admission.check_command(argv, "toy-only", caps, admission.business_deadline)
        self.assertEqual(admission.consumed, {("toy-only", "E", "1")})

    def fake_owner(self):
        owner = d.OwnedProcess.__new__(d.OwnedProcess)
        owner.argv = [str(Path(sys.executable).resolve())]
        owner.binary_before = d.sha(Path(owner.argv[0]).read_bytes())
        owner.output = self.root / "FAKE-OWNER"
        owner.directory = d.OwnedDirectory(owner.output)
        owner.identity = {"pid": 123, "start_identity": "toy", "process_group": 123,
                          "domain_id": "E", "writer_epoch": "1"}
        owner.domain, owner.epoch, owner.stage = "E", "1", "toy"
        owner.cleanup_deadline, owner.seal_deadline = time.monotonic() + 1, time.monotonic() + 2
        owner.term_grace_seconds, owner.kill_wait_seconds = .01, .02
        owner.stdout, owner.stderr = io.BytesIO(), io.BytesIO()
        owner.process = MagicMock()
        owner.process.stdin = io.BytesIO()
        owner.process.pid, owner.process.returncode = 123, 0
        owner.process.wait.return_value = 0
        owner.cleanup_result, owner.released = None, True
        return owner

    def test_cleanup_normal_exit_between_polls_reaps_and_closes(self):
        owner = self.fake_owner()
        owner.process.poll.side_effect = [None, 0]
        with patch.object(d.os, "kill") as kill:
            result = owner.cleanup("race")
            kill.assert_not_called()
        self.assertTrue(result["reaped"])
        self.assertTrue(owner.stdout.closed and owner.stderr.closed and owner.process.stdin.closed)
        self.assertTrue((owner.output / "CLEANUP.json").exists())
        self.assertIs(owner.cleanup(), result)

    def test_cleanup_exit_during_ps_and_hash_failure_still_seals(self):
        owner = self.fake_owner()
        owner.process.poll.side_effect = [None, None, 0]
        with patch.object(d, "process_identity", side_effect=ProcessLookupError()), \
             patch.object(d, "bounded_hash", side_effect=FileNotFoundError("changed")), \
             patch.object(d.os, "kill") as kill:
            result = owner.cleanup("ps-race")
            kill.assert_not_called()
        self.assertTrue(result["reaped"])
        self.assertIsNone(result["binary_sha256_after"])
        self.assertTrue(any("FileNotFoundError" in x for x in result["errors"]))
        self.assertTrue(owner.stdout.closed and owner.stderr.closed)
        self.assertTrue((owner.output / "CLEANUP.json").exists())

    def test_cleanup_all_keeps_other_owner_when_one_fails(self):
        admission = d.ExecutionAdmission.__new__(d.ExecutionAdmission)
        good = {"owner": {}, "returncode": 0, "reaped": True, "errors": [],
                "binary_sha256_before": "toy", "binary_sha256_after": "toy"}
        admission.active_owners = {("a", "E", "1"): SimpleNamespace(
            cleanup=lambda reason: (_ for _ in ()).throw(RuntimeError("bad")), binary_before="toy"),
            ("b", "E", "2"): SimpleNamespace(cleanup=lambda reason: good)}
        result = admission.cleanup_all("cancel")
        self.assertEqual(len(result), 2)
        self.assertFalse(result[0]["reaped"])
        self.assertIs(result[1], good)

    def toy_db(self, bad_json=False):
        path = self.root / "toy.sqlite"
        with sqlite3.connect(path) as db:
            db.executescript("CREATE TABLE meta(k,v); CREATE TABLE commits(seq,root,commit_ns,image);"
                             "CREATE TABLE receipts(k,json); CREATE TABLE messages(k,request_json);")
            db.execute("INSERT INTO meta VALUES ('writer_epoch','1')")
            db.execute("INSERT INTO commits VALUES (1,'root',1,?)", ('{"a":1e999}' if bad_json else '{}',))
            db.execute("INSERT INTO receipts VALUES ('r','{}')")
            db.execute("INSERT INTO messages VALUES ('q','{}')")
        db.close()
        return path

    def test_sqlite_explicit_close_success_parse_and_query_failure(self):
        path = self.toy_db()
        real_connect = d.sqlite3.connect
        closed = []
        class Tracking(sqlite3.Connection):
            def close(self):
                closed.append(True)
                super().close()
        for case in ("success", "parse", "query"):
            if case == "parse":
                with real_connect(path) as db: db.execute("UPDATE commits SET image='{bad'")
                db.close()
            if case == "query":
                with real_connect(path) as db: db.execute("DROP TABLE commits")
                db.close()
            with patch.object(d.sqlite3, "connect", side_effect=lambda *a, **k: real_connect(*a, factory=Tracking, **k)):
                budget = d.ReadBudget(time.monotonic()+1, 100, 10000)
                if case == "success": self.assertTrue(d.read_domain(path, budget=budget))
                else:
                    with self.assertRaises((ValueError, sqlite3.Error)): d.read_domain(path, budget=budget)
            self.assertEqual(len(closed), ("success", "parse", "query").index(case)+1)

    def test_sqlite_shared_cumulative_records_and_bytes(self):
        path = self.toy_db()
        budget = d.ReadBudget(time.monotonic()+1, 5, 10000)
        d.read_domain(path, budget=budget)
        self.assertEqual(budget.records, 4)
        with self.assertRaises(d.Rejected): d.read_domain(path, budget=budget)
        self.assertGreater(budget.records, 5)
        with self.assertRaises((d.Rejected, sqlite3.Error)):
            d.read_domain(path, budget=d.ReadBudget(time.monotonic()+1, 100, 2))
        with self.assertRaises(d.Rejected): d.read_domain(path)

    def test_sqlite_busy_deadline_closes_without_restarting_window(self):
        path = self.toy_db()
        lock = sqlite3.connect(path)
        lock.execute("BEGIN EXCLUSIVE")
        try:
            start = time.monotonic()
            with self.assertRaises((sqlite3.Error, d.Rejected)):
                d.read_domain(path, budget=d.ReadBudget(start+.03, 100, 10000))
            self.assertLess(time.monotonic()-start, .3)
        finally:
            lock.rollback()
            lock.close()


    def test_consumed_stage_remains_consumed_after_binary_hash_failure(self):
        argv = [str(Path(sys.executable).resolve()), "-c", "pass"]
        admission, _ = self.toy_admission(argv, "fake")
        with patch.object(d, "bounded_hash", return_value="wrong"):
            with self.assertRaises(d.Rejected):
                admission.check_command(argv, "toy-only", {"scope": "UNIT_TEST_ONLY"}, admission.business_deadline)
        with self.assertRaises(d.Rejected):
            admission.check_command(argv, "toy-only", {"scope": "UNIT_TEST_ONLY"}, admission.business_deadline)

    def test_real_gate_ack_loss_and_late_disk_reap_without_release(self):
        for failure in ("ack", "disk"):
            with self.subTest(failure=failure):
                # 每案例独立目录，门控仅执行 Python toy。
                sub = self.root / failure
                sub.mkdir()
                original_root, self.root = self.root, sub
                marker = sub / "MUST-NOT-EXEC"
                argv = [str(Path(sys.executable).resolve()), "-c",
                        "from pathlib import Path;Path(" + repr(str(marker)) + ").touch()"]
                admission, _ = self.toy_admission(argv, "fake")
                supervisor = d.Supervisor("fake", admission.contract_sha, admission.business_deadline, 8192)
                original_save = d.save_new
                now = [time.monotonic()]
                def save(path, value, **kw):
                    original_save(path, value, **kw)
                    if Path(path).name == "ACK.json": now[0] = admission.business_deadline + .01
                try:
                    with patch.object(supervisor, "ack", side_effect=(d.Rejected("ACK lost") if failure == "ack" else None), return_value={"toy": True}), \
                         patch.object(d, "save_new", side_effect=save), \
                         patch.object(d.time, "monotonic", side_effect=lambda: now[0]() if callable(now[0]) else now[0]):
                        with self.assertRaises(d.Rejected):
                            d.OwnedProcess(argv, sub / "owner", supervisor, "toy-only", {"scope": "UNIT_TEST_ONLY"}, admission.business_deadline, admission=admission)
                    self.assertEqual(len(admission.active_owners), 1)
                    owner = next(iter(admission.active_owners.values()))
                    self.assertTrue(owner.cleanup_result["reaped"])
                    self.assertTrue(owner.stdout.closed and owner.stderr.closed and owner.process.stdin.closed)
                    self.assertFalse(marker.exists())
                finally:
                    self.root = original_root

    def test_sqlite_progress_handler_interrupts_expensive_view(self):
        path = self.toy_db()
        with sqlite3.connect(path) as db:
            db.executescript("DROP TABLE commits; CREATE VIEW commits AS WITH RECURSIVE n(x) AS "
                             "(VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<100000000) "
                             "SELECT sum(x) AS seq, 'root' AS root, 1 AS commit_ns, '{}' AS image FROM n;")
        db.close()
        start = time.monotonic()
        with self.assertRaises((sqlite3.Error, d.Rejected)):
            d.read_domain(path, budget=d.ReadBudget(start+.02, 100, 10000))
        self.assertLess(time.monotonic()-start, .3)


    def test_custody_shared_byte_budget_counts_bytes_and_closes(self):
        path = self.root / "toy-main"
        path.write_bytes(b"abcdef")
        budget = d.ReadBudget(time.monotonic()+1, 100, 10)
        self.assertEqual(d.custody(path, budget=budget)["main"]["sha256"], d.sha(b"abcdef"))
        self.assertEqual(budget.bytes_read, 6)
        with self.assertRaises(d.Rejected): d.custody(path, budget=budget)

    def test_cleanup_term_timeout_escalates_then_reaps(self):
        owner = self.fake_owner()
        owner.process.poll.return_value = None
        owner.process.wait.side_effect = [subprocess.TimeoutExpired("toy", .01), 0]
        with patch.object(owner, "verify_identity", return_value=True), patch.object(d.os, "kill") as kill:
            result = owner.cleanup("term timeout")
        self.assertEqual([call.args[1] for call in kill.call_args_list], [signal.SIGTERM, signal.SIGKILL])
        self.assertTrue(result["reaped"])
        self.assertEqual(result["errors"], [])
        self.assertTrue(owner.stdout.closed and owner.stderr.closed and owner.process.stdin.closed)


if __name__ == "__main__":
    unittest.main(verbosity=2)
