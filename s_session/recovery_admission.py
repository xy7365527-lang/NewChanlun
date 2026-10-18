"""#1527：消费 R4 原持久锚；只续发布已锚定的 pending，不创建或重签。"""

import hashlib
import ctypes
import errno
import json
import math
import os
from pathlib import Path
import re
import stat
import time

CONTROL_MAX = 65536
PENDING = ".s-admission-pending"
PUBLISHED = "s-admission"
ENV_SHA = "S_CONTROL_ADMISSION_SHA256"
ENV_IDENTITY = "S_CONTROL_ADMISSION_IDENTITY"
LIMIT_FIELDS = frozenset({
    "max_total_wire_bytes", "max_total_node_bytes", "max_total_nodes",
    "max_total_records", "max_total_requests", "max_elapsed_ms",
    "max_staged_bytes", "max_store_bytes",
})
BINDING_FIELDS = frozenset({
    "db", "path", "sha256", "file_identity", "binding_identity", "directory_identity",
})
IDENTITY_FIELDS = frozenset({"device", "inode", "uid", "mode"})


class AdmissionError(ValueError):
    """只包含稳定分类、阶段和实读量；原字节/env 不进入错误文本。"""

    def __init__(self, code, stage, wire_bytes, first_read_at):
        self.code, self.stage = code, stage
        self.wire_bytes, self.first_read_at = wire_bytes, first_read_at
        self.elapsed_ms = (None if first_read_at is None else
                           max(0, (time.monotonic() - first_read_at) * 1000))
        super().__init__(f"R4 {code}；阶段={stage}；实读字节={wire_bytes}")


def _unique(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("重复键")
        value[key] = item
    return value


def _reject_number(_):
    raise ValueError("非整数")


def _object(raw):
    value = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique,
                       parse_float=_reject_number, parse_constant=_reject_number)
    if type(value) is not dict:
        raise ValueError("非对象")
    return value


def _identity(metadata):
    return {"device": metadata.st_dev, "inode": metadata.st_ino,
            "uid": metadata.st_uid, "mode": stat.S_IMODE(metadata.st_mode)}


def _snapshot(metadata):
    return (metadata.st_dev, metadata.st_ino, metadata.st_uid, metadata.st_mode,
            metadata.st_nlink, metadata.st_size, metadata.st_mtime_ns, metadata.st_ctime_ns)


def _directory(path):
    """逐段无跟随；中间路径也不能经 symlink 转到另一信任根。"""
    descriptor = os.open("/", os.O_RDONLY | os.O_DIRECTORY)
    try:
        for component in Path(path).parts[1:]:
            child = os.open(component, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                            dir_fd=descriptor)
            os.close(descriptor)
            descriptor = child
        return descriptor
    except BaseException:
        os.close(descriptor)
        raise


def _publish_noreplace(state_fd):
    """原子发布且禁止覆盖并发出现的 final；不采用先删后改名。"""
    libc = ctypes.CDLL(None, use_errno=True)
    if hasattr(libc, "renameatx_np"):
        rename, flag = libc.renameatx_np, 0x00000004  # Darwin RENAME_EXCL
    elif hasattr(libc, "renameat2"):
        rename, flag = libc.renameat2, 1  # Linux RENAME_NOREPLACE
    else:
        raise OSError(errno.ENOTSUP, "缺少不覆盖发布原语")
    rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
    rename.restype = ctypes.c_int
    if rename(state_fd, os.fsencode(PENDING), state_fd, os.fsencode(PUBLISHED), flag) != 0:
        raise OSError(ctypes.get_errno(), "原子发布失败")


