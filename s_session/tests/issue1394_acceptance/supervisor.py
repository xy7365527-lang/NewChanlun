"""外部监督的离线控制核：固定身份、持续采样、失联/取消与逐 owner 回收。

Probe 提供实际进程树/RSS/磁盘/日志观察；未知读数即停止。这里不默认提供 OS
硬界，不用聚合 toy 读数冒充产品隔离。操作系统实现及 Unix 服务仍待准入。
"""
from dataclasses import dataclass, asdict
from collections import deque
import math
import signal
import time
from driver import require, sha, bounded_hash


@dataclass(frozen=True)
class Identity:
    pid: int
    start: str
    group: int
    parent: int


@dataclass(frozen=True)
class Limits:
    work_deadline: float
    cleanup_deadline: float
    seal_deadline: float
    sample_interval: float
    heartbeat_timeout: float
    term_grace: float
    kill_wait: float
    max_processes: int
    max_rss_bytes: int
    max_disk_bytes: int
    max_output_bytes: int

    def validate(self):
        require(all(type(v) in (float, int) and math.isfinite(v) and v > 0
                    for v in asdict(self).values()), "监督所有资源必须显式有限正值，null 不可用")
        require(all(type(getattr(self, k)) is int for k in
                    ("max_processes", "max_rss_bytes", "max_disk_bytes", "max_output_bytes")),
                "监督资源帽须整数")
        require(self.work_deadline < self.cleanup_deadline < self.seal_deadline
                and self.term_grace + self.kill_wait <= self.cleanup_deadline - self.work_deadline,
                "监督回收窗不足")


