"""#1394：仅静态/假适配器调度；产品执行门保持关闭。"""
from __future__ import annotations

from dataclasses import asdict, dataclass
from hashlib import sha256
import json
import math
from pathlib import Path
import time
from typing import Callable, Protocol
from abc import ABC, abstractmethod


class Rejected(ValueError):
    pass


def require(ok: bool, message: str) -> None:
    if not ok:
        raise Rejected(message)


def digest(value: object) -> str:
    return sha256(json.dumps(value, sort_keys=True, separators=(",", ":"),
                             ensure_ascii=False, allow_nan=False).encode()).hexdigest()


def sha_ok(value: str) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value)


@dataclass(frozen=True)
class Scenario:
    name: str
    family: str
    repeat: str
    operation: str = ""
    domain: str = ""
    seam: str = ""


@dataclass(frozen=True)
class Requirements:
    preparation_sha: str
    scenarios: tuple[Scenario, ...]
    negative_domains: tuple[str, ...]
    comparisons: tuple[str, ...]


def load_requirements(path: Path, expected_sha: str) -> Requirements:
    """只读准备合同；从其原量词导出故障域，不能自己缩成另一组。"""
    with path.open("rb") as stream:
        raw = stream.read(131073)
    require(len(raw) <= 131072, "静态准备合同读取上界")
    require(sha256(raw).hexdigest() == expected_sha, "准备合同摘要不符")
    obj = json.loads(raw)
    require(obj["schema_revision"] == "issue1394-real-acceptance-preparation/2", "准备合同版本")
    require(obj["execution_authorized"] is False, "此模块只接静态准备合同")
    verify = obj["verification"]
    scenarios = [Scenario(f"normal/{run}", "normal", run)
                 for run in verify["fresh_runs"]]
    for row in verify["recovery_matrix"]:
        for seam in row["seams"]:
            for run in row["repeat_each"]:
                name = f"fault/{row['op']}/{seam}/{run}"
                scenarios.append(Scenario(name, "fault", run, row["op"], row["domain"], seam))
    # 逐项照原合同保留名称；不猜测额外域的 A/B 重复次数。
    for name in verify["extra_recovery_domains"]:
        scenarios.append(Scenario(f"extra/{name}", "extra", "", domain=name))
    require("cap80_joint_feasible_testonly_domain" in
            [p["phase"] for p in obj["stage_contracts"]], "原合同 cap80 阶段缺失")
    scenarios.append(Scenario("cap80_joint_feasible_testonly_domain", "cap80", ""))
    require(len({s.name for s in scenarios}) == len(scenarios), "原合同 scenario 重复")
    return Requirements(expected_sha, tuple(scenarios),
                        tuple(verify["normal_negative_domains"]), tuple(verify["compare"]))


BASE_STEPS = ("writer", "writer_reaped", "custody", "financial_ready",
              "economic_open", "same_domain_voice", "business",
              "query", "history", "originals", "oracle")
FAULT_STEPS = ("marker", "kill_reaped", "fault_custody", "recover",
               "resend", "reject_old_epoch", "recovery_oracle")
EXTRA_STEPS = ("extra_condition", "recover", "resend", "recovery_oracle")
PROOFS = {
    "writer": ("full_input", "input_order_clock"),
    "writer_reaped": ("wait_reap",),
    "custody": ("db_wal_identity", "writer_absent"),
    "financial_ready": ("ready_startup", "binary_resource_anchor"),
    "economic_open": ("fresh_domain", "new_E_B", "same_manifest"),
    "same_domain_voice": ("formal_S_authority", "same_domain_gamma", "voice_independent_review"),
    "business": ("business_receipt", "request_response_identity", "no_received_pass"),
    "query": ("same_cut", "query_semantics"),
    "history": ("same_cut", "persistent_history"),
    "originals": ("request_receipt_set", "db_wal_identity"),
    "oracle": ("independent_accounting", "closure_exact_set"),
    "marker": ("real_owner_marker", "same_request"),
    "kill_reaped": ("same_pid_SIGKILL", "wait_reap"),
    "fault_custody": ("db_wal_identity", "owner_absent"),
    "recover": ("new_epoch", "durable_recovery"),
    "resend": ("original_identity", "idempotent_no_loss_no_duplicate"),
    "reject_old_epoch": ("old_epoch_rejected",),
    "recovery_oracle": ("independent_accounting", "before_after_commit", "closure_exact_set"),
    "extra_condition": ("named_extra_condition",),
}


