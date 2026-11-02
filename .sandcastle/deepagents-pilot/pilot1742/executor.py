from __future__ import annotations

import argparse
import fcntl
import json
import os
import signal
import subprocess
import sys
import time
from collections.abc import Callable, Iterator
from contextlib import contextmanager, nullcontext
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

from approval_signing.protocol import CommitTarget, Scope

from .approval import (
    ApprovalDesk,
    Pending,
    SignedApprovals,
    decision_json,
    spec_body_target,
)
from .boat_execution import ExecutionUnknown
from .claims import (
    ClaimRefused,
    ClaimStore,
    ClaimTxn,
    Fenced,
    LockTimeout,
    TicketClaim,
    advance,
)
from .cutover import (
    DRAIN_ROOT,
    CutoverError,
    CutoverStore,
    FrozenSnapshot,
    LegacyProcess,
    legacy_processes,
    remote_barrier,
)
from .delivery import (
    EffectPending,
    ExactUpload,
    Journal,
    RepoGateway,
    Stale,
    Unpublishable,
    marker,
)
from .domain import (
    EffectUnknown,
    NotStarted,
    RunFacts,
    Running,
    Sha,
    candidate_conflict,
    current_head,
    parse_key_name,
    parse_sha,
)
from .github_app import WRITE_PERMISSIONS, AppIdentity, GitHubAppClient, IdentityError
from .run import (
    Busy,
    Halted,
    PilotRun,
    TaskSpec,
    ToolRefusal,
    _now,
    _ps_rows,
    lock_run_dir,
    parse_task,
)
from .workunit import (
    MIRROR_LABEL,
    REPO,
    DedicatedPilot,
    Ineligible,
    Playbook,
    Queued,
    TicketEvidence,
    WorkUnit,
    forbidden_for,
    judge_eligibility,
    parse_unit_run_id,
    parse_work_unit,
)

DEFAULT_STATE_ROOT = Path("/Volumes/AgentStorage/deepagents-exec")
WORKERS_LOG = Path("/Users/silencehan/Projects/NewChanlun/.sandcastle/logs/workers.jsonl")
STATUS_CONTEXT = "deepagents/review"
V2_CRASH_POINTS = ("after_label_before_receipt", "after_claim_before_receipt", "after_push_before_receipt",
                   "after_pr_before_receipt", "after_status_before_receipt", "after_deliver_before_receipt")
EXECUTOR_ERRORS = (ClaimRefused, Fenced, LockTimeout, EffectPending, IdentityError, Unpublishable)


class AdmissionRefused(ClaimRefused):
    pass


class ApprovalRequired(Exception):
    """动作需要的真人签批此刻不成立。不执行副作用，不转 halted；请求文件路径随异常给出。"""

    def __init__(self, decision: Pending):
        super().__init__(f"{decision.status}：{decision.reason}；请求 {decision.request_path}")
        self.decision = decision


class PullRequestRefused(Exception):
    """远端 PR 不符合本票交付合同（已关闭、目标不对、不是 draft、不是本 App 为本票所开）。不收养、不改动它。"""


def gather_evidence(gateway: RepoGateway, ticket: int, workers_log: Path) -> TicketEvidence:
    """票面、完整 timeline、父票、workers.jsonl 逐项读全；任何一项读不到都直接抛错，不当成空集。"""
    issue = gateway.issue(ticket)
    timeline = gateway.timeline(ticket)
    if not all(isinstance(event, dict) for event in timeline):
        raise IdentityError("timeline 读回不完整")
    has_parent = gateway.has_parent(ticket)
    try:
        log = workers_log.read_text()
    except OSError:
        raise AdmissionRefused(f"workers.jsonl 不可读，不能证明该票不在旧消费者记录中：{workers_log}") from None
    return TicketEvidence(ticket, "pull_request" in issue, str(issue.get("state")), str(issue.get("body") or ""),
                          gateway.label_names(issue), tuple(timeline), has_parent, log)


def publish_refusal(facts: RunFacts) -> str | None:
    conflict = candidate_conflict(facts)
    if conflict is not None:
        return conflict.code
    head = current_head(facts)
    if head == facts.base:
        return "no_candidate"
    verdict = facts.verdicts.get(head)
    if verdict is None or verdict.head != head:
        return "no_review_at_head"
    if verdict.kind != "PASS":
        return {"BLOCK": "review_block", "UNPARSED": "unparsed_verdict",
                "VERIFIER_BLOCKED": "verifier_blocked"}[verdict.kind]
    return None


def delivery_status(run_dir: Path) -> dict[str, Any]:
    folder = run_dir / "delivery"
    effects = {}
    if folder.exists():
        for path in sorted(folder.glob("*.json")):
            if path.name not in ("state.json", "stale.json"):
                effects[path.stem] = json.loads(path.read_text())["state"]
    state = json.loads((folder / "state.json").read_text()) if (folder / "state.json").exists() else None
    stale = json.loads((folder / "stale.json").read_text()) if (folder / "stale.json").exists() else None
    phase = "stale" if stale else (state["phase"] if state else ("publishing" if effects else None))
    return {"phase": phase, "effects": effects, "state": state, "stale": stale}


