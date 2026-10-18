"""#1527 限定反例：普通文件探针及 Python 状态模型，不执行 Rust/SQLite。"""

import copy
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[3]
BASE = "f7d34f47cba8a0ea0ceb55499206520f058da61f"
REVIEW = "3dde745d3909f882beac67de106a42d0f097aa65"
PROVIDER = "rust/src/bin/s_session_v3/admission_provider.rs"
ADMISSION = "rust/src/bin/s_session_v3/admission.rs"


def git_source(rev, path, review=False):
    repo = ROOT.parent / "formal-i1-review-worktree" if review else ROOT
    return subprocess.check_output(["git", "-C", str(repo), "show", f"{rev}:{path}"])


def function(source, signature):
    start = source.index(signature)
    opening = source.index("{", start)
    depth = 0
    for i in range(opening, len(source)):
        depth += (source[i] == "{") - (source[i] == "}")
        if depth == 0:
            return source[start:i + 1]
    raise AssertionError(signature)


def suffixes(source):
    body = function(source, "fn members(")
    return re.findall(r'"([^"\n]+)"', body.split("&& ![", 1)[1].split("]", 1)[0])


def family_allowed(name, allowed):
    basename = "main.db"
    if name == basename:
        return True
    if name.startswith(basename) and name[len(basename):len(basename) + 1] in (".", "-"):
        return name[len(basename):] in allowed
    return True


def stamp(st):
    return (st.st_dev, st.st_ino, st.st_mode, st.st_uid, st.st_nlink,
            st.st_size, st.st_blocks, st.st_mtime_ns, st.st_ctime_ns)