def required_steps(scenario: Scenario) -> tuple[str, ...]:
    if scenario.family == "fault":
        return BASE_STEPS + FAULT_STEPS
    if scenario.family == "extra":
        return BASE_STEPS + EXTRA_STEPS
    return BASE_STEPS


@dataclass(frozen=True)
class Binding:
    run_id: str
    contract_sha: str
    preparation_sha: str
    source_sha: str
    input_sha: str
    manifest_sha: str
    voice_sha: str
    db: str
    domain: str
    owner: str
    epoch: int
    cut: str
    request_sha: str
    receipt_key: str
    operation: str


@dataclass(frozen=True)
class Request:
    binding: Binding
    expected_outcome: str
    ordinal: int = 0
    case: str = ""


@dataclass(frozen=True)
class BusinessReceipt:
    request: Request
    request_original_sha: str
    reply_original_sha: str
    independent_verdict: str
    observed_outcome: str


@dataclass(frozen=True)
class Step:
    scenario: str
    stage: str
    kind: str
    binding: Binding
    argv: tuple[str, ...]
    resources_sha: str
    deadline: float
    expected_proofs: tuple[str, ...]
    requests: tuple[Request, ...] = ()

    def key(self) -> tuple[str, str, str, int, str]:
        b = self.binding
        return b.run_id, b.domain, b.owner, b.epoch, self.stage

    def sha(self) -> str:
        return digest(asdict(self))


@dataclass(frozen=True)
class Plan:
    requirements: Requirements
    run_id: str
    contract_sha: str
    seed_sha: str
    source_sha: str
    input_sha: str
    manifest_sha: str
    voice_sha: str
    # 缺正式原件即为 None；不能用旧 Voice 或虚构权威补洞。
    formal_voice_authority_sha: str | None
    frozen_resources_sha: str | None
    artifact_manifest_sha: str | None
    supervisor_sha: str | None
    policy_original_sha: str | None
    economic_revision: str
    outer_manifest_revision: str
    steps: tuple[Step, ...]
    work_deadline: float
    cleanup_deadline: float
    seal_deadline: float
    execution_authorized: bool = False


@dataclass(frozen=True)
class Evidence:
    step_sha: str
    binding: Binding
    predecessor_sha: str
    command_sha: str
    resources_sha: str
    deadline: float
    delivery: str
    business_status: str
    raw_originals_sha: str
    proofs: tuple[str, ...]
    request_receipts: tuple[BusinessReceipt, ...] = ()


@dataclass(frozen=True)
class StageReceipt:
    sequence: int
    stage: str
    scenario: str
    step_sha: str
    predecessor_sha: str
    evidence_sha: str
    status: str

    def sha(self) -> str:
        return digest(asdict(self))


@dataclass(frozen=True)
class CleanupReceipt:
    owner: str
    reaped: bool
    original_sha: str
    error: str = ""


@dataclass(frozen=True)
class SealReceipt:
    original_sha: str
    final_outcome: str
    cleanup_sha: str
    stage_receipts_sha: str


@dataclass(frozen=True)
class Result:
    outcome: str
    error: str
    stage_receipts: tuple[StageReceipt, ...]
    attempted: tuple[str, ...]
    cleanup: tuple[CleanupReceipt, ...]
    seal: SealReceipt | None
    execution_authorized: bool = False
    product_executed: bool = False


class SimulationAdapter(ABC):
    """显式模拟能力边界；真实 ControlBridge 不继承它，mode 文本不授予能力。"""
    @abstractmethod
    def perform(self, step, predecessor_sha): ...


class Adapter(Protocol):
    """P0/P1 真接线前的独立接口；本版本只接受显式 UNIT_TEST_ONLY。"""
    mode: str

    def perform(self, step: Step, predecessor_sha: str) -> Evidence: ...
    def active_owner_keys(self) -> tuple[str, ...]: ...
    def cleanup_all(self, reason: str) -> tuple[CleanupReceipt, ...]: ...
    def seal(self, outcome: str, receipts: tuple[StageReceipt, ...],
             cleanup: tuple[CleanupReceipt, ...], deadline: float) -> SealReceipt: ...


