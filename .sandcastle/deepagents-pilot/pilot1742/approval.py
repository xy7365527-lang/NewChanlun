from __future__ import annotations

import json
import os
import secrets
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Literal

from approval_signing.authority import AUTHORITY_ROOT
from approval_signing.decision import Status, decide
from approval_signing.protocol import (
    DEFAULT_TRUST_PATH,
    ApprovalRequest,
    CommitTarget,
    ProtocolError,
    Scope,
    Target,
    spec_target,
)

REPOSITORY_ID = 1150335160
REQUEST_TTL = timedelta(hours=24)
SIGN_COMMAND = '"/Library/Application Support/NewChanlun/newchanlun-approve" sign --request'

PendingStatus = Literal["awaiting_human", "rejected", "needs_reconciliation", "not_configured"]


@dataclass(frozen=True)
class SpecAuthorization:
    """接管票所属 SPEC。目标在每次动作边界按 GitHub 当前正文的原始 UTF-8 字节重新计算。"""

    spec_issue: int

    def to_json(self) -> dict[str, Any]:
        return {"spec_issue": self.spec_issue}

    @staticmethod
    def from_json(data: dict[str, Any]) -> SpecAuthorization:
        issue = data.get("spec_issue")
        if type(issue) is not int or issue <= 0 or set(data) != {"spec_issue"}:
            raise ValueError("spec_auth 须为 {spec_issue: 正整数}")
        return SpecAuthorization(issue)


def spec_body_target(spec_issue: int, issue: dict[str, Any]) -> Target:
    return spec_target(spec_issue, (issue.get("body") or "").encode("utf-8"))


@dataclass(frozen=True)
class Approved:
    request_path: str
    reason: str


@dataclass(frozen=True)
class Pending:
    status: PendingStatus
    request_path: str
    reason: str


ApprovalDecision = Approved | Pending


@dataclass(frozen=True)
class SignedApprovals:
    """唯一的批准来源：root 保护的 Secure Enclave 签名权威链。缺信任配置、根不受保护或链异常都不放行。"""

    trust_path: Path = DEFAULT_TRUST_PATH
    authority_root: Path = AUTHORITY_ROOT

    def evaluate(self, request: ApprovalRequest, current_target: Target,
                 now: datetime) -> tuple[PendingStatus | None, str]:
        try:
            result = decide(request, current_target, now=now, trust_path=self.trust_path,
                            authority_root=self.authority_root)
        except (OSError, ProtocolError) as exc:
            return "not_configured", f"签批信任配置不可用：{exc}"
        statuses: dict[Status, PendingStatus | None] = {
            Status.APPROVED: None, Status.REJECTED: "rejected", Status.AWAITING_HUMAN: "awaiting_human",
            Status.NEEDS_RECONCILIATION: "needs_reconciliation"}
        return statuses[result.status], result.reason


@dataclass
class ApprovalDesk:
    """一个 run 的待批请求。请求按规范字节存在 approvals/<scope>.request，签批 CLI 签的就是这份字节。

    目标、代次或期限任一变化都换新 nonce 重发，旧请求移入 history，对旧请求的批准不再放行任何动作。"""

    folder: Path
    ticket: int
    run_id: str
    approvals: SignedApprovals = field(default_factory=SignedApprovals)

    def path(self, scope: Scope) -> Path:
        return self.folder / f"{scope.value}.request"

    def _stored(self, scope: Scope) -> ApprovalRequest | None:
        try:
            return ApprovalRequest.parse(self.path(scope).read_bytes())
        except (FileNotFoundError, ProtocolError):
            return None

    def _issue(self, scope: Scope, generation: int, target: Target, now: datetime) -> ApprovalRequest:
        request = ApprovalRequest(REPOSITORY_ID, self.ticket, self.run_id, generation, scope, target,
                                  secrets.token_hex(16), (now + REQUEST_TTL).replace(microsecond=0))
        path = self.path(scope)
        self.folder.mkdir(parents=True, exist_ok=True)
        if path.exists():
            history = self.folder / "history"
            history.mkdir(exist_ok=True)
            os.replace(path, history / f"{scope.value}-{secrets.token_hex(8)}.request")
        _replace_bytes(path, request.encode())
        return request

    def check(self, scope: Scope, generation: int, target: Target, *, now: datetime | None = None) -> ApprovalDecision:
        now = now or datetime.now(timezone.utc)
        request = self._stored(scope)
        if (request is None or request.repository_id != REPOSITORY_ID or request.ticket != self.ticket
                or request.run_id != self.run_id or request.scope is not scope
                or request.claim_generation != generation or request.target != target or now >= request.expires_at):
            request = self._issue(scope, generation, target, now)
        status, reason = self.approvals.evaluate(request, target, now)
        path = str(self.path(scope))
        _replace_bytes(self.folder / f"{scope.value}.status.json", (json.dumps({
            "scope": scope.value, "status": status or "approved", "reason": reason,
            "request_nonce": request.request_nonce, "target": request.target.encode(),
            "claim_generation": generation, "expires_at": request.expires_at.isoformat(),
            "checked_at": now.isoformat(), "request_path": path, "sign_command": f"{SIGN_COMMAND} '{path}'",
        }, ensure_ascii=False, indent=2) + "\n").encode())
        return Approved(path, reason) if status is None else Pending(status, path, reason)


def _replace_bytes(path: Path, data: bytes) -> None:
    tmp = path.with_name(f"{path.name}.{os.getpid()}.tmp")
    tmp.write_bytes(data)
    os.replace(tmp, path)


def candidate_target(head: str) -> Target:
    return CommitTarget(head)


def pending_view(folder: Path) -> list[dict[str, Any]]:
    """只读：每个 scope 最近一次判定。供 status 与 Studio 展示，不是批准来源。"""
    return [json.loads(p.read_text()) for p in sorted(folder.glob("*.status.json"))] if folder.exists() else []


def decision_json(decision: ApprovalDecision) -> dict[str, Any]:
    if isinstance(decision, Approved):
        return {"status": "approved", "request_path": decision.request_path, "reason": decision.reason}
    return {"status": decision.status, "request_path": decision.request_path, "reason": decision.reason,
            "sign_command": f"{SIGN_COMMAND} '{decision.request_path}'"}
