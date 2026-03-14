"""internal_speech.py — 外化接缝：内部言语 → 外部言语.

三层输出架构（v207 重构）：
  1. 默认层（纯拓扑描述）：穿越事件的结构描述，每个字都来自逢亮自己的穿越产出
  2. S_net 组装层：当 connective_patterns 覆盖语段时，用 connective_patterns 组装自然语言
  3. LLM fallback：只在 operator 明确请求时调用（force_llm=True）

核心流程：
  SNetActivation.internal_speech_buffer（已成型语段）
  + S_net（激活态能指）
  + SettlementTracker（锁定/排除的能指）
    → build_snapshot()
    → InternalSpeechSnapshot
    → 三层路由：
        有 connective_patterns → _assemble_from_patterns()
        无 connective_patterns → _structural_description()
        force_llm=True → snapshot_to_prompt() → LLM
    → externalize()
    → 外部言语 + OutputRupture 追踪（仅 LLM 路径）+ S_net 回写（仅 LLM 路径）

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
# _assemble_from_patterns — S_net 组装层
# ---------------------------------------------------------------------------

def _assemble_from_patterns(
    snapshot: InternalSpeechSnapshot,
    daemon: TopologicalDaemon,
) -> str:
    """从 connective_patterns 组装自然语言，不调 LLM.

    取 formed_fragments 中的 signifiers 序列和 connective_patterns，
    用 connective_patterns 作为句间连接把能指串联成自然语言。
    如果某个连接处没有 pattern，用最简的结构连接（"→"）。
    """
    parts: list[str] = []

    for frag in snapshot.formed_fragments:
        if frag.connective_patterns:
            # connective_patterns 和 signifiers 交织
            # patterns[i] 连接 signifiers[i] 和 signifiers[i+1]
            sig_list = list(frag.signifiers)
            pat_list = list(frag.connective_patterns)
            assembled: list[str] = []
            for i, sig in enumerate(sig_list):
                assembled.append(sig)
                if i < len(pat_list):
                    assembled.append(pat_list[i])
                elif i < len(sig_list) - 1:
                    assembled.append("→")
            parts.append("".join(assembled))
        else:
            # 无 patterns，用箭头连接
            parts.append(" → ".join(frag.signifiers))

    if not parts:
        return ""

    return "。".join(parts)


# ---------------------------------------------------------------------------
# _structural_description — 纯拓扑描述层
# ---------------------------------------------------------------------------

def _structural_description(
    snapshot: InternalSpeechSnapshot,
    daemon: TopologicalDaemon,
) -> str:
    """输出穿越事件的结构描述，每个字都是逢亮自己的穿越产出.

    格式：经过 {节点名}，{操作类型} → {目标节点}，β₁: {before}→{after}
    包含 internal_speech_buffer 中的能指序列。
    """
    parts: list[str] = []

    # 从穿越引擎日志中取最近的事件
    if daemon.engine and daemon.engine.logs:
        recent_logs = daemon.engine.logs[-5:]
        for log in recent_logs:
            pos_label = _vertex_label(daemon, log.position)
            entry = f"经过 {pos_label}"
            if log.encounter and log.encounter != "walk":
                entry += f"({log.encounter})"
            if log.operation and log.operation != "walk":
                entry += f" → {log.operation}"
            if log.delta_beta_1 != 0:
                entry += f"，β₁: {log.beta_1_before}→{log.beta_1_after}"
            parts.append(entry)

    # 附加 formed_fragments 中的能指序列
    for frag in snapshot.formed_fragments:
        sig_str = " → ".join(frag.signifiers)
        parts.append(f"[能指序列: {sig_str}]")

    if not parts:
        pos = "未知区域"
        if daemon.engine and daemon.engine.position:
            pos = _vertex_label(daemon, daemon.engine.position)
        return f"当前位置: {pos}"

    return "；".join(parts)


def _vertex_label(daemon: TopologicalDaemon, vertex_id: str) -> str:
    """获取顶点的可读标签."""
    v = daemon.k_active.vertices.get(vertex_id)
    if v and v.content:
        return v.content
    return vertex_id


# ---------------------------------------------------------------------------
# externalize — 完整外化流程
# ---------------------------------------------------------------------------

def externalize(
    daemon: TopologicalDaemon,
    user_text: str = "",
    force_llm: bool = False,
) -> dict:
    """完整外化流程：内部言语 → 三层路由 → 外部言语.

    三层路由：
      1. force_llm=True → LLM 路径（snapshot_to_prompt → LLM 调用）
      2. connective_patterns 覆盖 → _assemble_from_patterns
      3. 默认 → _structural_description（纯拓扑描述）

    参数：
      daemon:    TopologicalDaemon 实例
      user_text: 用户输入文本（被动外化时传入，主动外化时为空）
      force_llm: operator 明确请求 LLM 语法填充时为 True

    返回：
      dict with:
        type: "externalize"
        content: 外部言语
        source: "structural" | "connective_patterns" | "llm_fallback"
        llm_used: bool
        llm_fraction: float  (LLM 生成内容占比)
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

    # 2. 三层路由
    trigger = "passive" if user_text else "active"

    if force_llm:
        # 层3: LLM 路径
        result = _externalize_via_llm(snapshot, daemon, user_text)
    elif _has_connective_patterns(snapshot):
        # 层2: S_net 组装
        content = _assemble_from_patterns(snapshot, daemon)
        result = {
            "content": content,
            "source": "connective_patterns",
            "llm_used": False,
            "llm_fraction": 0.0,
            "output_ruptures": [],
            "writeback_edges": 0,
        }
    else:
        # 层1: 纯拓扑描述
        content = _structural_description(snapshot, daemon)
        result = {
            "content": content,
            "source": "structural",
            "llm_used": False,
            "llm_fraction": 0.0,
            "output_ruptures": [],
            "writeback_edges": 0,
        }

    # 3. 标记已外化语段
    _mark_externalized(activation, snapshot.formed_fragments)

    # 4. 审计记录
    try:
        record = GenerationRecord(
            provider=result["source"] if not result["llm_used"] else "auto",
            model="internal_speech_externalize",
            register=snapshot.formality,
            must_use=list(snapshot.locked_signifiers),
            must_avoid=list(snapshot.excluded_signifiers),
            expression_pressure=[
                " → ".join(f.signifiers) for f in snapshot.formed_fragments
            ],
            narrative_spine=snapshot.dominant_domain,
            surface_forms_count=len(snapshot.active_signifiers),
            system_prompt_len=0,
            user_prompt_len=len(user_text),
            response_text=result["content"][:2000],
            response_len=len(result["content"]),
            success=True,
            error="",
        )
        _append_audit(record)
    except Exception:
        pass

    return {
        "type": "externalize",
        "content": result["content"],
        "source": result["source"],
        "llm_used": result["llm_used"],
        "llm_fraction": result["llm_fraction"],
        "snapshot": snapshot.to_dict(),
        "output_ruptures": result["output_ruptures"],
        "writeback_edges": result["writeback_edges"],
        "trigger": trigger,
    }