def validate_plan(plan: Plan) -> None:
    require(plan.execution_authorized is False, "静态调度包不能接产品运行授权")
    for name in ("contract_sha", "seed_sha", "source_sha", "input_sha", "manifest_sha", "voice_sha"):
        require(sha_ok(getattr(plan, name)), f"无效 {name}")
    require(sha_ok(plan.requirements.preparation_sha), "准备合同未绑定")
    for name in ("formal_voice_authority_sha", "frozen_resources_sha",
                 "artifact_manifest_sha", "supervisor_sha", "policy_original_sha"):
        require(sha_ok(getattr(plan, name)), f"缺少正式原件 {name}")
    require(plan.economic_revision == "economic-session/2", "经济协议版本")
    require(plan.outer_manifest_revision == "economic-manifest/1", "外层 manifest 版本")
    require(bool(plan.run_id), "空 run_id")
    deadlines = (plan.work_deadline, plan.cleanup_deadline, plan.seal_deadline)
    require(all(isinstance(d, (int, float)) and not isinstance(d, bool) and
                math.isfinite(d) and d > 0 for d in deadlines), "期限不是有限正值")
    require(plan.work_deadline < plan.cleanup_deadline < plan.seal_deadline, "回收/封口预留窗无效")
    expected = [(s.name, kind) for s in plan.requirements.scenarios
                for kind in required_steps(s)]
    require([(s.scenario, s.kind) for s in plan.steps] == expected, "阶段缺漏/重排/重复")
    require(len({s.key() for s in plan.steps}) == len(plan.steps), "阶段生命周期重复")
    require(len({s.stage for s in plan.steps}) == len(plan.steps), "stage 名必须具名唯一")
    # 每个域的 owner→原库稳定；恢复只增 epoch，不偷偷换库。
    owner_dbs: dict[tuple[str, str, str], str] = {}
    epochs: dict[tuple[str, str, str], int] = {}
    normal_dbs: dict[str, set[str]] = {"A": set(), "B": set()}
    scenarios = {s.name: s for s in plan.requirements.scenarios}
    recovery_previous: dict[tuple[str, str, str], int] = {}
    original_requests: dict[str, tuple[str, str, str]] = {}
    for step in plan.steps:
        b = step.binding
        require(b.run_id == plan.run_id and b.contract_sha == plan.contract_sha, "运行/合同错绑")
        require(b.preparation_sha == plan.requirements.preparation_sha, "准备合同错绑")
        require((b.source_sha, b.input_sha, b.manifest_sha, b.voice_sha) ==
                (plan.source_sha, plan.input_sha, plan.manifest_sha, plan.voice_sha), "来源/输入/manifest/Voice错绑")
        require(sha_ok(b.request_sha) and bool(b.receipt_key) and bool(b.operation), "请求回执身份未冻")
        require(Path(b.db).is_absolute() and bool(b.domain) and bool(b.owner), "DB/域/owner未冻")
        require(isinstance(b.epoch, int) and not isinstance(b.epoch, bool) and b.epoch > 0 and bool(b.cut), "epoch/cut未冻")
        require(bool(step.argv) and all(isinstance(a, str) and a for a in step.argv), "命令未冻")
        require(step.resources_sha == plan.frozen_resources_sha, "资源对象错绑")
        require(isinstance(step.deadline, (int, float)) and not isinstance(step.deadline, bool) and
                math.isfinite(step.deadline) and 0 < step.deadline <= plan.work_deadline, "阶段延长总截止")
        mandatory_proofs = set(PROOFS[step.kind])
        if step.kind in ("oracle", "recovery_oracle"):
            mandatory_proofs.update("compare/" + key for key in plan.requirements.comparisons)
        require(mandatory_proofs.issubset(step.expected_proofs), "步骤判据缺漏")
        require(len(set(step.expected_proofs)) == len(step.expected_proofs), "步骤判据重复")
        if step.kind in ("business", "resend"):
            require(bool(step.requests), "业务请求全集为空")
        request_keys = []
        occurrences = {}
        request_originals = {}
        for request in step.requests:
            rb = request.binding
            require((rb.run_id, rb.contract_sha, rb.preparation_sha, rb.source_sha,
                     rb.input_sha, rb.manifest_sha, rb.voice_sha, rb.cut) ==
                    (b.run_id, b.contract_sha, b.preparation_sha, b.source_sha,
                     b.input_sha, b.manifest_sha, b.voice_sha, b.cut), "请求全集运行/来源/cut错绑")
            require(Path(rb.db).is_absolute() and rb.owner and rb.domain and
                    type(rb.epoch) is int and rb.epoch > 0 and sha_ok(rb.request_sha) and
                    rb.receipt_key and rb.operation and request.expected_outcome, "请求全集身份未冻")
            require(type(request.ordinal) is int and request.ordinal >= 0, "请求出现序号无效")
            identity = (rb.domain, rb.owner, rb.epoch, rb.receipt_key, rb.operation)
            require(request.ordinal == occurrences.get(identity, 0), "请求出现序号不连续")
            occurrences[identity] = request.ordinal + 1
            require(request_originals.setdefault(identity, rb.request_sha) == rb.request_sha,
                    "幂等出现改变原请求摘要")
            request_domain = step.scenario, rb.domain, rb.owner
            require(owner_dbs.setdefault(request_domain, rb.db) == rb.db, "请求全集同owner换库")
            scenario_for_request = scenarios[step.scenario]
            if scenario_for_request.family == "normal":
                normal_dbs[scenario_for_request.repeat].add(rb.db)
            request_keys.append((rb.domain, rb.owner, rb.epoch, rb.receipt_key, rb.operation, request.ordinal))
        require(len(request_keys) == len(set(request_keys)), "请求全集重复身份")
        domain_key = step.scenario, b.domain, b.owner
        require(owner_dbs.setdefault(domain_key, b.db) == b.db, "同owner换库")
        old_epoch = epochs.get(domain_key)
        if old_epoch is not None:
            if step.kind == "recover":
                require(b.epoch > old_epoch, "恢复必须新epoch")
                recovery_previous[domain_key] = old_epoch
            else:
                require(b.epoch == old_epoch, "非恢复阶段换epoch")
        epochs[domain_key] = b.epoch
        scenario = scenarios[step.scenario]
        if scenario.family == "normal" and step.kind == "business":
            require(set(plan.requirements.negative_domains).issubset(r.case for r in step.requests),
                    "完整负域业务请求缺漏")
        if scenario.family == "normal":
            normal_dbs[scenario.repeat].add(b.db)
        if step.kind == "marker":
            require((b.operation, b.domain) == (scenario.operation, scenario.domain), "故障操作/owner域错绑")
            original_requests[step.scenario] = b.request_sha, b.receipt_key, b.operation
        if scenario.family == "fault" and step.kind in ("kill_reaped", "fault_custody", "recover", "resend", "reject_old_epoch", "recovery_oracle"):
            require((b.operation, b.domain) == (scenario.operation, scenario.domain), "故障恢复操作/域错绑")
            require((b.request_sha, b.receipt_key, b.operation) ==
                    original_requests[step.scenario], "恢复重送原身份错绑")
        if step.kind == "reject_old_epoch":
            require(domain_key in recovery_previous, "无旧epoch可拒绝")
    require(not (normal_dbs["A"] & normal_dbs["B"]), "A/B复用数据库")


