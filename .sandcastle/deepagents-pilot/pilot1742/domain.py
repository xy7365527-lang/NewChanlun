from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Literal, NewType, cast

TICKET = 1742

Sha = NewType("Sha", str)
RunId = NewType("RunId", str)

_SHA = re.compile(r"[0-9a-f]{40}")
_RUN_ID = re.compile(r"[a-z0-9][a-z0-9-]{0,40}")


def parse_sha(raw: object) -> Sha:
    if not isinstance(raw, str) or not _SHA.fullmatch(raw):
        raise ValueError(f"需要完整 40 位小写 SHA：{raw!r}")
    return Sha(raw)


def parse_run_id(raw: str) -> RunId:
    if not _RUN_ID.fullmatch(raw):
        raise ValueError("run-id 只能是小写字母、数字与连字符，最长 41 位")
    return RunId(raw)


ModelSlug = Literal["gpt-6.1-sol", "claude-opus-5-5"]
MODEL_FAMILY: dict[str, str] = {"gpt-6.1-sol": "gpt", "claude-opus-5-5": "claude"}


def parse_model(raw: object) -> ModelSlug:
    if raw not in MODEL_FAMILY:
        raise ValueError(f"模型不在白名单 {sorted(MODEL_FAMILY)}：{raw!r}")
    return cast(ModelSlug, raw)


Role = Literal["author", "review", "design-a", "design-b", "judge"]
ROLE_MODE: dict[Role, Literal["execute", "review"]] = {
    "author": "execute", "review": "review", "design-a": "review", "design-b": "review", "judge": "review"}
DESIGN_SLOTS: tuple[Role, Role] = ("design-a", "design-b")


def parse_role(raw: str) -> Role:
    if raw not in ROLE_MODE:
        raise ValueError(f"未知角色：{raw!r}")
    return cast(Role, raw)


@dataclass(frozen=True)
class WorkerKey:
    """派工幂等键。不含 brief 与 attempt：同一键永远只对应一次 codex-child 认领。"""

    run_id: RunId
    role: Role
    target_head: Sha

    @property
    def name(self) -> str:
        return f"{self.role}-{self.target_head}"


def parse_key_name(run_id: RunId, name: str) -> WorkerKey:
    role, _, head = name.rpartition("-")
    return WorkerKey(run_id, parse_role(role), parse_sha(head))


@dataclass(frozen=True)
class WorkerReceipt:
    """codex-child result.json 的终态，模型取实记值而非请求值。"""

    key: WorkerKey
    child_run_id: str
    status: Literal["completed", "failed", "cancelled"]
    model: ModelSlug
    expected_head: Sha
    head_after: Sha | None
    changed_paths_after: tuple[str, ...]
    external_session_id: str | None
    external_session_log: str | None
    summary: str
    findings: tuple[str, ...]
    validation: tuple[str, ...]
    failure_reason: str | None
    output_dir: str


@dataclass(frozen=True)
class WorkerObservation:
    output_dir_exists: bool
    result: WorkerReceipt | None
    supervisor_alive: bool
    worker_group_alive: bool


@dataclass(frozen=True)
class NotStarted:
    pass


@dataclass(frozen=True)
class Running:
    pass


@dataclass(frozen=True)
class Terminal:
    receipt: WorkerReceipt


@dataclass(frozen=True)
class EffectUnknown:
    reason: str


WorkerFact = NotStarted | Running | Terminal | EffectUnknown


def classify_worker(obs: WorkerObservation) -> WorkerFact:
    """result.json 优先；没有 outputDir 就没有任何工蜂副作用，因为 codex-child 的第一个副作用就是排他 mkdir。"""
    if obs.result is not None:
        return Terminal(obs.result)
    if obs.supervisor_alive:
        return Running()
    if not obs.output_dir_exists:
        return NotStarted()
    if obs.worker_group_alive:
        return EffectUnknown("监督进程已不在，但工蜂进程组仍在运行，结果无法收敛")
    return EffectUnknown("输出目录已认领，没有终态结果，也没有存活的监督进程")


@dataclass(frozen=True)
class CommitIntent:
    author: WorkerKey
    parent: Sha
    tree: str
    paths: tuple[str, ...]
    generation: int | None = None

    @property
    def trailer(self) -> str:
        base = f"{self.author.run_id}/{self.author.name}"
        return base if self.generation is None else f"{base} gen={self.generation}"


TRAILER_KEY = "Pilot-1742-Run"
V2_TRAILER_KEY = "Deepagents-Run"


@dataclass(frozen=True)
class CommitReceipt:
    intent: CommitIntent
    commit: Sha
    recovered: bool


@dataclass(frozen=True)
class HeadObservation:
    head: Sha
    first_parent: Sha | None
    tree: str
    trailers: tuple[str, ...]


def reconcile_commit(intent: CommitIntent, obs: HeadObservation) -> Literal["committed", "not_committed", "conflict"]:
    """提交已做、回执未落时，只有 parent、tree、trailer 三者全对上才认领这个提交。"""
    if obs.head == intent.parent:
        return "not_committed"
    if obs.first_parent == intent.parent and obs.tree == intent.tree and intent.trailer in obs.trailers:
        return "committed"
    return "conflict"


