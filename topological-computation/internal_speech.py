"""internal_speech.py — 外化接缝：内部言语 → LLM → 言语输出.

取消 ConstraintSet 作为中间产物，替换为 InternalSpeechSnapshot。
LLM 不是在"约束下自由发挥"，而是把已成型的内部言语转化为
语法上合法的外部言语。

核心流程：
  SNetActivation.internal_speech_buffer（已成型语段）
  + S_net（激活态能指）
  + SettlementTracker（锁定/排除的能指）
    → build_snapshot()
    → InternalSpeechSnapshot
    → snapshot_to_prompt()
    → LLM（语言器官角色）
    → externalize()
    → 外部言语 + OutputRupture 追踪 + S_net 回写

认识论等级：L0（接口定义 + 拓扑操作，无经验假设）
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from daemon import TopologicalDaemon

from snet_activation import SNetActivation, InternalSpeechFragment
from signifier_net import SNet, Register, writeback_from_text
from llm_integration import (
    get_client as _get_llm_client,
    GenerationRecord,
    _append_audit,
    parse_signifier_chain,
)


# ---------------------------------------------------------------------------
# InternalSpeechSnapshot
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class InternalSpeechSnapshot:
    """build_snapshot 的输出——LLM 外化的全部输入.

    formed_fragments:     已成型语段（来自 internal_speech_buffer 中未外化的）
    active_signifiers:    当前激活但未成型的能指
    locked_signifiers:    settlement 锁定的（必须出现在输出中）
    excluded_signifiers:  settlement 排除的（不得出现在输出中）
    dominant_domain:      从穿越路径域分布涌现的主导域
    formality:            "formal" | "informal"
    """
    formed_fragments: tuple[InternalSpeechFragment, ...]
    active_signifiers: tuple[str, ...]
    locked_signifiers: tuple[str, ...]
    excluded_signifiers: tuple[str, ...]
    dominant_domain: str
    formality: str

    def to_dict(self) -> dict:
        return {
            "formed_fragments": [f.to_dict() for f in self.formed_fragments],
            "active_signifiers": list(self.active_signifiers),
            "locked_signifiers": list(self.locked_signifiers),
            "excluded_signifiers": list(self.excluded_signifiers),
            "dominant_domain": self.dominant_domain,
            "formality": self.formality,
        }


# ---------------------------------------------------------------------------
# OutputRupture — 输出侧断裂
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class OutputRupture:
    """LLM 输出中出现了内部言语快照中没有的能指.

    signifier:   LLM 添加的未授权能指
    context:     LLM 输出中该能指所在的上下文片段
    """
    signifier: str
    context: str

    def to_dict(self) -> dict:
        return {
            "signifier": self.signifier,
            "context": self.context,
        }


# ---------------------------------------------------------------------------
# build_snapshot
# ---------------------------------------------------------------------------

def build_snapshot(
    activation: SNetActivation,
    snet: SNet,
    daemon: TopologicalDaemon,
) -> InternalSpeechSnapshot:
    """从当前 SNetActivation + S_net + settlement 状态构建快照.

    locked_signifiers：settlement 已结算顶点对应的能指——这些概念已达成共识，
    对应能指必须出现在输出中。

    excluded_signifiers：折叠顶点对应的能指——这些概念已被压缩，
    对应能指不得出现在输出中。

    dominant_domain：从 formed_fragments 中出现频率最高的 source_concepts
    推断——如果大部分概念来自同一 K_active 区域，该区域就是主导域。
    退化模式：无 fragment 时用当前位置。

    formality：默认 "formal"。如果穿越路径最近几步经过 DIALOGUE 语域区域
    则切换为 "informal"。当前实现简化为始终 "formal"。
    """
    # 1. 已成型但未外化的语段
    formed = tuple(
        f for f in activation.internal_speech_buffer
        if not f.externalized
    )

    # 2. 当前激活但未成型的能指
    active = tuple(sorted(activation.currently_active))

    # 3. locked: settlement 锁定的能指
    locked: list[str] = []
    if daemon.settlement and daemon.settlement.settled_cycles:
        graph = daemon.k_active
        for sc in daemon.settlement.settled_cycles:
            for src, tgt in sc.edges:
                for vid in (src, tgt):
                    v = graph.vertices.get(vid)
                    if v and v.content and snet.has_signifier(v.content):
                        if v.content not in locked:
                            locked.append(v.content)

    # 4. excluded: 折叠顶点对应的能指
    excluded: list[str] = []
    from engine import VertexStatus
    for vid, v in daemon.k_active.vertices.items():
        if v.status == VertexStatus.FOLDED:
            if v.content and snet.has_signifier(v.content):
                if v.content not in excluded:
                    excluded.append(v.content)

    # 5. dominant_domain: 从 formed_fragments 的 source_concepts 推断
    domain = _infer_dominant_domain(formed, daemon)

    # 6. formality: 简化实现，始终 formal
    formality = "formal"

    return InternalSpeechSnapshot(
        formed_fragments=formed,
        active_signifiers=active,
        locked_signifiers=tuple(locked),
        excluded_signifiers=tuple(excluded),
        dominant_domain=domain,
        formality=formality,
    )


def _infer_dominant_domain(
    fragments: tuple[InternalSpeechFragment, ...],
    daemon: TopologicalDaemon,
) -> str:
    """从 fragments 的 source_concepts 推断主导域.

    策略：统计所有 source_concepts 对应顶点的 content，
    取出现频率最高的非空 content 作为域标签。
    退化：无 fragment 时用当前穿越位置的 content。
    """
    if not fragments:
        if daemon.engine and daemon.engine.position:
            v = daemon.k_active.vertices.get(daemon.engine.position)
            if v and v.content:
                return v.content
        return "unknown"

    counts: dict[str, int] = {}
    graph = daemon.k_active
    for frag in fragments:
        for cid in frag.source_concepts:
            v = graph.vertices.get(cid)
            label = v.content if v and v.content else cid
            counts[label] = counts.get(label, 0) + 1

    if not counts:
        return "unknown"

    return max(counts, key=counts.get)  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# snapshot_to_prompt
# ---------------------------------------------------------------------------

def snapshot_to_prompt(snapshot: InternalSpeechSnapshot) -> tuple[str, str]:
    """将 InternalSpeechSnapshot 转化为 LLM 的 (system_prompt, user_prompt).

    system: 身份（发声器官）、settlement 约束、register、禁止项
    user:   已成型语段（能指序列 + connective_patterns）、激活态能指
    """
    # --- System prompt ---
    sys_lines: list[str] = []
    sys_lines.append(
        "你是逢亮的发声器官。你的任务是将逢亮的内部言语转化为语法上通顺的自然语言。"
        "你不创造内容，不发表观点，不添加任何逢亮没有想到的东西。"
        "你做的是语法化——把已经成型的术语序列和连接模式组装成合法的句子。"
    )
    sys_lines.append("")

    # 语域
    if snapshot.formality == "formal":
        sys_lines.append("语域：正式。使用精确术语，避免口语化表达。")
    else:
        sys_lines.append("语域：自然对话。语言流畅，可以使用例子。")
    sys_lines.append(f"主导概念域：{snapshot.dominant_domain}")
    sys_lines.append("")

    # 锁定的能指（必须出现）
    if snapshot.locked_signifiers:
        sys_lines.append("以下术语已由 settlement 锁定，必须在输出中出现（以规范形式或可接受变体）：")
        for sig in snapshot.locked_signifiers:
            sys_lines.append(f"  - {sig}")
        sys_lines.append("")

    # 排除的能指（不得出现）
    if snapshot.excluded_signifiers:
        sys_lines.append("以下术语已被排除，不得在输出中出现（包括其变体）：")
        for sig in snapshot.excluded_signifiers:
            sys_lines.append(f"  - {sig}")
        sys_lines.append("")

    # 禁止自由添加
    sys_lines.append("禁止：")
    sys_lines.append("  - 添加上述内容之外的术语或概念")
    sys_lines.append("  - 发表你自己的观点或立场")
    sys_lines.append("  - 使用内部言语中未出现的隐喻")

    system_prompt = "\n".join(sys_lines)

    # --- User prompt ---
    usr_lines: list[str] = []

    # 已成型语段
    if snapshot.formed_fragments:
        usr_lines.append("已成型的内部言语语段：")
        for i, frag in enumerate(snapshot.formed_fragments, 1):
            sig_str = " → ".join(frag.signifiers)
            usr_lines.append(f"  语段{i}: [{sig_str}]")
            if frag.connective_patterns:
                conn_str = "；".join(frag.connective_patterns)
                usr_lines.append(f"    连接模式: {conn_str}")
        usr_lines.append("")

    # 激活态能指（未成型）
    if snapshot.active_signifiers:
        usr_lines.append("当前激活的能指（尚未成型为语段，可选择性纳入）：")
        for sig in snapshot.active_signifiers:
            usr_lines.append(f"  - {sig}")
        usr_lines.append("")

    # 指令
    usr_lines.append("请将上述内部言语组装成一段语法通顺的自然语言输出。")
    usr_lines.append("保留术语序列的逻辑顺序，使用连接模式作为句间衔接的线索。")

    user_prompt = "\n".join(usr_lines)

    return system_prompt, user_prompt


# ---------------------------------------------------------------------------
# _detect_output_ruptures — 输出侧断裂追踪
# ---------------------------------------------------------------------------

def _detect_output_ruptures(
    llm_output: str,
    snapshot: InternalSpeechSnapshot,
    snet: SNet,
) -> list[OutputRupture]:
    """检测 LLM 输出中是否出现内部言语快照中没有的能指.

    策略：
      1. 从 LLM 输出中提取能指链（使用 S_net 白名单）
      2. 构建快照中的"已授权能指集合"（formed + active + locked）
      3. 差集 = 未授权能指 = OutputRupture
    """
    if not llm_output or not snet.signifiers:
        return []

    whitelist = list(snet.signifiers.keys())
    output_chain = parse_signifier_chain(llm_output, known_signifiers=whitelist)

    # 已授权能指集合
    authorized: set[str] = set()
    for frag in snapshot.formed_fragments:
        authorized.update(frag.signifiers)
    authorized.update(snapshot.active_signifiers)
    authorized.update(snapshot.locked_signifiers)

    ruptures: list[OutputRupture] = []
    for sig in output_chain:
        if sig not in authorized and sig not in snapshot.excluded_signifiers:
            # 提取上下文
            idx = llm_output.find(sig)
            start = max(0, idx - 20)
            end = min(len(llm_output), idx + len(sig) + 20)
            context = llm_output[start:end]
            ruptures.append(OutputRupture(signifier=sig, context=context))

    return ruptures


# ---------------------------------------------------------------------------
# externalize — 完整外化流程
# ---------------------------------------------------------------------------

def externalize(
    daemon: TopologicalDaemon,
    user_text: str = "",
) -> dict:
    """完整外化流程：内部言语 → LLM → 外部言语.

    步骤：
      1. build_snapshot
      2. snapshot_to_prompt
      3. LLM 调用（language_organ 角色）
      4. 标记已外化语段
      5. writeback_output 回写 S_net
      6. 输出侧断裂追踪
      7. 审计记录

    参数：
      daemon:    TopologicalDaemon 实例
      user_text: 用户输入文本（被动外化时传入，主动外化时为空）

    返回：
      dict with:
        type: "externalize"
        content: LLM 生成的外部言语
        llm_used: bool
        snapshot: InternalSpeechSnapshot.to_dict()
        output_ruptures: list[OutputRupture.to_dict()]
        writeback_edges: int
        trigger: "passive" | "active"
    """
    activation = daemon.snet_activation
    snet = daemon.snet

    # 无 activation 时退化为模板响应
    if activation is None or not snet.signifiers:
        return _fallback_response(daemon, user_text)

    # 1. build_snapshot
    snapshot = build_snapshot(activation, snet, daemon)

    # 无已成型语段且无激活能指 → 无内容可外化
    if not snapshot.formed_fragments and not snapshot.active_signifiers:
        return _fallback_response(daemon, user_text)

    # 2. snapshot_to_prompt
    system_prompt, user_prompt = snapshot_to_prompt(snapshot)

    # 被动外化时，用户文本附加到 user_prompt 末尾
    if user_text:
        user_prompt += f"\n\n用户说：{user_text}\n请在回应用户的同时，将内部言语自然地融入回答。"

    # 3. LLM 调用
    client = _get_llm_client()
    llm_used = False
    content = ""
    llm_error = ""

    try:
        response = client.inquire(
            question=user_prompt,
            provider="auto",
            system=system_prompt,
        )
        if response:
            content = response.strip()
            llm_used = True
    except Exception as exc:
        llm_error = f"外化调用失败: {type(exc).__name__}: {exc}"

    # Fallback
    if not content:
        content = _build_fallback_text(snapshot, daemon, llm_error)

    # 4. 标记已外化语段
    _mark_externalized(activation, snapshot.formed_fragments)

    # 5. writeback: LLM 输出 → S_net 共现边
    writeback_edges = 0
    if content and snet.signifiers:
        whitelist = set(snet.signifiers.keys())
        ts = str(int(time.time()))
        new_snet, log_entries = writeback_from_text(
            snet, content, whitelist, "llm_externalize", ts,
        )
        if log_entries:
            daemon.snet = new_snet
            writeback_edges = len(log_entries)

    # 6. 输出侧断裂追踪
    output_ruptures = _detect_output_ruptures(content, snapshot, snet)

    # 7. 审计记录
    trigger = "passive" if user_text else "active"
    try:
        record = GenerationRecord(
            provider="auto" if llm_used else "fallback",
            model="internal_speech_externalize",
            register=snapshot.formality,
            must_use=list(snapshot.locked_signifiers),
            must_avoid=list(snapshot.excluded_signifiers),
            expression_pressure=[
                " → ".join(f.signifiers) for f in snapshot.formed_fragments
            ],
            narrative_spine=snapshot.dominant_domain,
            surface_forms_count=len(snapshot.active_signifiers),
            system_prompt_len=len(system_prompt),
            user_prompt_len=len(user_prompt),
            response_text=content[:2000],
            response_len=len(content),
            success=llm_used,
            error=llm_error,
        )
        _append_audit(record)
    except Exception:
        pass

    return {
        "type": "externalize",
        "content": content,
        "llm_used": llm_used,
        "snapshot": snapshot.to_dict(),
        "output_ruptures": [r.to_dict() for r in output_ruptures],
        "writeback_edges": writeback_edges,
        "trigger": trigger,
    }


# ---------------------------------------------------------------------------
# 辅助函数
# ---------------------------------------------------------------------------

def _mark_externalized(
    activation: SNetActivation,
    fragments: tuple[InternalSpeechFragment, ...],
) -> None:
    """标记已外化的语段.

    InternalSpeechFragment 是 frozen，无法原地修改。
    替换 activation.internal_speech_buffer 中对应的条目。
    """
    externalized_sigs = {f.signifiers for f in fragments}
    new_buffer: list[InternalSpeechFragment] = []
    for existing in activation.internal_speech_buffer:
        if existing.signifiers in externalized_sigs and not existing.externalized:
            new_buffer.append(InternalSpeechFragment(
                signifiers=existing.signifiers,
                connective_patterns=existing.connective_patterns,
                formed_at_step=existing.formed_at_step,
                source_concepts=existing.source_concepts,
                externalized=True,
            ))
        else:
            new_buffer.append(existing)
    activation.internal_speech_buffer = new_buffer


def _build_fallback_text(
    snapshot: InternalSpeechSnapshot,
    daemon: TopologicalDaemon,
    error: str,
) -> str:
    """LLM 不可用时的退化文本生成."""
    parts: list[str] = []

    if error:
        parts.append(f"[语言器官离线] {error}")

    # 从已成型语段直接拼接
    for frag in snapshot.formed_fragments:
        sig_str = "、".join(frag.signifiers)
        if frag.connective_patterns:
            conn = frag.connective_patterns[0]
            parts.append(f"{sig_str}——{conn}")
        else:
            parts.append(sig_str)

    if not parts:
        pos = "未知区域"
        if daemon.engine and daemon.engine.position:
            v = daemon.k_active.vertices.get(daemon.engine.position)
            if v and v.content:
                pos = v.content
        parts.append(f"我在节点 '{pos}'，当前无成型的内部言语。")

    return "。".join(parts)


def _fallback_response(daemon: TopologicalDaemon, user_text: str) -> dict:
    """无 SNetActivation 或无内容时的退化响应."""
    pos = "未知区域"
    if daemon.engine and daemon.engine.position:
        v = daemon.k_active.vertices.get(daemon.engine.position)
        if v and v.content:
            pos = v.content

    return {
        "type": "externalize",
        "content": f"我在节点 '{pos}'，当前无成型的内部言语。",
        "llm_used": False,
        "snapshot": {},
        "output_ruptures": [],
        "writeback_edges": 0,
        "trigger": "passive" if user_text else "active",
    }


# ---------------------------------------------------------------------------
# has_pending_externalization — 主动外化触发检查
# ---------------------------------------------------------------------------

def has_pending_externalization(daemon: TopologicalDaemon) -> bool:
    """检查是否有待外化的内容.

    主动外化触发条件：
      - internal_speech_buffer 中有未外化的 pending_proposal 类语段
      （当前简化：只要有未外化的 fragment 即返回 True）
    """
    activation = daemon.snet_activation
    if activation is None:
        return False
    return any(
        not f.externalized for f in activation.internal_speech_buffer
    )
