"""#1742 试点的领域模型：纯数据与纯决策，不做 I/O。

外部输入（codex-child 的 result.json、git 观测、批准账本、CLI 参数）在 run.py 边界解析成这里的类型。
"""
from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Literal, NewType

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
    return raw  # type: ignore[return-value]


Role = Literal["author", "review"]
ROLE_MODE: dict[Role, Literal["execute", "review"]] = {"author": "execute", "review": "review"}


@dataclass(frozen=True)
class WorkerKey:
    """派工幂等键。不含 brief 与 attempt：同一键永远只对应一次 codex-child 认领。"""

    run_id: RunId
    role: Role
    target_head: Sha

    @property
    def name(self) -> str:
        return f"{self.role}-{self.target_head}"


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

    @property
    def trailer(self) -> str:
        return f"{self.author.run_id}/{self.author.name}"


TRAILER_KEY = "Pilot-1742-Run"


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
_VERDICT = re.compile(r"\s*VERDICT:\s*(PASS|BLOCK)\b")


@dataclass(frozen=True)
class ReviewVerdict:
    head: Sha
    kind: VerdictKind
    author_model: ModelSlug
    reviewer_model: ModelSlug


def parse_verdict(review: WorkerReceipt, author_model: ModelSlug) -> ReviewVerdict:
    """completed 只说明工蜂跑完；结论只认 summary 首行的 VERDICT 协议，且作者与独审必须不同家族。"""
    head = review.key.target_head
    if MODEL_FAMILY[review.model] == MODEL_FAMILY[author_model]:
        return ReviewVerdict(head, "VERIFIER_BLOCKED", author_model, review.model)
    match = _VERDICT.match(review.summary) if review.status == "completed" else None
    return ReviewVerdict(head, match.group(1) if match else "UNPARSED", author_model, review.model)  # type: ignore[arg-type]


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


def decide_gate(*, ticket: int, scope: Scope, candidate: Sha, verdict: ReviewVerdict | None,
                records: list[ApprovalRecord]) -> GateDecision:
    """两因子门：当前 head 的异家族 PASS，加同票、同 scope、同 head 的最新人类批准。"""
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
    commits: tuple[CommitReceipt, ...]
    workers: dict[WorkerKey, WorkerFact]
    verdicts: dict[Sha, ReviewVerdict]


def current_head(facts: RunFacts) -> Sha:
    by_parent = {c.intent.parent: c.commit for c in facts.commits}
    head = facts.base
    while head in by_parent:
        head = by_parent[head]
    return head


def _stuck(fact: WorkerFact) -> bool:
    if isinstance(fact, EffectUnknown):
        return True
    if not isinstance(fact, Terminal):
        return False
    receipt = fact.receipt
    return receipt.status != "completed" or (receipt.key.role == "author" and not receipt.changed_paths_after)


def dispatch_refusal(facts: RunFacts, key: WorkerKey) -> str | None:
    """新键能否开派；返回拒绝原因或 None。已存在的键不经过这里，由调用方收养。"""
    busy = [k.name for k, f in facts.workers.items() if isinstance(f, Running)]
    if busy:
        return f"已有工蜂在运行：{busy}"
    stuck = [k.name for k, f in facts.workers.items() if _stuck(f)]
    if stuck:
        return f"本 run 停在未成功、无产出或结果不明的工蜂上，需主控处置：{stuck}"
    head = current_head(facts)
    if key.role == "review":
        return "还没有本 run 的候选提交，不能独审" if head == facts.base else None
    if head == facts.base:
        return None
    verdict = facts.verdicts.get(head)
    if verdict is not None and verdict.kind == "BLOCK":
        return None
    return f"候选 {head} 没有明确的独审 BLOCK，不能再派作者"


Phase = Literal["needs_author", "author_running", "needs_commit", "needs_review", "review_running",
                "rework", "gate", "halted"]


def derive_phase(facts: RunFacts) -> Phase:
    if any(_stuck(f) for f in facts.workers.values()):
        return "halted"
    head = current_head(facts)
    author = facts.workers.get(WorkerKey(facts.run_id, "author", head))
    review = facts.workers.get(WorkerKey(facts.run_id, "review", head))
    if isinstance(author, Running):
        return "author_running"
    if isinstance(review, Running):
        return "review_running"
    if isinstance(author, Terminal):
        return "needs_commit"
    if head == facts.base:
        return "needs_author"
    if review is None or isinstance(review, NotStarted):
        return "needs_review"
    verdict = facts.verdicts.get(head)
    if verdict is not None and verdict.kind == "BLOCK":
        return "rework"
    if verdict is not None and verdict.kind == "PASS":
        return "gate"
    return "halted"