class SupervisorCore:
    """Probe.identity/alive/descendants/sample/signal/wait 均须可失败，不能猜身份。"""
    def __init__(self, *, contract_sha, script, script_sha, worker, worker_script,
                 worker_script_sha, limits, stages, probe, clock=time.monotonic):
        limits.validate()
        require(bounded_hash(script, limits.work_deadline) == script_sha
                and bounded_hash(worker_script, limits.work_deadline) == worker_script_sha,
                "supervisor/worker 脚本摘要不符")
        self.contract_sha, self.limits, self.probe, self.clock = contract_sha, limits, probe, clock
        self.scripts = ((script, script_sha), (worker_script, worker_script_sha))
        self.worker, self.stages = worker, stages
        require(type(worker) is Identity and probe.identity(worker.pid) == worker, "worker 身份未核")
        require(bool(stages), "缺冻结阶段和资源对象")
        self.owners, self.nonces, self.events = {}, set(), []
        self.last_heartbeat, self.last_sample = clock(), clock()
        self.stopped, self.cleanup_result = False, None
        self.failures = []

    def live_identity(self, identity):
        require(self.probe.identity(identity.pid) == identity, "pid/start/group/parent 漂移")

    def ack(self, binding):
        try:
            self._register_ack(binding)
            return self._sample_and_ack((binding,))[0]
        except BaseException:
            self.failures.append("ACK_FAILURE")
            self.stop("ACK_FAILURE")
            raise

    def _register_ack(self, binding):
        """只验证并登记身份；排队批次结束并采样前不产生ACK。"""
        require(not self.stopped and self.clock() < self.limits.work_deadline, "监督已停止/过期")
        self.live_identity(self.worker)
        require(binding["contract_sha256"] == self.contract_sha
                and binding["worker_pid"] == self.worker.pid
                and binding["worker_start_identity"] == self.worker.start
                and binding["worker_process_group"] == self.worker.group, "ACK worker/合同错绑")
        require(binding["nonce"] and binding["nonce"] not in self.nonces, "ACK nonce 重放")
        stage = self.stages[binding["stage"]]
        require(binding["resource_caps"] == stage["resource_caps"]
                and binding["absolute_deadline_monotonic"] == stage["deadline"]
                and stage["deadline"] <= self.limits.work_deadline, "ACK 资源/同截止不符")
        row = binding["owner"]
        identity = self.probe.identity(row["pid"])
        require(identity.start == row["start_identity"] and identity.group == row["process_group"]
                and identity.parent == self.worker.pid and row["domain_id"] == stage["domain_id"]
                and row["writer_epoch"] == stage["writer_epoch"], "ACK owner 不是本 worker 子进程")
        self.register(identity, parent=self.worker.pid)
        self.nonces.add(binding["nonce"])

    def _sample_and_ack(self, bindings, *, deadline=None):
        # 直接ack与monitor排队路径共用强制采样，不能发出尚未经采样的ACK。
        self.tick(force=True)
        require(not self.stopped and self.clock() < self.limits.work_deadline
                and self.clock() < self.last_heartbeat + self.limits.heartbeat_timeout
                and (deadline is None or self.clock() < deadline),
                "ACK 采样后已停止/过期")
        for binding in bindings:
            require(self.clock() < self.stages[binding["stage"]]["deadline"], "ACK 采样后已停止/过期")
        replies = [{"kind": "ACK", "binding": binding} for binding in bindings]
        self.events.extend(replies)
        return replies

    def register(self, identity, *, parent):
        require(not self.stopped and type(identity) is Identity and identity.pid != self.worker.pid,
                "非法 owner 注册")
        require(identity.parent == parent and (parent == self.worker.pid or parent in self.owners),
                "后代不属于登记父进程")
        self.live_identity(identity)
        if identity.pid in self.owners:
            require(self.owners[identity.pid] == identity, "pid 被新进程占用")
            return
        require(len(self.owners) < self.limits.max_processes, "owner 数量超过冻结帽")
        self.owners[identity.pid] = identity

    def heartbeat(self, worker):
        require(not self.stopped and worker == self.worker, "失联后不得恢复或换 worker")
        self.live_identity(worker)
        require(self.clock() - self.last_heartbeat < self.limits.heartbeat_timeout,
                "心跳已过原截止，不能补发续命")
        self.last_heartbeat = self.clock()

    def tick(self, *, force=False):
        if self.stopped:
            return
        try:
            now = self.clock()
            require(now < self.limits.work_deadline, "工作截止耗尽")
            require(now - self.last_heartbeat < self.limits.heartbeat_timeout, "worker 心跳失联")
            self.live_identity(self.worker)
            if not force and now - self.last_sample < self.limits.sample_interval:
                return
            for script, expected in self.scripts:
                require(bounded_hash(script, self.limits.work_deadline) == expected,
                        "监督/worker 脚本字节偏离冻结摘要")
            # 不能因后代逃离父 PID 就忽略已注册项；也不能自动收养陌生 PID。
            live = {pid for pid, identity in self.owners.items() if self.probe.alive(identity)}
            observed = set(self.probe.descendants(self.worker))
            require(observed <= set(self.owners), "存在未登记后代；监督不完整")
            for pid in live:
                self.live_identity(self.owners[pid])
            sample = self.probe.sample((self.worker, *(self.owners[p] for p in sorted(live))))
            require(set(sample) == {"rss_bytes", "disk_bytes", "output_bytes"}, "监督采样缺维度")
            for key, value in sample.items():
                require(type(value) is int and value >= 0 and value <= getattr(self.limits, "max_" + key),
                        "监督采样未知/超过冻结帽：" + key)
            require(self.clock() < self.limits.work_deadline, "采样耗尽同截止")
            self.last_sample = self.clock()
            self.events.append({"kind": "SAMPLE", "at": self.last_sample, **sample})
        except BaseException as error:
            self.failures.append(type(error).__name__ + ": " + str(error))
            self.stop(type(error).__name__ + ": " + str(error))
            raise

    def stop(self, reason):
        """取消、失联、异常、正常结束同入口；无新 ACK，无自动重启。"""
        if self.cleanup_result is not None:
            return self.cleanup_result
        self.stopped = True
        rows = []
        # 全owner共享一次TERM窗；先发信号再wait，某次阻塞不得跳过其他owner。
        term_until = min(self.limits.cleanup_deadline - self.limits.kill_wait,
                         self.clock() + self.limits.term_grace)
        for identity in reversed(tuple(self.owners.values())):
            row = {"identity": asdict(identity), "reaped": False, "errors": []}
            rows.append(row)
            try:
                if self.probe.alive(identity):
                    self.live_identity(identity)
                    self.probe.signal(identity, signal.SIGTERM)
            except BaseException as error:
                row["errors"].append(type(error).__name__ + ": " + str(error))
        for row in rows:
            identity = Identity(**row["identity"])
            # Probe.wait遇到已过的绝对deadline必须只做非阻塞wait/poll。
            # 即使先前wait越界，仍做具名身份核验、KILL及非阻塞reap；不重启宽限期。
            try:
                row["reaped"] = self.probe.wait(identity, term_until)
                if self.clock() >= self.limits.cleanup_deadline:
                    row["errors"].append("wait 后回收截止耗尽")
            except BaseException as error:
                row["errors"].append(type(error).__name__ + ": " + str(error))
            if not row["reaped"]:
                try:
                    if self.probe.alive(identity):
                        self.live_identity(identity)
                        self.probe.signal(identity, signal.SIGKILL)
                except BaseException as error:
                    row["errors"].append(type(error).__name__ + ": " + str(error))
                try:
                    row["reaped"] = self.probe.wait(identity, self.limits.cleanup_deadline)
                    if self.clock() >= self.limits.cleanup_deadline:
                        row["errors"].append("wait 后回收截止耗尽")
                except BaseException as error:
                    row["errors"].append(type(error).__name__ + ": " + str(error))
            if not row["reaped"]:
                row["errors"].append("未取得 wait/reap 证明")
        if self.clock() >= self.limits.cleanup_deadline:
            self.failures.append("汇总时回收截止耗尽")
        self.cleanup_result = {"reason": reason, "owners": rows, "failures": self.failures[:],
                               "status": "CONTROL_REAPED" if not self.failures and all(r["reaped"] and not r["errors"] for r in rows)
                               else "INCOMPLETE", "product_status": "NOT_RUN"}
        return self.cleanup_result


