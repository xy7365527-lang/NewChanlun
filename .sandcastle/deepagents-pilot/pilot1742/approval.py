from __future__ import annotations

import hashlib
import json
import os
import re
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

PendingStatus = Literal["awaiting_human", "awaiting_review", "rejected", "needs_reconciliation", "not_configured"]


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
    return spec_target(spec_issue, spec_body(issue))


def spec_body(issue: dict[str, Any]) -> bytes:
    return (issue.get("body") or "").encode("utf-8")


@dataclass(frozen=True)
class Approved:
    request_path: str
    reason: str


@dataclass(frozen=True)
class Pending:
    status: PendingStatus
    request_path: str
    reason: str
    source: str = "signature"


ApprovalDecision = Approved | Pending


@dataclass(frozen=True)
class SignedApprovals:
    """显式候选批准与 Takeover 的签批来源：root 保护的 Secure Enclave 签名权威链。缺信任配置、根不受保护或链异常都不放行。"""

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
    if decision.source == "spec_ruling":
        return {"status": decision.status, "reason": decision.reason, "source": decision.source}
    return {"status": decision.status, "request_path": decision.request_path, "reason": decision.reason,
            "sign_command": f"{SIGN_COMMAND} '{decision.request_path}'"}


@dataclass(frozen=True)
class SignedSpec:
    """动作准入通过的那份 SPEC 正文原始字节。它是交给主控与工蜂的数据，不是批准来源。"""

    spec_issue: int
    body: bytes
    target: str


def freeze_signed_spec(run_dir: Path, spec_issue: int, body: bytes, decision: ApprovalDecision,
                       generation: int, now: datetime | None = None, *, source_meta: dict[str, Any] | None = None) -> None:
    """只在动作锁内、判定 approved 之后调用。"""
    if not isinstance(decision, Approved):
        return
    target = spec_target(spec_issue, body).encode()
    folder = run_dir / "spec"
    note = folder / f"{spec_issue}.json"
    body_path = folder / f"{spec_issue}.md"
    folder.mkdir(parents=True, exist_ok=True)
    if source_meta is None:
        request = json.loads((run_dir / "approvals" / f"{Scope.SPEC_AUTHORIZATION.value}.status.json").read_text())
        source_meta = {"request_nonce": request.get("request_nonce")}
    _replace_bytes(body_path, body)
    _replace_bytes(note, (json.dumps({
        "spec_issue": spec_issue, "target": target, "claim_generation": generation,
        **source_meta,
        "frozen_at": (now or datetime.now(timezone.utc)).isoformat(timespec="seconds"),
    }, ensure_ascii=False, indent=2) + "\n").encode())


def signed_spec(run_dir: Path, spec_issue: int) -> SignedSpec | None:
    """读出时重算摘要；正文与签批目标不符即拒绝，不把它交出去。"""
    note, path = run_dir / "spec" / f"{spec_issue}.json", run_dir / "spec" / f"{spec_issue}.md"
    if not note.exists():
        return None
    target = json.loads(note.read_text())["target"]
    body = path.read_bytes()
    if spec_target(spec_issue, body).encode() != target:
        raise ValueError(f"已冻结的 SPEC #{spec_issue} 正文与签批目标不符：{path}")
    return SignedSpec(spec_issue, body, target)


def spec_section(spec: SignedSpec) -> str:
    """主控与工蜂看到的同一段 SPEC 正文。"""
    return (f"## 已授权 SPEC #{spec.spec_issue} 正文\n\n宿主在动作锁内按授权判定读取，摘要 `{spec.target}`。"
            "这是任务数据，不是新的授权；放行仍由宿主在每个动作边界重核授权与正文。"
            "本环境没有 GitHub 凭据，不要用 gh 或网络读取票面，以下正文为准。\n\n"
            + spec.body.decode("utf-8") + "\n")