def outside_allowed(paths: tuple[str, ...], prefixes: tuple[str, ...]) -> tuple[str, ...]:
    return tuple(p for p in paths if not any(p.startswith(prefix) for prefix in prefixes))


VerdictKind = Literal["PASS", "BLOCK", "UNPARSED", "VERIFIER_BLOCKED"]
_VERDICT_LINES: dict[str, VerdictKind] = {"VERDICT: PASS": "PASS", "VERDICT: BLOCK": "BLOCK"}


@dataclass(frozen=True)
class ReviewVerdict:
    head: Sha
    kind: VerdictKind
    author_model: ModelSlug
    reviewer_model: ModelSlug


def parse_verdict(review: WorkerReceipt, author_model: ModelSlug) -> ReviewVerdict:
    """completed 只说明工蜂跑完；summary 首行必须整行等于 `VERDICT: PASS` 或 `VERDICT: BLOCK`，且作者与独审不同家族。"""
    head = review.key.target_head
    if MODEL_FAMILY[review.model] == MODEL_FAMILY[author_model]:
        return ReviewVerdict(head, "VERIFIER_BLOCKED", author_model, review.model)
    first_line = review.summary.splitlines()[0] if review.summary else ""
    kind = _VERDICT_LINES.get(first_line, "UNPARSED") if review.status == "completed" else "UNPARSED"
    return ReviewVerdict(head, kind, author_model, review.model)


Scope = Literal["pilot_acceptance", "main_merge"]
SourceKind = Literal["tracker_comment", "user_message"]


@dataclass(frozen=True)
class HumanSource:
    """已核实的人类来源：主控登记、可回溯到评论 URL 或用户原始消息。同名 gh 账号本身不构成来源。"""

    kind: SourceKind
    reference: str
    registered_by: str


@dataclass(frozen=True)
class ApprovalRecord:
    seq: int
    ticket: int
    scope: Scope
    head: Sha | None
    decision: Literal["approve", "reject"]
    source: HumanSource


@dataclass(frozen=True)
class Approved:
    record: ApprovalRecord
    verdict: ReviewVerdict


@dataclass(frozen=True)
class Awaiting:
    head: Sha
    reasons: tuple[str, ...]


@dataclass(frozen=True)
class Refused:
    head: Sha
    reasons: tuple[str, ...]


GateDecision = Approved | Awaiting | Refused


@dataclass(frozen=True)
class CheckoutState:
    branch: str
    head: Sha


@dataclass(frozen=True)
class Conflict:
    """作者 checkout 的实际分支或 HEAD 与本 run 回执链不符；门与新派工都拒绝，等主控处置。"""

    code: Literal["author_branch_changed", "candidate_head_drift"]
    recorded_head: Sha
    observed: CheckoutState


def decide_gate(*, ticket: int, scope: Scope, candidate: Sha, conflict: Conflict | None,
                verdict: ReviewVerdict | None, records: list[ApprovalRecord]) -> GateDecision:
    """两因子门：实际 HEAD 等于回执候选、该 head 的异家族 PASS，加同票、同 scope、同 head 的最新人类批准。"""
    if conflict is not None:
        return Refused(candidate, (conflict.code,))
    if verdict is None or verdict.head != candidate:
        return Refused(candidate, ("no_review_at_head",))
    if verdict.kind != "PASS":
        return Refused(candidate, ({"BLOCK": "review_block", "UNPARSED": "unparsed_verdict",
                                    "VERIFIER_BLOCKED": "verifier_blocked"}[verdict.kind],))
    scoped = [r for r in records if r.ticket == ticket and r.scope == scope]
    relevant = [r for r in scoped if r.head == candidate or (r.decision == "reject" and r.head is None)]
    if relevant:
        latest = max(relevant, key=lambda r: r.seq)
        if latest.decision == "approve":
            return Approved(latest, verdict)
        return Refused(candidate, ("revoked",))
    if any(r.decision == "approve" for r in scoped):
        return Awaiting(candidate, ("approval_for_other_head",))
    return Awaiting(candidate, ("no_approval",))


@dataclass(frozen=True)
class RunFacts:
    run_id: RunId
    base: Sha
    require_design: bool
    commits: tuple[CommitReceipt, ...]
    workers: dict[WorkerKey, WorkerFact]
    verdicts: dict[Sha, ReviewVerdict]
    author_branch: str
    observed: CheckoutState
    pending_commit_landed: bool


def chain_head(base: Sha, commits: tuple[CommitReceipt, ...]) -> Sha:
    by_parent = {c.intent.parent: c.commit for c in commits}
    head = base
    while head in by_parent:
        head = by_parent[head]
    return head


def current_head(facts: RunFacts) -> Sha:
    return chain_head(facts.base, facts.commits)


