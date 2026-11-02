"""#1527 离线合同模型与临时文件负控；不导入/执行 Rust 或生产 R4。"""

import errno
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import time
import unittest


LIMIT_FIELDS = (
    "max_total_wire_bytes", "max_total_node_bytes", "max_total_nodes",
    "max_total_records", "max_total_requests", "max_elapsed_ms",
    "max_staged_bytes", "max_store_bytes",
)
IDENTITY_FIELDS = {"device", "inode", "uid", "mode"}
BINDING_FIELDS = {
    "db", "path", "sha256", "file_identity", "binding_identity", "directory_identity",
}
MAX_BYTES = 65536
FLAGS = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC


class Refusal(Exception):
    """错误不得携带底层异常、路径或 JSON 原文。"""

    def __init__(self, kind, account, phase):
        self.kind = kind
        self.phase = phase
        self.wire = account.wire
        self.started = account.started
        self.trusted = account.limits is not None
        super().__init__(f"模型拒绝：{kind}；阶段={phase}；实读={self.wire}；可信={self.trusted}")


class Account:
    def __init__(self, now=time.monotonic_ns):
        self.now = now
        self.started = None
        self.wire = 0
        self.limits = None

    def fail(self, kind, phase):
        raise Refusal(kind, self, phase) from None

    def gate(self, phase):
        if self.limits is None:
            return
        now = self.now()
        deadline = self.started + self.limits["max_elapsed_ms"] * 1_000_000
        if now < self.started or now >= deadline:
            self.fail("Deadline", phase)
        if self.wire > self.limits["max_total_wire_bytes"]:
            self.fail("Limit", phase)


def exact(value, fields):
    if type(value) is not dict or value.keys() != set(fields):
        raise ValueError("字段集合")
    return value


