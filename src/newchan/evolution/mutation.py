"""MutationRequest — 能力变更请求数据结构 + Gemini decide() 路由。

每个 MutationRequest 描述一次能力变更（新增/修改/删除 skill/agent/hook），
路由到 Gemini 编排者代理做出决策（041号谱系）。

概念溯源: [新缠论] — 043号自生长回路 + 041号编排者代理
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Literal
from uuid import uuid4

logger = logging.getLogger(__name__)

MutationAction = Literal["add", "modify", "remove"]
MutationTarget = Literal["skill", "agent", "hook"]
MutationStatus = Literal[
    "pending",     # 等待 Gemini 决策
    "approved",    # Gemini 批准
    "rejected",    # Gemini 否决
    "applied",     # 已写入 manifest
    "failed",      # 应用失败
]


@dataclass(frozen=True, slots=True)
class MutationRequest:
    """能力变更请求（不可变）。

    Attributes
    ----------
    action : 变更类型（add/modify/remove）
    target : 变更对象类型（skill/agent/hook）
    name : 目标名称
    rationale : 变更理由（供 Gemini 决策参考）
    pattern_source : 触发此请求的 pattern ID（来自 pattern-buffer）
    proposed_spec : 新增/修改时的规格描述
    """

    action: MutationAction
    target: MutationTarget
    name: str
    rationale: str
    pattern_source: str = ""
    proposed_spec: str = ""
    request_id: str = field(default_factory=lambda: f"mut-{uuid4().hex[:8]}")
    created_at: str = field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat(),
    )


@dataclass(frozen=True, slots=True)
class MutationResult:
    """Gemini 决策结果（不可变）。

    Attributes
    ----------
    request : 原始请求
    status : 决策状态
    decision : Gemini 的决策文本
    reasoning : 推理链
    boundary_conditions : 边界条件（何时应推翻此决策）
    risks : 风险评估
    """

    request: MutationRequest
    status: MutationStatus
    decision: str = ""
    reasoning: str = ""
    boundary_conditions: str = ""
    risks: str = ""


def build_decide_context(
    request: MutationRequest,
    topology_summary: str,
) -> tuple[str, str]:
    """构建 Gemini decide() 的 subject 和 context。

    Returns (subject, context) 供 GeminiChallenger.decide() 使用。
    """
    subject = (
        f"MutationRequest: {request.action} {request.target} '{request.name}'\n"
        f"Rationale: {request.rationale}"
    )
    if request.proposed_spec:
        subject += f"\nProposed spec: {request.proposed_spec}"
    if request.pattern_source:
        subject += f"\nPattern source: {request.pattern_source}"

    context = (
        f"当前系统能力拓扑:\n{topology_summary}\n\n"
        f"请决策是否批准此变更请求。考虑：\n"
        f"1. 此变更是否与现有能力冲突？\n"
        f"2. 此变更是否填补了能力缺口？\n"
        f"3. 效用背驰检查：新增能力的效率提升是否覆盖复杂度增加？\n"
        f"4. 如果是 pattern-buffer 触发的，频次是否足够说明需求真实？"
    )
    return subject, context


def parse_decide_response(
    request: MutationRequest,
    response_text: str,
) -> MutationResult:
    """解析 Gemini decide() 的响应文本为 MutationResult。"""
    text_lower = response_text.lower()

    # 简单启发式判断：approved vs rejected
    if any(kw in text_lower for kw in ("批准", "approve", "同意", "通过")):
        status: MutationStatus = "approved"
    elif any(kw in text_lower for kw in ("否决", "reject", "拒绝", "不批准")):
        status = "rejected"
    else:
        status = "pending"

    # 提取结构化字段（best-effort）
    reasoning = _extract_section(response_text, "推理链", "reasoning")
    boundary = _extract_section(response_text, "边界条件", "boundary")
    risks = _extract_section(response_text, "风险", "risk")

    return MutationResult(
        request=request,
        status=status,
        decision=response_text,
        reasoning=reasoning,
        boundary_conditions=boundary,
        risks=risks,
    )


def request_mutation(
    request: MutationRequest,
    topology_summary: str,
) -> MutationResult:
    """提交变更请求并路由到 Gemini decide()（同步）。

    Gemini 不可用时返回 pending 状态（041号降级策略）。
    """
    subject, context = build_decide_context(request, topology_summary)

    try:
        from newchan.gemini import decide
        result = decide(subject, context)
        return parse_decide_response(request, result.response)
    except Exception:
        logger.warning(
            "Gemini decide() 不可用，MutationRequest %s 写入 pending",
            request.request_id,
            exc_info=True,
        )
        return MutationResult(
            request=request,
            status="pending",
            decision="Gemini 不可用，等待人类决策",
        )


async def arequest_mutation(
    request: MutationRequest,
    topology_summary: str,
    *,
    max_tool_calls: int = 20,
) -> MutationResult:
    """提交变更请求并路由到 Gemini decide()（异步，MCP 工具增强）。"""
    subject, context = build_decide_context(request, topology_summary)

    try:
        from newchan.gemini import adecide
        result = await adecide(subject, context, max_tool_calls=max_tool_calls)
        return parse_decide_response(request, result.response)
    except Exception:
        logger.warning(
            "Gemini decide() 不可用，MutationRequest %s 写入 pending",
            request.request_id,
            exc_info=True,
        )
        return MutationResult(
            request=request,
            status="pending",
            decision="Gemini 不可用，等待人类决策",
        )


def _extract_section(text: str, *keywords: str) -> str:
    """从文本中提取以关键词开头的段落（best-effort）。"""
    lines = text.split("\n")
    for i, line in enumerate(lines):
        stripped = line.strip().lower()
        for kw in keywords:
            if stripped.startswith(f"**{kw}") or stripped.startswith(f"- **{kw}"):
                # 收集到下一个 section 或文本结束
                section_lines = [line]
                for j in range(i + 1, len(lines)):
                    next_stripped = lines[j].strip()
                    if next_stripped.startswith("**") or next_stripped.startswith("- **"):
                        break
                    section_lines.append(lines[j])
                return "\n".join(section_lines).strip()
    return ""