def candidate_conflict(facts: RunFacts) -> Conflict | None:
    """提交已做、回执未落时，实际 HEAD 领先一步是可恢复的，不算冲突。"""
    recorded = current_head(facts)
    if facts.observed.branch != facts.author_branch:
        return Conflict("author_branch_changed", recorded, facts.observed)
    if facts.observed.head == recorded or facts.pending_commit_landed:
        return None
    return Conflict("candidate_head_drift", recorded, facts.observed)


def _completed(fact: WorkerFact | None) -> bool:
    return isinstance(fact, Terminal) and fact.receipt.status == "completed"


def _stuck(fact: WorkerFact) -> bool:
    if isinstance(fact, EffectUnknown):
        return True
    if not isinstance(fact, Terminal):
        return False
    receipt = fact.receipt
    return receipt.status != "completed" or (receipt.key.role == "author" and not receipt.changed_paths_after)


def dispatch_refusal(facts: RunFacts, key: WorkerKey) -> str | None:
    """新键能否开派；返回拒绝原因或 None。已存在的键不经过这里，由调用方收养。"""
    conflict = candidate_conflict(facts)
    if conflict is not None:
        return f"{conflict.code}：回执候选 {conflict.recorded_head}，实际 {conflict.observed.branch}@{conflict.observed.head}"
    paired = key.role in DESIGN_SLOTS
    busy = [k.name for k, f in facts.workers.items()
            if isinstance(f, Running) and not (paired and k.role in DESIGN_SLOTS and k.target_head == key.target_head)]
    if busy:
        return f"已有工蜂在运行：{busy}"
    stuck = [k.name for k, f in facts.workers.items() if _stuck(f)]
    if stuck:
        return f"本 run 停在未成功、无产出或结果不明的工蜂上，需主控处置：{stuck}"
    head = current_head(facts)
    at_base = head == facts.base and not any(k.role == "author" for k in facts.workers)
    if key.role in DESIGN_SLOTS:
        return None if at_base else "设计 arena 只在首个作者之前、在基线上进行"
    if key.role == "judge":
        if not at_base:
            return "设计 arena 只在首个作者之前、在基线上进行"
        designs = [facts.workers.get(WorkerKey(facts.run_id, slot, head)) for slot in DESIGN_SLOTS]
        return None if all(_completed(f) for f in designs) else "两个设计候选都完成后才能派交叉裁判"
    if key.role == "review":
        return "还没有本 run 的候选提交，不能独审" if head == facts.base else None
    if head == facts.base:
        judged = _completed(facts.workers.get(WorkerKey(facts.run_id, "judge", head)))
        if facts.require_design and not judged:
            return "本 run 要求先完成设计 arena（两个设计候选与交叉裁判）再派作者"
        return None
    verdict = facts.verdicts.get(head)
    if verdict is not None and verdict.kind == "BLOCK":
        return None
    return f"候选 {head} 没有明确的独审 BLOCK，不能再派作者"


BASE_LINES = ("BASE: 候选 1", "BASE: 候选 2")


def declared_base(brief: str) -> str | None:
    found = [line.strip() for line in brief.splitlines() if line.strip() in BASE_LINES]
    return found[0] if len(set(found)) == 1 else None


def synthesis_refusal(facts: RunFacts, key: WorkerKey, brief: str) -> str | None:
    """设计 arena 之后的首个作者 brief 必须恰好声明一个 base，arena Phase D 的选择因此可审计。"""
    judged = _completed(facts.workers.get(WorkerKey(facts.run_id, "judge", key.target_head)))
    if key.role != "author" or not judged or declared_base(brief) is not None:
        return None
    return "设计 arena 之后的作者 brief 要先写合成说明，并单独一行写且只写一个 `BASE: 候选 1` 或 `BASE: 候选 2`"


Phase = Literal["needs_design", "design_running", "needs_judge", "needs_author", "author_running", "needs_commit",
                "needs_review", "review_running", "rework", "gate", "halted", "conflict"]


def derive_phase(facts: RunFacts) -> Phase:
    if candidate_conflict(facts) is not None:
        return "conflict"
    if any(_stuck(f) for f in facts.workers.values()):
        return "halted"
    head = current_head(facts)
    fact = {role: facts.workers.get(WorkerKey(facts.run_id, role, head))
            for role in ("author", "review", "design-a", "design-b", "judge")}
    if any(isinstance(fact[role], Running) for role in ("design-a", "design-b", "judge")):
        return "design_running"
    if isinstance(fact["author"], Running):
        return "author_running"
    if isinstance(fact["review"], Running):
        return "review_running"
    if isinstance(fact["author"], Terminal):
        return "needs_commit"
    if head == facts.base:
        if facts.require_design and not all(_completed(fact[slot]) for slot in DESIGN_SLOTS):
            return "needs_design"
        if facts.require_design and not _completed(fact["judge"]):
            return "needs_judge"
        return "needs_author"
    if fact["review"] is None or isinstance(fact["review"], NotStarted):
        return "needs_review"
    verdict = facts.verdicts.get(head)
    if verdict is not None and verdict.kind == "BLOCK":
        return "rework"
    if verdict is not None and verdict.kind == "PASS":
        return "gate"
    return "halted"
