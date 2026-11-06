"""#1527 本轮 Python 实码测试：真实小文件/SQLite，外部程序仅封闭 stub。"""

import contextlib
import errno
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import sqlite3
import stat
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
import recovery_admission as admission
import s_service_control as control


def encode(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def identity(path):
    item = path.stat()
    return {"device": item.st_dev, "inode": item.st_ino, "uid": item.st_uid,
            "mode": stat.S_IMODE(item.st_mode)}


class Fixture:
    """手工提供已锚原件，非生产首次发布算法。"""

    def __init__(self, root, pending=False, limits=None, owner_mode=0o600):
        self.root = root.resolve()
        self.db = self.root / "source.sqlite"
        with contextlib.closing(sqlite3.connect(self.db)) as db:
            with db:
                db.execute("CREATE TABLE sentinel(value TEXT)")
                db.execute("INSERT INTO sentinel VALUES ('保留原库')")
        self.state = self.root / "state"
        self.state.mkdir(mode=0o755)
        self.container = self.state / (admission.PENDING if pending else admission.PUBLISHED)
        self.container.mkdir(mode=0o700)
        self.limits = {key: "1000000" for key in admission.LIMIT_FIELDS}
        if limits:
            self.limits.update(limits)
        self.raw = b"  " + encode(self.limits) + b"\n"  # 原始 SHA，不以规范 JSON 替换。
        self.path = self.container / "limits.json"
        self.path.write_bytes(self.raw)
        self.path.chmod(0o600)
        self.binding_path = self.container / "binding.json"
        self.binding_path.touch(mode=0o600)
        self.binding = {"db": str(self.db), "path": str(self.state / admission.PUBLISHED / "limits.json"),
                        "sha256": hashlib.sha256(self.raw).hexdigest(),
                        "file_identity": identity(self.path), "binding_identity": identity(self.binding_path),
                        "directory_identity": identity(self.container)}
        self.binding_path.write_bytes(encode(self.binding))
        self.owner_path = Path(str(self.db) + ".control-owner.json")
        self.owner = {"db": str(self.db), "state_dir": str(self.state), "admission": self.binding}
        self.owner_path.write_bytes(encode(self.owner))
        self.owner_path.chmod(owner_mode)
        self.db_hash = hashlib.sha256(self.db.read_bytes()).hexdigest()
        self.stub()

    def load(self, env=None, deadline=None):
        return admission.RecoveryAdmission(self.db, self.state, deadline or time.monotonic() + 10,
                                            {} if env is None else env)

    def env(self):
        return {admission.ENV_SHA: self.binding["sha256"], admission.ENV_IDENTITY: encode(self.binding).decode()}

    def rewrite_binding(self):
        self.binding_path.write_bytes(encode(self.binding))
        self.owner_path.write_bytes(encode(self.owner))

    def config(self, revision="s-launcher/3"):
        return {"schema_revision": revision, "db": str(self.db), "socket": str(self.root / "s.sock"),
                "binary": str(self.root / "stub.py"), "python": sys.executable,
                "catalog": str(self.root / "catalog"), "profile": str(self.root / "profile"),
                "clock_plan": str(self.root / "clock"), "browser": str(self.root / "browser"),
                "query_resource_config": str(self.root / "query"),
                "session_id": "test", "session_generation": "1", "source_namespace": "test",
                "source_epoch": "1", "producer_id": "test", "producer_epoch": "1", "writer_epoch": "1",
                "query_epoch": "1", "port": "12345", "startup_timeout_ms": "10000",
                "delivery_retain_generations": "1", "admission_contract_version": "test-only/1",
                "s_resources": {key: "1000" for key in control.S_RESOURCE_FIELDS}}

    def stub(self, output=None, rc=0):
        path = self.root / "stub.py"
        body = ("import hashlib,json,os,sys\n"
                "args=sys.argv[1:]\n"
                "p=args[args.index('--admission-job-limits')+1]\n"
                "with open(p,'rb') as f: sha=hashlib.sha256(f.read()).hexdigest()\n"
                "result={'stub':True,'argv':args,'sha_match':sha==os.environ.get('S_CONTROL_ADMISSION_SHA256')}\n"
                "print(json.dumps(result))\n") if output is None else "print(" + repr(output) + ")\n"
        contract = ("import json,os,sys\n"
                    "if sys.argv[1]=='admission-contract':\n"
                    " print(json.dumps({'ok':True,'contract_version':'test-only/1',"
                    "'sha256':os.environ['S_CONTROL_ADMISSION_SHA256']}))\n"
                    " sys.exit(0)\n")
        path.write_text("#!" + sys.executable + "\n" + contract + body + f"raise SystemExit({rc})\n")
        path.chmod(0o700)
        return path


class R4Tests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="issue1527-r4-")
        self.root = Path(self.temporary.name)
        self.env_guard = patch.dict(os.environ, {}, clear=True)
        self.env_guard.start()

    def tearDown(self):
        self.env_guard.stop()
        self.temporary.cleanup()

    def fixture(self, **kwargs):
        return Fixture(self.root, **kwargs)

    def assert_source(self, fixture):
        self.assertEqual(hashlib.sha256(fixture.db.read_bytes()).hexdigest(), fixture.db_hash)

    def test_fixture_contract_version_mismatch_still_rejects_before_spawn(self):
        f = self.fixture()
        config = {**f.config(), "admission_contract_version": "different-test-contract/1"}
        with contextlib.closing(control.Controller(config, f.state)) as controller:
            with patch.object(controller, "runtime_environment", return_value={}), \
                 patch.object(controller, "start_process") as spawn:
                with self.assertRaisesRegex(ValueError, "AdmissionContractReplyInvalid"):
                    controller.start_s("1")
                spawn.assert_not_called()

    def test_valid_anchor_no_env_counts_exact_payload_holds_fds_and_keeps_state_0755(self):
        f = self.fixture(owner_mode=0o644)
        expected = sum(p.stat().st_size for p in (f.owner_path, f.binding_path, f.path))
        with contextlib.closing(f.load()) as lease:
            self.assertEqual(lease.wire_bytes, expected)
            t0 = lease.first_read_at
            held = list(lease._fds)
            lease.publish()
            lease.revalidate()
            self.assertEqual(lease.first_read_at, t0)
            self.assertEqual(lease.wire_bytes, expected)
            self.assertEqual(lease.limits_path, f.binding["path"])
            self.assertEqual(json.loads(lease.child_environment({})[admission.ENV_IDENTITY]), f.binding)
            self.assertEqual(identity(f.state)["mode"], 0o755)
        for descriptor in held:
            with self.assertRaises(OSError):
                os.fstat(descriptor)
        self.assert_source(f)

    def test_pending_resume_keeps_all_original_inodes_bytes_and_anchor(self):
        f = self.fixture(pending=True)
        owner_bytes, owner_id = f.owner_path.read_bytes(), identity(f.owner_path)
        with contextlib.closing(f.load(f.env())) as lease:
            with self.assertRaises(admission.AdmissionError):
                _ = lease.limits_path
            lease.publish()
            target = Path(lease.limits_path)
            self.assertEqual(identity(target), f.binding["file_identity"])
            self.assertEqual(identity(target.parent), f.binding["directory_identity"])
            self.assertEqual(identity(target.parent / "binding.json"), f.binding["binding_identity"])
            self.assertEqual(target.read_bytes(), f.raw)
            self.assertFalse(f.container.exists())
            lease.publish()
        self.assertEqual(f.owner_path.read_bytes(), owner_bytes)
        self.assertEqual(identity(f.owner_path), owner_id)
        self.assert_source(f)

    def test_persistent_post_rename_parent_fsync_failure_never_succeeds_on_retry(self):
        for parent in ("state", "root"):
            with self.subTest(parent=parent):
                root = self.root / parent
                root.mkdir()
                f = Fixture(root, pending=True)
                saved = f.owner_path.read_bytes()
                real_sync = admission.os.fsync
                failures = []
                def broken(fd):
                    if (f.state / admission.PUBLISHED).exists() and _same_directory(fd, getattr(f, parent)):
                        failures.append(fd)
                        raise OSError(errno.EIO, "注入持续失败")
                    real_sync(fd)
                for _ in range(2):
                    with contextlib.closing(f.load()) as lease, patch.object(admission.os, "fsync", side_effect=broken):
                        with self.assertRaises(admission.AdmissionError):
                            lease.publish()
                        self.assertFalse(lease._published)
                self.assertEqual(len(failures), 2)
                self.assertTrue((f.state / admission.PUBLISHED).exists())
                self.assertEqual(f.owner_path.read_bytes(), saved)
                with contextlib.closing(f.load()) as lease:
                    lease.publish()
                self.assert_source(f)

    def test_sync_order_pending_then_owner_then_rename_then_both_parents(self):
        f = self.fixture(pending=True)
        events = []
        real_sync, real_publish = admission.os.fsync, admission._publish_noreplace
        with contextlib.closing(f.load()) as lease:
            names = {fd: key for key, (fd, _) in lease._files.items()}
            names.update({lease.directory_fd: "pending", lease.state_fd: "state", lease.parent_fd: "db_parent"})
            def sync(fd):
                events.append(names[fd])
                real_sync(fd)
            def publish(fd):
                events.append("rename")
                real_publish(fd)
            with patch.object(admission.os, "fsync", side_effect=sync), patch.object(admission, "_publish_noreplace", side_effect=publish):
                lease.publish()
        self.assertEqual(events, ["limits", "binding", "pending", "state", "owner", "db_parent",
                                  "rename", "db_parent", "state"])

    def test_pending_fsync_failure_preserves_pending_and_owner(self):
        f = self.fixture(pending=True)
        original = f.owner_path.read_bytes()
        with contextlib.closing(f.load()) as lease, patch.object(admission.os, "fsync", side_effect=OSError(errno.EIO, "失败")):
            with self.assertRaises(admission.AdmissionError):
                lease.publish()
        self.assertTrue(f.container.exists())
        self.assertFalse((f.state / admission.PUBLISHED).exists())
        self.assertEqual(f.owner_path.read_bytes(), original)

    def test_atomic_publish_does_not_replace_racing_empty_final(self):
        f = self.fixture(pending=True)
        real_publish = admission._publish_noreplace
        def raced(fd):
            (f.state / admission.PUBLISHED).mkdir(mode=0o700)
            real_publish(fd)
        with contextlib.closing(f.load()) as lease, patch.object(admission, "_publish_noreplace", side_effect=raced):
            with self.assertRaises(admission.AdmissionError):
                lease.publish()
        self.assertTrue(f.container.exists())
        self.assertEqual(list((f.state / admission.PUBLISHED).iterdir()), [])

    def test_missing_owner_does_not_bootstrap_even_without_env(self):
        f = self.fixture()
        f.owner_path.unlink()
        with self.assertRaises(admission.AdmissionError) as caught:
            f.load()
        self.assertEqual(caught.exception.code, "AnchorMissing")
        self.assertEqual(caught.exception.wire_bytes, 0)
        self.assertFalse(f.owner_path.exists())

    def test_missing_entire_final_is_lost_not_first_publication(self):
        f = self.fixture()
        f.container.rename(f.root / "saved")
        with self.assertRaises(admission.AdmissionError) as caught:
            f.load()
        self.assertEqual(caught.exception.code, "AnchorLost")
        self.assertFalse(f.container.exists())

    def test_final_plus_pending_is_conflict_and_preserved(self):
        f = self.fixture()
        pending = f.state / admission.PENDING
        pending.mkdir(mode=0o700)
        with self.assertRaises(admission.AdmissionError) as caught:
            f.load()
        self.assertEqual(caught.exception.code, "AnchorConflict")
        self.assertTrue(pending.exists())

    def test_unanchored_pending_or_owner_without_admission_never_resigns(self):
        f = self.fixture(pending=True)
        f.owner.pop("admission")
        f.owner_path.write_bytes(encode(f.owner))
        saved = f.owner_path.read_bytes()
        with self.assertRaises(admission.AdmissionError):
            f.load()
        self.assertEqual(f.owner_path.read_bytes(), saved)
        self.assertTrue(f.container.exists())

    def test_half_env_fails_before_payload_and_cannot_redirect_db(self):
        f = self.fixture()
        for env in ({admission.ENV_SHA: f.binding["sha256"]},
                    {admission.ENV_IDENTITY: encode(f.binding).decode()}):
            with self.subTest(keys=list(env)), self.assertRaises(admission.AdmissionError) as caught:
                f.load(env)
            self.assertEqual(caught.exception.wire_bytes, 0)
            self.assertIsNone(caught.exception.first_read_at)
        other = {**f.binding, "db": str(f.root / "other.sqlite")}
        with self.assertRaises(admission.AdmissionError):
            f.load({**f.env(), admission.ENV_IDENTITY: encode(other).decode()})
        self.assertFalse((f.root / "other.sqlite").exists())

    def test_invalid_env_and_crosschecks_never_echo_values(self):
        f = self.fixture()
        secret = "不应回显的控制内容"
        for env in ({admission.ENV_SHA: secret, admission.ENV_IDENTITY: "{}"},
                    {admission.ENV_SHA: "0" * 64, admission.ENV_IDENTITY: encode(f.binding).decode()},
                    {**f.env(), admission.ENV_IDENTITY: '{"' + secret + '":1,"' + secret + '":2}'}):
            with self.subTest(env_keys=list(env)), self.assertRaises(admission.AdmissionError) as caught:
                f.load(env)
            self.assertNotIn(secret, str(caught.exception))

    def test_all_limit_fields_reject_noncanonical_values_without_defaults(self):
        f = self.fixture()
        for value in (True, 1, 1.0, "0", "01", "-1", "1.0", "9223372036854775808", "١"):
            for key in admission.LIMIT_FIELDS:
                with self.subTest(key=key, value=value):
                    values = {**f.limits, key: value}
                    raw = encode(values)
                    f.path.write_bytes(raw)
                    f.binding["sha256"] = hashlib.sha256(raw).hexdigest()
                    f.rewrite_binding()
                    with self.assertRaises(admission.AdmissionError):
                        f.load()
        for values in ({}, {**f.limits, "extra": "1"}, {key: value for key, value in f.limits.items() if key != "max_store_bytes"}):
            f.path.write_bytes(encode(values))
            f.binding["sha256"] = hashlib.sha256(encode(values)).hexdigest()
            f.rewrite_binding()
            with self.assertRaises(admission.AdmissionError):
                f.load()

    def test_identity_fields_are_exact_typed_integers(self):
        f = self.fixture()
        original = json.loads(encode(f.binding))
        for field in ("file_identity", "binding_identity", "directory_identity"):
            for value in (True, 1.0, "1", -1, 2**64):
                f.binding[field]["inode"] = value
                f.rewrite_binding()
                with self.subTest(field=field, value=value), self.assertRaises(admission.AdmissionError):
                    f.load()
                f.binding[field] = original[field].copy()
        f.binding["file_identity"]["extra"] = 1
        f.rewrite_binding()
        with self.assertRaises(admission.AdmissionError):
            f.load()

    def test_each_control_file_rejects_links_permissions_and_fifo(self):
        f = self.fixture()
        for path in (f.owner_path, f.binding_path, f.path):
            original, mode = path.read_bytes(), stat.S_IMODE(path.stat().st_mode)
            for kind in ("hardlink", "symlink", "fifo", "mode"):
                with self.subTest(path=path.name, kind=kind):
                    saved = path.with_name(path.name + ".saved")
                    if kind == "hardlink":
                        os.link(path, saved)
                    elif kind == "mode":
                        path.chmod(0o666)
                    else:
                        path.rename(saved)
                        if kind == "symlink":
                            path.symlink_to(saved)
                        else:
                            os.mkfifo(path, 0o600)
                    with self.assertRaises(admission.AdmissionError):
                        f.load()
                    if kind == "hardlink":
                        saved.unlink()
                    elif kind == "mode":
                        path.chmod(mode)
                    else:
                        path.unlink()
                        saved.rename(path)
                    self.assertEqual(path.read_bytes(), original)

    def test_bounded_read_charges_65537th_byte(self):
        f = self.fixture()
        f.owner_path.write_bytes(b" " * (admission.CONTROL_MAX + 1000))
        with self.assertRaises(admission.AdmissionError) as caught:
            f.load()
        self.assertEqual(caught.exception.code, "ControlTooLarge")
        self.assertEqual(caught.exception.wire_bytes, 65537)

    def test_partial_read_then_io_error_keeps_actual_count(self):
        f = self.fixture()
        actual = admission.os.read
        calls = []
        def read(fd, size):
            calls.append(size)
            if len(calls) == 1:
                return actual(fd, 7)
            raise OSError(errno.EIO, "失败")
        with patch.object(admission.os, "read", side_effect=read), self.assertRaises(admission.AdmissionError) as caught:
            f.load()
        self.assertEqual(caught.exception.wire_bytes, 7)

    def test_expired_read_keeps_returned_bytes(self):
        f = self.fixture()
        actual = admission.os.read
        clock = [10.0]
        def read(fd, size):
            result = actual(fd, size)
            clock[0] = 11.0
            return result
        with patch.object(admission.time, "monotonic", side_effect=lambda: clock[0]), \
             patch.object(admission.os, "read", side_effect=read), self.assertRaises(admission.AdmissionError) as caught:
            f.load(deadline=11.0)
        self.assertEqual(caught.exception.code, "Deadline")
        self.assertEqual(caught.exception.wire_bytes, f.owner_path.stat().st_size)
        self.assertEqual(caught.exception.first_read_at, 10.0)

    def test_last_sync_expiry_or_inode_replacement_prevents_success(self):
        f = self.fixture()
        real_sync = admission.os.fsync
        for kind in ("deadline", "replacement"):
            clock = [10.0]
            with self.subTest(kind=kind), patch.object(admission.time, "monotonic", side_effect=lambda: clock[0]):
                with contextlib.closing(f.load(deadline=11.0)) as lease:
                    count = []
                    def sync(fd):
                        real_sync(fd)
                        count.append(fd)
                        if len(count) == 8:
                            if kind == "deadline":
                                clock[0] = 11.0
                            else:
                                saved = f.path.with_suffix(".saved")
                                f.path.rename(saved)
                                f.path.write_bytes(saved.read_bytes())
                                f.path.chmod(0o600)
                    with patch.object(admission.os, "fsync", side_effect=sync), self.assertRaises(admission.AdmissionError):
                        lease.publish()
                    self.assertFalse(lease._published)

    def test_named_budget_is_only_crosscheck_and_counts_its_actual_read(self):
        f = self.fixture(pending=True)
        source = f.root / "configured.json"
        source.write_bytes(f.raw)
        source.chmod(0o644)
        with contextlib.closing(f.load()) as lease:
            before = lease.wire_bytes
            lease.crosscheck_limits(str(source))
            self.assertEqual(lease.wire_bytes, before + len(f.raw))
            lease.publish()
            self.assertEqual(lease.limits_path, f.binding["path"])
            saved = source.with_suffix(".saved")
            source.rename(saved)
            source.write_bytes(saved.read_bytes())
            source.chmod(0o644)
            with self.assertRaises(admission.AdmissionError):
                lease.revalidate()

    def test_explicit_budget_mismatch_rejects_before_pending_publish_or_child(self):
        f = self.fixture(pending=True)
        source = f.root / "configured.json"
        source.write_bytes(encode(f.limits))  # 同值不同原字节也不得替换。
        source.chmod(0o600)
        config = {**f.config(), "admission_job_limits": str(source)}
        with contextlib.closing(control.Controller(config, f.state)) as controller:
            with patch.object(controller, "runtime_environment") as runtime, self.assertRaises(admission.AdmissionError):
                controller.start_s("1")
            runtime.assert_not_called()
        self.assertTrue(f.container.exists())
        self.assertFalse((f.state / admission.PUBLISHED).exists())

    def test_read_metadata_change_and_same_bytes_new_inode_are_rejected(self):
        f = self.fixture()
        actual = admission.os.read
        changed = []
        def read(fd, size):
            chunk = actual(fd, size)
            if not changed:
                changed.append(True)
                f.owner_path.chmod(0o644)
            return chunk
        with patch.object(admission.os, "read", side_effect=read), self.assertRaises(admission.AdmissionError) as caught:
            f.load()
        self.assertEqual(caught.exception.code, "Changed")
        f.owner_path.chmod(0o600)
        for path in (f.owner_path, f.binding_path, f.path):
            with contextlib.closing(f.load()) as lease:
                saved = path.with_name(path.name + ".saved")
                path.rename(saved)
                path.write_bytes(saved.read_bytes())
                path.chmod(0o600)
                with self.assertRaises(admission.AdmissionError):
                    lease.revalidate()
                path.unlink()
                saved.rename(path)

    def test_directory_state_db_and_late_pending_substitutions_are_rejected(self):
        f = self.fixture()
        for path in (f.container, f.state, f.db):
            with self.subTest(path=path.name), contextlib.closing(f.load()) as lease:
                saved = path.with_name(path.name + ".saved")
                path.rename(saved)
                if saved.is_dir():
                    path.mkdir(mode=0o700)
                else:
                    path.write_bytes(saved.read_bytes())
                with self.assertRaises(admission.AdmissionError):
                    lease.revalidate()
                if path.is_dir():
                    path.rmdir()
                else:
                    path.unlink()
                saved.rename(path)
        with contextlib.closing(f.load()) as lease:
            (f.state / admission.PENDING).mkdir()
            with self.assertRaises(admission.AdmissionError):
                lease.revalidate()

    def test_wire_limit_and_original_elapsed_gate_precede_publish(self):
        f = self.fixture(limits={"max_total_wire_bytes": "1"})
        with self.assertRaises(admission.AdmissionError) as caught:
            f.load()
        self.assertEqual(caught.exception.code, "WireLimit")
        self.assertGreater(caught.exception.wire_bytes, 1)
        f.limits["max_total_wire_bytes"] = "1000000"
        f.limits["max_elapsed_ms"] = "20"
        f.path.write_bytes(encode(f.limits))
        f.binding["sha256"] = hashlib.sha256(encode(f.limits)).hexdigest()
        f.rewrite_binding()
        clock = [100.0]
        with patch.object(admission.time, "monotonic", side_effect=lambda: clock[0]):
            with contextlib.closing(f.load(deadline=101.0)) as lease:
                clock[0] = 100.02
                for _ in range(2):
                    with self.assertRaises(admission.AdmissionError) as expired:
                        lease.publish()
                    self.assertEqual(expired.exception.code, "Deadline")
                self.assertEqual(lease.first_read_at, 100.0)

    def test_owner_and_binding_exact_schema_and_duplicate_keys(self):
        f = self.fixture()
        for path, raw in ((f.owner_path, encode({**f.owner, "extra": None})),
                          (f.binding_path, encode({**f.binding, "extra": None})),
                          (f.path, b'{"max_elapsed_ms":"1","max_elapsed_ms":"2"}')):
            original = path.read_bytes()
            path.write_bytes(raw)
            if path == f.path:
                f.binding["sha256"] = hashlib.sha256(raw).hexdigest()
                f.rewrite_binding()
            with self.assertRaises(admission.AdmissionError):
                f.load()
            path.write_bytes(original)

    def test_controller_missing_anchor_cannot_create_owner_or_start_init(self):
        f = self.fixture()
        f.owner_path.unlink()
        with patch.object(control.subprocess, "run") as run, \
             contextlib.closing(control.Controller(f.config(), f.state)) as controller:
            for action in (lambda: controller.status("s"), lambda: controller.start_s("1")):
                with self.assertRaises(admission.AdmissionError):
                    action()
        run.assert_not_called()
        self.assertFalse(f.owner_path.exists())
        self.assert_source(f)

    def test_status_still_reads_owner2_or_lost_budget_but_writer_cannot_use_them(self):
        f = self.fixture()
        f.container.rename(f.root / "saved")
        for owner in (f.owner, {"db": str(f.db), "state_dir": str(f.state)}):
            f.owner_path.write_bytes(encode(owner))
            with contextlib.closing(control.Controller(f.config(), f.state)) as controller:
                self.assertEqual(controller.status("s")["state"], "not_started")
                with self.assertRaises(admission.AdmissionError):
                    controller.start_s("1")
            self.assertFalse(f.container.exists())

    def test_lazy_preparation_cannot_renew_t0_or_adopt_replacement_after_failure(self):
        f = self.fixture()
        clock = [10.0]
        f.limits["max_elapsed_ms"] = "1"
        f.path.write_bytes(encode(f.limits))
        f.binding["sha256"] = hashlib.sha256(encode(f.limits)).hexdigest()
        f.rewrite_binding()
        with patch.object(admission.time, "monotonic", side_effect=lambda: clock[0]):
            with contextlib.closing(control.Controller(f.config(), f.state)) as controller:
                # #1527 H1：owner 首读在动作入口；从该次真实读取起，写准备不得续钟。
                controller.status("s")
                t0 = controller.admission.first_read_at
                self.assertEqual(t0, 10.0)
                clock[0] = 10.002
                with self.assertRaises(admission.AdmissionError) as first:
                    controller.start_s("1")
                before = controller.admission.wire_bytes
                with self.assertRaises(admission.AdmissionError) as retry:
                    controller.start_s("1")
                self.assertIs(retry.exception, first.exception)
                self.assertEqual(first.exception.code, "Deadline")
                self.assertEqual(controller.admission.first_read_at, t0)
                self.assertEqual(controller.admission.wire_bytes, before)

    def test_new_db_is_not_prepared_or_initialized(self):
        f = self.fixture()
        f.db.unlink()
        with patch.object(control.subprocess, "run") as run, self.assertRaises(FileNotFoundError):
            control.Controller(f.config(), f.state)
        run.assert_not_called()
        self.assertFalse(f.db.exists())

    def test_shared_economic_constructor_keeps_original_owner2_contract(self):
        root = self.root.resolve()
        db, state = root / "economic.sqlite", root / "economic-state"
        config = {"db": str(db), "socket_service": "owner", "startup_timeout_ms": "1000"}
        with contextlib.closing(control.Controller(config, state)) as controller:
            self.assertIsNone(controller.admission)
            self.assertEqual(json.loads(Path(str(db) + ".control-owner.json").read_bytes()),
                             {"db": str(db), "state_dir": str(state)})
        self.assertFalse(db.exists())

    def test_real_stub_recover_receives_published_path_env_and_remaining_deadline(self):
        f = self.fixture(pending=True)
        f.stub()
        clock = [10.0]
        real_run = subprocess.run
        calls = []
        def run(argv, **kwargs):
            calls.append((argv, kwargs["timeout"]))
            return real_run(argv, **kwargs)
        def runtime(deadline):
            self.assertEqual(deadline, 20.0)
            clock[0] = 13.0
            return {}
        with patch.object(control.time, "monotonic", side_effect=lambda: clock[0]):
            with contextlib.closing(control.Controller(f.config(), f.state)) as controller:
                with patch.object(controller, "status", return_value={"state": "stopped"}), \
                     patch.object(controller, "runtime_environment", side_effect=runtime), \
                     patch.object(control.subprocess, "run", side_effect=run):
                    result = controller.recover("2", "test-event")
        self.assertEqual(calls[0][1], 7.0)
        self.assertTrue(result["reply"]["stub"])
        self.assertTrue(result["reply"]["sha_match"])
        argv = result["reply"]["argv"]
        self.assertEqual(argv[0], "recover")
        self.assertEqual(argv[argv.index("--admission-job-limits") + 1], f.binding["path"])
        self.assert_source(f)

    def test_start_s2_s3_stub_argv_seven_fields_and_shared_deadline(self):
        f = self.fixture()
        f.stub()
        original = encode(f.config()["s_resources"])
        for revision in ("s-launcher/2", "s-launcher/3"):
            clock = [30.0]
            captured = []
            def start(service, argv, env, **kwargs):
                captured.append((argv, kwargs))
                result = subprocess.run(argv, env=env, capture_output=True, check=True,
                                        timeout=control.remaining(kwargs["deadline"]))
                self.assertTrue(json.loads(result.stdout)["sha_match"])
                clock[0] = 35.0
            def runtime(deadline):
                clock[0] = 32.0
                return {}
            with self.subTest(revision=revision), patch.object(control.time, "monotonic", side_effect=lambda: clock[0]):
                with contextlib.closing(control.Controller(f.config(revision), f.state)) as controller:
                    with patch.object(controller, "runtime_environment", side_effect=runtime), \
                         patch.object(controller, "prepare_socket", return_value=None), \
                         patch.object(controller, "start_process", side_effect=start), \
                         patch.object(controller, "wait_ready", return_value={"state": "ready"}) as ready:
                        result = controller.start_s("1")
                    ready.assert_called_once_with("s", "1", deadline=40.0)
                    self.assertIsNone(result["initialization"])
                    argv, kwargs = captured[0]
                    self.assertEqual(argv[1], "serve")
                    self.assertEqual(argv[argv.index("--admission-job-limits") + 1], f.binding["path"])
                    self.assertEqual(kwargs["deadline"], 40.0)
                    if revision == "s-launcher/3":
                        frozen = json.loads(Path(kwargs["resource_identity"]["path"]).read_bytes())
                        self.assertEqual(set(frozen["s_resources"]), control.S_RESOURCE_FIELDS)
                        self.assertEqual(encode(frozen["s_resources"]), original)
        self.assert_source(f)

    def test_expired_runtime_does_not_spawn_or_renew_action_deadline(self):
        f = self.fixture()
        clock = [1.0]
        with patch.object(control.time, "monotonic", side_effect=lambda: clock[0]):
            with contextlib.closing(control.Controller(f.config(), f.state)) as controller:
                def runtime(deadline):
                    clock[0] = deadline
                    return {}
                with patch.object(controller, "runtime_environment", side_effect=runtime), \
                     patch.object(controller, "start_process") as spawn:
                    for _ in range(2):
                        with self.assertRaises(admission.AdmissionError) as caught:
                            controller.start_s("1")
                        self.assertEqual(caught.exception.code, "Deadline")
                    spawn.assert_not_called()
                self.assertEqual(controller.admission_deadline, 11.0)

    def test_subprocess_error_and_invalid_reply_are_redacted_at_cli(self):
        f = self.fixture()
        secret = "不得回显原控制内容"
        for output, rc in ((secret, 7), ('{"' + secret + '":1,"' + secret + '":2}', 0)):
            f.stub(output=output, rc=rc)
            config = f.root / "config.json"
            config.write_bytes(encode(f.config()))
            error = io.StringIO()
            argv = ["s_service_control.py", "recover", "--config", str(config), "--state-dir", str(f.state),
                    "--new-epoch", "2", "--clock-event-id", "event"]
            with patch.object(sys, "argv", argv), patch.object(control.Controller, "status", return_value={"state": "stopped"}), \
                 patch.object(control.Controller, "runtime_environment", return_value={}), contextlib.redirect_stderr(error):
                self.assertEqual(control.main(), 1)
            self.assertNotIn(secret, error.getvalue())
            parsed = json.loads(error.getvalue())
            self.assertFalse(parsed["ok"])
            if rc:
                self.assertEqual(parsed["returncode"], rc)