class Scheduler:
    """失败也消费；cleanup/seal不需要业务ACK或成功前驱。"""
    def __init__(self, plan: Plan, adapter: Adapter,
                 clock: Callable[[], float] = time.monotonic, *, journal=None) -> None:
        self.plan, self.adapter, self.clock = plan, adapter, clock
        self.journal = journal
        self._started = False
        self._consumed: set[tuple[str, str, str, int, str]] = set()

    def _remaining(self, deadline: float) -> None:
        require(self.clock() < deadline, "固定绝对截止耗尽")

    def _evidence(self, step: Step, previous: str, ev: Evidence) -> None:
        require(isinstance(ev, Evidence), "适配器未返回有类型证据")
        require(ev.step_sha == step.sha() and ev.binding == step.binding, "阶段证据身份错绑")
        require(ev.predecessor_sha == previous, "前驱收据错绑")
        require(ev.command_sha == digest(step.argv), "命令回执错绑")
        require(ev.resources_sha == step.resources_sha and ev.deadline == step.deadline, "资源/截止回执错绑")
        require(ev.delivery == "Received", "原回复尚未收到")
        require(ev.business_status == "INDEPENDENTLY_VERIFIED", "Received不能算业务PASS")
        require(sha_ok(ev.raw_originals_sha), "缺原请求回复原件")
        require(set(step.expected_proofs) == set(ev.proofs) and
                len(ev.proofs) == len(set(ev.proofs)), "步骤独立判据未完成")
        require(tuple(r.request for r in ev.request_receipts) == step.requests, "请求回执全集缺漏/额外/重排")
        for receipt in ev.request_receipts:
            require(receipt.independent_verdict == "INDEPENDENTLY_VERIFIED" and
                    receipt.request_original_sha == receipt.request.binding.request_sha and
                    sha_ok(receipt.reply_original_sha) and
                    receipt.observed_outcome == receipt.request.expected_outcome,
                    "请求回执未独立核验或原件/预期拒绝结果错绑")

    def run(self) -> Result:
        require(not self._started, "调度器一次消费，失败也不重试")
        self._started = True
        receipts: list[StageReceipt] = []
        attempted: list[str] = []
        cleanup: tuple[CleanupReceipt, ...] = ()
        seal = None
        error = ""
        outcome = "FAILED"
        try:
            require(isinstance(self.adapter, SimulationAdapter)
                    and self.adapter.mode == "UNIT_TEST_ONLY", "真实适配器未准入；mode 不授予模拟能力")
            validate_plan(self.plan)
            self._remaining(self.plan.work_deadline)
            previous = self.plan.seed_sha
            for step in self.plan.steps:
                require(step.key() not in self._consumed, "重复生命周期")
                self._consumed.add(step.key())
                attempted.append(step.stage)
                self._remaining(min(step.deadline, self.plan.work_deadline))
                if self.journal is not None:
                    self.journal.consume(step, previous)
                ev = self.adapter.perform(step, previous)
                # 包含ACK、落盘、解析等全部耗时；过期原件永不放行下一阶段。
                self._remaining(min(step.deadline, self.plan.work_deadline))
                self._evidence(step, previous, ev)
                receipt = StageReceipt(len(receipts), step.stage, step.scenario,
                                       step.sha(), previous, digest(asdict(ev)), "SIMULATED_VERIFIED")
                if self.journal is not None:
                    self.journal.finish(step, receipt)
                receipts.append(receipt)
                previous = receipt.sha()
            outcome = "SIMULATED_COMPLETE_NOT_PRODUCT_ACCEPTANCE"
        except BaseException as exc:
            error = f"{type(exc).__name__}: {exc}"
            outcome = "CANCELLED" if isinstance(exc, (KeyboardInterrupt, SystemExit)) else "FAILED"
        finally:
            # cleanup_all 必须是P0注册表快照，逐owner隔离异常；此处不接管陌生pid。
            try:
                self._remaining(self.plan.cleanup_deadline)
                registered = self.adapter.active_owner_keys()
                require(len(registered) == len(set(registered)), "owner注册表重复")
                cleanup = self.adapter.cleanup_all("scheduler-finally/" + outcome)
                self._remaining(self.plan.cleanup_deadline)
                require(isinstance(cleanup, tuple) and
                        all(isinstance(c, CleanupReceipt) for c in cleanup), "回收回执类型错误")
                require(len({c.owner for c in cleanup}) == len(cleanup), "回收owner重复")
                require({c.owner for c in cleanup} == set(registered), "回收未覆盖完整owner注册表")
                require(all(c.reaped and not c.error and sha_ok(c.original_sha) for c in cleanup),
                        "存在未回收/未知owner")
            except BaseException as exc:
                error += f"; cleanup {type(exc).__name__}: {exc}"
                outcome = "INCOMPLETE"
            try:
                self._remaining(self.plan.seal_deadline)
                seal = self.adapter.seal(outcome, tuple(receipts), cleanup, self.plan.seal_deadline)
                self._remaining(self.plan.seal_deadline)
                require(isinstance(seal, SealReceipt) and sha_ok(seal.original_sha), "封口原件缺失")
                require(seal.final_outcome == outcome and
                        seal.cleanup_sha == digest([asdict(c) for c in cleanup]) and
                        seal.stage_receipts_sha == digest([asdict(r) for r in receipts]), "封口回执错绑")
            except BaseException as exc:
                error += f"; seal {type(exc).__name__}: {exc}"
                outcome, seal = "INCOMPLETE", None
        return Result(outcome, error, tuple(receipts), tuple(attempted), cleanup, seal)
