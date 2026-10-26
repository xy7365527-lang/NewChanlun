from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal, Protocol

ApprovalScope = Literal["spec_authorization"]


@dataclass(frozen=True)
class SpecTarget:
    spec_issue: int
    spec_revision: str


@dataclass(frozen=True)
class ApprovalRef:
    """只是指针。是否有效每次在动作边界按 SPEC 第 8 节从 GitHub 重新判定。"""

    ticket: int
    target: SpecTarget
    github_object: str
    scope: ApprovalScope = "spec_authorization"
    v: Literal[2] = 2

    def to_json(self) -> dict[str, Any]:
        return {"v": 2, "ticket": self.ticket, "scope": self.scope,
                "target": {"spec_issue": self.target.spec_issue, "spec_revision": self.target.spec_revision},
                "source": {"github_object": self.github_object}}

    @staticmethod
    def from_json(data: dict[str, Any]) -> ApprovalRef:
        if data.get("v") != 2 or data.get("scope") != "spec_authorization":
            raise ValueError("ApprovalRef 须为 v2 spec_authorization")
        target = data["target"]
        return ApprovalRef(int(data["ticket"]), SpecTarget(int(target["spec_issue"]), str(target["spec_revision"])),
                           str(data["source"]["github_object"]))


@dataclass(frozen=True)
class ApprovalObject:
    """GitHub 读回的批准对象。author 取 GitHub 返回值；order 是 GitHub 给出的先后。"""

    object_id: str
    author_login: str
    author_type: str
    ticket: int
    scope: str
    target: SpecTarget
    decision: Literal["approve", "reject"]
    order: int


@dataclass(frozen=True)
class ApprovalValid:
    object_id: str


@dataclass(frozen=True)
class AwaitingHuman:
    reasons: tuple[str, ...]


@dataclass(frozen=True)
class ApprovalRejected:
    reasons: tuple[str, ...]


ApprovalDecision = ApprovalValid | AwaitingHuman | ApprovalRejected


def judge_spec_authorization(ref: ApprovalRef, *, ticket: int, current_target: SpecTarget | None,
                             objects: list[ApprovalObject], approvers: frozenset[str]) -> ApprovalDecision:
    """SPEC 8.1–8.3。集合为空时一律等待人；只认集合内人类账号在 GitHub 上留下的、目标未变的最后一次决定。"""
    if not approvers:
        return AwaitingHuman(("human_exclusive_approvers_empty",))
    if ref.ticket != ticket or ref.scope != "spec_authorization":
        return ApprovalRejected(("ref_for_other_ticket_or_scope",))
    if current_target is None or current_target != ref.target:
        return AwaitingHuman(("target_changed",))
    group = sorted((o for o in objects if o.ticket == ticket and o.scope == ref.scope and o.target == ref.target
                    and o.author_login in approvers and o.author_type == "User"), key=lambda o: o.order)
    if not group:
        return AwaitingHuman(("no_exclusive_approval",))
    latest = group[-1]
    if latest.decision == "reject":
        return ApprovalRejected(("rejected",))
    if latest.object_id != ref.github_object:
        return ApprovalRejected(("ref_points_elsewhere",))
    return ApprovalValid(latest.object_id)


def load_approvers(path: Path) -> frozenset[str]:
    """登记器配置。不存在即空集；bot 身份永远不能入集合。"""
    if not path.exists():
        return frozenset()
    data = json.loads(path.read_text())
    logins = data.get("human_exclusive_approvers")
    if not isinstance(logins, list) or not all(isinstance(x, str) and x for x in logins):
        raise ValueError("human_exclusive_approvers 须为非空字符串列表")
    if any(x.endswith("[bot]") for x in logins):
        raise ValueError("bot 身份不能加入人类独占签批集合")
    return frozenset(logins)


class ApprovalVerifier(Protocol):
    """批准核验的注入边界。执行器只通过它判定 ApprovalRef；具体验证机制（D8）可整体替换。"""

    def verify(self, ref: ApprovalRef, *, ticket: int, current_target: SpecTarget | None) -> ApprovalDecision: ...


class ClosedVerifier:
    """缺省核验器：D8 未决，任何批准都等待人。"""

    def verify(self, ref: ApprovalRef, *, ticket: int, current_target: SpecTarget | None) -> ApprovalDecision:
        return AwaitingHuman(("approval_verifier_not_configured",))


@dataclass(frozen=True)
class ExclusiveAuthorVerifier:
    """r3 第 8 节的作者独占判定。objects 由调用方从 GitHub 读回；集合为空时等同 ClosedVerifier。"""

    objects: tuple[ApprovalObject, ...]
    approvers: frozenset[str]

    def verify(self, ref: ApprovalRef, *, ticket: int, current_target: SpecTarget | None) -> ApprovalDecision:
        return judge_spec_authorization(ref, ticket=ticket, current_target=current_target,
                                        objects=list(self.objects), approvers=self.approvers)
