from __future__ import annotations

import json
import re
from dataclasses import dataclass
from typing import Any, Literal, cast

from .approval import SpecAuthorization
from .domain import RunId, Sha, parse_run_id, parse_sha

REPO = "xy7365527-lang/NewChanlun"
FORBIDDEN_LABELS = frozenset({"sandcastle", "ready-for-agent", "wayfinder:impl", "wayfinder:grilling"})
FORBIDDEN_PREFIX = "agent:"
MIRROR_LABEL = "deepagents"
DEDICATED_MARKER = "deepagents:dedicated-pilot #1742"
PARENT_EVENTS = frozenset({"parent_issue_added", "parent_issue_removed"})

Playbook = Literal["feature", "bug-fix", "refactoring"]


def forbidden(labels: frozenset[str] | set[str]) -> tuple[str, ...]:
    return tuple(sorted(name for name in labels if name in FORBIDDEN_LABELS or name.startswith(FORBIDDEN_PREFIX)))


@dataclass(frozen=True)
class EligibilityProof:
    ticket: int
    labels_ever: tuple[str, ...]
    has_parent: Literal[False]
    in_workers_log: Literal[False]
    forbidden_hits: tuple[()]


@dataclass(frozen=True)
class Ineligible:
    ticket: int
    reasons: tuple[str, ...]


@dataclass(frozen=True)
class TicketEvidence:
    """登记器用 App 身份读到的票面证据。每一项都是读完的完整结果；读不全时不构造本对象。"""

    ticket: int
    is_pull_request: bool
    state: str
    body: str
    current_labels: frozenset[str]
    timeline_events: tuple[dict[str, Any], ...]
    has_parent: bool
    workers_log: str


def workers_log_hits(ticket: int, text: str) -> tuple[bool, tuple[str, ...]]:
    """workers.jsonl 每行必须是 JSON 对象；任何不可解析的行都使结论不成立。"""
    problems: list[str] = []
    hit = False
    for n, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            problems.append(f"workers_log_line_{n}_unparsed")
            continue
        if not isinstance(row, dict):
            problems.append(f"workers_log_line_{n}_not_object")
            continue
        if row.get("ticket") in (ticket, str(ticket)) or row.get("branch") == f"sandcastle/issue-{ticket}":
            hit = True
    return hit, tuple(problems)


def judge_eligibility(evidence: TicketEvidence) -> EligibilityProof | Ineligible:
    """SPEC 7.2：首证专用票从未进入任何旧可选集。不比较任何时间。"""
    reasons: list[str] = []
    if evidence.is_pull_request:
        reasons.append("is_pull_request")
    if evidence.state != "open":
        reasons.append("not_open")
    if DEDICATED_MARKER not in evidence.body:
        reasons.append("missing_dedicated_marker")
    ever = set(evidence.current_labels)
    for event in evidence.timeline_events:
        label = event.get("label")
        if event.get("event") == "labeled":
            if not isinstance(label, dict) or not isinstance(label.get("name"), str):
                reasons.append("labeled_event_without_name")
                continue
            ever.add(label["name"])
        if event.get("event") in PARENT_EVENTS:
            reasons.append("timeline_parent_event")
    hits = forbidden(ever)
    if hits:
        reasons.append("forbidden_label_history:" + ",".join(hits))
    if evidence.has_parent:
        reasons.append("has_parent")
    in_log, problems = workers_log_hits(evidence.ticket, evidence.workers_log)
    reasons.extend(problems)
    if in_log:
        reasons.append("in_workers_log")
    if reasons:
        return Ineligible(evidence.ticket, tuple(dict.fromkeys(reasons)))
    return EligibilityProof(evidence.ticket, tuple(sorted(ever)), False, False, ())


@dataclass(frozen=True)
class DedicatedPilot:
    eligibility: EligibilityProof
    scope_ref: str
    kind: Literal["dedicated_pilot"] = "dedicated_pilot"


@dataclass(frozen=True)
class Takeover:
    """SPEC 7.3 未实施：形状保留，准入一律关闭。"""

    handoff: dict[str, Any]
    spec_auth: SpecAuthorization
    kind: Literal["takeover"] = "takeover"