def ordinary_files(old, current):
    """suffix 从实际源码提取；FD/flock 由 Python 执行，非 Rust provider。"""
    results = []
    for held in (True, False):
        with tempfile.TemporaryDirectory(prefix="issue1527-h01-") as tmp:
            root = Path(tmp)
            control = root / "main.db.control.lock"
            db = root / "main.db"
            db.write_bytes(b"ordinary bytes; not SQLite")
            controller = os.open(control, os.O_RDWR | os.O_CREAT | os.O_EXCL, 0o600)
            original_fd = None
            probe = None
            try:
                assert os.write(controller, b"R" * 4096) == 4096
                os.fsync(controller)
                fcntl.flock(controller, fcntl.LOCK_EX | fcntl.LOCK_NB)
                original = stamp(os.fstat(controller))
                if not held:
                    os.close(controller)
                    controller = None
                assert not family_allowed(control.name, old)
                admitted = family_allowed(control.name, current)
                # 模拟候选通用只读 pin：不 create/truncate/unlink/flock 此 FD。
                original_fd = os.open(control, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
                assert stamp(os.fstat(original_fd)) == original == stamp(control.lstat())
                physical = os.fstat(original_fd).st_blocks * 512
                assert physical > 0
                without = sum(p.lstat().st_blocks * 512 for p in (root, db))
                total = without + physical
                assert total > without
                assert not family_allowed("main.db.unknown", current)
                assert family_allowed("main.db.writer.lock", current)
                assert family_allowed("main.db.maintenance.lock", current)
                probe = os.open(control, os.O_RDWR)
                busy = False
                try:
                    fcntl.flock(probe, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    busy = True
                assert busy == held
                if held:
                    os.close(controller)
                    controller = None
                    fcntl.flock(probe, fcntl.LOCK_EX | fcntl.LOCK_NB)
                assert control.read_bytes() == b"R" * 4096
                assert stamp(control.lstat()) == original
                replacement = root / "replacement"
                replacement.write_bytes(b"R" * 4096)
                replacement.replace(control)  # 仅测试目录内的负控。
                assert os.fstat(original_fd).st_ino == original[1]
                assert control.stat().st_ino != original[1]
                results.append({"controller_held": held, "baseline_admitted": False,
                                "candidate_admitted_by_suffix": admitted,
                                "physical_bytes": physical, "model_store_bytes": total,
                                "unknown_rejected": True, "identity_replacement_detected": True,
                                "controller_lock_retained_until_close": True})
            finally:
                for fd in (controller, original_fd, probe):
                    if fd is not None:
                        os.close(fd)
    return results


class JobModel:
    """只建模与失败有关的原 Job 语义；不表示 Rust 运行结果。"""

    def __init__(self, cause):
        self.wire = 17
        self.pending = 33
        self.t0 = 123456
        self.stopped = "原超限首错" if cause == "stopped" else None
        self.expired = cause == "deadline"
        if cause != "reservation":
            self.pending = 0
        self.reservations = int(self.pending > 0)

    def gate(self):
        return self.stopped or ("原期限到期" if self.expired else None)

    def finish(self):
        if self.stopped:
            return self.stopped
        if self.reservations:
            return "原未结预留"
        if self.expired:
            self.stopped = "原期限到期"
            return self.stopped
        return None


class ScopeModel:
    def __init__(self, job, take_early, latch):
        self.job = job
        self.terminal = job
        self.take_early = take_early
        self.latch = latch
        self.first = None

    def finish(self):
        if self.first:
            return self.first
        job = self.job
        if self.take_early:
            self.job = None
        error = job.finish() if job else None
        if error:
            if self.latch:
                self.first = error
            return error
        self.job = None
        self.terminal = None
        return None

    def require(self):
        return self.first or (self.job.gate() if self.job else None)


def policies(source):
    finish = function(source, "pub(crate) fn finish_startup(")
    gate = function(source, "fn startup_gate(")
    return {"take_early": finish.index("state.take()") < finish.index("finish_output()"),
            "latch": "startup_failure" in finish and "startup_failure" in gate}


def state_model(source):
    policy = policies(source)
    rows = []
    for cause in ("stopped", "deadline", "reservation", "success"):
        original = JobModel(cause)
        scope = ScopeModel(original, **policy)
        before = copy.deepcopy(original.__dict__)
        first = scope.finish()
        second = scope.finish()
        require = scope.require()
        expected = cause != "success"
        if expected:
            preserved = (scope.job is original and original.wire == before["wire"]
                         and original.pending == before["pending"]
                         and original.t0 == before["t0"])
            terminal_retained = scope.terminal is original
            after = copy.deepcopy(original.__dict__)
            # 清理不重开资格；再造另一 Job 首错也不能覆盖 scope 首次收尾错。
            if cause == "reservation":
                original.pending = original.reservations = 0
            retry_after_cleanup = scope.finish()
            original.stopped = original.stopped or "后续另一错误"
            first_preserved = scope.require() == first
            fixed = bool(first and second == first and require == first and preserved
                         and retry_after_cleanup == first and first_preserved and terminal_retained)
        else:
            preserved = terminal_retained = None
            after = copy.deepcopy(original.__dict__)
            retry_after_cleanup = None
            first_preserved = None
            fixed = first is None and second is None and require is None and scope.job is None
        rows.append({"cause": cause, "first": first, "repeat_finish": second,
                     "require": require, "same_object_and_quantities_retained": preserved,
                     "terminal_original_retained": terminal_retained,
                     "after_cleanup_finish": retry_after_cleanup,
                     "first_error_not_replaced": first_preserved, "before": before,
                     "after_failed_finish": after, "contract_satisfied_in_model": fixed})
    return {"policy_from_source": policy, "cases": rows}


def run():
    baseline_provider = git_source(BASE, PROVIDER).decode()
    baseline_admission = git_source(BASE, ADMISSION).decode()
    reviewed_admission = git_source(REVIEW, ADMISSION, review=True).decode()
    candidate_provider = (ROOT / PROVIDER).read_text()
    candidate_admission = (ROOT / ADMISSION).read_text()
    return {
        "scope": "Python 模型/普通文件，非 Rust/SQLite 验收",
        "candidate_sha256": {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest()
                             for p in (PROVIDER, ADMISSION)},
        "h01": ordinary_files(suffixes(baseline_provider), suffixes(candidate_provider)),
        "m02_reviewed": state_model(reviewed_admission),
        "m02_fixed_baseline": state_model(baseline_admission),
        "m02_candidate": state_model(candidate_admission),
    }


if __name__ == "__main__":
    print(json.dumps(run(), ensure_ascii=False, indent=2))
