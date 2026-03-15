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

from cooccurrence_hyperedge import (
    CooccurrenceHyperedge,
    hyperedge_to_dict,
    hyperedge_from_dict,
)


# ---------------------------------------------------------------------------
# 轴类型
# ---------------------------------------------------------------------------

class AxisType(str, Enum):
    SYNTAGMATIC = "syntagmatic"   # 组合轴：共现/邻接
    PARADIGMATIC = "paradigmatic" # 聚合轴：替换/同义
    MORPHEME = "morpheme"         # 语素轴：语素共享连接


# ---------------------------------------------------------------------------
# S_net 核心数据结构
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class Signifier:
    """S_net 中的一个能指节点。

    id: 规范形式（通常是术语本身）
    surface_forms: 该能指在语料中出现的表层变体（list，不可变）
    source: 来源层 — 'k_active_projection' | 'corpus' | 'manual'
    lang: 语言标记 — 'zh' | 'en' | 'de' | 'fr' | ''(legacy)
    domain: 域标记（如 'hegel', 'chanlun'）
    """
    id: str
    surface_forms: tuple[str, ...] = ()
    source: str = "k_active_projection"
    lang: str = ""
    domain: str = ""


@dataclass(frozen=True, slots=True)
class SignifierEdge:
    """S_net 中的一条边。

    source: 来源能指 id
    target: 目标能指 id
    axis: 组合轴、聚合轴或语素轴
    weight: 共现强度（组合轴：PMI 值）或替换概率（聚合轴）
    evidence: 原始语言片段（surface），用于调试和叙事还原
    relation: 关系子类型 — 'synonym'|'contrast'|'translation'|'morpheme_link'|''
    differential: 翻译损失/增益描述（跨语言边专用）
    """
    source: str
    target: str
    axis: AxisType
    weight: float = 1.0
    evidence: str = ""
    relation: str = ""
    differential: str = ""