Admission = DedicatedPilot | Takeover


@dataclass(frozen=True)
class WorkUnit:
    ticket: int
    graph: int | None
    base: Sha
    allowed_prefixes: tuple[str, ...]
    playbook: Playbook
    require_design: bool
    admission: Admission
    run_id: RunId
    candidate_approval: bool = False
    v: Literal[2] = 2

    @property
    def branch(self) -> str:
        return f"codex/deepagents/t{self.ticket}"

    @property
    def requires_candidate_approval(self) -> bool:
        """发布前须有对确切候选 head 的真人签批：接管票一律需要，专用票由登记方显式要求。"""
        return self.candidate_approval or isinstance(self.admission, Takeover)

    def to_json(self) -> dict[str, Any]:
        admission: dict[str, Any]
        if isinstance(self.admission, DedicatedPilot):
            proof = self.admission.eligibility
            admission = {"kind": "dedicated_pilot", "scope_ref": self.admission.scope_ref, "eligibility": {
                "ticket": proof.ticket, "labels_ever": list(proof.labels_ever), "has_parent": False,
                "in_workers_log": False, "forbidden_hits": []}}
        else:
            admission = {"kind": "takeover", "handoff": self.admission.handoff,
                         "spec_auth": self.admission.spec_auth.to_json()}
        data = {"v": 2, "ticket": self.ticket, "graph": self.graph, "base": self.base,
                "allowed_prefixes": list(self.allowed_prefixes), "playbook": self.playbook,
                "require_design": self.require_design, "admission": admission, "run_id": self.run_id}
        if self.candidate_approval:
            data["candidate_approval"] = True
        return data


def parse_unit_run_id(ticket: int, raw: str) -> RunId:
    run_id = parse_run_id(raw)
    if not re.fullmatch(rf"t{ticket}-[0-9]+", run_id):
        raise ValueError(f"run-id 须为 t{ticket}-<序号>")
    return run_id


def parse_work_unit(data: dict[str, Any]) -> WorkUnit:
    if data.get("v") != 2:
        raise ValueError("WorkUnit 须带 v: 2")
    ticket = data["ticket"]
    if type(ticket) is not int or ticket <= 0:
        raise ValueError("ticket 须为正整数")
    raw = data["admission"]
    admission: Admission
    if raw.get("kind") == "dedicated_pilot":
        proof = raw["eligibility"]
        if (proof.get("ticket") != ticket or proof.get("has_parent") is not False
                or proof.get("in_workers_log") is not False or proof.get("forbidden_hits") != []):
            raise ValueError("EligibilityProof 不成立")
        labels = tuple(proof["labels_ever"])
        if forbidden(set(labels)):
            raise ValueError("EligibilityProof 含禁用标签")
        admission = DedicatedPilot(EligibilityProof(ticket, labels, False, False, ()), str(raw["scope_ref"]))
    elif raw.get("kind") == "takeover":
        admission = Takeover(dict(raw.get("handoff") or {}), SpecAuthorization.from_json(raw["spec_auth"]))
    else:
        raise ValueError("未知准入类型")
    prefixes = tuple(data["allowed_prefixes"])
    if not prefixes or not all(isinstance(p, str) and p.endswith("/") and not p.startswith("/") and ".." not in p
                               for p in prefixes):
        raise ValueError("allowed_prefixes 须为以 / 结尾的相对目录")
    if data["playbook"] not in ("feature", "bug-fix", "refactoring"):
        raise ValueError("playbook 无效")
    graph = data.get("graph")
    if graph is not None and type(graph) is not int:
        raise ValueError("graph 须为整数或 null")
    if not isinstance(data["require_design"], bool):
        raise ValueError("require_design 须为布尔值")
    candidate_approval = data.get("candidate_approval", False)
    if type(candidate_approval) is not bool or ("candidate_approval" in data and not candidate_approval):
        raise ValueError("candidate_approval 只能省略或为 true")
    return WorkUnit(ticket, graph, parse_sha(data["base"]), prefixes, cast(Playbook, data["playbook"]),
                    data["require_design"], admission, parse_unit_run_id(ticket, data["run_id"]),
                    candidate_approval)