def parse(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("重复字段")
            result[key] = value
        return result

    def constant(_):
        raise ValueError("非法 JSON 常数")

    return json.loads(raw, object_pairs_hook=pairs, parse_constant=constant)


def abs_path(value):
    if type(value) is not str or not value.startswith("/") or "\0" in value:
        raise ValueError("路径形状")
    if str(Path(value)) != value or ".." in Path(value).parts:
        raise ValueError("未解析路径")
    return value


def identity(meta):
    return {"device": meta.st_dev, "inode": meta.st_ino,
            "uid": meta.st_uid, "mode": stat.S_IMODE(meta.st_mode)}


def typed_identity(value, mode):
    exact(value, IDENTITY_FIELDS)
    # 模型域固定采用常见 Unix u64 dev/ino、u32 uid；Rust 另按 libc 本机域 checked。
    for key, number in value.items():
        if type(number) is not int or not 0 <= number <= (2**64 - 1):
            raise ValueError("身份整数")
        if key in ("uid", "mode") and number > (2**32 - 1):
            raise ValueError("身份整数域")
    if value["uid"] != os.geteuid() or value["mode"] != mode:
        raise ValueError("身份权限")


def binding(value):
    exact(value, BINDING_FIELDS)
    abs_path(value["db"])
    abs_path(value["path"])
    digest = value["sha256"]
    if type(digest) is not str or len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
        raise ValueError("摘要")
    for key, mode in (("file_identity", 0o600), ("binding_identity", 0o600), ("directory_identity", 0o700)):
        typed_identity(value[key], mode)
    return value


def limits(raw):
    obj = exact(parse(raw), LIMIT_FIELDS)
    result = {}
    for field, text in obj.items():
        if (type(text) is not str or not text or text[0] == "0"
                or any(c not in "0123456789" for c in text) or len(text) > 19):
            raise ValueError("非规范限额")
        number = int(text)
        if not 1 <= number <= 2**63 - 1:
            raise ValueError("限额域")
        result[field] = number
    return result


def stamp(meta):
    return (identity(meta), meta.st_size, meta.st_mtime_ns, meta.st_ctime_ns,
            meta.st_nlink, stat.S_IFMT(meta.st_mode))


def directory_path(path):
    parts = Path(path).parts
    if not parts or parts[0] != "/":
        raise ValueError("非绝对目录")
    fd = os.open("/", FLAGS | os.O_DIRECTORY)
    try:
        for name in parts[1:]:
            if name in (".", ".."):
                raise ValueError("目录分量")
            next_fd = os.open(name, FLAGS | os.O_DIRECTORY, dir_fd=fd)
            os.close(fd)
            fd = next_fd
        return fd
    except BaseException:
        os.close(fd)
        raise


def exists_at(fd, name):
    try:
        os.stat(name, dir_fd=fd, follow_symlinks=False)
        return True
    except FileNotFoundError:
        return False


def read_payload(fd, account, phase, reader=os.read):
    data = bytearray()
    while True:
        account.gate(phase)
        try:
            part = reader(fd, min(4096, MAX_BYTES + 1 - len(data)))
        except OSError:
            account.gate(phase)
            account.fail("ReadFailed", phase)
        account.wire += len(part)
        account.gate(phase)
        data.extend(part)
        if len(data) > MAX_BYTES:
            account.fail("TooLarge", phase)
        if not part:
            return bytes(data)


class Lease:
    """模型只保留 FD/快照/原字节；无预算时钟或重新解析替身的入口。"""

    def __init__(self):
        self.fds = []
        self.controls = []
        self.dirs = []

    def hold(self, fd):
        self.fds.append(fd)
        return fd

    def pin_dir(self, path, fd):
        self.dirs.append((path, fd, identity(os.fstat(fd))))
        return fd

    def read(self, directory, path, account, phase, owner=False):
        try:
            fd = self.hold(os.open(path.name, FLAGS, dir_fd=directory))
        except FileNotFoundError:
            account.fail("AnchorMissing" if owner else "AnchorLost", phase)
        except OSError:
            account.fail("Changed", phase)
        meta = os.fstat(fd)
        allowed = (0o600, 0o644) if owner else (0o600,)
        if (not stat.S_ISREG(meta.st_mode) or meta.st_nlink != 1
                or meta.st_uid != os.geteuid() or stat.S_IMODE(meta.st_mode) not in allowed):
            account.fail("Changed", phase)
        before = stamp(meta)
        raw = read_payload(fd, account, phase)
        if stamp(os.fstat(fd)) != before:
            account.fail("Changed", phase)
        self.controls.append((directory, path, fd, before, raw))
        return raw, identity(meta)

    def revalidate(self):
        for path, fd, original in self.dirs:
            fresh = directory_path(path)
            try:
                if identity(os.fstat(fd)) != original or identity(os.fstat(fresh)) != original:
                    raise ValueError("目录变化")
            finally:
                os.close(fresh)
        for parent, path, pinned, before, _ in self.controls:
            absolute_parent = directory_path(path.parent)
            try:
                for directory in (parent, absolute_parent):
                    fresh = os.open(path.name, FLAGS, dir_fd=directory)
                    try:
                        if stamp(os.fstat(fresh)) != before or stamp(os.fstat(pinned)) != before:
                            raise ValueError("控制件变化")
                    finally:
                        os.close(fresh)
            finally:
                os.close(absolute_parent)
        db, fd, original = self.db
        fresh = os.open(db, FLAGS)
        try:
            if identity(os.fstat(fd)) != original or identity(os.fstat(fresh)) != original:
                raise ValueError("库身份变化")
        finally:
            os.close(fresh)
        if exists_at(self.state, ".s-admission-pending"):
            raise ValueError("待发布")

    def close(self):
        while self.fds:
            os.close(self.fds.pop())


class Prepared:
    def __init__(self, lease, account):
        self.lease = lease
        self.account = account
        self.consumed = False

    def consume(self):
        if self.consumed:
            self.account.fail("Consumed", "E5")
        self.consumed = True
        self.account.gate("E5")
        try:
            self.lease.revalidate()
        except (OSError, ValueError):
            self.account.gate("E5")
            self.account.fail("Changed", "E5")
        self.account.gate("E5")
        return self.lease


def load(db, path, env=(None, None), account=None):
    account = account if account is not None else Account()
    lease = Lease()
    phase = "E0"
    parse_kind = "ControlMismatch"
    try:
        environment = None
        sha, wire = env
        if (sha is None) != (wire is None):
            account.fail("ControlMismatch", phase)
        if sha is not None:
            if not isinstance(wire, bytes) or len(wire) > MAX_BYTES:
                account.fail("ControlMismatch", phase)
            environment = binding(parse(wire))
            if environment["sha256"] != sha:
                account.fail("ControlMismatch", phase)
        parse_kind = "ResourcesInvalid"
        abs_path(str(path))
        db = db.resolve(strict=True)
        parent = lease.hold(directory_path(db.parent))
        lease.pin_dir(db.parent, parent)
        db_fd = lease.hold(os.open(db.name, FLAGS, dir_fd=parent))
        if not stat.S_ISREG(os.fstat(db_fd).st_mode):
            account.fail("Identity", phase)
        lease.db = (db, db_fd, identity(os.fstat(db_fd)))
        phase, parse_kind = "E1", "AnchorInvalid"
        account.started = account.now()
        owner_path = Path(str(db) + ".control-owner.json")
        raw, _ = lease.read(parent, owner_path, account, phase, owner=True)
        owner = exact(parse(raw), ("db", "state_dir", "admission"))
        state = Path(abs_path(owner["state_dir"]))
        anchor = binding(owner["admission"])
        expected = state / "s-admission" / "limits.json"
        if owner["db"] != str(db) or anchor["db"] != str(db) or anchor["path"] != str(expected) or path != expected:
            account.fail("Identity", phase)
        phase = "E2"
        state_fd = lease.hold(directory_path(state))
        lease.pin_dir(state, state_fd)
        lease.state = state_fd
        if exists_at(state_fd, ".s-admission-pending"):
            account.fail("AnchorUnpublished", phase)
        published = state / "s-admission"
        pub_fd = lease.hold(os.open("s-admission", FLAGS | os.O_DIRECTORY, dir_fd=state_fd))
        lease.pin_dir(published, pub_fd)
        if identity(os.fstat(pub_fd)) != anchor["directory_identity"]:
            account.fail("Changed", phase)
        phase, parse_kind = "E3", "BindingInvalid"
        raw, bid = lease.read(pub_fd, published / "binding.json", account, phase)
        actual = binding(parse(raw))
        if actual != anchor or bid != anchor["binding_identity"]:
            account.fail("Changed", phase)
        raw, lid = lease.read(pub_fd, expected, account, phase)
        if lid != anchor["file_identity"] or hashlib.sha256(raw).hexdigest() != anchor["sha256"]:
            account.fail("Changed", phase)
        parse_kind = "ResourcesInvalid"
        account.limits = limits(raw)
        phase = "E4"
        account.gate(phase)
        if environment is not None and environment != anchor:
            account.fail("ControlMismatch", phase)
        parse_kind = "Changed"
        lease.revalidate()
        account.gate(phase)
        return Prepared(lease, account)
    except Refusal:
        lease.close()
        raise
    except (ValueError, UnicodeError, TypeError, KeyError):
        lease.close()
        account.gate(phase)
        account.fail(parse_kind, phase)
    except OSError:
        lease.close()
        account.gate(phase)
        account.fail("AnchorLost" if phase in ("E2", "E3") else "Changed", phase)


def encode(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()


class Fixture:
    """直接构造 TestOnly wire，不模拟 producer 已完成 fsync 的证明。"""

    def __init__(self, parent, raw=None):
        self.root = Path(tempfile.mkdtemp(dir=parent)).resolve()
        self.db = self.root / "db"
        self.db.write_bytes(b"TestOnly-not-SQLite")
        self.state = self.root / "state"
        self.state.mkdir(mode=0o755)
        self.published = self.state / "s-admission"
        self.published.mkdir(mode=0o700)
        self.path = self.published / "limits.json"
        self.binding_path = self.published / "binding.json"
        self.owner_path = Path(str(self.db) + ".control-owner.json")
        self.raw = raw if raw is not None else encode(dict.fromkeys(LIMIT_FIELDS, "1000000"))
        self.path.write_bytes(self.raw)
        self.path.chmod(0o600)
        self.binding_path.write_bytes(b"")
        self.binding_path.chmod(0o600)
        self.anchor = {
            "db": str(self.db), "path": str(self.path), "sha256": hashlib.sha256(self.raw).hexdigest(),
            "file_identity": identity(self.path.stat()),
            "binding_identity": identity(self.binding_path.stat()),
            "directory_identity": identity(self.published.stat()),
        }
        self.owner = {"db": str(self.db), "state_dir": str(self.state), "admission": self.anchor}
        self.binding_path.write_bytes(encode(self.anchor))
        self.owner_path.write_bytes(encode(self.owner))
        self.owner_path.chmod(0o600)

    def load(self, **kwargs):
        return load(self.db, self.path, **kwargs)

    def env(self):
        return self.anchor["sha256"], encode(self.anchor)


class ContractChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="issue1527-r4-model-")
        self.addCleanup(self.temp.cleanup)

    def fixture(self, raw=None):
        return Fixture(self.temp.name, raw)

    def ready(self, f, **kwargs):
        result = f.load(**kwargs)
        self.addCleanup(result.lease.close)
        return result

    def refused(self, f, kind=None, **kwargs):
        with self.assertRaises(Refusal) as ctx:
            f.load(**kwargs)
        if kind is not None:
            self.assertEqual(ctx.exception.kind, kind)
        return ctx.exception

    def test_valid_anchor_without_env_counts_exact_payload_only(self):
        f = self.fixture()
        p = self.ready(f)
        self.assertEqual(p.account.wire, sum(x.stat().st_size for x in (f.path, f.binding_path, f.owner_path)))
        self.assertIsNotNone(p.account.started)
        before = p.account.wire
        p.consume().revalidate()
        self.assertEqual(p.account.wire, before)

    def test_valid_both_env_and_legacy_owner_0644(self):
        f = self.fixture()
        f.owner_path.chmod(0o644)
        self.ready(f, env=f.env()).consume()

    def test_no_env_missing_owner_is_not_bootstrap(self):
        f = self.fixture()
        f.owner_path.unlink()
        for _ in range(2):
            self.refused(f, "AnchorMissing")
            self.assertFalse(f.owner_path.exists())

    def test_empty_invalid_and_duplicate_owner(self):
        for raw in (b"", b"{}", b"{", b'{"db":1,"db":2}', b'{"secret-raw":1}'):
            f = self.fixture()
            f.owner_path.write_bytes(raw)
            error = self.refused(f, "AnchorInvalid")
            self.assertEqual(error.wire, len(raw))
            self.assertNotIn("secret-raw", str(error))

    def test_owner_exact_three_and_nonnull_admission(self):
        for edit in (lambda x: x.pop("admission"), lambda x: x.update(extra=1), lambda x: x.update(admission=None)):
            f = self.fixture()
            edit(f.owner)
            f.owner_path.write_bytes(encode(f.owner))
            self.refused(f, "AnchorInvalid")

    def test_binding_exact_six_duplicate_keys(self):
        for raw in (b"{}", b'{"db":1,"db":1}'):
            f = self.fixture()
            f.binding_path.write_bytes(raw)
            self.refused(f, "BindingInvalid")

    def test_invalid_limits_types_and_integer_domain(self):
        for value in (True, 1, 1.0, "", "0", "01", "-1", "+1", "1.0", " 1", "1e1", "9223372036854775808"):
            obj = dict.fromkeys(LIMIT_FIELDS, "1000000")
            obj["max_total_nodes"] = value
            self.refused(self.fixture(encode(obj)), "ResourcesInvalid")

    def test_limits_exact_eight_and_duplicate_keys(self):
        base = dict.fromkeys(LIMIT_FIELDS, "1000000")
        for raw in (b"", b"{}", encode({**base, "extra": "1"}), encode(base)[:-1] + b',"max_elapsed_ms":"1"}'):
            self.refused(self.fixture(raw), "ResourcesInvalid")

    def test_identity_wrong_types_and_uid(self):
        for bad in (True, -1, 1.0, "1", 2**64, os.geteuid() + 1):
            f = self.fixture()
            f.anchor["file_identity"]["uid"] = bad
            f.owner_path.write_bytes(encode(f.owner))
            self.refused(f, "AnchorInvalid")

    def test_identity_missing_and_extra_fields(self):
        for field in ("inode", "extra"):
            f = self.fixture()
            obj = f.anchor["directory_identity"]
            if field == "extra":
                obj[field] = 1
            else:
                obj.pop(field)
            f.owner_path.write_bytes(encode(f.owner))
            self.refused(f, "AnchorInvalid")

    def test_same_bytes_new_inode_rejected_at_load(self):
        for name in ("path", "binding_path"):
            f = self.fixture()
            path = getattr(f, name)
            old = path.with_suffix(".old")
            path.rename(old)
            path.write_bytes(old.read_bytes())
            path.chmod(0o600)
            self.refused(f, "Changed")

    def test_same_bytes_new_inode_rejected_after_preparation(self):
        for name in ("owner_path", "path", "binding_path", "db"):
            f = self.fixture()
            p = self.ready(f)
            path = getattr(f, name)
            old = path.with_suffix(".old")
            path.rename(old)
            path.write_bytes(old.read_bytes())
            path.chmod(0o600)
            with self.assertRaises(Refusal) as ctx:
                p.consume()
            self.assertEqual(ctx.exception.kind, "Changed")

    def test_in_place_mutation_detected_without_rereading(self):
        f = self.fixture()
        p = self.ready(f)
        before = p.account.wire
        f.path.write_bytes(b"changed")
        with self.assertRaises(Refusal) as ctx:
            p.consume()
        self.assertEqual(ctx.exception.wire, before)

    def test_symlink_fifo_hardlink_all_control_files(self):
        for name in ("owner_path", "path", "binding_path"):
            for kind in ("symlink", "fifo", "hardlink"):
                f = self.fixture()
                path = getattr(f, name)
                old = path.with_suffix(".old")
                path.rename(old)
                if kind == "symlink":
                    path.symlink_to(old)
                elif kind == "fifo":
                    os.mkfifo(path, 0o600)
                else:
                    os.link(old, path)
                self.refused(f, "Changed")

    def test_control_modes_and_published_mode(self):
        for name in ("owner_path", "path", "binding_path", "published"):
            f = self.fixture()
            getattr(f, name).chmod(0o777)
            self.refused(f, "Changed")

    def test_pending_with_final_without_final_and_dangling_symlink(self):
        for kind in ("both", "pending_only", "dangling"):
            f = self.fixture()
            pending = f.state / ".s-admission-pending"
            if kind == "pending_only":
                f.published.rename(pending)
            elif kind == "dangling":
                pending.symlink_to("absent")
            else:
                pending.mkdir()
            for _ in range(2):
                self.refused(f, "AnchorUnpublished")
                self.assertTrue(os.path.lexists(pending))

    def test_lost_published_never_resigns(self):
        f = self.fixture()
        raw_owner = f.owner_path.read_bytes()
        f.published.rename(f.state / "retained")
        for _ in range(2):
            self.refused(f, "AnchorLost")
        self.assertEqual(f.owner_path.read_bytes(), raw_owner)
        self.assertFalse(f.published.exists())

    def test_moved_state_and_parent_and_published_rejected(self):
        for name in ("state", "root", "published"):
            f = self.fixture()
            p = self.ready(f)
            path = getattr(f, name)
            path.rename(path.with_suffix(".old"))
            path.mkdir()
            with self.assertRaises(Refusal):
                p.consume()

    def test_env_half_nonutf8_empty_and_wrong_identity(self):
        f = self.fixture()
        for env in (("a" * 64, None), (None, b"{}"), ("", b""), ("a" * 64, b"\xff")):
            self.assertEqual(self.refused(f, "ControlMismatch", env=env).wire, 0)
        other = self.fixture()
        self.refused(f, "ControlMismatch", env=other.env())

    def test_actual_db_binds_owner_not_env(self):
        f, other = self.fixture(), self.fixture()
        f.owner_path.write_bytes(other.owner_path.read_bytes())
        self.refused(f, "Identity", env=other.env())

    def test_other_same_byte_cli_file_rejected(self):
        f = self.fixture()
        alias = f.root / "same-bytes"
        alias.write_bytes(f.raw)
        with self.assertRaises(Refusal) as ctx:
            load(f.db, alias)
        self.assertEqual(ctx.exception.kind, "Identity")

    def test_raw_sha_not_canonical_json_sha(self):
        f = self.fixture()
        f.path.write_bytes(json.dumps(parse(f.raw), indent=2).encode())
        self.refused(f, "Changed")

    def test_partial_read_failure_retains_count(self):
        account = Account()
        account.started = account.now()
        reads = iter((b"abc", OSError(errno.EIO, "不导出底层原文")))

        def reader(_fd, _length):
            value = next(reads)
            if isinstance(value, OSError):
                raise value
            return value

        with self.assertRaises(Refusal) as ctx:
            read_payload(-1, account, "E1", reader)
        self.assertEqual(ctx.exception.wire, 3)
        self.assertNotIn("底层原文", str(ctx.exception))
        self.assertFalse(ctx.exception.trusted)

    def test_65537th_byte_charged_and_no_more_read(self):
        f = self.fixture()
        f.owner_path.write_bytes(b" " * 70000)
        error = self.refused(f, "TooLarge")
        self.assertEqual(error.wire, 65537)

    def test_65536_bytes_exact_is_read_and_eof_does_not_add(self):
        f = self.fixture()
        raw = f.owner_path.read_bytes()
        f.owner_path.write_bytes(raw + b" " * (65536 - len(raw)))
        p = self.ready(f)
        self.assertEqual(p.account.wire, 65536 + len(f.raw) + f.binding_path.stat().st_size)

    def test_deadline_wins_dual_excess_and_never_renews(self):
        account = Account(now=lambda: 11_000_000)
        account.started = 1_000_000
        account.wire = 2
        account.limits = dict.fromkeys(LIMIT_FIELDS, 1)
        account.limits["max_elapsed_ms"] = 10
        for phase in ("E4", "E5", "E7", "E8"):
            with self.assertRaises(Refusal) as ctx:
                account.gate(phase)
            self.assertEqual(ctx.exception.kind, "Deadline")
            self.assertEqual(ctx.exception.started, 1_000_000)
            self.assertEqual(ctx.exception.wire, 2)
        account.now = lambda: 1_000_000
        with self.assertRaises(Refusal) as ctx:
            account.gate("E4")
        self.assertEqual(ctx.exception.kind, "Limit")

    def test_trusted_loader_retroactively_checks_elapsed_and_wire(self):
        obj = dict.fromkeys(LIMIT_FIELDS, "1")
        f = self.fixture(encode(obj))
        values = iter((0, 2_000_000))
        account = Account(now=lambda: next(values))
        self.assertEqual(self.refused(f, "Deadline", account=account).wire,
                         sum(p.stat().st_size for p in (f.path, f.owner_path, f.binding_path)))
        account = Account(now=lambda: 0)
        self.refused(f, "Limit", account=account)

    def test_one_consumption_and_fd_lifetime(self):
        f = self.fixture()
        p = self.ready(f)
        lease = p.consume()
        with self.assertRaises(Refusal) as ctx:
            p.consume()
        self.assertEqual(ctx.exception.kind, "Consumed")
        del p
        lease.revalidate()
        fds = list(lease.fds)
        lease.close()
        for fd in fds:
            with self.assertRaises(OSError) as error:
                os.fstat(fd)
            self.assertEqual(error.exception.errno, errno.EBADF)


class RecordedResult(unittest.TextTestResult):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.passed = []

    def addSuccess(self, test):
        self.passed.append(test.id())
        super().addSuccess(test)


if __name__ == "__main__":
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(ContractChecks)
    result = unittest.TextTestRunner(verbosity=2, resultclass=RecordedResult).run(suite)
    print(json.dumps({
        "范围": "独立合同模型与临时普通文件，不是Rust验收或producer发布资格",
        "执行数": result.testsRun, "通过": result.passed,
        "失败数": len(result.failures), "错误数": len(result.errors),
        "Rust": "NOT_RUN", "正式恢复": "NOT_RUN",
    }, ensure_ascii=False, indent=2))
    sys.exit(0 if result.wasSuccessful() else 1)