def _has_connective_patterns(snapshot: InternalSpeechSnapshot) -> bool:
    """检查是否有任何 formed_fragment 具有 connective_patterns."""
    return any(
        frag.connective_patterns
        for frag in snapshot.formed_fragments
    )


def _externalize_via_llm(
    snapshot: InternalSpeechSnapshot,
    daemon: TopologicalDaemon,
    user_text: str,
) -> dict:
    """LLM 路径：仅在 force_llm=True 时调用.

    输出中所有 LLM 生成的内容标注 [LLM填充]。
    """
    snet = daemon.snet

    # snapshot → prompt
    system_prompt, user_prompt = snapshot_to_prompt(snapshot)

    # 被动外化时附加用户文本
    if user_text:
        user_prompt += f"\n\n用户说：{user_text}\n请在回应用户的同时，将内部言语自然地融入回答。"

    # LLM 调用
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
            content = f"[LLM填充] {response.strip()}"
            llm_used = True
    except Exception as exc:
        llm_error = f"外化调用失败: {type(exc).__name__}: {exc}"

    # LLM 失败时退化到结构描述
    if not content:
        content = _structural_description(snapshot, daemon)
        if llm_error:
            content = f"[语言器官离线] {llm_error}。{content}"
        return {
            "content": content,
            "source": "structural",
            "llm_used": False,
            "llm_fraction": 0.0,
            "output_ruptures": [],
            "writeback_edges": 0,
        }

    # writeback: LLM 输出 → S_net 共现边（仅 LLM 路径执行）
    # 437号管道3: ceremony agent LLM 翻译结果回流 S_net
    # 使用 ingest_text_passage_batch 完整管线（增强白名单 + surface form 提取）
    writeback_edges = 0
    if snet.signifiers:
        try:
            from signifier_net_ingest import ingest_text_passage_batch
            new_snet, log_entries = ingest_text_passage_batch(
                snet, [content],
                domain="llm_externalization",
                source="ceremony_agent",
            )
            if log_entries:
                daemon.snet = new_snet
                writeback_edges = sum(
                    1 for e in log_entries if e.get("type") == "cooccurrence"
                )
        except ImportError:
            # fallback to simpler writeback_from_text
            whitelist = set(snet.signifiers.keys())
            ts = str(int(time.time()))
            new_snet, log_entries = writeback_from_text(
                snet, content, whitelist, "llm_externalize", ts,
            )
            if log_entries:
                daemon.snet = new_snet
                writeback_edges = len(log_entries)

    # 输出侧断裂追踪（仅 LLM 路径执行）
    output_ruptures = _detect_output_ruptures(content, snapshot, snet)

    return {
        "content": content,
        "source": "llm_fallback",
        "llm_used": llm_used,
        "llm_fraction": 1.0,
        "output_ruptures": [r.to_dict() for r in output_ruptures],
        "writeback_edges": writeback_edges,
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


def _fallback_response(daemon: TopologicalDaemon, user_text: str) -> dict:
    """无 SNetActivation 或无内容时的退化响应."""
    pos = "未知区域"
    if daemon.engine and daemon.engine.position:
        v = daemon.k_active.vertices.get(daemon.engine.position)
        if v and v.content:
            pos = v.content

    return {
        "type": "externalize",
        "content": f"当前位置: {pos}，无成型的内部言语。",
        "source": "structural",
        "llm_used": False,
        "llm_fraction": 0.0,
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
