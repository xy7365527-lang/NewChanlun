"""signifier_net.py — S_net: 能指网络数据结构 + ConstraintSet.

S_net 是逢亮的语言拓扑空间，与 K_active（概念拓扑）并行运行。

结构：
  顶点 = 能指 (signifier)
  边分两轴：
    - 组合轴 (syntagmatic)：能指之间的共现/邻接关系，来自语料
    - 聚合轴 (paradigmatic)：能指之间的替换/同义关系（待填充）

ConstraintSet 是 psi_L（概念层）传给 LLM（能指链引擎）的约束包。
LLM 在约束集内运动（组合轴自由度），逢亮通过约束集限制聚合轴选择。

变更记录：
  v2（Gemini Round 2 反馈）:
    - SNet.degree_normalized_neighbors(): degree-normalized 邻居选择
      （修复 hub 垄断：高度数节点的边按度数折算，避免高频节点劫持穿越）
    - build_constraint_set(): 新增工厂函数，从 SNet + expression_pressure 构建
      ConstraintSet，内部使用 degree-normalized 选择而非直接高频选择

认识论等级：L0（纯数据结构定义，无经验假设）
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Optional


# ---------------------------------------------------------------------------
# 轴类型
# ---------------------------------------------------------------------------

class AxisType(str, Enum):
    SYNTAGMATIC = "syntagmatic"   # 组合轴：共现/邻接
    PARADIGMATIC = "paradigmatic" # 聚合轴：替换/同义


# ---------------------------------------------------------------------------
# S_net 核心数据结构
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class Signifier:
    """S_net 中的一个能指节点。

    id: 规范形式（通常是术语本身）
    surface_forms: 该能指在语料中出现的表层变体（list，不可变）
    source: 来源层 — 'k_active_projection' | 'corpus' | 'manual'
    """
    id: str
    surface_forms: tuple[str, ...] = ()
    source: str = "k_active_projection"


@dataclass(frozen=True, slots=True)
class SignifierEdge:
    """S_net 中的一条边。

    source: 来源能指 id
    target: 目标能指 id
    axis: 组合轴或聚合轴
    weight: 共现强度（组合轴：PMI 值）或替换概率（聚合轴）
    evidence: 原始语言片段（surface），用于调试和叙事还原
    """
    source: str
    target: str
    axis: AxisType
    weight: float = 1.0
    evidence: str = ""


class SNet:
    """能指网络。

    设计约束（不可变操作模式，与 engine.Graph 同构）：
      - add_signifier / add_edge 返回新 SNet 实例
      - 内部用 dict + list 存储，不用 frozenset（有序、可重复）
    """

    def __init__(
        self,
        signifiers: dict[str, Signifier] | None = None,
        edges: list[SignifierEdge] | None = None,
    ) -> None:
        self._signifiers: dict[str, Signifier] = dict(signifiers) if signifiers else {}
        self._edges: list[SignifierEdge] = list(edges) if edges else []
        # 组合轴邻接索引（source -> list[SignifierEdge]）
        self._syn_out: dict[str, list[SignifierEdge]] = {}
        # 聚合轴替换索引（id -> list[SignifierEdge]）
        self._par_out: dict[str, list[SignifierEdge]] = {}
        # 度数索引（组合轴，无向）
        self._syn_degree: dict[str, int] = {}
        for e in self._edges:
            if e.axis == AxisType.SYNTAGMATIC:
                self._syn_out.setdefault(e.source, []).append(e)
                self._syn_degree[e.source] = self._syn_degree.get(e.source, 0) + 1
                self._syn_degree[e.target] = self._syn_degree.get(e.target, 0) + 1
            else:
                self._par_out.setdefault(e.source, []).append(e)

    @property
    def signifiers(self) -> dict[str, Signifier]:
        return dict(self._signifiers)

    @property
    def edges(self) -> list[SignifierEdge]:
        return list(self._edges)

    # ------------------------------------------------------------------
    # 只读查询
    # ------------------------------------------------------------------

    def has_signifier(self, sid: str) -> bool:
        return sid in self._signifiers

    def syntagmatic_neighbors(self, sid: str) -> list[SignifierEdge]:
        """返回能指 sid 的所有组合轴邻居（按 weight 降序）。"""
        edges = self._syn_out.get(sid, [])
        return sorted(edges, key=lambda e: -e.weight)

    def paradigmatic_alternatives(self, sid: str) -> list[SignifierEdge]:
        """返回能指 sid 的所有聚合轴替换项（按 weight 降序）。"""
        edges = self._par_out.get(sid, [])
        return sorted(edges, key=lambda e: -e.weight)

    def cooccurrence_weight(self, sid_a: str, sid_b: str) -> float:
        """返回两个能指之间的组合轴共现权重（无边则 0.0）。"""
        for e in self._syn_out.get(sid_a, []):
            if e.target == sid_b:
                return e.weight
        return 0.0

    def top_neighbors(self, sid: str, n: int = 5) -> list[str]:
        """返回 sid 的 top-n 组合轴邻居 id 列表。"""
        return [e.target for e in self.syntagmatic_neighbors(sid)[:n]]

    def syn_degree(self, sid: str) -> int:
        """返回能指 sid 的组合轴度数（无向）。"""
        return self._syn_degree.get(sid, 0)

    def degree_normalized_neighbors(
        self,
        sid: str,
        n: int = 5,
        alpha: float = 0.5,
    ) -> list[SignifierEdge]:
        """返回 sid 的 degree-normalized 组合轴邻居（按归一化分数降序）。

        归一化公式：
          score(e) = e.weight / (degree(e.target) ** alpha)

        含义：
          - alpha=0.0: 等价于原始 syntagmatic_neighbors（无归一化）
          - alpha=0.5: 对高度数 hub 折半惩罚（默认）
          - alpha=1.0: 完全按度数归一化

        目的：防止高度数 hub（如"级别"、"走势"）劫持所有邻居选择。
        高度数节点与任何其他节点都有边，即使 PMI 不特别高，
        原始权重也会累积优势；degree-normalization 将其折算。

        认识论等级：L0（代数操作，不依赖数据假设）
        """
        edges = self._syn_out.get(sid, [])
        if not edges:
            return []

        def score(e: SignifierEdge) -> float:
            tgt_degree = self._syn_degree.get(e.target, 1)
            if tgt_degree <= 0:
                tgt_degree = 1
            return e.weight / (tgt_degree ** alpha)

        return sorted(edges, key=lambda e: -score(e))[:n]

    # ------------------------------------------------------------------
    # 不可变操作（返回新 SNet）
    # ------------------------------------------------------------------

    def add_signifier(self, sig: Signifier) -> "SNet":
        """添加能指。若 id 已存在则覆盖。"""
        new_sigs = dict(self._signifiers)
        new_sigs[sig.id] = sig
        return SNet(new_sigs, self._edges)

    def add_edge(self, edge: SignifierEdge) -> "SNet":
        """添加边。不去重（允许累积权重后外部聚合）。"""
        return SNet(self._signifiers, self._edges + [edge])

    def merge_edge_weights(self) -> "SNet":
        """将同 (source, target, axis) 的边合并，weight 求和。

        用于从语料批量 add_edge 后的聚合。
        """
        merged: dict[tuple[str, str, AxisType], float] = {}
        evidences: dict[tuple[str, str, AxisType], list[str]] = {}
        for e in self._edges:
            key = (e.source, e.target, e.axis)
            merged[key] = merged.get(key, 0.0) + e.weight
            if e.evidence:
                evidences.setdefault(key, []).append(e.evidence)
        new_edges = [
            SignifierEdge(
                source=k[0],
                target=k[1],
                axis=k[2],
                weight=w,
                evidence=evidences.get(k, [""])[0],  # 只保留第一条证据
            )
            for k, w in merged.items()
        ]
        return SNet(self._signifiers, new_edges)

    def __repr__(self) -> str:
        return (
            f"SNet(signifiers={len(self._signifiers)}, "
            f"edges={len(self._edges)})"
        )


# ---------------------------------------------------------------------------
# 语域枚举
# ---------------------------------------------------------------------------

class Register(str, Enum):
    THEORETICAL = "theoretical"   # 理论层：精确定义、推导链
    OPERATIONAL = "operational"   # 操盘层：行动指令、条件判断
    DIALOGUE = "dialogue"         # 对话层：自然语言交互
    INTERNAL = "internal"         # 内部层：逢亮自身推理（不输出给用户）


# ---------------------------------------------------------------------------
# ConstraintSet
# ---------------------------------------------------------------------------

@dataclass
class ConstraintSet:
    """psi_L 传给 LLM 的约束包。

    字段说明：
      must_use:           必须出现的能指 id 列表（来自 K_active settlement）
      must_avoid:         禁止出现的能指 id 列表（来自 settlement 的反面）
      register:           语域（决定句法风格和词汇层次）
      narrative_spine:    穿越路径的叙事骨架（顶点序列的文字摘要）
      expression_pressure: 待表达的核心概念 id 列表（来自当前 K_active 活跃顶点）
      surface_forms:      可用的语言模板（从 S_net 组合轴提取）

    认识论等级：L0（数据结构定义）
    """
    must_use: list[str] = field(default_factory=list)
    must_avoid: list[str] = field(default_factory=list)
    register: Register = Register.DIALOGUE
    narrative_spine: str = ""
    expression_pressure: list[str] = field(default_factory=list)
    surface_forms: list[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        return {
            "must_use": self.must_use,
            "must_avoid": self.must_avoid,
            "register": self.register.value,
            "narrative_spine": self.narrative_spine,
            "expression_pressure": self.expression_pressure,
            "surface_forms": self.surface_forms,
        }

    def to_json(self, indent: int = 2) -> str:
        return json.dumps(self.to_dict(), ensure_ascii=False, indent=indent)


# ---------------------------------------------------------------------------
# ConstraintSet 工厂函数（degree-normalized 选择）
# ---------------------------------------------------------------------------

def build_constraint_set(
    snet: SNet,
    expression_pressure: list[str],
    must_use: list[str] | None = None,
    must_avoid: list[str] | None = None,
    register: Register = Register.DIALOGUE,
    narrative_spine: str = "",
    neighbors_per_concept: int = 3,
    degree_alpha: float = 0.5,
    max_surface_forms: int = 8,
) -> ConstraintSet:
    """从 SNet + expression_pressure 构建 ConstraintSet。

    使用 degree-normalized 邻居选择（修复 hub 垄断问题）：
    对每个 expression_pressure 中的概念，用 degree_normalized_neighbors
    而非 syntagmatic_neighbors，避免高度数 hub 劫持所有邻居选择。

    参数：
      snet:                S_net 实例（已 bootstrap）
      expression_pressure: 待表达的核心概念列表（按优先级）
      must_use:            强制使用的能指（None 时默认为 expression_pressure 前 3 个）
      must_avoid:          禁止使用的能指
      register:            语域
      narrative_spine:     叙事骨架描述
      neighbors_per_concept: 每个概念提取几条 degree-normalized 邻居
      degree_alpha:        degree-normalization 强度（0=无，0.5=默认，1=全归一化）
      max_surface_forms:   最多保留几条 surface_forms（避免 prompt 过长）

    Surface forms 提取逻辑：
      1. 对每个 expression_pressure 中的概念，取 degree-normalized 前 N 邻居
      2. 每个邻居边的 evidence 如果非空，加入 surface_forms
      3. 去重，截断到 max_surface_forms

    认识论等级：L0（代数操作，选择函数不依赖经验假设）
    """
    # must_use 默认为 expression_pressure 前 3 个（已在 snet 中的）
    if must_use is None:
        must_use = [
            s for s in expression_pressure[:3]
            if snet.has_signifier(s)
        ]

    # 从每个概念的 degree-normalized 邻居提取 surface forms
    surface_forms_set: list[str] = []
    seen_evidence: set[str] = set()

    for concept in expression_pressure:
        neighbors = snet.degree_normalized_neighbors(
            concept,
            n=neighbors_per_concept,
            alpha=degree_alpha,
        )
        for edge in neighbors:
            ev = edge.evidence.strip()
            if ev and ev not in seen_evidence:
                seen_evidence.add(ev)
                surface_forms_set.append(ev)
            if len(surface_forms_set) >= max_surface_forms:
                break
        if len(surface_forms_set) >= max_surface_forms:
            break

    return ConstraintSet(
        must_use=list(must_use),
        must_avoid=list(must_avoid) if must_avoid else [],
        register=register,
        narrative_spine=narrative_spine,
        expression_pressure=list(expression_pressure),
        surface_forms=surface_forms_set[:max_surface_forms],
    )


# ---------------------------------------------------------------------------
# ConstraintSet -> LLM prompt 转换
# ---------------------------------------------------------------------------

_REGISTER_PREAMBLES: dict[Register, str] = {
    Register.THEORETICAL: (
        "你正在处理一个概念精确性要求极高的理论表达任务。"
        "使用精确的术语定义，避免隐喻和模糊措辞。"
        "句法结构应清晰，每个判断都可追溯到明确的前提。"
    ),
    Register.OPERATIONAL: (
        "你正在处理一个操盘行动层的表达任务。"
        "使用简洁的条件-行动句式（如[若...则...]格式）。"
        "避免不必要的理论解释，聚焦于可执行的判断。"
    ),
    Register.DIALOGUE: (
        "你正在处理一个自然对话层的表达任务。"
        "语言应流畅自然，可以使用比喻和例子，但不失准确性。"
        "适度引导，不要将所有概念一次性倾倒。"
    ),
    Register.INTERNAL: (
        "你正在处理逢亮的内部推理层。"
        "输出用于逢亮内部状态更新，不直接给用户看。"
        "可以使用形式化符号和缩略表达。"
    ),
}


def constraint_set_to_prompt(cs: ConstraintSet) -> str:
    """将 ConstraintSet 转换为 LLM system prompt + user prompt 字符串。

    返回格式：
      [SYSTEM]\\n{system_text}\\n[USER]\\n{user_text}

    调用方按此分隔符拆分，分别传给 LLM 的 system/user 字段。
    """
    lines: list[str] = []

    # System: 语域前导
    preamble = _REGISTER_PREAMBLES.get(cs.register, "")
    lines.append("[SYSTEM]")
    lines.append(preamble)
    lines.append("")

    # System: 必须使用 / 禁止使用
    if cs.must_use:
        lines.append("必须使用以下能指（以其规范形式或可接受变体出现）：")
        for s in cs.must_use:
            lines.append(f"  - {s}")
        lines.append("")

    if cs.must_avoid:
        lines.append("禁止使用以下能指（包括其变体）：")
        for s in cs.must_avoid:
            lines.append(f"  - {s}")
        lines.append("")

    # System: 可用语言模板
    if cs.surface_forms:
        lines.append("可参考以下语言模板（来自缠论语料的真实表达）：")
        for sf in cs.surface_forms[:8]:  # 最多 8 条，避免 prompt 过长
            lines.append(f"  「{sf}」")
        lines.append("")

    # User: 叙事骨架 + 表达压力
    lines.append("[USER]")
    if cs.narrative_spine:
        lines.append(f"叙事骨架（概念穿越路径）：{cs.narrative_spine}")
        lines.append("")

    if cs.expression_pressure:
        lines.append("待表达的核心概念（按优先级排序）：")
        for i, concept in enumerate(cs.expression_pressure, 1):
            lines.append(f"  {i}. {concept}")
        lines.append("")

    lines.append("请在上述约束下，生成一段连贯的表达。")

    return "\n".join(lines)


def parse_llm_prompt(prompt: str) -> tuple[str, str]:
    """从 constraint_set_to_prompt 输出中拆分 system/user 部分。

    返回 (system_text, user_text)。
    若格式不符，返回 ("", prompt)。
    """
    if "[SYSTEM]" not in prompt or "[USER]" not in prompt:
        return ("", prompt)
    parts = prompt.split("[USER]", 1)
    system = parts[0].replace("[SYSTEM]", "").strip()
    user = parts[1].strip()
    return system, user


# ---------------------------------------------------------------------------
# 断裂追踪数据结构（Phase 4 接口，暂不实现检测逻辑）
# ---------------------------------------------------------------------------

class RuptureType(str, Enum):
    ABSENCE = "absence"                 # 预期能指缺位
    ANOMALOUS_CHOICE = "anomalous_choice"  # 异常替换（聚合轴偏差）
    SYNTACTIC_BREAK = "syntactic_break"   # 组合轴断裂
    UNKNOWN_SIGNIFIER = "unknown_signifier"  # S_net 盲区
    INSISTENCE = "insistence"           # 异常重复


@dataclass(frozen=True, slots=True)
class Rupture:
    """对话输入中检测到的断裂事件。

    rupture_type: 断裂类型
    signifier: 涉及的能指（或未知能指的原始字符串）
    significance: 显著性分数 [0.0, 1.0]
    context: 原始对话片段
    tuche_candidate: 是否提名为 tuché 候选（高显著性断裂 -> K_active encounter）
    """
    rupture_type: RuptureType
    signifier: str
    significance: float = 0.0
    context: str = ""
    tuche_candidate: bool = False


def detect_ruptures(
    text: str,
    snet: SNet,
    expected_signifiers: list[str] | None = None,
) -> list[Rupture]:
    """Phase 4 占位符：从对话文本中检测 S_net 层断裂。

    当前实现：仅检测 UNKNOWN_SIGNIFIER（S_net 盲区）。
    完整实现待 Phase 4。

    认识论等级：L1（占位符，仅检测一类断裂，无 L2 验证）
    """
    ruptures: list[Rupture] = []

    # 只做最基础的：检查已知能指是否出现在文本中
    # 缺位检测需要 expected_signifiers（由穿越路径给出）
    if expected_signifiers:
        for sid in expected_signifiers:
            if sid not in text:
                ruptures.append(Rupture(
                    rupture_type=RuptureType.ABSENCE,
                    signifier=sid,
                    significance=0.3,
                    context=text[:200],
                    tuche_candidate=False,
                ))

    return ruptures