class RecoveryAdmission:
    """控制器持有至 close 的原 FD/原字节租约；一份租约不更新预算或起钟。"""

    def __init__(self, db, state_dir, deadline, environment, *, owner_only=False):
        self.stage, self.wire_bytes, self.first_read_at = "E0", 0, None
        self.deadline, self.job_deadline = deadline, None
        self._fds, self._files = [], {}
        self._named = None
        self._limits = None
        self._published = False
        self._loaded = self._load_attempted = False
        self._preparation_error = None
        try:
            if not math.isfinite(deadline):
                self._fail("Deadline")
            self._gate()
            self._env = self._parse_env(environment)
            self.db = Path(db).resolve(strict=True)
            self.state = Path(state_dir)
            if not self.state.is_absolute() or self.state != self.state.resolve(strict=True):
                self._fail("Identity")
            self.parent_fd = self._keep(_directory(self.db.parent))
            self.parent_identity = _identity(os.fstat(self.parent_fd))
            self.db_fd = self._keep(os.open(self.db.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK,
                                           dir_fd=self.parent_fd))
            source = os.fstat(self.db_fd)
            if not stat.S_ISREG(source.st_mode) or source.st_nlink != 1 or source.st_uid != os.geteuid():
                self._fail("Identity")
            self.db_identity = _identity(source)
            self.owner_name = self.db.name + ".control-owner.json"
            self.stage = "E1"
            self.first_read_at = time.monotonic()
            self._owner = self._read("owner", self.parent_fd, self.owner_name, {0o600, 0o644})
            owner = _object(self._owner)
            if (set(owner) != {"db", "state_dir", "admission"}
                    and not (owner_only and set(owner) == {"db", "state_dir"})):
                self._fail("AnchorInvalid")
            if owner["db"] != str(self.db) or owner["state_dir"] != str(self.state):
                self._fail("Identity")
            self._binding = owner.get("admission")
            if self._binding is not None:
                self._validate_binding(self._binding)
            if not owner_only:
                self._prepare()
        except BaseException as exc:
            self.close()
            self._reraise(exc)

    def _prepare(self):
        """仅写入口消费完整锚；status/stop 不因预算文件损坏而丧失控制能力。"""
        if self._loaded:
            return
        if self._load_attempted:
            raise self._preparation_error
        self._load_attempted = True
        try:
            self._gate()
            if self._binding is None:
                self._fail("AnchorInvalid")
            self.stage = "E2"
            self.state_fd = self._keep(_directory(self.state))
            self.state_identity = _identity(os.fstat(self.state_fd))
            # state 自身不加 0700 限制；0700 仅约束预算容器。
            final, pending = self._exists(PUBLISHED), self._exists(PENDING)
            if final and pending:
                self._fail("AnchorConflict")
            if not final and not pending:
                self._fail("AnchorLost")
            self.container = PUBLISHED if final else PENDING
            self.directory_fd = self._keep(os.open(
                self.container, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=self.state_fd))
            if _identity(os.fstat(self.directory_fd)) != self._binding["directory_identity"]:
                self._fail("Changed")
            self.stage = "E3"
            binding_raw = self._read("binding", self.directory_fd, "binding.json", {0o600})
            binding = _object(binding_raw)
            self._validate_binding(binding)
            if binding != self._binding or _identity(os.fstat(self._files["binding"][0])) != binding["binding_identity"]:
                self._fail("BindingInvalid")
            self._raw_limits = self._read("limits", self.directory_fd, "limits.json", {0o600})
            if (_identity(os.fstat(self._files["limits"][0])) != binding["file_identity"]
                    or hashlib.sha256(self._raw_limits).hexdigest() != binding["sha256"]):
                self._fail("Changed")
            limits = _object(self._raw_limits)
            if set(limits) != LIMIT_FIELDS:
                self._fail("ResourcesInvalid")
            for value in limits.values():
                if (type(value) is not str or re.fullmatch(r"[1-9][0-9]{0,18}", value) is None
                        or int(value) > 2**63 - 1):
                    self._fail("ResourcesInvalid")
            self._limits = limits
            self.job_deadline = self.first_read_at + int(limits["max_elapsed_ms"]) / 1000
            self.stage = "E4"
            self._gate()
            self._crosscheck_env(self._env)
            self._revalidate()
            self._loaded = True
        except BaseException as exc:
            try:
                self._reraise(exc)
            except BaseException as error:
                self._preparation_error = error
                raise

    def _keep(self, descriptor):
        self._fds.append(descriptor)
        return descriptor

    def close(self):
        while self._fds:
            os.close(self._fds.pop())

    def _fail(self, code):
        raise AdmissionError(code, self.stage, self.wire_bytes, self.first_read_at) from None

    def _reraise(self, exc):
        if isinstance(exc, AdmissionError):
            raise exc
        if isinstance(exc, (OSError, ValueError, TypeError, RecursionError, OverflowError)):
            code = "AnchorMissing" if isinstance(exc, FileNotFoundError) and self.stage == "E1" else "Invalid"
            if isinstance(exc, OSError) and code == "Invalid":
                code = "IO"
            self._fail(code)
        raise exc

    def _gate(self):
        now = time.monotonic()
        if now >= self.deadline or (self.job_deadline is not None and now >= self.job_deadline):
            self._fail("Deadline")
        if self._limits is not None and self.wire_bytes > int(self._limits["max_total_wire_bytes"]):
            self._fail("WireLimit")

    def _read(self, key, parent, name, modes):
        self._gate()
        descriptor = self._keep(os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent))
        before = os.fstat(descriptor)
        if (not stat.S_ISREG(before.st_mode) or before.st_nlink != 1
                or before.st_uid != os.geteuid() or stat.S_IMODE(before.st_mode) not in modes):
            self._fail("Identity")
        raw = bytearray()
        while len(raw) <= CONTROL_MAX:
            self._gate()
            chunk = os.read(descriptor, CONTROL_MAX + 1 - len(raw))
            self.wire_bytes += len(chunk)  # 成功返回就计费；下一次 read/时限失败也不抹去。
            raw.extend(chunk)
            self._gate()
            if not chunk or len(raw) > CONTROL_MAX:
                break
        if len(raw) > CONTROL_MAX:
            self._fail("ControlTooLarge")
        if _snapshot(before) != _snapshot(os.fstat(descriptor)):
            self._fail("Changed")
        self._files[key] = (descriptor, _snapshot(before))
        return bytes(raw)

    def _binding_shape(self, binding):
        if type(binding) is not dict or set(binding) != BINDING_FIELDS:
            self._fail("BindingInvalid")
        if (any(type(binding[key]) is not str or not Path(binding[key]).is_absolute() for key in ("db", "path"))
                or type(binding["sha256"]) is not str
                or re.fullmatch(r"[0-9a-f]{64}", binding["sha256"]) is None):
            self._fail("BindingInvalid")
        for field, mode in (("file_identity", 0o600), ("binding_identity", 0o600), ("directory_identity", 0o700)):
            identity = binding[field]
            if (type(identity) is not dict or set(identity) != IDENTITY_FIELDS
                    or any(type(value) is not int or not 0 <= value <= 2**64 - 1 for value in identity.values())
                    or identity["uid"] != os.geteuid() or identity["mode"] != mode):
                self._fail("BindingInvalid")

    def _validate_binding(self, binding):
        self._binding_shape(binding)
        if binding["db"] != str(self.db) or binding["path"] != str(self.state / PUBLISHED / "limits.json"):
            self._fail("BindingInvalid")

    def _parse_env(self, environment):
        sha, identity = environment.get(ENV_SHA), environment.get(ENV_IDENTITY)
        if (sha is None) != (identity is None):
            self._fail("ControlMismatch")
        if sha is None:
            return None
        if (type(sha) is not str or re.fullmatch(r"[0-9a-f]{64}", sha) is None
                or type(identity) is not str or len(identity) > CONTROL_MAX):
            self._fail("ControlMismatch")
        raw = identity.encode("utf-8")
        if len(raw) > CONTROL_MAX:
            self._fail("ControlMismatch")
        binding = _object(raw)
        self._binding_shape(binding)
        return sha, binding

    def _crosscheck_env(self, pair):
        if pair is not None:
            self._validate_binding(pair[1])
            if pair != (self._binding["sha256"], self._binding):
                self._fail("ControlMismatch")

    def _exists(self, name):
        try:
            os.stat(name, dir_fd=self.state_fd, follow_symlinks=False)
            return True
        except FileNotFoundError:
            return False

    def _same_file(self, key, parent, name):
        original, snapshot = self._files[key]
        if _snapshot(os.fstat(original)) != snapshot:
            self._fail("Changed")
        descriptor = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
        try:
            if _snapshot(os.fstat(descriptor)) != snapshot:
                self._fail("Changed")
        finally:
            os.close(descriptor)

    def _revalidate(self):
        self._gate()
        # 复查只取 metadata，绝不采纳替身或重新解析另一份 authority。
        parent = _directory(self.db.parent)
        try:
            if _identity(os.fstat(parent)) != self.parent_identity:
                self._fail("Changed")
            self._same_file("owner", self.parent_fd, self.owner_name)
            self._same_file("owner", parent, self.owner_name)
            source = os.stat(self.db.name, dir_fd=parent, follow_symlinks=False)
            held = os.fstat(self.db_fd)
            if (not stat.S_ISREG(source.st_mode) or source.st_nlink != 1 or held.st_nlink != 1
                    or _identity(source) != self.db_identity or _identity(held) != self.db_identity):
                self._fail("Changed")
        finally:
            os.close(parent)
        state_fd = _directory(self.state)
        try:
            if _identity(os.fstat(state_fd)) != self.state_identity:
                self._fail("Changed")
            current = os.open(self.container, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=state_fd)
            try:
                if (_identity(os.fstat(current)) != self._binding["directory_identity"]
                        or _identity(os.fstat(self.directory_fd)) != self._binding["directory_identity"]):
                    self._fail("Changed")
                for key in ("binding", "limits"):
                    self._same_file(key, self.directory_fd, key + ".json")
                    self._same_file(key, current, key + ".json")
            finally:
                os.close(current)
        finally:
            os.close(state_fd)
        other = PENDING if self.container == PUBLISHED else PUBLISHED
        if self._exists(other):
            self._fail("AnchorConflict")
        if self._named is not None:
            name, held_parent = self._named
            parent = _directory(name.parent)
            try:
                if _identity(os.fstat(parent)) != _identity(os.fstat(held_parent)):
                    self._fail("Changed")
                self._same_file("configured", held_parent, name.name)
                self._same_file("configured", parent, name.name)
            finally:
                os.close(parent)
        self._gate()

    def revalidate(self):
        try:
            self._revalidate()
        except BaseException as exc:
            self._reraise(exc)

    def _sync(self, descriptor):
        self._gate()
        os.fsync(descriptor)
        self._gate()

    def crosscheck_limits(self, named):
        """兼容旧具名配置的交叉核；不能据此创建锚或改签预算。"""
        try:
            self._prepare()
            if type(named) is not str or not named or not Path(named).is_absolute():
                self._fail("ResourcesInvalid")
            self._revalidate()
            if named == self._binding["path"]:
                # 已发布位置是当前锚指定的唯一目标；pending 续发仍绑定所持原 FD。
                return
            path = Path(named)
            if self._named is not None:
                if self._named[0] != path:
                    self._fail("Changed")
                return
            parent = self._keep(_directory(path.parent))
            body = self._read("configured", parent, path.name, {0o600, 0o644})
            if body != self._raw_limits:
                self._fail("ControlMismatch")
            self._named = path, parent
            self._revalidate()
        except BaseException as exc:
            self._reraise(exc)

    def publish(self):
        """锚已存在：补足原对象持久化，按原 inode 续发布；失败不清理现场。"""
        try:
            self._prepare()
            self._revalidate()
            # 即使上次 rename 已发生，重试也不能略过持久化。
            for key in ("limits", "binding"):
                self._sync(self._files[key][0])
            self._sync(self.directory_fd)
            self._sync(self.state_fd)
            self._sync(self._files["owner"][0])
            self._sync(self.parent_fd)
            self._revalidate()
            if self.container == PENDING:
                # 本控制器已持有 state 与 DB 控制锁；不覆盖任何已发现的 final。
                if self._exists(PUBLISHED):
                    self._fail("AnchorConflict")
                _publish_noreplace(self.state_fd)
                self.container = PUBLISHED
            self._sync(self.parent_fd)
            self._sync(self.state_fd)
            self._revalidate()
            self._published = True
        except BaseException as exc:
            self._reraise(exc)

    @property
    def limits_path(self):
        if not self._published:
            self._fail("AnchorUnpublished")
        return self._binding["path"]

    def child_environment(self, environment):
        try:
            self._gate()
            self._crosscheck_env(self._parse_env(environment))
            result = dict(environment)
            result[ENV_SHA] = self._binding["sha256"]
            result[ENV_IDENTITY] = json.dumps(self._binding, sort_keys=True, separators=(",", ":"))
            return result
        except BaseException as exc:
            self._reraise(exc)