class RepairTests(unittest.TestCase):
    """#1527 R4 独评返修锁；不发送真实信号，不启动正式服务。"""

    def setUp(self):
        # macOS 默认临时根过长，子场景名会先撞 launcher 的 100 字节 socket 门，
        # 尚未到达本组欲验证的 admission/receipt 分支；用解析后的短根，不放宽生产门。
        self.temporary = tempfile.TemporaryDirectory(prefix="r4-repair-", dir=os.path.realpath("/tmp"))
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        env = patch.dict(os.environ, {}, clear=True)
        env.start()
        self.addCleanup(env.stop)

    def fixture(self, name, **kwargs):
        root = self.root / name
        root.mkdir()
        return Fixture(root, **kwargs)

    def test_control_actions_ignore_write_env_and_admission(self):
        for owner_version, broken in ((2, "half"), (3, "half"), (3, "binding"), (3, "null")):
            with self.subTest(owner=owner_version, broken=broken):
                f = self.fixture(f"owner{owner_version}-{broken}")
                if owner_version == 2:
                    del f.owner["admission"]
                if broken == "binding":
                    f.binding["file_identity"]["mode"] = 0o644
                if broken == "null":
                    f.owner["admission"] = None
                f.owner_path.write_bytes(encode(f.owner))
                env = {admission.ENV_SHA: f.binding["sha256"]} if broken == "half" else {}
                for service in ("s", "q"):
                    record = dict(service=service, pid=12345, db=str(f.db), process_identity="repair-identity",
                                  command=["repair-stub"], control_instance_id="repair-instance")
                    (f.state / (service + ".process.json")).write_bytes(encode(record))
                with patch.dict(os.environ, env, clear=True), \
                     contextlib.closing(control.Controller(f.config("s-launcher/2"), f.state)) as c:
                    for service in ("s", "q"):
                        with patch.object(control, "process_identity", return_value="repair-identity"):
                            self.assertEqual(c.status(service)["state"], "running")
                        with patch.object(control, "process_identity", side_effect=["repair-identity", "repair-identity", None]), \
                             patch.object(control.os, "kill") as kill:
                            self.assertEqual(c.stop(service, signal.SIGTERM)["state"], "stopped")
                            kill.assert_called_once_with(12345, signal.SIGTERM)
                    t0, wire = c.admission.first_read_at, c.admission.wire_bytes
                    with patch.object(c, "runtime_environment") as runtime, self.assertRaises(admission.AdmissionError) as caught:
                        c.start_s("1")
                    runtime.assert_not_called()
                    self.assertEqual(c.admission.first_read_at, t0)
                    self.assertEqual(c.admission.wire_bytes, wire)
                    self.assertEqual(caught.exception.wire_bytes, wire)
                    self.assertEqual(caught.exception.first_read_at, t0)
                    if broken == "half":
                        self.assertEqual(caught.exception.stage, "E0")
                    with patch.object(c, "query_resource_revision", return_value="legacy/2"), \
                         patch.object(c, "start_process") as start, patch.object(c, "wait_ready", return_value={"state": "ready"}):
                        self.assertEqual(c.start_q("1")["state"], "ready")
                        start.assert_called_once()

    def _h1_r3_snapshot(self, lease):
        return (lease.first_read_at, lease.wire_bytes, lease.deadline, lease.job_deadline,
                tuple(lease._fds), dict(lease._files), lease._owner)

    def _h1_r3_reject_write(self, c, raw, codes=("Invalid",)):
        before = self._h1_r3_snapshot(c.admission)
        with patch.object(admission.os, "read", side_effect=AssertionError("不得再读 payload")), \
             patch.object(admission, "_object", wraps=admission._object) as strict, \
             patch.object(c, "runtime_environment") as runtime:
            with self.assertRaises(admission.AdmissionError) as first:
                c.start_s("1")
            strict.assert_called_once_with(raw)
            self.assertIn(first.exception.code, codes)
            self.assertEqual(first.exception.stage, "E1")
            self.assertEqual((first.exception.first_read_at, first.exception.wire_bytes), before[:2])
            runtime.assert_not_called()
            with patch.object(admission.os, "open", side_effect=AssertionError("首错后不得重开")):
                for retry in (lambda: c.recover("2", "h1"), lambda: c.start_s("1"), c.admission.publish):
                    with self.assertRaises(admission.AdmissionError) as again:
                        retry()
                    self.assertIs(again.exception, first.exception)
            self.assertEqual(self._h1_r3_snapshot(c.admission), before)

    def test_h1_r3_fresh_cli_decimal_controls_then_strict_write(self):
        for variant in ("float_mode", "float_payload"):
            for action in ("status", "stop-s", "start-q"):
                with self.subTest(variant=variant, action=action):
                    f = self.fixture(variant + action)
                    owner = json.loads(encode(f.owner))
                    if variant == "float_mode":
                        owner["admission"]["file_identity"]["mode"] = 384.0
                    else:
                        owner["admission"] = 0.5
                    raw = encode(owner)
                    f.owner_path.write_bytes(raw)
                    config = f.root / "config.json"
                    config.write_bytes(encode(f.config("s-launcher/2")))
                    for service in ("s", "q"):
                        record = dict(service=service, pid=12345, db=str(f.db), process_identity="h1-identity",
                                      command=["h1-stub"], control_instance_id="h1-instance")
                        (f.state / (service + ".process.json")).write_bytes(encode(record))
                    argv = ["control", action, "--config", str(config), "--state-dir", str(f.state)]
                    instances, stdout, stderr = [], io.StringIO(), io.StringIO()
                    original_close = control.Controller.close
                    observed = ["h1-identity", "h1-identity", None] if action == "stop-s" else ["h1-identity"] * 2
                    try:
                        with patch.object(sys, "argv", argv), \
                             patch.object(control, "process_identity", side_effect=observed) as observe, \
                             patch.object(control.os, "kill") as kill, \
                             patch.object(control.subprocess, "run", side_effect=AssertionError("禁止真实子进程")), \
                             patch.object(control.Controller, "close", autospec=True, side_effect=instances.append), \
                             patch.object(control.Controller, "query_resource_revision", return_value="legacy/2"), \
                             patch.object(control.Controller, "start_process") as start, \
                             patch.object(control.Controller, "wait_ready", return_value={"state": "ready"}), \
                             contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                            rc = control.main()
                            self.assertEqual(rc, 0, stderr.getvalue())
                            self.assertEqual(stderr.getvalue(), "")
                            result = json.loads(stdout.getvalue())
                            self.assertTrue(result["ok"])
                            self.assertEqual(len(instances), 1)
                            c = instances[0]
                            self.assertEqual(c.admission.wire_bytes, len(raw))
                            self.assertIsNotNone(c.admission.first_read_at)
                            if action == "status":
                                self.assertEqual([r["state"] for r in result["result"]], ["running", "running"])
                                self.assertEqual(observe.call_count, 2)
                            elif action == "stop-s":
                                self.assertEqual(result["result"][0]["state"], "stopped")
                                self.assertEqual(observe.call_count, 3)
                                kill.assert_called_once_with(12345, signal.SIGTERM)
                            else:
                                self.assertEqual(result["result"]["state"], "ready")
                                observe.assert_not_called()
                                start.assert_called_once()
                                self.assertEqual(start.call_args.args[0], "q")
                                self.assertIn(str(f.db), start.call_args.args[1])
                            if action != "stop-s":
                                kill.assert_not_called()
                            if action != "start-q":
                                start.assert_not_called()
                            self._h1_r3_reject_write(c, raw)
                            self.assertEqual(c.admission.deadline, c.admission_deadline)
                    finally:
                        for c in instances:
                            held = tuple(c.admission._fds)
                            original_close(c)
                            for fd in held:
                                with self.assertRaises(OSError):
                                    os.fstat(fd)
                    self.assertEqual(hashlib.sha256(f.db.read_bytes()).hexdigest(), f.db_hash)

    def test_h1_r3_nested_duplicates_and_numeric_tokens_are_write_only(self):
        payloads = ("duplicate_plain", "duplicate_escaped",
                    b'[{"x":1,"x":2},0.5]', b'1e400', b'-0.5', b'0e0', b'9' * 5000)
        for i, payload in enumerate(payloads):
            with self.subTest(variant=i):
                f = self.fixture(str(i))
                if isinstance(payload, str):
                    duplicate = b'"mode":384' if payload == "duplicate_plain" else b'"m\\u006fde":384'
                    payload = encode(f.binding).replace(b'"mode":384', b'"mode":384,' + duplicate, 1)
                raw = encode({"db": str(f.db), "state_dir": str(f.state)})[:-1] + b',"admission":' + payload + b'}'
                f.owner_path.write_bytes(raw)
                with contextlib.closing(control.Controller(f.config(), f.state)) as c:
                    self.assertEqual(c.status("s")["state"], "not_started")
                    # 超长整数可在解码器或完整 binding 类型检查处拒绝，均不得取得写资格。
                    codes = ("Invalid", "BindingInvalid") if i == 6 else ("Invalid",)
                    self._h1_r3_reject_write(c, raw, codes)

    def test_h1_r3_owner_structure_rejects_before_process_observation(self):
        variants = ("duplicate_db", "duplicate_admission", "escaped_db", "nested_nan", "nested_infinity",
                    "nested_negative_infinity", "utf8", "syntax", "array", "scalar", "extra", "wrong_db", "wrong_state")
        for variant in variants:
            with self.subTest(variant=variant):
                f = self.fixture(variant)
                owner = {**f.owner, "admission": 0.5}
                raw = encode(owner)
                if variant == "duplicate_db":
                    raw = raw[:-1] + b',"db":' + encode(str(f.db)) + b'}'
                elif variant == "duplicate_admission":
                    raw = raw[:-1] + b',"admission":0.5}'
                elif variant == "escaped_db":
                    raw = raw[:-1] + b',"d\\u0062":' + encode(str(f.db)) + b'}'
                elif variant.startswith("nested_"):
                    token = {"nested_nan": b'NaN', "nested_infinity": b'Infinity',
                             "nested_negative_infinity": b'-Infinity'}[variant]
                    raw = raw.replace(b'0.5', b'{"n":' + token + b'}')
                elif variant == "utf8":
                    raw = raw.replace(b'0.5', b'"\xff"')
                elif variant == "syntax":
                    raw = raw[:-1]
                elif variant == "array":
                    raw = encode(list(owner.items()))
                elif variant == "scalar":
                    raw = b'0.5'
                elif variant == "extra":
                    raw = encode({**owner, "extra": None})
                else:
                    owner["db" if variant == "wrong_db" else "state_dir"] = str(f.root / "wrong")
                    raw = encode(owner)
                f.owner_path.write_bytes(raw)
                with contextlib.closing(control.Controller(f.config(), f.state)) as c, \
                     patch.object(control, "process_identity") as observe, patch.object(control.os, "kill") as kill:
                    with self.assertRaises(admission.AdmissionError) as first:
                        c.status("s")
                    self.assertEqual(first.exception.stage, "E1")
                    self.assertEqual(first.exception.wire_bytes, len(raw))
                    with self.assertRaises(admission.AdmissionError) as retry:
                        c.stop("s", signal.SIGTERM)
                    self.assertIs(retry.exception, first.exception)
                    observe.assert_not_called()
                    kill.assert_not_called()

    def test_h1_r3_control_cache_rejects_owner_substitute_without_payload_read(self):
        for next_action in ("status", "write"):
            f = self.fixture(next_action)
            raw = encode({**f.owner, "admission": 0.5})
            f.owner_path.write_bytes(raw)
            with contextlib.closing(control.Controller(f.config(), f.state)) as c:
                self.assertEqual(c.status("s")["state"], "not_started")
                before = self._h1_r3_snapshot(c.admission)
                f.owner_path.rename(f.root / "original-owner.json")
                f.owner_path.write_bytes(encode(f.owner))
                f.owner_path.chmod(0o600)
                with patch.object(admission.os, "read", side_effect=AssertionError("不得读替身")), \
                     patch.object(c, "runtime_environment") as runtime:
                    with self.assertRaises(admission.AdmissionError) as caught:
                        c.status("s") if next_action == "status" else c.start_s("1")
                    self.assertEqual(caught.exception.code, "Changed")
                    self.assertEqual(self._h1_r3_snapshot(c.admission), before)
                    runtime.assert_not_called()

    def test_h1_r3_control_cache_does_not_renew_write_deadline(self):
        f = self.fixture("cached-deadline")
        f.owner_path.write_bytes(encode({**f.owner, "admission": 0.5}))
        clock = [10.0]
        with patch.object(control.time, "monotonic", side_effect=lambda: clock[0]), \
             contextlib.closing(control.Controller(f.config(), f.state)) as c:
            c.status("s")
            before = self._h1_r3_snapshot(c.admission)
            clock[0] = c.admission_deadline
            with patch.object(admission.os, "read", side_effect=AssertionError("过期不得读")):
                with self.assertRaises(admission.AdmissionError) as first:
                    c.start_s("1")
                self.assertEqual(first.exception.code, "Deadline")
                with self.assertRaises(admission.AdmissionError) as retry:
                    c.recover("2", "h1")
                self.assertIs(retry.exception, first.exception)
            self.assertEqual(self._h1_r3_snapshot(c.admission), before)

    def test_h1_r3_valid_control_then_write_preserves_original_owner_and_budget(self):
        for pending in (False, True):
            f = self.fixture(str(pending), pending=pending)
            with contextlib.closing(control.Controller(f.config("s-launcher/2"), f.state)) as c:
                self.assertEqual(c.status("s")["state"], "not_started")
                before = self._h1_r3_snapshot(c.admission)
                raw = f.owner_path.read_bytes()
                reads, real_read = [], admission.os.read
                def read(fd, size):
                    chunk = real_read(fd, size)
                    if stat.S_ISREG(os.fstat(fd).st_mode):
                        reads.append((fd, len(chunk)))
                    return chunk
                with patch.object(admission.os, "read", side_effect=read), \
                     patch.object(admission, "_object", wraps=admission._object) as strict, \
                     patch.object(c, "runtime_environment", return_value={}) as runtime, \
                     patch.object(c, "prepare_socket"), patch.object(c, "start_process") as start, \
                     patch.object(c, "wait_ready", return_value={"state": "ready"}):
                    self.assertEqual(c.start_s("1")["state"], "ready")
                    self.assertEqual(strict.call_args_list[0].args, (raw,))
                    runtime.assert_called_once_with(before[2])
                    start.assert_called_once()
                    self.assertEqual(start.call_args.kwargs["deadline"], before[2])
                self.assertEqual(c.admission.first_read_at, before[0])
                self.assertEqual(c.admission.deadline, before[2])
                self.assertEqual(c.admission.job_deadline, before[0] + int(f.limits["max_elapsed_ms"]) / 1000)
                self.assertEqual(tuple(c.admission._fds[:len(before[4])]), before[4])
                self.assertEqual(c.admission._files["owner"], before[5]["owner"])
                self.assertNotIn(before[5]["owner"][0], [fd for fd, size in reads])
                self.assertEqual(c.admission.wire_bytes, len(raw) + sum(size for fd, size in reads))
                self.assertEqual(c.admission.wire_bytes, len(raw) + len(encode(f.binding)) + len(f.raw))
                self.assertEqual(Path(c.admission.limits_path).read_bytes(), f.raw)
                self.assertEqual(f.owner_path.read_bytes(), raw)
            self.assertEqual(hashlib.sha256(f.db.read_bytes()).hexdigest(), f.db_hash)

    def test_deferred_first_read_keeps_original_startup_deadline(self):
        f = self.fixture("deadline")
        clock = [10.0]
        with patch.object(control.time, "monotonic", side_effect=lambda: clock[0]), \
             contextlib.closing(control.Controller(f.config(), f.state)) as c:
            self.assertEqual(c.admission_deadline, 20.0)
            self.assertIsNone(c.admission.first_read_at)
            clock[0] = 20.0
            with patch.object(admission.os, "read") as read, self.assertRaises(admission.AdmissionError) as caught:
                c.start_s("1")
            self.assertEqual(caught.exception.code, "Deadline")
            self.assertEqual(c.admission_deadline, 20.0)
            read.assert_not_called()

    def test_fresh_write_half_env_is_e0_zero_reads_and_latched(self):
        for version in (2, 3):
            for action in ("start_s", "recover"):
                f = self.fixture(f"{version}-{action}")
                if version == 2:
                    del f.owner["admission"]
                    f.owner_path.write_bytes(encode(f.owner))
                with patch.dict(os.environ, {admission.ENV_SHA: f.binding["sha256"]}, clear=True), \
                     patch.object(admission.os, "read", wraps=admission.os.read) as read, \
                     contextlib.closing(control.Controller(f.config(), f.state)) as c:
                    args = ("1",) if action == "start_s" else ("2", "repair")
                    with self.assertRaises(admission.AdmissionError) as first:
                        getattr(c, action)(*args)
                    self.assertEqual((first.exception.code, first.exception.stage, first.exception.wire_bytes),
                                     ("ControlMismatch", "E0", 0))
                    self.assertIsNone(first.exception.first_read_at)
                    with patch.dict(os.environ, f.env(), clear=True), self.assertRaises(admission.AdmissionError) as retry:
                        getattr(c, action)(*args)
                    self.assertIs(retry.exception, first.exception)
                    read.assert_not_called()

    def test_control_authority_and_process_identity_still_required(self):
        for bad in ("db", "state_dir", "process"):
            f = self.fixture(bad)
            if bad != "process":
                f.owner[bad] = str(f.root / "other")
                f.owner_path.write_bytes(encode(f.owner))
            record = dict(service="q", pid=12345, db=str(f.db), process_identity="expected",
                          command=["repair-stub"], control_instance_id="repair-instance")
            (f.state / "q.process.json").write_bytes(encode(record))
            with patch.dict(os.environ, {admission.ENV_SHA: "half"}, clear=True), \
                 patch.object(control, "process_identity", return_value="reused"), patch.object(control.os, "kill") as kill:
                with self.assertRaises(ValueError), contextlib.closing(control.Controller(f.config(), f.state)) as c:
                    c.stop("q", signal.SIGTERM)
                kill.assert_not_called()

    def test_configured_errors_pin_first_failure_without_reopen(self):
        for kind in ("permission", "mismatch", "read"):
            f = self.fixture(kind, pending=True)
            named = f.root / "configured.json"
            named.write_bytes(b"{}" if kind == "mismatch" else f.raw)
            named.chmod(0o666 if kind == "permission" else 0o600)
            old_inode = named.stat().st_ino
            real_read = admission.os.read
            partial = []
            def read(fd, size):
                if kind == "read" and os.fstat(fd).st_ino == old_inode:
                    if not partial:
                        partial.append(True)
                        return real_read(fd, 7)
                    raise OSError(errno.EIO, "人工读取故障")
                return real_read(fd, size)
            with contextlib.closing(control.Controller({**f.config(), "admission_job_limits": str(named)}, f.state)) as c:
                with patch.object(admission.os, "read", side_effect=read), self.assertRaises(admission.AdmissionError) as first:
                    c.start_s("1")
                t0, wire, fds = c.admission.first_read_at, c.admission.wire_bytes, list(c.admission._fds)
                expected_wire = sum(p.stat().st_size for p in (f.owner_path, f.binding_path, f.path))
                self.assertEqual(wire, expected_wire + {"permission": 0, "mismatch": 2, "read": 7}[kind])
                named.rename(f.root / "saved.json")
                named.write_bytes(f.raw)
                named.chmod(0o600)
                self.assertNotEqual(named.stat().st_ino, old_inode)
                with patch.object(admission.os, "open", side_effect=AssertionError("失败后禁止重开")), \
                     patch.object(c, "runtime_environment") as runtime:
                    for retry_action in (lambda: c.start_s("1"), c.admission.publish,
                                         lambda: c.admission.crosscheck_limits(f.binding["path"])):
                        with self.assertRaises(admission.AdmissionError) as retry:
                            retry_action()
                        self.assertIs(retry.exception, first.exception)
                    runtime.assert_not_called()
                self.assertEqual((c.admission.first_read_at, c.admission.wire_bytes, c.admission._fds), (t0, wire, fds))
                self.assertTrue(f.container.exists())
                self.assertFalse((f.state / admission.PUBLISHED).exists())
            for fd in fds:
                with self.assertRaises(OSError):
                    os.fstat(fd)

    def test_valid_crosscheck_keeps_same_object_fsync_retry(self):
        for parent in ("root", "state"):
            f = self.fixture(parent, pending=True)
            named = f.root / "configured.json"
            named.write_bytes(f.raw)
            named.chmod(0o600)
            real_sync = admission.os.fsync
            def sync(fd):
                if (f.state / admission.PUBLISHED).exists() and _same_directory(fd, getattr(f, parent)):
                    raise OSError(errno.EIO, "人工持续同步故障")
                real_sync(fd)
            with contextlib.closing(control.Controller({**f.config(), "admission_job_limits": str(named)}, f.state)) as c:
                with patch.object(admission.os, "fsync", side_effect=sync):
                    for _ in range(2):
                        with self.assertRaises(admission.AdmissionError) as caught:
                            c.start_s("1")
                        self.assertEqual(caught.exception.code, "IO")
                t0, wire, held = c.admission.first_read_at, c.admission.wire_bytes, c.admission._files["configured"]
                with patch.object(c, "runtime_environment", return_value={}), patch.object(c, "prepare_socket"), \
                     patch.object(c, "start_process") as start, patch.object(c, "wait_ready", return_value={"state": "ready"}):
                    self.assertEqual(c.start_s("1")["state"], "ready")
                    start.assert_called_once()
                self.assertEqual((c.admission.first_read_at, c.admission.wire_bytes, c.admission._files["configured"]),
                                 (t0, wire, held))

    def test_non_admission_errors_keep_public_receipts_and_redaction(self):
        for kind in ("probe", "probe_reply", "child", "reply"):
            for action in (("start-s", "recover") if kind.startswith("probe") else ("recover",)):
                f = self.fixture(kind + action)
                marker = "repair-private-marker"
                f.stub(output=marker, rc=7 if kind == "child" else 0)
                config = f.root / "config.json"
                config_value = f.config("s-launcher/2")
                if kind.startswith("probe"):
                    probe = f.root / "probe.py"
                    probe.write_text("#!" + sys.executable + "\nimport sys\n"
                                     "sys.stderr.write(" + repr(marker) + ")\nprint('null')\n"
                                     f"sys.exit({9 if kind == 'probe' else 0})\n")
                    probe.chmod(0o700)
                    config_value["python"] = str(probe)
                config.write_bytes(encode(config_value))
                argv = ["control", action, "--config", str(config), "--state-dir", str(f.state)]
                if action == "recover":
                    argv += ["--new-epoch", "2", "--clock-event-id", "repair"]
                output = io.StringIO()
                runtime = (contextlib.nullcontext() if kind.startswith("probe") else
                           patch.object(control.Controller, "runtime_environment", return_value={}))
                observed = []
                original_close = control.Controller.close
                def close(c):
                    observed.append((c.admission.wire_bytes, c.admission.first_read_at))
                    original_close(c)
                with patch.object(sys, "argv", argv), patch.object(control.Controller, "status", return_value={"state": "stopped"}), \
                     runtime, \
                     patch.object(control.Controller, "close", close), contextlib.redirect_stderr(output):
                    self.assertEqual(control.main(), 1)
                receipt = json.loads(output.getvalue())
                stage = {"probe": "runtime_probe", "probe_reply": "runtime_probe",
                         "child": "recover_process", "reply": "recover_reply"}[kind]
                self.assertEqual(receipt["stage"], stage)
                self.assertEqual(receipt["admission_stage"], "E4")
                self.assertEqual(receipt["wire_bytes"], observed[0][0])
                self.assertEqual(receipt["wire_bytes"], sum(p.stat().st_size for p in (f.owner_path, f.binding_path, f.path)))
                self.assertEqual(receipt["wire_scope"], "python_control_prelude")
                self.assertIsNotNone(observed[0][1])
                self.assertGreaterEqual(receipt["elapsed_ms"], 0)
                self.assertEqual(receipt["returncode"], {"probe": 9, "probe_reply": 0, "child": 7, "reply": 0}[kind])
                self.assertNotIn(marker, output.getvalue())
                self.assertNotIn("first_read_at", receipt)

    def test_runtime_failure_retries_keep_first_receipt_and_t0(self):
        f = self.fixture("retry")
        error = ValueError("人工私有错误")
        with contextlib.closing(control.Controller(f.config(), f.state)) as c:
            with patch.object(c, "runtime_environment", side_effect=error) as runtime:
                with self.assertRaises(ValueError) as first:
                    c.start_s("1")
                wire, t0 = c.admission.wire_bytes, c.admission.first_read_at
                receipt = dict(first.exception.control_receipt)
                with self.assertRaises(ValueError) as retry:
                    c.start_s("1")
                self.assertIs(first.exception, retry.exception)
                self.assertEqual(retry.exception.control_receipt, receipt)
                runtime.assert_called_once()
                self.assertEqual((c.admission.wire_bytes, c.admission.first_read_at), (wire, t0))


def _same_directory(fd, path):
    a, b = os.fstat(fd), path.stat()
    return (a.st_dev, a.st_ino) == (b.st_dev, b.st_ino)


if __name__ == "__main__":
    unittest.main()
