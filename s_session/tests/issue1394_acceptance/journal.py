"""同步落盘的控制日记；消费只追加且不退款，不自动恢复业务。

同一冻结 inode 上 flock 串行化多个进程。截断/未知尾部失败封闭；不宣称抵御
有权限重写整个历史及其根指针的攻击者，也不提供产品执行授权。
落盘的candidate不是接受凭证。只有本进程解锁返回并经截止检查后可接受；
重开时所有candidate均隔离，保留原件且禁止自动重跑。同步系统调用不可中断。
唯有持锁的candidate尾段在写入失败/超时时撤去，消费仍保持pending。
"""
from contextlib import contextmanager
from dataclasses import asdict
import fcntl
import os
from pathlib import Path
import stat
import time
from driver import canonical, sha, strict, require, remaining


class Journal:
    def __init__(self, path, identity, header, max_bytes, max_records):
        require(type(max_bytes) is int and max_bytes > 0 and type(max_records) is int
                and max_records > 0, "日记帽必须显式正整数")
        self.path, self.identity, self.header = Path(path), tuple(identity), header
        require(header.get("max_bytes") == max_bytes and header.get("max_records") == max_records,
                "重开日记不得改变冻结帽")
        self.max_bytes, self.max_records = max_bytes, max_records
        self._accepted, self._offered = set(), None
        require(header.get("revision") == "issue1394-control-journal/2", "旧日记须隔离，不自动迁移")
        self.fd = os.open(self.path, os.O_RDWR | os.O_APPEND | os.O_NOFOLLOW)
        try:
            self.verify()
            with self.locked(header["seal_deadline"]):
                self.read()
        except BaseException:
            self.close()
            raise

    @classmethod
    def create(cls, path, plan, *, max_bytes, max_records):
        require(plan.execution_authorized is False and
                plan.work_deadline < plan.cleanup_deadline < plan.seal_deadline, "日记不授予产品执行/续期")
        header = {"revision": "issue1394-control-journal/2", "run": plan.run_id,
                  "contract": plan.contract_sha, "seed": plan.seed_sha,
                  "plan": sha(canonical(asdict(plan))), "steps": [s.sha() for s in plan.steps],
                  "work_deadline": plan.work_deadline, "cleanup_deadline": plan.cleanup_deadline,
                  "seal_deadline": plan.seal_deadline, "execution_authorized": False,
                  "max_bytes": max_bytes, "max_records": max_records}
        require(type(max_bytes) is int and max_bytes > 0 and type(max_records) is int
                and max_records > 0, "日记帽必须冻结")
        raw = canonical({"header": header}) + b"\n"
        require(len(raw) <= max_bytes, "日记头超过冻结帽")
        remaining(plan.work_deadline)
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        try:
            info = os.fstat(fd)
            with os.fdopen(fd, "wb", closefd=False) as out:
                out.write(raw)
                out.flush()
                os.fsync(fd)
        finally:
            os.close(fd)
        return cls(path, (info.st_dev, info.st_ino), header, max_bytes, max_records)

    def verify(self):
        observed, held = self.path.lstat(), os.fstat(self.fd)
        require(stat.S_ISREG(observed.st_mode) and observed.st_nlink == 1
                and (observed.st_dev, observed.st_ino) == self.identity == (held.st_dev, held.st_ino),
                "日记路径身份改变")

    @contextmanager
    def locked(self, deadline, *, check_exit=True):
        while True:
            remaining(deadline)
            self.verify()
            try:
                fcntl.flock(self.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                time.sleep(min(.005, remaining(deadline)))
        try:
            self.verify()
            yield
            if check_exit:
                self.verify()
                remaining(deadline)
        finally:
            fcntl.flock(self.fd, fcntl.LOCK_UN)

    def read(self):
        self.verify()
        os.lseek(self.fd, 0, os.SEEK_SET)
        raw = bytearray()
        while True:
            remaining(self.header["seal_deadline"])
            part = os.read(self.fd, min(65536, self.max_bytes + 1 - len(raw)))
            raw.extend(part)
            require(len(raw) <= self.max_bytes, "日记字节帽耗尽")
            if not part:
                break
        require(raw.endswith(b"\n"), "日记缺完整末行；不修复不重跑")
        lines = bytes(raw).splitlines()
        require(len(lines) <= self.max_records + 1 and strict(lines[0]) == {"header": self.header},
                "日记头/数量不符")
        previous, entries, next_step, pending = sha(lines[0]), [], 0, None
        quarantine = None
        for line in lines[1:]:
            row = strict(line)
            require(set(row) == {"previous", "event"} and row["previous"] == previous, "日记链损坏")
            event = row["event"]
            if event["kind"] == "consume":
                require(pending is None and next_step < len(self.header["steps"])
                        and set(event) == {"kind", "step", "predecessor"}
                        and event["step"] == self.header["steps"][next_step]
                        and event["predecessor"] == (entries[-1]["sha"] if entries else self.header["seed"]),
                        "日记消费重叠/错序/前驱不符")
                pending = event["step"]
            elif event["kind"] == "candidate":
                require(pending is not None and set(event) == {"kind", "step", "sha"}
                        and event["step"] == pending, "回执没有消费")
                require(isinstance(event["sha"], str) and len(event["sha"]) == 64, "回执摘要无效")
                if quarantine is None and (pending, event["sha"]) not in self._accepted:
                    quarantine = (next_step, pending)
                pending, next_step = None, next_step + 1
            else:
                require(False, "未知日记事件")
            entries.append(event)
            previous = sha(line)
        if quarantine is not None:
            next_step, pending = quarantine
        return entries, previous, len(raw), next_step, pending

    def _append(self, event, deadline):
        entries, previous, size, _, _ = self.read()
        wire = canonical({"previous": previous, "event": event}) + b"\n"
        require(len(entries) < self.max_records and size + len(wire) <= self.max_bytes, "日记帽耗尽")
        sent = 0
        try:
            while sent < len(wire):
                remaining(deadline)
                n = os.write(self.fd, wire[sent:])
                require(n > 0, "日记写入中断")
                sent += n
            os.fsync(self.fd)
            self.verify()
            remaining(deadline)
        except BaseException:
            if event["kind"] == "candidate":
                # 仍持排他锁：撤去本次未获准成功的尾段，保留已消费 pending。
                # 不改旧记录/不退款；失败 fsync 或迟到写不能在重开后变成成功。
                self.verify()
                os.ftruncate(self.fd, size)
                os.fsync(self.fd)
            raise

    def consume(self, step, predecessor_sha):
        with self.locked(min(step.deadline, self.header["work_deadline"])):
            entries, _, _, next_step, pending = self.read()
            require(pending is None, "已有失败/未封口消费，禁止重试")
            require(next_step < len(self.header["steps"])
                    and step.sha() == self.header["steps"][next_step], "阶段偏离冻结日记")
            previous = entries[-1]["sha"] if entries else self.header["seed"]
            require(predecessor_sha == previous, "前驱原回执不匹配")
            self._append({"kind": "consume", "step": step.sha(), "predecessor": previous},
                         min(step.deadline, self.header["work_deadline"]))

    def finish(self, step, receipt):
        # 解锁之后不再截断。迟到/异常保留candidate，重开同样是隔离状态。
        self._offered = None
        with self.locked(min(step.deadline, self.header["work_deadline"]), check_exit=False):
            entries, _, _, index, pending = self.read()
            require(pending == step.sha() and receipt.step_sha == pending
                    and receipt.sequence == index and receipt.stage == step.stage
                    and receipt.scenario == step.scenario
                    and receipt.predecessor_sha == entries[-1]["predecessor"], "完成回执错绑")
            offered = (step.sha(), receipt.sha())
            self._append({"kind": "candidate", "step": offered[0], "sha": offered[1]},
                         min(step.deadline, self.header["work_deadline"]))
        remaining(min(step.deadline, self.header["work_deadline"]))
        self._offered = offered

    def accept(self, step, receipt):
        """仅当前调用者接受；不增加I/O、资源或重开恢复权限。"""
        offered = (step.sha(), receipt.sha())
        require(self._offered == offered, "未完成解锁的candidate不能接受")
        remaining(min(step.deadline, self.header["work_deadline"]))
        self._accepted.add(offered)
        self._offered = None

    def close(self):
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()