def _write_json(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(f"{path.name}.{os.getpid()}.tmp")
    tmp.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
    os.replace(tmp, path)


@dataclass
class Boundary:
    """一次外部副作用的锁内上下文。halt 在同一把锁内把持有转为 halted。"""

    txn: ClaimTxn
    claim: TicketClaim

    def halt(self, reason: str) -> Halted:
        self.claim = self.txn.write(advance(self.claim, "halt", reason=reason))
        return Halted(f"票级持有转 halted：{reason}")


def _no_crash(_point: str) -> None:
    return None


class TicketExecutor:
    """一张票在新执行器里的持有、动作锁与精确交付。所有 GitHub 写都经 App 身份。"""

    def __init__(self, unit: WorkUnit, gateway: RepoGateway, *, claims: ClaimStore, workers_log: Path = WORKERS_LOG,
                 approvals: SignedApprovals | None = None, crash: Callable[[str], None] = _no_crash,
                 cutover: CutoverStore | None = None):
        self.unit = unit
        self.gateway = gateway
        self.claims = claims
        self.workers_log = workers_log
        self.approvals = approvals or SignedApprovals()
        self.crash = crash
        self.cutover = cutover or CutoverStore()

    def _marker(self, kind: str, run_id: str, generation: int, head: str) -> str:
        return marker(kind, ticket=self.unit.ticket, run_id=run_id, generation=generation, head=head)

    def desk(self, run_dir: Path) -> ApprovalDesk:
        return ApprovalDesk(run_dir / "approvals", self.unit.ticket, self.unit.run_id, self.approvals)

    def spec_approval(self, run_dir: Path, generation: int) -> Pending | None:
        """需要 SPEC 授权的准入：目标是 SPEC 票此刻正文的原始字节摘要，正文一变就换请求。"""
        admission = self.unit.admission
        assert not isinstance(admission, DedicatedPilot)
        target = spec_body_target(admission.spec_auth.spec_issue, self.gateway.issue(admission.spec_auth.spec_issue))
        decision = self.desk(run_dir).check(Scope.SPEC_AUTHORIZATION, generation, target)
        return decision if isinstance(decision, Pending) else None

    def candidate_approval(self, run_dir: Path, generation: int, head: Sha) -> Pending | None:
        decision = self.desk(run_dir).check(Scope.CANDIDATE_HEAD, generation, CommitTarget(head))
        return decision if isinstance(decision, Pending) else None

    def admit(self, run_dir: Path, generation: int) -> None:
        if isinstance(self.unit.admission, DedicatedPilot):
            proof = judge_eligibility(gather_evidence(self.gateway, self.unit.ticket, self.workers_log))
            if isinstance(proof, Ineligible):
                raise AdmissionRefused("专用票准入不成立：" + ", ".join(proof.reasons))
            return
        if isinstance(self.unit.admission, Queued):
            self._queue_check()
            pending = self.spec_approval(run_dir, generation)
            if pending is not None:
                raise ApprovalRequired(pending)
            return
        pending = self.spec_approval(run_dir, generation)
        if pending is not None:
            raise AdmissionRefused(f"{pending.status}：{pending.reason}；请求 {pending.request_path}")
        raise AdmissionRefused("takeover_barrier_unimplemented：旧票冻结排空屏障（SPEC 7.3）未实施，签批有效也不接管")

    def _cutover_lock(self):
        return self.cutover.frozen() if isinstance(self.unit.admission, Queued) else nullcontext()

    def _queue_check(self) -> None:
        from .queue import approved_scope, prove_queue
        assert isinstance(self.unit.admission, Queued)
        prove_queue(self.gateway, self.unit.ticket, self.cutover, self.unit.admission.proof)
        scope = approved_scope(str(self.gateway.issue(self.unit.admission.spec_auth.spec_issue).get("body") or ""))
        if any(not any(prefix.startswith(allowed) for allowed in scope) for prefix in self.unit.allowed_prefixes):
            raise AdmissionRefused("registered scope exceeds current signed SPEC deepagents-scope")

    def _previous_run_quiet(self, previous: TicketClaim) -> None:
        run_dir = Path(previous.run_dir)
        lock = run_dir / "owner.lock"
        if lock.exists():
            fd = os.open(lock, os.O_RDWR)
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise ClaimRefused(f"上一代次 run {previous.run_id} 的控制进程仍持锁") from None
            finally:
                os.close(fd)
        needle = f"--output-dir {run_dir}/workers/"
        if any(needle in cmd and not stat.startswith("Z") for _, _, stat, cmd in _ps_rows()):
            raise ClaimRefused(f"上一代次 run {previous.run_id} 仍有工蜂在运行")
        # Boat 工蜂不在本机 PID 表；仅观察冻结绑定，不恢复、收集或补提交旧工蜂。
        try:
            if not (run_dir / "run.json").exists():
                if (run_dir / "boat").exists():
                    raise ExecutionUnknown("缺少旧 run 清单")
                return
            run = PilotRun.open(run_dir.parent.parent, previous.run_id)
            if run.execution is None:
                if (run_dir / "boat").exists():
                    raise ExecutionUnknown("旧执行后端与 Boat 绑定不符")
                return
            if (run.spec.run_id != previous.run_id or run.dir.resolve() != run_dir.resolve()
                    or run.spec.ticket != previous.ticket or run.generation != previous.generation):
                raise ExecutionUnknown("旧 run 的票号或持有代次不符")
            keys = set(run._keys())
            keys.update(parse_key_name(previous.run_id, p.stem) for p in (run_dir / "boat").glob("*.json")
                        if "." not in p.stem)
            for key in keys:
                fact = run.execution.observe(key, reconcile=False)
                if isinstance(fact, (NotStarted, Running, EffectUnknown)) and not run.execution.is_drained(key):
                    raise ExecutionUnknown(f"{key.name}: {fact}")
        except (ExecutionUnknown, OSError, ValueError, TypeError, KeyError) as error:
            raise ClaimRefused(f"上一代次 run {previous.run_id} 的 Boat 工蜂尚未排空：{error}") from None

    def claim(self, run_dir: Path) -> TicketClaim:
        """SPEC 7.4：intent → 镜像 → post-check → active，全程在票级动作锁内；崩溃后同 run 从记录处续做。"""
        unit = self.unit
        resolved = str(run_dir.resolve())
        with self.claims.locked(unit.ticket) as txn, self._cutover_lock():
            record = txn.record
            if record is not None and record.run_id == unit.run_id:
                if record.run_dir != resolved:
                    raise ClaimRefused(f"run {unit.run_id} 已由另一状态根持有：{record.run_dir}")
                if record.state in ("halted", "released"):
                    raise ClaimRefused(f"run {unit.run_id} 的持有已是 {record.state}，只能由主控以新 run 重新持有")
                claim = record
            else:
                if record is not None and record.state != "released":
                    raise ClaimRefused(f"票 #{unit.ticket} 已由 {record.run_id} 持有（{record.state}）")
                if record is not None:
                    self._previous_run_quiet(record)
                self.admit(run_dir, record.generation + 1 if record else 1)
                labels = self.gateway.label_names(self.gateway.issue(unit.ticket))
                if record is None and MIRROR_LABEL in labels:
                    raise ClaimRefused("票面已有 deepagents 镜像但本机无持有记录，拒绝收养")
                claim = txn.write(TicketClaim(REPO, unit.ticket, unit.run_id, resolved,
                                              record.generation + 1 if record else 1, "intent",
                                              unit.admission.kind, False, None, None, _now()))
            if isinstance(unit.admission, Queued):
                self.admit(run_dir, claim.generation)
            journal = Journal(claim_effects(self.claims, unit.ticket, claim.run_id, claim.generation))
            if claim.state == "intent":
                journal.converge("label", lambda: self._label_probe(), lambda: self.gateway.add_label(
                    unit.ticket, MIRROR_LABEL), idempotent=True,
                    what={"kind": "label", "issue": unit.ticket, "name": MIRROR_LABEL}, crash=self.crash)
                text = self._marker("claim", claim.run_id, claim.generation, unit.base)
                body = (f"[deepagents run={claim.run_id} gen={claim.generation}]\n\n"
                        f"新执行器（{self.gateway.bot}）持有本票，基线 `{unit.base}`。这是持有标记，不是批准。\n\n{text}")
                comment = journal.converge("claim", lambda: self.gateway.find_marker(unit.ticket, text),
                                           lambda: self.gateway.post_comment(unit.ticket, body), idempotent=False,
                                           what={"kind": "comment", "issue": unit.ticket, "marker": text},
                                           crash=self.crash)
                claim = txn.write(advance(claim, "mirror", mirror_label=True, marker_comment=comment.get("url")))
            if claim.state == "mirrored":
                problems = self._post_check(claim)
                if isinstance(unit.admission, Queued) and not problems:
                    try:
                        self.admit(run_dir, claim.generation)
                    except (ClaimRefused, CutoverError, ValueError) as error:
                        problems.append(str(error))
                if problems:
                    reason = "post_check_failed:" + ",".join(problems) + self._withdraw(journal, claim)
                    txn.write(advance(claim, "halt", reason=reason, mirror_label=False))
                    raise ClaimRefused(reason)
                claim = txn.write(advance(claim, "activate"))
            return claim

    def _label_probe(self) -> dict[str, Any] | None:
        labels = self.gateway.label_names(self.gateway.issue(self.unit.ticket))
        return {"label": MIRROR_LABEL} if MIRROR_LABEL in labels else None

    def _post_check(self, claim: TicketClaim) -> list[str]:
        issue = self.gateway.issue(self.unit.ticket)
        labels = self.gateway.label_names(issue)
        problems = []
        if issue.get("state") != "open":
            problems.append("not_open")
        if MIRROR_LABEL not in labels:
            problems.append("mirror_label_missing")
        problems.extend(f"forbidden:{name}" for name in forbidden_for(self.unit.admission, labels))
        if self.gateway.find_marker(self.unit.ticket, self._marker(
                "claim", claim.run_id, claim.generation, self.unit.base)) is None:
            problems.append("claim_marker_missing")
        if isinstance(self.unit.admission, DedicatedPilot):
            proof = judge_eligibility(gather_evidence(self.gateway, self.unit.ticket, self.workers_log))
            if isinstance(proof, Ineligible):
                problems.extend(proof.reasons)
        return list(dict.fromkeys(problems))

    def _withdraw(self, journal: Journal, claim: TicketClaim) -> str:
        text = self._marker("withdraw", claim.run_id, claim.generation, self.unit.base)
        body = f"[deepagents run={claim.run_id} gen={claim.generation}]\n\n持有 post-check 未通过，撤回持有标记。\n\n{text}"
        try:
            self.gateway.remove_label(self.unit.ticket, MIRROR_LABEL)
            journal.converge("withdraw", lambda: self.gateway.find_marker(self.unit.ticket, text),
                             lambda: self.gateway.post_comment(self.unit.ticket, body), idempotent=False,
                             what={"kind": "comment", "issue": self.unit.ticket, "marker": text})
        except (IdentityError, EffectPending):
            return "; withdraw_incomplete"
        return ""

    def _boundary_problem(self, pr_number: int | None, run_dir: Path, generation: int) -> str | None:
        issue = self.gateway.issue(self.unit.ticket)
        labels = self.gateway.label_names(issue)
        if issue.get("state") != "open":
            return "ticket_not_open"
        if MIRROR_LABEL not in labels:
            return "mirror_label_removed"
        hits = forbidden_for(self.unit.admission, labels)
        if hits:
            return "forbidden_label:" + ",".join(hits)
        if isinstance(self.unit.admission, Queued):
            try:
                self._queue_check()
            except (CutoverError, ClaimRefused, ValueError, OSError) as error:
                return str(error)
        if pr_number is not None:
            pr_hits = forbidden_for(self.unit.admission, self.gateway.label_names(self.gateway.issue(pr_number)))
            if pr_hits:
                return "pr_forbidden_label:" + ",".join(pr_hits)
        if not isinstance(self.unit.admission, DedicatedPilot):
            pending = self.spec_approval(run_dir, generation)
            if pending is not None:
                if isinstance(self.unit.admission, Queued):
                    raise ApprovalRequired(pending)
                return f"spec_authorization:{pending.status}"
        return None

    @contextmanager
    def action(self, effect: str, run_dir: Path, generation: int, pr_number: int | None = None,
               head: Sha | None = None) -> Iterator[Boundary]:
        """SPEC 6：取票级锁 → 锁内复核持有与实时票面 → 锁内执行并记账 → 释放。复核不符即 halted，不执行。
        派工、本地提交与每一次 GitHub 写都经这里。发布动作带 head，需要候选签批的票在锁内重读权威链。"""
        if effect == "cancel":
            # 取消是撤回既有能力，不要求仍可派工；票级锁仍防止与换代交错。
            with self.claims.locked(self.unit.ticket) as txn:
                claim = txn.record
                if (claim is None or claim.repo != REPO or claim.ticket != self.unit.ticket
                        or claim.run_id != self.unit.run_id or claim.run_dir != str(run_dir.resolve())
                        or claim.generation != generation):
                    raise Fenced("取消身份不符：须为原 run、状态目录与持有代次")
                yield Boundary(txn, claim)
            return
        with self.claims.locked(self.unit.ticket) as txn, self._cutover_lock():
            boundary = Boundary(txn, txn.require_holder(self.unit.run_id, run_dir, generation))
            problem = self._boundary_problem(pr_number, run_dir, generation)
            if problem is not None:
                raise boundary.halt(f"{effect}:{problem}")
            if head is not None and self.unit.requires_candidate_approval:
                pending = self.candidate_approval(run_dir, generation, head)
                if pending is not None:
                    raise ApprovalRequired(pending)
            yield boundary

    def _assert_frozen(self, run: PilotRun, head: Sha, chain: set[str]) -> None:
        checkout = run.spec.checkout_for("author")
        tip = subprocess.run(["git", "-C", str(checkout), "rev-parse", "HEAD"], check=True, capture_output=True,
                             text=True).stdout.strip()
        branch = subprocess.run(["git", "-C", str(checkout), "branch", "--show-current"], check=True,
                                capture_output=True, text=True).stdout.strip()
        if tip != head or branch != run.spec.branch_for("author"):
            raise ToolRefusal(f"local_tip_drift：冻结 head {head}，作者 checkout 现为 {branch}@{tip}")
        span = subprocess.run(["git", "-C", str(checkout), "rev-list", f"{self.unit.base}..{head}"], check=True,
                              capture_output=True, text=True).stdout.split()
        if set(span) != chain:
            raise ToolRefusal("foreign_commits_in_range：基线到候选之间含非本 run 回执的提交")

    def _require_ref(self, head: Sha) -> None:
        remote = self.gateway.get_ref(self.unit.branch)
        if remote != head:
            raise Stale(remote or "absent")

    def _ref_probe(self, head: Sha) -> dict[str, Any] | None:
        remote = self.gateway.get_ref(self.unit.branch)
        if remote is None:
            return None
        if remote != head:
            raise Stale(remote)
        return {"branch": self.unit.branch, "sha": head}


    def _pr_problems(self, pr: dict[str, Any]) -> list[str]:
        """本票交付合同：本 App 开的、本票分支到 main、open 时必须是 draft、正文带本票 PR 标记。不看 head SHA。"""
        head, base, user = pr.get("head") or {}, pr.get("base") or {}, pr.get("user") or {}
        problems = []
        if head.get("ref") != self.unit.branch or (head.get("repo") or {}).get("full_name") != self.gateway.repo:
            problems.append("head_branch")
        if base.get("ref") != "main":
            problems.append("base_not_main")
        if user.get("login") != self.gateway.bot or user.get("type") != "Bot":
            problems.append("not_opened_by_app")
        if f"<!-- deepagents:pr ticket={self.unit.ticket} " not in (pr.get("body") or ""):
            problems.append("missing_pr_marker")
        if pr.get("state") == "open" and pr.get("draft") is not True:
            problems.append("not_draft")
        return problems

    def _judge_pr(self, pr: dict[str, Any], head: Sha) -> str:
        """返回 open 或 merged；合同不符或已关闭抛 PullRequestRefused，head 不是冻结 h 抛 Stale。"""
        problems = self._pr_problems(pr)
        if problems:
            raise PullRequestRefused(f"pr_contract:{pr.get('number')}:" + ",".join(problems))
        if pr.get("state") != "open" and not pr.get("merged"):
            raise PullRequestRefused(f"closed_pull_request:{pr.get('number')}")
        if (pr.get("head") or {}).get("sha") != head:
            raise Stale(str((pr.get("head") or {}).get("sha")))
        if f" head={head} -->" not in (pr.get("body") or ""):
            raise PullRequestRefused(f"pr_contract:{pr.get('number')}:marker_head")
        return "merged" if pr.get("merged") else "open"

    def _pr_summary(self, pr: dict[str, Any]) -> dict[str, Any]:
        return {"number": pr["number"], "url": pr.get("html_url"), "head": pr["head"]["sha"]}

    def _pr_probe(self, head: Sha) -> dict[str, Any] | None:
        prs = [p for p in self.gateway.pulls_for(self.unit.branch) if (p.get("head") or {}).get("ref") == self.unit.branch]
        if not prs:
            return None
        live = [p for p in prs if p.get("state") == "open"]
        if len(live) > 1:
            raise PullRequestRefused("multiple_open_pull_requests")
        chosen = live[0] if live else max(prs, key=lambda p: p["number"])
        self._judge_pr(chosen, head)
        return self._pr_summary(chosen)

    def _pr_body(self, run: PilotRun, head: Sha, generation: int) -> str:
        spec = run.spec
        return (f"Refs #{self.unit.ticket}\n\n新执行器交付冻结 head `{head}`。作者 {spec.author_model}，"
                f"异家族独审 {spec.reviewer_model} 对该 head 判 PASS。合入 main 由人决定。\n\n"
                + self._marker("pr", spec.run_id, generation, head))

    def _verify(self, head: Sha, number: int) -> str:
        """每个后续写之前与交付之后：PR 仍合同、head 仍是 h、远端分支仍是 h。缓存的回执不代替这一步。"""
        observed = self._judge_pr(self.gateway.get_pr(number), head)
        if observed == "open":
            self._require_ref(head)
        return observed

    def reconcile_delivery(self, run_dir: Path) -> dict[str, Any]:
        fd = lock_run_dir(run_dir)
        try:
            with self.claims.locked(self.unit.ticket) as txn:
                claim = txn.record
                if claim is None:
                    raise Fenced("交付对账缺少本机持有记录")
                txn.require_holder(self.unit.run_id, run_dir, claim.generation)
                delivery = delivery_status(run_dir)
                if delivery["phase"] == "stale":
                    return delivery
                state = delivery["state"]
                head = parse_sha(state["head"])
                try:
                    observed = self._checked(Boundary(txn, claim), head, int(state["pr"]["number"]))
                except Stale as stale:
                    _write_json(run_dir / "delivery" / "stale.json",
                                {"head": head, "observed": stale.observed, "at": _now()})
                else:
                    phase = "merged" if observed == "merged" else "awaiting_merge"
                    if state["phase"] != phase:
                        _write_json(run_dir / "delivery" / "state.json", {**state, "phase": phase, "at": _now()})
                return delivery_status(run_dir)
        finally:
            os.close(fd)

    def _status_probe(self, head: Sha) -> dict[str, Any] | None:
        for status in self.gateway.statuses(head):
            if (status.get("context") == STATUS_CONTEXT and status.get("state") == "success"
                    and (status.get("creator") or {}).get("login") == self.gateway.bot):
                return {"id": status.get("id"), "context": STATUS_CONTEXT, "sha": head}
        return None

    def publish(self, run: PilotRun) -> dict[str, Any]:
        """SPEC 6：只发布同 head 异家族 PASS 的冻结 SHA。远端任何其他 head（含后代）都 stale；
        每一步写之前都重新核远端 ref 与 PR，发现漂移先 stale，不先写状态或交付。"""
        facts = run.facts()
        head = current_head(facts)
        refusal = publish_refusal(facts)
        if refusal is not None:
            run.trail("publish_refused", head=head, reason=refusal)
            return {"ok": False, "decision": "refused", "head": head, "reasons": [refusal]}
        generation = run.generation
        journal = Journal(run.dir / "delivery")
        chain: set[str] = {c.commit for c in facts.commits}
        spec = run.spec
        branch = self.unit.branch
        try:
            with self.action("push", run.dir, generation, head=head):
                self._assert_frozen(run, head, chain)
                checkout = run.spec.checkout_for("author")
                journal.converge(f"push-{head}", lambda: self._ref_probe(head),
                                 lambda: self.gateway.create_ref(branch, head), idempotent=True,
                                 what={"kind": "ref", "branch": branch, "sha": head},
                                 prepare=lambda: ExactUpload(self.gateway, checkout, self.unit.base).upload(head),
                                 crash=self.crash)
            with self.action("pr", run.dir, generation, head=head) as boundary:
                try:
                    if not journal.done(f"pr-{head}"):
                        self._require_ref(head)
                    pr = journal.converge(f"pr-{head}", lambda: self._pr_probe(head),
                                          lambda: self.gateway.create_pr(branch, f"#{self.unit.ticket} {spec.task.title}",
                                                                         self._pr_body(run, head, generation)),
                                          idempotent=False,
                                          what={"kind": "pull_request", "head_branch": branch, "head": head,
                                                "base": "main"}, crash=self.crash)
                    number = int(pr["number"])
                    observed = self._verify(head, number)
                except PullRequestRefused as refused:
                    raise boundary.halt(f"pr:{refused}") from None
            if observed == "open":
                with self.action("status", run.dir, generation, pr_number=number, head=head) as boundary:
                    observed = self._checked(boundary, head, number)
                    if observed == "open":
                        journal.converge(
                            f"status-{head}", lambda: self._status_probe(head),
                            lambda: self.gateway.create_status(head, STATUS_CONTEXT, f"异家族独审 PASS：作者 {spec.author_model}，独审 {spec.reviewer_model}"),
                            idempotent=False, what={"kind": "status", "sha": head, "context": STATUS_CONTEXT},
                            crash=self.crash)
            if observed == "open":
                text = self._marker("deliver", spec.run_id, generation, head)
                body = (f"[deepagents run={spec.run_id} gen={generation}]\n\n交付冻结 head `{head}` 到 "
                        f"`{branch}`，PR {pr.get('url')}，等待人合入。\n\n{text}")
                with self.action("deliver", run.dir, generation, pr_number=number, head=head) as boundary:
                    observed = self._checked(boundary, head, number)
                    if observed == "open":
                        journal.converge(f"deliver-{head}", lambda: self.gateway.find_marker(self.unit.ticket, text),
                                         lambda: self.gateway.post_comment(self.unit.ticket, body), idempotent=False,
                                         what={"kind": "comment", "issue": self.unit.ticket, "marker": text},
                                         crash=self.crash)
                        observed = self._checked(boundary, head, number)
        except ApprovalRequired as required:
            run.trail("publish_awaiting_approval", head=head, **decision_json(required.decision))
            return {"ok": False, "decision": "awaiting_approval", "head": head,
                    "approval": decision_json(required.decision)}
        except Stale as stale:
            _write_json(run.dir / "delivery" / "stale.json", {"head": head, "observed": stale.observed, "at": _now()})
            run.trail("publish_stale", head=head, observed=stale.observed)
            return {"ok": False, "decision": "stale", "head": head, "observed": stale.observed}
        state = {"phase": "merged" if observed == "merged" else "awaiting_merge", "head": head, "branch": branch,
                 "pr": pr, "at": _now()}
        (run.dir / "delivery" / "stale.json").unlink(missing_ok=True)
        _write_json(run.dir / "delivery" / "state.json", state)
        run.trail("published", head=head, pr=pr.get("url"), observed=observed)
        return {"ok": True, **state}

    def _checked(self, boundary: Boundary, head: Sha, number: int) -> str:
        try:
            return self._verify(head, number)
        except PullRequestRefused as refused:
            raise boundary.halt(f"pr:{refused}") from None


def claim_effects(store: ClaimStore, ticket: int, run_id: str, generation: int) -> Path:
    return store.root / f"{ticket}.effects" / f"{run_id}-g{generation}"


def release_claim(store: ClaimStore, ticket: int, run_id: str, reason: str) -> TicketClaim:
    """主控释放。等在飞副作用做完才拿到锁；旧控制进程此后在下一次锁内复核时停下。"""
    with store.locked(ticket) as txn:
        record = txn.record
        if record is None or record.run_id != run_id:
            raise ClaimRefused(f"票 #{ticket} 当前不由 {run_id} 持有")
        return txn.write(advance(record, "release", reason=reason))


def reconcile_effect(state_root: Path, store: ClaimStore, run_id: str, effect: str, note: str) -> dict[str, Any]:
    """主控在 GitHub 上核实某个 pending 效果确实没有生效后，允许它再发一次。

    整个对账持有该 run 的控制锁，锁序与控制进程相同（先 run 锁、后票级锁），所以对账期间控制进程起不来，
    控制进程在时对账直接 Busy。"""
    unit, _task = load_unit(state_root, run_id)
    run_dir = PilotRun.run_dir(state_root, unit.run_id)
    fd = lock_run_dir(run_dir)
    try:
        folders = [run_dir / "delivery", *sorted((store.root / f"{unit.ticket}.effects").glob(f"{unit.run_id}-g*"))]
        matches = [folder for folder in folders if (folder / f"{effect}.json").exists()]
        if len(matches) != 1:
            raise EffectPending(f"找不到唯一的效果记录 {effect}：{[str(m) for m in matches]}")
        with store.locked(unit.ticket):
            return Journal(matches[0]).reconcile_not_applied(effect, note)
    finally:
        os.close(fd)


def unit_path(state_root: Path, run_id: str) -> Path:
    return state_root / "units" / f"{run_id}.json"


def _task_json(task: TaskSpec) -> dict[str, Any]:
    return {"title": task.title, "requirement": task.requirement, "allowed_prefixes": list(task.allowed_prefixes),
            "require_design": task.require_design}


def register(state_root: Path, gateway: RepoGateway, *, ticket: int, run_id: str, base: str, task: TaskSpec,
             playbook: Playbook, scope_ref: str, workers_log: Path, source_repo: Path,
             candidate_approval: bool = False) -> WorkUnit:
    """登记器：主控 CLI 专用，不在 Deep Agents 工具面里。只生成首证专用票的 WorkUnit。"""
    run = parse_unit_run_id(ticket, run_id)
    sha = parse_sha(base)
    if subprocess.run(["git", "-C", str(source_repo), "cat-file", "-e", f"{sha}^{{commit}}"],
                      check=False, capture_output=True).returncode != 0:
        raise AdmissionRefused(f"本地没有基线 {sha}")
    if not gateway.commit_exists(sha):
        raise AdmissionRefused(f"远端没有基线 {sha}")
    proof = judge_eligibility(gather_evidence(gateway, ticket, workers_log))
    if isinstance(proof, Ineligible):
        raise AdmissionRefused("专用票准入不成立：" + ", ".join(proof.reasons))
    unit = WorkUnit(ticket, None, sha, task.allowed_prefixes, playbook, task.require_design,
                    DedicatedPilot(proof, scope_ref), run, candidate_approval)
    record = {"work_unit": unit.to_json(), "task": _task_json(task)}
    path = unit_path(state_root, run)
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        if json.loads(path.read_text()) != record:
            raise AdmissionRefused(f"run {run} 已登记为不同的 WorkUnit") from None
        return unit
    with os.fdopen(fd, "w") as fh:
        fh.write(json.dumps(record, ensure_ascii=False, indent=2) + "\n")
    return unit


def load_unit(state_root: Path, run_id: str) -> tuple[WorkUnit, TaskSpec]:
    data = json.loads(unit_path(state_root, run_id).read_text())
    raw = data["task"]
    task = TaskSpec(raw["title"], raw["requirement"], tuple(raw["allowed_prefixes"]), bool(raw["require_design"]))
    return parse_work_unit(data["work_unit"]), task


def open_v2_run(state_root: Path, unit: WorkUnit, task: TaskSpec, executor: TicketExecutor, models_text: str,
                **kw: Any) -> PilotRun:
    if (PilotRun.run_dir(state_root, unit.run_id) / "run.json").exists():
        run = PilotRun.open(state_root, unit.run_id, executor=executor, **kw)
        if run.spec.unit != unit:
            raise Halted("已冻结 run 的 WorkUnit 与登记不同")
        run.acquire()
        try:
            run.bind_claim()
            run.ensure_checkouts()
        except BaseException:
            run.release()
            raise
        return run
    return PilotRun.create_v2(state_root, unit, task, models_text, executor, **kw)


def _sigkill_at(point: str | None) -> Callable[[str], None]:
    def crash(at: str) -> None:
        if at == point:
            os.kill(os.getpid(), signal.SIGKILL)
    return crash


def _client(args: argparse.Namespace) -> GitHubAppClient:
    """登记只要读权限；run 才申请写权限。两者都先核 App、安装与仓库身份。"""
    identity = AppIdentity(5486877, 175617978, 1150335160, REPO, "newchanlun-deepagents-1742", args.private_key)
    client = GitHubAppClient(identity, permissions=WRITE_PERMISSIONS if args.command == "run" else None)
    client.verify()
    return client


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="pilot1742.executor", description="新执行器：v2 票登记、引擎队列、持有、运行与释放")
    parser.add_argument("--state-root", type=Path, default=DEFAULT_STATE_ROOT)
    parser.add_argument("--drain-root", type=Path, default=DRAIN_ROOT, help="隔离测试用；部署保持缺省全局根")
    sub = parser.add_subparsers(dest="command", required=True)
    reg = sub.add_parser("register", help="主控登记首证专用票（只读 GitHub，写本机 WorkUnit）")
    reg.add_argument("--ticket", type=int, required=True)
    reg.add_argument("--run-id", required=True)
    reg.add_argument("--base", required=True)
    reg.add_argument("--task", type=Path, required=True)
    reg.add_argument("--playbook", choices=("feature", "bug-fix", "refactoring"), required=True)
    reg.add_argument("--scope-ref", required=True, help="用户批准本 SPEC 的记录引用，只作引用")
    reg.add_argument("--workers-log", type=Path, default=WORKERS_LOG)
    reg.add_argument("--require-candidate-approval", action="store_true",
                     help="发布前要求对确切候选 head 的真人 Secure Enclave 签批")
    reg.add_argument("--private-key", type=Path, required=True)
    run_parser = sub.add_parser("run", help="持有并运行或恢复一个已登记的 v2 run")
    run_parser.add_argument("--run-id", required=True)
    run_parser.add_argument("--workers-log", type=Path, default=WORKERS_LOG)
    run_parser.add_argument("--private-key", type=Path, required=True)
    run_parser.add_argument("--crash-at", choices=V2_CRASH_POINTS, help="受控故障注入：在该点对本进程 SIGKILL")
    run_parser.add_argument("--resume", action="store_true",
                     help="停在中断处时恢复：清除操作员中断请求，按权威签批链重新判定，恢复载荷不作批准")
    serve = sub.add_parser("serve", help="读取引擎放行队列，逐票运行或恢复")
    modes = serve.add_mutually_exclusive_group(required=True)
    modes.add_argument("--once", action="store_true")
    modes.add_argument("--loop", action="store_true")
    serve.add_argument("--poll-seconds", type=float, default=60)
    serve.add_argument("--workers-log", type=Path, default=WORKERS_LOG)
    serve.add_argument("--private-key", type=Path, required=True)
    cutover = sub.add_parser("cutover", help="全局冻结排空证据；不执行生产切换")
    cutover.add_argument("operation", choices=("snapshot", "pending", "audit-release", "verify", "record"))
    cutover.add_argument("--snapshot", type=Path)
    cutover.add_argument("--request", type=Path, help="审计释放请求；只用于 audit-release")
    cutover.add_argument("--workers-log", type=Path, default=WORKERS_LOG)
    cutover.add_argument("--private-key", type=Path)
    status = sub.add_parser("status", help="只读本机状态，不访问 GitHub")
    status.add_argument("--run-id", required=True)
    cancel = sub.add_parser("cancel-worker", help="在票级动作锁下取消已有 Boat 工蜂绑定，不重派")
    cancel.add_argument("--run-id", required=True)
    cancel.add_argument("--role", required=True)
    cancel.add_argument("--head", required=True)
    cancel.add_argument("--private-key", type=Path, required=True)
    reconcile = sub.add_parser("reconcile", help="主控核实 pending 效果在 GitHub 上没有生效后，允许再发一次")
    reconcile.add_argument("--run-id", required=True)
    reconcile.add_argument("--effect", required=True, help="效果名，如 deliver-<head>、status-<head>、claim")
    reconcile.add_argument("--note", required=True, help="核实依据，写进记录")
    release = sub.add_parser("release", help="主控释放持有（等在飞副作用结束）")
    release.add_argument("--ticket", type=int, required=True)
    release.add_argument("--run-id", required=True)
    release.add_argument("--reason", required=True)
    args = parser.parse_args(argv)
    store = ClaimStore()
    try:
        if args.command == "release":
            print(json.dumps(asdict(release_claim(store, args.ticket, args.run_id, args.reason)), ensure_ascii=False))
            return 0
        if args.command == "reconcile":
            try:
                result = reconcile_effect(args.state_root, store, args.run_id, args.effect, args.note)
            except Busy as error:
                print(str(error), file=sys.stderr)
                return 75
            print(json.dumps(result, ensure_ascii=False, indent=2))
            return 0
        if args.command == "status":
            unit, _task = load_unit(args.state_root, args.run_id)
            record = store.read(unit.ticket)
            report = PilotRun.open(args.state_root, unit.run_id).status()
            print(json.dumps({"claim": asdict(record) if record else None, "status": report}, ensure_ascii=False,
                             indent=2))
            return 0
        source_repo = Path(__file__).resolve().parents[3]
        cutover_store = CutoverStore(args.state_root, args.drain_root, getattr(args, "workers_log", WORKERS_LOG))
        if args.command == "serve":
            if args.poll_seconds <= 0:
                raise ValueError("poll-seconds 须为正数")
            try:
                cutover_store.load()
            except CutoverError as error:
                print(f"queue inactive: {error}")
                return 0
        if args.command == "cutover":
            snapshot_path = args.snapshot or args.state_root / "cutover-snapshot.json"
            if args.operation == "pending":
                from .audit_release import pending_releases
                print(json.dumps(pending_releases(cutover_store), ensure_ascii=False, indent=2))
                return 0
            if args.operation == "audit-release" and args.request is None:
                raise ValueError("audit-release 需要 --request 显式路径")
            if args.operation != "audit-release" and args.request is not None:
                raise ValueError("--request 只用于 audit-release")
            if args.operation == "snapshot":
                snapshot = cutover_store.snapshot()
                snapshot_path.parent.mkdir(parents=True, exist_ok=True)
                fd = os.open(snapshot_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
                with os.fdopen(fd, "w") as stream:
                    json.dump(asdict(snapshot), stream, ensure_ascii=False, indent=2)
                    stream.write("\n")
                print(str(snapshot_path))
                return 0
            if args.private_key is None:
                raise ValueError("cutover verify/record/audit-release 需要 --private-key 显式路径")
        with _client(args) as client:
            gateway = RepoGateway(client)
            if args.command == "cutover":
                data = json.loads(snapshot_path.read_text())
                data["legacy_snapshot"] = tuple(LegacyProcess(**p) for p in data["legacy_snapshot"])
                if args.operation == "audit-release":
                    from .audit_release import audit_release
                    audit_record = audit_release(cutover_store, FrozenSnapshot(**data),
                                                 json.loads(args.request.read_text()), gateway)
                    print(json.dumps(audit_record, ensure_ascii=False, indent=2))
                    return 0
                head = subprocess.run(["git", "-C", str(source_repo), "rev-parse", "HEAD"], check=True,
                                      capture_output=True, text=True).stdout.strip()
                with (cutover_store.drain_root / "admission.lock").open("a") as lock:
                    fcntl.flock(lock, fcntl.LOCK_SH)
                    cutover_record = cutover_store.verify(FrozenSnapshot(**data), remote_barrier(gateway), legacy_processes(), head)
                    if args.operation == "record":
                        cutover_store.record(cutover_record)
                print(json.dumps(asdict(cutover_record), ensure_ascii=False, indent=2))
                return 0
            if args.command == "serve":
                from .queue import QueueIntake
                intake = QueueIntake(args.state_root, gateway, store, cutover_store, source_repo)
                def child(unit: WorkUnit) -> int:
                    command = [sys.executable, "-m", "pilot1742.executor", "--state-root", str(args.state_root),
                               "--drain-root", str(args.drain_root), "run", "--run-id", unit.run_id,
                               "--workers-log", str(args.workers_log), "--private-key", str(args.private_key), "--resume"]
                    return subprocess.run(command, check=False).returncode
                while True:
                    try:
                        cutover_store.load()
                    except CutoverError as error:
                        print(f"queue inactive: {error}")
                        return 0
                    print(json.dumps(intake.once(child), ensure_ascii=False, indent=2), flush=True)
                    if args.once:
                        return 0
                    time.sleep(args.poll_seconds)
            if args.command == "register":
                unit = register(args.state_root, gateway, ticket=args.ticket, run_id=args.run_id, base=args.base,
                                task=parse_task(args.task.read_text()), playbook=args.playbook,
                                scope_ref=args.scope_ref, workers_log=args.workers_log,
                                source_repo=Path(__file__).resolve().parents[3],
                                candidate_approval=args.require_candidate_approval)
                print(json.dumps(unit.to_json(), ensure_ascii=False, indent=2))
                return 0
            from .cli import EXIT_BUSY, cancel_worker, drive
            from .run import PSTACK_MODELS

            unit, task = load_unit(args.state_root, args.run_id)
            if args.command == "cancel-worker":
                run = PilotRun.open(args.state_root, unit.run_id, executor=TicketExecutor(
                    unit, gateway, claims=store, cutover=cutover_store))
                if run.execution is None:
                    raise ToolRefusal("该run不是Boat绑定，不能经此入口取消")
                return cancel_worker(run, args.role, args.head)
            executor = TicketExecutor(unit, gateway, claims=store, workers_log=args.workers_log,
                                      crash=_sigkill_at(args.crash_at), cutover=cutover_store)
            try:
                pilot = open_v2_run(args.state_root, unit, task, executor, PSTACK_MODELS.read_text())
            except Busy as error:
                print(str(error), file=sys.stderr)
                return EXIT_BUSY
            return drive(pilot, recheck_approval=args.resume)
    except ApprovalRequired as required:
        print(json.dumps({"phase": "awaiting_human", "approval": decision_json(required.decision)}, ensure_ascii=False))
        return 0
    except (*EXECUTOR_ERRORS, Halted, ToolRefusal, ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"{type(error).__name__}: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