@dataclass(frozen=True, slots=True)
class Morpheme:
    """语素。

    form: 语素本身（如 "Auf", "中", "re-"）
    meaning: 语素含义
    lang: 语言
    shared_with: 该语素参与的其他能指 ID
    """
    form: str
    meaning: str
    lang: str
    shared_with: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class MorphemeStructure:
    """能指的语素分解结构。

    signifier_id: 对应的能指 ID
    morphemes: 语素序列
    etymology: 词源/说明
    """
    signifier_id: str
    morphemes: tuple[Morpheme, ...] = ()
    etymology: str = ""


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
        morphemes: dict[str, MorphemeStructure] | None = None,
        hyperedges: list[CooccurrenceHyperedge] | None = None,
    ) -> None:
        self._signifiers: dict[str, Signifier] = dict(signifiers) if signifiers else {}
        self._edges: list[SignifierEdge] = list(edges) if edges else []
        self._morphemes: dict[str, MorphemeStructure] = dict(morphemes) if morphemes else {}
        self._hyperedges: list[CooccurrenceHyperedge] = list(hyperedges) if hyperedges else []
        # 组合轴邻接索引（source -> list[SignifierEdge]）
        self._syn_out: dict[str, list[SignifierEdge]] = {}
        # 聚合轴替换索引（id -> list[SignifierEdge]）
        self._par_out: dict[str, list[SignifierEdge]] = {}
        # 语素轴索引（source -> list[SignifierEdge]）
        self._morpheme_out: dict[str, list[SignifierEdge]] = {}
        # 度数索引（组合轴，无向）
        self._syn_degree: dict[str, int] = {}
        for e in self._edges:
            if e.axis == AxisType.SYNTAGMATIC:
                self._syn_out.setdefault(e.source, []).append(e)
                self._syn_degree[e.source] = self._syn_degree.get(e.source, 0) + 1
                self._syn_degree[e.target] = self._syn_degree.get(e.target, 0) + 1
            elif e.axis == AxisType.MORPHEME:
                self._morpheme_out.setdefault(e.source, []).append(e)
            else:
                self._par_out.setdefault(e.source, []).append(e)
        # 超边索引（vertex_id -> list[hyperedge index]）
        self._vertex_to_hyperedges: dict[str, list[int]] = {}
        for idx, he in enumerate(self._hyperedges):
            for v in he.vertices:
                self._vertex_to_hyperedges.setdefault(v, []).append(idx)

    @property
    def signifiers(self) -> dict[str, Signifier]:
        return dict(self._signifiers)

    @property
    def edges(self) -> list[SignifierEdge]:
        return list(self._edges)

    @property
    def hyperedges(self) -> list[CooccurrenceHyperedge]:
        return list(self._hyperedges)

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

    def morpheme_links(self, sid: str) -> list[SignifierEdge]:
        """返回能指 sid 的所有语素轴连接（按 weight 降序）。"""
        edges = self._morpheme_out.get(sid, [])
        return sorted(edges, key=lambda e: -e.weight)

    def get_morpheme_structure(self, signifier_id: str) -> MorphemeStructure | None:
        """返回能指的语素分解结构，不存在时返回 None。"""
        return self._morphemes.get(signifier_id)

    def cooccurrence_weight(self, sid_a: str, sid_b: str) -> float:
        """返回两个能指之间的共现权重。

        优先使用超边计数（a 和 b 共同参与的超边数量）。
        如果无超边，fallback 到组合轴边权重（向后兼容旧数据）。
        """
        # 超边计数：共同参与的超边数量
        a_indices = set(self._vertex_to_hyperedges.get(sid_a, []))
        if a_indices:
            b_indices = set(self._vertex_to_hyperedges.get(sid_b, []))
            shared_count = len(a_indices & b_indices)
            if shared_count > 0:
                return float(shared_count)
        # Fallback: 组合轴边权重（旧路径，向后兼容）
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

    def hyperedges_containing(self, vertex_id: str) -> list[CooccurrenceHyperedge]:
        """返回包含指定顶点的所有超边。"""
        indices = self._vertex_to_hyperedges.get(vertex_id, [])
        return [self._hyperedges[i] for i in indices]

    def cooccurrence_context(
        self, a: str, b: str,
    ) -> list[CooccurrenceHyperedge]:
        """返回同时包含 a 和 b 的所有超边。"""
        a_indices = set(self._vertex_to_hyperedges.get(a, []))
        b_indices = set(self._vertex_to_hyperedges.get(b, []))
        shared = sorted(a_indices & b_indices)
        return [self._hyperedges[i] for i in shared]

    def derive_pairwise_edges(
        self, hyperedge: CooccurrenceHyperedge, weight: float = 1.0,
    ) -> list[SignifierEdge]:
        """从超边 lazy 派生成对组合轴边。

        将 N 个术语的超边展开为 C(N,2) 条 SignifierEdge。
        排序保证确定性：vertices 按字典序排列后生成有序对。

        认识论等级：L0（代数操作，从超边确定性生成成对边）
        """
        sorted_verts = sorted(hyperedge.vertices)
        edges: list[SignifierEdge] = []
        for i in range(len(sorted_verts)):
            for j in range(i + 1, len(sorted_verts)):
                edges.append(SignifierEdge(
                    source=sorted_verts[i],
                    target=sorted_verts[j],
                    axis=AxisType.SYNTAGMATIC,
                    weight=weight,
                    evidence=hyperedge.evidence_tag,
                ))
        return edges

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
        return SNet(new_sigs, self._edges, self._morphemes, self._hyperedges)

    def add_edge(self, edge: SignifierEdge) -> "SNet":
        """添加边。不去重（允许累积权重后外部聚合）。"""
        return SNet(self._signifiers, self._edges + [edge], self._morphemes, self._hyperedges)

    def add_edges(self, edges: list[SignifierEdge]) -> "SNet":
        """批量添加边（一次性创建新 SNet，避免逐条 add_edge 的 O(n^2) 复制）。"""
        if not edges:
            return self
        return SNet(self._signifiers, self._edges + edges, self._morphemes, self._hyperedges)

    def add_signifiers(self, sigs: list[Signifier]) -> "SNet":
        """批量添加能指（一次性创建新 SNet，避免逐条 add_signifier 的重复复制）。"""
        if not sigs:
            return self
        new_sigs = dict(self._signifiers)
        for sig in sigs:
            new_sigs[sig.id] = sig
        return SNet(new_sigs, self._edges, self._morphemes, self._hyperedges)

    def add_morpheme_structure(self, ms: MorphemeStructure) -> "SNet":
        """添加语素分解结构。若 signifier_id 已存在则覆盖。"""
        new_morphemes = dict(self._morphemes)
        new_morphemes[ms.signifier_id] = ms
        return SNet(self._signifiers, self._edges, new_morphemes, self._hyperedges)

    def add_hyperedge(self, he: CooccurrenceHyperedge) -> "SNet":
        """添加超边。返回新 SNet 实例。"""
        return SNet(self._signifiers, self._edges, self._morphemes,
                    self._hyperedges + [he])

    def add_hyperedges(self, hes: list[CooccurrenceHyperedge]) -> "SNet":
        """批量添加超边。返回新 SNet 实例。"""
        if not hes:
            return self
        return SNet(self._signifiers, self._edges, self._morphemes,
                    self._hyperedges + hes)

    def merge_edge_weights(self) -> "SNet":
        """将同 (source, target, axis) 的边合并，weight 求和。

        用于从语料批量 add_edge 后的聚合。
        """
        merged: dict[tuple[str, str, AxisType], float] = {}
        evidences: dict[tuple[str, str, AxisType], list[str]] = {}
        relations: dict[tuple[str, str, AxisType], str] = {}
        differentials: dict[tuple[str, str, AxisType], str] = {}
        for e in self._edges:
            key = (e.source, e.target, e.axis)
            merged[key] = merged.get(key, 0.0) + e.weight
            if e.evidence:
                evidences.setdefault(key, []).append(e.evidence)
            if e.relation and key not in relations:
                relations[key] = e.relation
            if e.differential and key not in differentials:
                differentials[key] = e.differential
        new_edges = [
            SignifierEdge(
                source=k[0],
                target=k[1],
                axis=k[2],
                weight=w,
                evidence=evidences.get(k, [""])[0],  # 只保留第一条证据
                relation=relations.get(k, ""),
                differential=differentials.get(k, ""),
            )
            for k, w in merged.items()
        ]
        return SNet(self._signifiers, new_edges, self._morphemes, self._hyperedges)

    # ------------------------------------------------------------------
    # 序列化 / 反序列化（持久化缓存用）
    # ------------------------------------------------------------------

    def to_dict(self) -> dict:
        """将 SNet 序列化为纯 Python dict（可 JSON / pickle）。

        包含所有数据：signifiers、edges、morphemes。
        索引（_syn_out 等）不序列化——从 edges 重建。

        认识论等级：L0（无损序列化，代数等价）
        """
        return {
            "signifiers": {
                sid: {
                    "id": sig.id,
                    "surface_forms": list(sig.surface_forms),
                    "source": sig.source,
                    "lang": sig.lang,
                    "domain": sig.domain,
                }
                for sid, sig in self._signifiers.items()
            },
            "edges": [
                {
                    "source": e.source,
                    "target": e.target,
                    "axis": e.axis.value,
                    "weight": e.weight,
                    "evidence": e.evidence,
                    "relation": e.relation,
                    "differential": e.differential,
                }
                for e in self._edges
            ],
            "morphemes": {
                sid: {
                    "signifier_id": ms.signifier_id,
                    "morphemes": [
                        {
                            "form": m.form,
                            "meaning": m.meaning,
                            "lang": m.lang,
                            "shared_with": list(m.shared_with),
                        }
                        for m in ms.morphemes
                    ],
                    "etymology": ms.etymology,
                }
                for sid, ms in self._morphemes.items()
            },
            "hyperedges": [
                hyperedge_to_dict(he) for he in self._hyperedges
            ],
        }

    @classmethod
    def from_dict(cls, data: dict) -> "SNet":
        """从 to_dict() 输出重建 SNet。

        索引从 edges 自动重建（SNet.__init__ 内部完成）。

        认识论等级：L0（无损反序列化，代数等价）
        """
        signifiers: dict[str, Signifier] = {}
        for sid, sd in data.get("signifiers", {}).items():
            signifiers[sid] = Signifier(
                id=sd["id"],
                surface_forms=tuple(sd.get("surface_forms", ())),
                source=sd.get("source", "k_active_projection"),
                lang=sd.get("lang", ""),
                domain=sd.get("domain", ""),
            )

        edges: list[SignifierEdge] = []
        for ed in data.get("edges", []):
            edges.append(SignifierEdge(
                source=ed["source"],
                target=ed["target"],
                axis=AxisType(ed["axis"]),
                weight=ed.get("weight", 1.0),
                evidence=ed.get("evidence", ""),
                relation=ed.get("relation", ""),
                differential=ed.get("differential", ""),
            ))

        morphemes: dict[str, MorphemeStructure] = {}
        for sid, md in data.get("morphemes", {}).items():
            morph_list = tuple(
                Morpheme(
                    form=m["form"],
                    meaning=m["meaning"],
                    lang=m["lang"],
                    shared_with=tuple(m.get("shared_with", ())),
                )
                for m in md.get("morphemes", [])
            )
            morphemes[sid] = MorphemeStructure(
                signifier_id=md["signifier_id"],
                morphemes=morph_list,
                etymology=md.get("etymology", ""),
            )

        hyperedges: list[CooccurrenceHyperedge] = [
            hyperedge_from_dict(hd) for hd in data.get("hyperedges", [])
        ]

        return cls(signifiers=signifiers, edges=edges, morphemes=morphemes,
                   hyperedges=hyperedges)

    def __repr__(self) -> str:
        return (
            f"SNet(signifiers={len(self._signifiers)}, "
            f"edges={len(self._edges)}, "
            f"hyperedges={len(self._hyperedges)}, "
            f"morphemes={len(self._morphemes)})"
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
# 断裂追踪数据结构 + 五种断裂检测（Phase 5）
# ---------------------------------------------------------------------------

class RuptureType(str, Enum):
    METAPHOR = "metaphor"               # 隐喻替代：能指 A 出现在 B 的典型位置
    METONYMY = "metonymy"               # 换喻滑动：能指链中相邻能指在 S_net 中不相邻
    FORECLOSURE = "foreclosure"         # 排除：缺少高 PMI 必现伴随能指
    REPETITION = "repetition"           # 重复强迫：同一能指反复出现
    CONDENSATION = "condensation"       # 凝缩：一个能指承载多个 S_net 节点的语义负载


@dataclass(frozen=True, slots=True)
class Rupture:
    """对话输入中检测到的断裂事件。

    rupture_type: 断裂类型
    signifier: 涉及的能指（或未知能指的原始字符串）
    significance: 显著性分数 [0.0, 1.0]
    context: 原始对话片段
    tuche_candidate: 是否提名为 tuché 候选（高显著性断裂 -> K_active encounter）
    detail: 检测细节（可选，用于调试和叙事还原）
    """
    rupture_type: RuptureType
    signifier: str
    significance: float = 0.0
    context: str = ""
    tuche_candidate: bool = False
    detail: str = ""


# ---------------------------------------------------------------------------
# tuché 候选阈值
# ---------------------------------------------------------------------------

_TUCHE_THRESHOLD = 0.6  # significance >= 此值提名为 tuché 候选


# ---------------------------------------------------------------------------
# 五种断裂检测器
# ---------------------------------------------------------------------------

def _detect_metaphor(
    signifier_chain: list[str],
    snet: SNet,
) -> list[Rupture]:
    """隐喻替代检测：能指 A 出现在 S_net 中能指 B 的典型位置。

    检测逻辑：
      对能指链中每个能指 A（在 S_net 中），检查 A 的组合轴邻居在链中
      是否更典型地与另一个能指 B 共现。如果 B 的高权重邻居包含 A 的链邻居
      但 A 本身不是 B 的典型邻居，则 A 在 B 的位置上——隐喻替代。

    认识论等级：L0（基于 S_net 拓扑的确定性检测）
    """
    ruptures: list[Rupture] = []
    if len(signifier_chain) < 2:
        return ruptures

    chain_set = set(signifier_chain)

    for i, sig_a in enumerate(signifier_chain):
        if not snet.has_signifier(sig_a):
            continue

        # A 在链中的邻居（前后各一）
        chain_neighbors: list[str] = []
        if i > 0:
            chain_neighbors.append(signifier_chain[i - 1])
        if i < len(signifier_chain) - 1:
            chain_neighbors.append(signifier_chain[i + 1])

        chain_neighbors_in_snet = [cn for cn in chain_neighbors if snet.has_signifier(cn)]
        if not chain_neighbors_in_snet:
            continue

        # 对每个链邻居 cn，查看 cn 的 top 邻居中谁最典型地出现在该位置
        for cn in chain_neighbors_in_snet:
            cn_top = snet.top_neighbors(cn, n=5)
            # A 不在 cn 的 top 邻居中，但 cn 的某个 top 邻居 B 也在链中
            if sig_a not in cn_top:
                for b_candidate in cn_top:
                    if b_candidate != sig_a and b_candidate in chain_set:
                        # A 出现在 B 通常出现的位置（cn 的邻域）
                        sig = min(0.9, 0.4 + snet.cooccurrence_weight(cn, b_candidate) * 0.1)
                        ruptures.append(Rupture(
                            rupture_type=RuptureType.METAPHOR,
                            signifier=sig_a,
                            significance=sig,
                            context=f"{sig_a} 替代 {b_candidate} 出现在 {cn} 的邻域",
                            tuche_candidate=sig >= _TUCHE_THRESHOLD,
                            detail=f"chain_pos={i}, expected={b_candidate}, neighbor={cn}",
                        ))
                        break  # 每个 (sig_a, cn) 对只报告一次

    return ruptures


def _detect_metonymy(
    signifier_chain: list[str],
    snet: SNet,
) -> list[Rupture]:
    """换喻滑动检测：能指链中相邻能指在 S_net 中不相邻（滑动）。

    检测逻辑：
      对能指链中每对相邻能指 (A, B)，如果两者都在 S_net 中但 S_net
      中没有组合轴边（cooccurrence_weight == 0），说明链条在 S_net 上
      发生了滑动——换喻。

    显著性：与两个能指各自的度数相关。两个高度数节点无连接比
    两个低度数节点无连接更显著。

    认识论等级：L0（基于 S_net 拓扑的确定性检测）
    """
    ruptures: list[Rupture] = []
    if len(signifier_chain) < 2:
        return ruptures

    for i in range(len(signifier_chain) - 1):
        a, b = signifier_chain[i], signifier_chain[i + 1]
        if not snet.has_signifier(a) or not snet.has_signifier(b):
            continue

        weight = snet.cooccurrence_weight(a, b)
        # 也检查反向（S_net 边可能是单向存储的）
        if weight == 0.0:
            weight = snet.cooccurrence_weight(b, a)

        if weight == 0.0:
            # 无共现：滑动
            deg_a = snet.syn_degree(a)
            deg_b = snet.syn_degree(b)
            # 高度数节点之间无连接更显著
            sig = min(0.9, 0.3 + math.log1p(deg_a + deg_b) * 0.1)
            ruptures.append(Rupture(
                rupture_type=RuptureType.METONYMY,
                signifier=f"{a}->{b}",
                significance=sig,
                context=f"链中 {a} 和 {b} 相邻但 S_net 中无共现边",
                tuche_candidate=sig >= _TUCHE_THRESHOLD,
                detail=f"chain_pos={i}-{i+1}, deg_a={deg_a}, deg_b={deg_b}",
            ))

    return ruptures


def _detect_foreclosure(
    signifier_chain: list[str],
    snet: SNet,
) -> list[Rupture]:
    """排除检测：输入中缺少 S_net 中高 PMI 的必现伴随能指。

    检测逻辑：
      对能指链中每个在 S_net 中的能指 A，取 A 的 top-3 高权重
      组合轴邻居。如果这些高 PMI 伙伴都不在能指链中，则伙伴被
      foreclosed（排除）——缺位比一般的缺席更强，因为高 PMI 意味
      着"几乎总是一起出现"。

    认识论等级：L0（基于 S_net 权重的确定性检测）
    """
    ruptures: list[Rupture] = []
    chain_set = set(signifier_chain)

    for sig in signifier_chain:
        if not snet.has_signifier(sig):
            continue

        top_neighbors = snet.syntagmatic_neighbors(sig)[:3]
        if not top_neighbors:
            continue

        for edge in top_neighbors:
            if edge.target not in chain_set:
                # 高 PMI 伙伴缺位
                sig_score = min(0.9, 0.3 + edge.weight * 0.15)
                ruptures.append(Rupture(
                    rupture_type=RuptureType.FORECLOSURE,
                    signifier=edge.target,
                    significance=sig_score,
                    context=f"{edge.target} 是 {sig} 的高 PMI 伙伴(w={edge.weight:.2f})但未出现",
                    tuche_candidate=sig_score >= _TUCHE_THRESHOLD,
                    detail=f"anchor={sig}, missing={edge.target}, weight={edge.weight:.3f}",
                ))

    return ruptures


def _detect_repetition(
    signifier_chain: list[str],
    snet: SNet,
) -> list[Rupture]:
    """重复强迫检测：同一能指在输入中反复出现。

    检测逻辑：
      统计能指链中每个能指的出现次数。出现 >= 2 次的能指标记为重复。
      显著性随重复次数增长（对数增长，避免爆炸）。

    注意：signifier_chain 是去重的（parse_signifier_chain 策略2 去重），
    所以这里直接在原始文本的切分结果上计数。但由于我们接收的是去重后
    的链，这里用一个替代方法：在链中能指里，检查原始文本中该能指出现
    的次数（通过 text 参数传入）。

    但 detect_ruptures 不接收原始 text（为了解耦），所以我们在
    signifier_chain 中查重复——如果调用方传入的是未去重链则直接计数。

    认识论等级：L0（字符串计数操作）
    """
    ruptures: list[Rupture] = []

    counts: dict[str, int] = {}
    for sig in signifier_chain:
        counts[sig] = counts.get(sig, 0) + 1

    for sig, count in counts.items():
        if count >= 2:
            sig_score = min(0.9, 0.3 + math.log(count) * 0.3)
            ruptures.append(Rupture(
                rupture_type=RuptureType.REPETITION,
                signifier=sig,
                significance=sig_score,
                context=f"{sig} 在能指链中出现 {count} 次",
                tuche_candidate=sig_score >= _TUCHE_THRESHOLD,
                detail=f"count={count}",
            ))

    return ruptures


def _detect_condensation(
    signifier_chain: list[str],
    snet: SNet,
) -> list[Rupture]:
    """凝缩检测：一个能指在输入中承载了多个 S_net 节点的语义负载。

    检测逻辑：
      对能指链中每个在 S_net 中的能指 A，如果 A 的多个高权重邻居
      也在能指链中（即 A 同时扮演多个邻居的共现伙伴），则 A 承载了
      凝缩的语义负载——一个能指节点吸引了过多的关联。

    阈值：A 在链中命中的邻居数 >= 3 时触发凝缩。

    认识论等级：L0（基于 S_net 拓扑的确定性检测）
    """
    ruptures: list[Rupture] = []
    chain_set = set(signifier_chain)
    _CONDENSATION_THRESHOLD = 3

    for sig in signifier_chain:
        if not snet.has_signifier(sig):
            continue

        neighbors = snet.syntagmatic_neighbors(sig)
        # 在链中命中的邻居
        hits = [e for e in neighbors if e.target in chain_set and e.target != sig]

        if len(hits) >= _CONDENSATION_THRESHOLD:
            hit_ids = [e.target for e in hits]
            sig_score = min(0.9, 0.3 + len(hits) * 0.1)
            ruptures.append(Rupture(
                rupture_type=RuptureType.CONDENSATION,
                signifier=sig,
                significance=sig_score,
                context=f"{sig} 同时关联 {len(hits)} 个链中邻居: {', '.join(hit_ids[:5])}",
                tuche_candidate=sig_score >= _TUCHE_THRESHOLD,
                detail=f"hit_count={len(hits)}, hits={hit_ids[:5]}",
            ))

    return ruptures


# ---------------------------------------------------------------------------
# detect_ruptures — 主入口（五种断裂检测）
# ---------------------------------------------------------------------------

def detect_ruptures(
    signifier_chain: list[str],
    snet: SNet,
) -> list[Rupture]:
    """从能指链中检测 S_net 层断裂（五种类型）。

    输入：
      signifier_chain: 由 parse_signifier_chain() 从对话文本提取的有序能指链
      snet:            已 bootstrap 的 S_net（71 signifiers, 276 edges）

    检测类型：
      1. Metaphor（隐喻替代）：能指 A 出现在 B 的典型位置
      2. Metonymy（换喻滑动）：链中相邻能指在 S_net 中不相邻
      3. Foreclosure（排除）：缺少高 PMI 必现伴随能指
      4. Repetition（重复强迫）：同一能指反复出现
      5. Condensation（凝缩）：一个能指承载多个节点的语义负载

    显著性 >= 0.6 的断裂提名为 tuché 候选。

    认识论等级：L0（基于 S_net 拓扑的确定性检测，无统计假设）
    """
    if not signifier_chain or not snet.signifiers:
        return []

    ruptures: list[Rupture] = []
    ruptures.extend(_detect_metaphor(signifier_chain, snet))
    ruptures.extend(_detect_metonymy(signifier_chain, snet))
    ruptures.extend(_detect_foreclosure(signifier_chain, snet))
    ruptures.extend(_detect_repetition(signifier_chain, snet))
    ruptures.extend(_detect_condensation(signifier_chain, snet))

    # 按显著性降序排列
    ruptures.sort(key=lambda r: -r.significance)

    return ruptures


# ---------------------------------------------------------------------------
# 断裂日志持久化
# ---------------------------------------------------------------------------

def persist_rupture_log(
    input_text: str,
    signifier_chain: list[str],
    ruptures: list[Rupture],
    log_dir: Path | None = None,
) -> Path | None:
    """将断裂检测结果持久化到 rupture_logs/ 目录。

    每条日志包含：
      - timestamp: Unix 时间戳
      - input_text: 原始输入文本（截断到 2000 字符）
      - signifier_chain: 提取的能指链
      - ruptures: 检测到的断裂列表
      - tuche_candidates: tuché 候选能指列表

    日志格式：JSONL（追加写入）。

    返回：日志文件路径，失败时返回 None。
    """
    import time as _time

    if log_dir is None:
        log_dir = Path(__file__).resolve().parent / "rupture_logs"

    try:
        log_dir.mkdir(exist_ok=True)
        log_path = log_dir / "ruptures.jsonl"

        entry = {
            "timestamp": _time.time(),
            "input_text": input_text[:2000],
            "signifier_chain": signifier_chain,
            "rupture_count": len(ruptures),
            "ruptures": [
                {
                    "type": r.rupture_type.value,
                    "signifier": r.signifier,
                    "significance": round(r.significance, 3),
                    "context": r.context,
                    "tuche_candidate": r.tuche_candidate,
                    "detail": r.detail,
                }
                for r in ruptures
            ],
            "tuche_candidates": [
                r.signifier for r in ruptures if r.tuche_candidate
            ],
        }

        with open(log_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")

        return log_path

    except Exception:
        return None


# ---------------------------------------------------------------------------
# 对话回写（writeback）— 从对话文本中提取术语共现关系回写到 S_net
# ---------------------------------------------------------------------------

def phi_L_whitelist_match(
    text: str,
    whitelist: set[str],
) -> list[tuple[str, int]]:
    """在文本中定位白名单术语，按位置排序返回。

    返回 list[tuple[str, int]]：(术语, 首次出现位置)。
    同一术语只返回首次出现。长术语优先匹配（避免短术语吞噬长术语的子串）。

    认识论等级：L0（确定性字符串操作）
    """
    if not text or not whitelist:
        return []

    # 按长度降序排列，优先匹配长术语
    sorted_terms = sorted(whitelist, key=len, reverse=True)
    found: list[tuple[str, int]] = []
    seen: set[str] = set()

    for term in sorted_terms:
        if term in seen:
            continue
        pos = text.find(term)
        if pos >= 0:
            found.append((term, pos))
            seen.add(term)

    # 按位置排序
    found.sort(key=lambda t: t[1])
    return found


def extract_between(
    text: str,
    term_a: str,
    pos_a: int,
    term_b: str,
    pos_b: int,
) -> str:
    """提取两个术语之间的连接文本片段。

    返回 term_a 和 term_b 之间的文本（去掉首尾空白）。
    如果两个术语重叠或间距过大（>100字符），返回空字符串。

    认识论等级：L0（确定性字符串操作）
    """
    start = pos_a + len(term_a)
    end = pos_b
    if start >= end or (end - start) > 100:
        return ""
    return text[start:end].strip()


def writeback_from_text(
    snet: SNet,
    text: str,
    whitelist: set[str],
    source_type: str,
    timestamp: str,
) -> tuple[SNet, list[dict]]:
    """从文本中提取白名单术语共现对，回写到 S_net。

    遵循不可变操作模式：返回新的 SNet 实例（不修改传入的 snet）。
    同时返回回写日志条目列表（用于持久化到 generation_log.jsonl）。

    参数：
      snet:        当前 S_net 实例
      text:        对话文本（用户输入或 LLM 输出）
      whitelist:   白名单术语集合（S_net 中所有能指 id）
      source_type: 来源类型标记
                   - 'operator_dialogue': 操作者对话输入
                   - 'llm_generation': LLM 语法填充产出
                   - 'self_traversal': 逢亮自己的微穿越产出
                   - 'agent_interrogation': 询问代理的回应
      timestamp:   时间戳字符串

    返回：
      (new_snet, log_entries)
      - new_snet: 包含新增共现边的 SNet（已 merge_edge_weights）
      - log_entries: 回写日志条目列表，每条包含 source/target/pattern/corpus_ref

    认识论等级：L0（确定性字符串操作 + S_net 拓扑更新）
    """
    matches = phi_L_whitelist_match(text, whitelist)
    if len(matches) < 2:
        return snet, []

    new_snet = snet
    log_entries: list[dict] = []

    for i in range(len(matches) - 1):
        sig_a, pos_a = matches[i]
        sig_b, pos_b = matches[i + 1]

        # 两个术语都必须在 S_net 中
        if not new_snet.has_signifier(sig_a) or not new_snet.has_signifier(sig_b):
            continue

        # 提取连接模式
        pattern = extract_between(text, sig_a, pos_a, sig_b, pos_b)

        # 添加组合轴边（weight=1.0，后续 merge_edge_weights 会累积）
        edge = SignifierEdge(
            source=sig_a,
            target=sig_b,
            axis=AxisType.SYNTAGMATIC,
            weight=1.0,
            evidence=pattern if pattern else "",
        )
        new_snet = new_snet.add_edge(edge)

        corpus_ref = f"{source_type}:{timestamp}"
        log_entries.append({
            "source": sig_a,
            "target": sig_b,
            "pattern": pattern,
            "corpus_ref": corpus_ref,
        })

    # 合并同 (source, target, axis) 的边权重
    if log_entries:
        new_snet = new_snet.merge_edge_weights()

    return new_snet, log_entries