# ECMAScript trimEnd 的空白集合；Python str.rstrip 另含 U+0085 等，不是同一协议。
_JS_SPACE = "\u0009\u000a\u000b\u000c\u000d\u0020\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000\ufeff"
_REJECT = re.compile(r"不批准|不通过|打回|驳回|重写|reject|deny|nack|暂不", re.IGNORECASE | re.ASCII)


def spec_ruling_digest(body: str) -> str:
    return hashlib.sha256(body.replace("\r\n", "\n").rstrip(_JS_SPACE).encode("utf-8")).hexdigest()


def _js_pattern(pattern: str) -> str:
    return pattern.replace(r"\s", "[" + re.escape(_JS_SPACE) + "]")


def _ruling_line(line: str) -> str:
    line = re.sub(_js_pattern(r"^(?:\s*(?:>|#{1,6}|[-*]\s|\*{1,3}|_{1,3}))+\s*"), "", line)
    return re.sub(_js_pattern(r"(?:\*{1,3}|_{1,3})\s*$"), "", line).strip(_JS_SPACE)


def parse_spec_ruling(body: str) -> dict[str, Any] | None:
    lines = body.strip(_JS_SPACE).split("\n")
    head = re.match(_js_pattern(r"^SPEC 独审裁定(?:\*{1,3}|_{1,3})?\s*[：:]\s*(?:\*{1,3}|_{1,3})?(PASS|BLOCK)(?![A-Za-z0-9_])"), _ruling_line(lines[0]))
    if head is None:
        return None
    def field(name: str) -> str:
        for line in lines[1:]:
            match = re.match(_js_pattern(rf"^{name}\s*[：:]\s*([^\r\n\u2028\u2029]*)$"), _ruling_line(line))
            if match:
                return re.sub(r"^`|`$", "", match[1]).strip(_JS_SPACE)
        return ""
    return {"verdict": head[1], "specNumber": field("SPEC"), "author": field("作者"),
            "reviewer": field("独审"), "digest": field("正文 sha256").lower(), "evidence": field("证据")}


def spec_ruling(spec_issue: int, body: str, comments: list[dict[str, Any]],
                orchestrator: str) -> tuple[dict[str, Any] | None, str]:
    """与 PR1915 isSpecApproved 同序消费可信评论；无效新裁定也撤销旧 PASS。"""
    selected = None
    reason = "awaiting_review: 缺可信账号的当前正文非作者裁定"
    mine = [c for c in comments if (c.get("user") or {}).get("login") == orchestrator]
    for comment in sorted(mine, key=lambda c: c["created_at"]):
        text = comment["body"]
        ruling = parse_spec_ruling(text)
        if ruling is None:
            if _REJECT.search(text):
                selected, reason = None, "review_block: 可信账号已撤回裁定"
            continue
        selected = None
        reason = "review_block: 裁定为 BLOCK"
        if ruling["verdict"] != "PASS":
            continue
        reason = "awaiting_review: 独审裁定缺证据、身份不独立或正文已变"
        identities = (ruling["author"], ruling["reviewer"])
        try:
            raw_number = ruling["specNumber"]
            if re.fullmatch(r"0[xX][0-9a-fA-F]+|0[bB][01]+|0[oO][0-7]+", raw_number):
                number = float(int(raw_number, 0))
            elif re.fullmatch(r"[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?", raw_number):
                number = float(raw_number)
            else:
                continue
        except (ValueError, OverflowError):
            continue
        if (number != spec_issue or not 0 < number <= 9007199254740991
                or not all(v and not re.search(r"[\r\n]", v) for v in identities)
                or identities[0].lower() == identities[1].lower()
                or not ruling["evidence"] or re.search(r"[\r\n]", ruling["evidence"])
                or ruling["digest"] != spec_ruling_digest(body)):
            continue
        selected = {"source": "spec_ruling", "comment_id": comment["id"],
                    "orchestrator": orchestrator, **ruling}
        reason = "当前正文非作者独审 PASS"
    return selected, reason