def monitor(core, receive, emit):
    """可放入独立进程的持续循环；receive(timeout) 须受同截止且返回 None 表示无消息。

    receive(0)必须非阻塞。每批只排空立即可读消息，最多持续原sample_interval；
    原工作/心跳截止不随本批心跳延长。空队列、固定采样边界或COMPLETE/CANCEL
    都结束本批并强制采样；结束标记后的登记不能追认此前的陌生PID。
    传输 EOF 必须抛异常，不能伪装为心跳。这里不把 stdio/toy 传输声称为产品
    Unix ACK 服务；真实 OS Probe 与服务原件仍是独立准入条件。
    """
    reason = "MONITOR_FAILED"
    try:
        while not core.stopped:
            receive_deadline = min(core.limits.work_deadline,
                                   core.last_heartbeat + core.limits.heartbeat_timeout)
            timeout = min(core.limits.sample_interval, receive_deadline - core.clock())
            require(timeout > 0, "监督接收截止已到")
            message = receive(timeout)
            if message is None:
                core.tick()
                continue
            terminal = None

            def drain(message):
                nonlocal terminal
                batch_until = min(receive_deadline, core.clock() + core.limits.sample_interval)
                pending_acks = []
                while message is not None:
                    require(core.clock() < receive_deadline, "接收超过原工作/心跳截止")
                    kind = message["kind"]
                    if kind == "ACK_REQUEST":
                        core._register_ack(message["binding"])
                        pending_acks.append(message["binding"])
                    elif kind == "HEARTBEAT":
                        core.heartbeat(Identity(**message["worker"]))
                    elif kind == "REGISTER_DESCENDANT":
                        require(Identity(**message["worker"]) == core.worker
                                and message["contract_sha256"] == core.contract_sha,
                                "后代登记 worker/合同错绑")
                        core.live_identity(core.worker)
                        require(message["parent"] in core.owners, "后代登记缺已握手父owner")
                        require(core.clock() < core.limits.work_deadline, "后代登记过期")
                        core.register(Identity(**message["identity"]), parent=message["parent"])
                    elif kind in {"CANCEL", "COMPLETE"}:
                        require(Identity(**message["worker"]) == core.worker, "取消者身份不同")
                        terminal = kind
                    else:
                        require(False, "未知监督命令")
                    if terminal is not None or core.clock() >= batch_until:
                        break
                    message = receive(0)
                require(core.clock() < receive_deadline, "接收超过原工作/心跳截止")
                return core._sample_and_ack(pending_acks, deadline=receive_deadline)

            replies = deque(drain(message))

            def send_boundary():
                require(not core.stopped and core.clock() < receive_deadline,
                        "发送超过原工作/心跳截止或监督已停止")
                if terminal is None and core.clock() - core.last_sample >= core.limits.sample_interval:
                    # 发送期间也可能排入合法登记；沿用有限排队规则再采样。
                    # 始终保留本轮最初的receive_deadline，心跳不能替发送续期。
                    replies.extend(drain(receive(0)))
                else:
                    core.tick()
                require(not core.stopped and core.clock() < receive_deadline
                        and core.clock() < core.last_heartbeat + core.limits.heartbeat_timeout,
                        "发送采样后超过原工作/心跳截止或监督已停止")

            while replies:
                send_boundary()
                reply = replies.popleft()
                require(core.clock() < core.stages[reply["binding"]["stage"]]["deadline"],
                        "ACK 发送前阶段已过期")
                emit(reply)
                # 最后一条ACK也要核验；不能由COMPLETE/CANCEL跳过返回后的到期采样。
                send_boundary()
            if terminal is not None:
                reason = terminal
                break
    except BaseException as error:
        reason = type(error).__name__ + ": " + str(error)
        core.failures.append(reason)
        raise
    finally:
        emit(core.stop(reason))
