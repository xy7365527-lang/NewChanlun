"""snet_activation.py — S_net 激活态管理 + 耦合振荡.

K_active 每步进一步，S_net 同步共振。S_net 的共振反馈给 K_active
的下一步选择（pull，不是 override）。

核心类：
  SNetActivation — 管理 S_net 的激活态（当前处于激活状态的能指节点集合）

数据结构：
  InternalSpeechFragment — 被清出的能指如果构成连贯语段，存入内部言语缓冲区
  EdgeSuggestion — S_net 中两个激活能指有组合轴连接，但对应概念在 K_active 中
                   没有 edge → 注册为 edge_suggestion（articulation feedback）

认识论等级：L0（数据结构 + 拓扑操作，无经验假设）
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Optional

from signifier_net import SNet, AxisType


# ---------------------------------------------------------------------------
# 数据结构
# ---------------------------------------------------------------------------

@dataclass(frozen=True, slots=True)
class InternalSpeechFragment:
    """被清出的能指如果构成连贯语段 → 存入 internal_speech_buffer.

    signifiers:          构成语段的能指列表
    connective_patterns: 能指之间的连接模式（来自 S_net 边的 evidence）
    formed_at_step:      生成时的穿越步数
    source_concepts:     对应的 K_active 概念 ID
    externalized:        是否已被 psi_L 消费
    """
    signifiers: tuple[str, ...]
    connective_patterns: tuple[str, ...]
    formed_at_step: int
    source_concepts: tuple[str, ...]
    externalized: bool = False

    def to_dict(self) -> dict:
        return {
            "signifiers": list(self.signifiers),
            "connective_patterns": list(self.connective_patterns),
            "formed_at_step": self.formed_at_step,
            "source_concepts": list(self.source_concepts),
            "externalized": self.externalized,
        }

    @staticmethod
    def from_dict(d: dict) -> "InternalSpeechFragment":
        return InternalSpeechFragment(
            signifiers=tuple(d.get("signifiers", [])),
            connective_patterns=tuple(d.get("connective_patterns", [])),
            formed_at_step=d.get("formed_at_step", 0),
            source_concepts=tuple(d.get("source_concepts", [])),
            externalized=d.get("externalized", False),
        )


@dataclass(frozen=True, slots=True)
class EdgeSuggestion:
    """articulation feedback: S_net 组合轴连接 → K_active 缺失 edge 建议.

    source_concept:     源概念 ID (K_active vertex id)
    target_concept:     目标概念 ID (K_active vertex id)
    evidence_signifiers: 产生建议的能指对
    evidence_patterns:  组合轴边的 evidence 字符串列表
    origin:             来源标记 ("articulation_feedback")
    step:               生成时的穿越步数
    reviewed:           是否已被审查
    """
    source_concept: str
    target_concept: str
    evidence_signifiers: tuple[str, str]
    evidence_patterns: tuple[str, ...]
    origin: str = "articulation_feedback"
    step: int = 0
    reviewed: bool = False

    def to_dict(self) -> dict:
        return {
            "source_concept": self.source_concept,
            "target_concept": self.target_concept,
            "evidence_signifiers": list(self.evidence_signifiers),
            "evidence_patterns": list(self.evidence_patterns),
            "origin": self.origin,
            "step": self.step,
            "reviewed": self.reviewed,
        }

    @staticmethod
    def from_dict(d: dict) -> "EdgeSuggestion":
        ev_sigs = d.get("evidence_signifiers", ["", ""])
        return EdgeSuggestion(
            source_concept=d.get("source_concept", ""),
            target_concept=d.get("target_concept", ""),
            evidence_signifiers=(ev_sigs[0], ev_sigs[1]) if len(ev_sigs) >= 2 else ("", ""),
            evidence_patterns=tuple(d.get("evidence_patterns", [])),
            origin=d.get("origin", "articulation_feedback"),
            step=d.get("step", 0),
            reviewed=d.get("reviewed", False),
        )


# ---------------------------------------------------------------------------
# concept_id <-> signifier_id 映射辅助
# ---------------------------------------------------------------------------

def _build_concept_to_signifier(s_net: SNet, graph) -> dict[str, str]:
    """构建 K_active concept_id -> signifier_id 映射.

    Layer A bootstrap 中，Signifier.id = vertex.content，
    Signifier.surface_forms 包含原始 vertex.id。
    所以映射逻辑：vertex.content 如果在 S_net 中 → 映射到该能指。

    参数：
      s_net: 已 bootstrap 的 SNet
      graph: K_active 图（提供 vertex.id -> vertex.content 映射）

    返回：
      dict[concept_id, signifier_id]
    """
    mapping: dict[str, str] = {}
    for vid in graph.active_vertex_ids():
        v = graph.vertices.get(vid)
        if v is None or not v.content:
            continue
        content = v.content.strip()
        if content and s_net.has_signifier(content):
            mapping[vid] = content
    return mapping


def _build_signifier_to_concepts(s_net: SNet, graph) -> dict[str, list[str]]:
    """构建 signifier_id -> [concept_id] 反向映射.

    一个能指可能对应多个概念顶点（不同 vertex.id 但相同 content）。
    """
    mapping: dict[str, list[str]] = {}
    for vid in graph.active_vertex_ids():
        v = graph.vertices.get(vid)
        if v is None or not v.content:
            continue
        content = v.content.strip()
        if content and s_net.has_signifier(content):
            mapping.setdefault(content, []).append(vid)
    return mapping


# ---------------------------------------------------------------------------
# SNetActivation
# ---------------------------------------------------------------------------

class SNetActivation:
    """管理 S_net 的激活态——当前处于激活状态的能指节点集合.

    K_active 步进到新概念时调用 activate()，激活对应能指，
    清理失去连接的旧激活。被清出的能指如果构成连贯语段，
    存入 internal_speech_buffer。
    """

    def __init__(
        self,
        s_net: SNet,
        concept_to_signifier: dict[str, str],
        signifier_to_concepts: dict[str, list[str]],
    ) -> None:
        self.s_net = s_net
        self._concept_to_sig = dict(concept_to_signifier)
        self._sig_to_concepts = dict(signifier_to_concepts)
        self.currently_active: set[str] = set()
        self._dialogue_focus_set: set[str] = set()
        self.internal_speech_buffer: list[InternalSpeechFragment] = []
        self.edge_suggestions: list[EdgeSuggestion] = []
        self.current_step: int = 0

    def activate(self, concept_id: str) -> None:
        """K_active 步进到新概念时调用.

        激活对应能指，清理失去组合轴连接的旧激活。
        """
        new_signifier = self._concept_to_sig.get(concept_id)
        if not new_signifier:
            return

        new_active: set[str] = {new_signifier}

        # 保留与新能指有组合轴连接的旧激活
        for sig in self.currently_active:
            if sig == new_signifier:
                continue
            if self._has_syntagmatic_edge(sig, new_signifier):
                new_active.add(sig)

        self._check_fragment_formation(self.currently_active, new_active)
        self.currently_active = new_active

    def resonate(self, signifier_ids: list[str]) -> None:
        """Operator 输入引起的共振——通过对话焦点集控制激活态.

        与 activate() 的区别：
          activate() 是穿越引擎的焦点切换（单点），resonate() 是对话累积（多点焦点集）。

        对话焦点集（_dialogue_focus_set）= 最近几轮对话中通过 phi_L 匹配到的能指。
        能指保持激活只要它和焦点集中任一成员有组合轴连接。
        焦点集本身随对话推进而更新：
          1. 新能指加入焦点集
          2. 旧焦点集成员如果和新能指中的任何一个都没有组合轴连接 → 退出焦点集
          3. currently_active 中不在焦点集、且和焦点集中任一成员都没有组合轴连接的 → 退出

        不用衰减（权重/时间/频率衰减全部被否定）——只用拓扑判据。
        """
        if not signifier_ids:
            return

        # 验证并收集有效的新能指
        incoming: set[str] = set()
        for sid in signifier_ids:
            if self.s_net.has_signifier(sid):
                incoming.add(sid)

        if not incoming:
            return

        # --- 步骤1：新能指加入焦点集 ---
        # --- 步骤2：焦点集内部清理 ---
        # 旧焦点集成员如果和所有新能指都没有组合轴连接 → 退出焦点集
        surviving_focus: set[str] = set(incoming)
        for old_focus in self._dialogue_focus_set:
            if old_focus in incoming:
                surviving_focus.add(old_focus)
                continue
            has_link_to_incoming = any(
                self._has_syntagmatic_edge(old_focus, new_sig)
                for new_sig in incoming
            )
            if has_link_to_incoming:
                surviving_focus.add(old_focus)

        self._dialogue_focus_set = surviving_focus

        # --- 步骤3：currently_active 清理 ---
        # 焦点集成员一定保留在 currently_active 中
        # 非焦点集成员：和焦点集中任一成员有组合轴连接 → 保留，否则退出
        new_active: set[str] = set(self._dialogue_focus_set)
        for sig in self.currently_active:
            if sig in new_active:
                continue
            has_link_to_focus = any(
                self._has_syntagmatic_edge(sig, focus_sig)
                for focus_sig in self._dialogue_focus_set
            )
            if has_link_to_focus:
                new_active.add(sig)

        self._check_fragment_formation(self.currently_active, new_active)
        self.currently_active = new_active

    def _has_syntagmatic_edge(self, sig_a: str, sig_b: str) -> bool:
        """检查两个能指之间是否有组合轴连接（任意方向）."""
        w = self.s_net.cooccurrence_weight(sig_a, sig_b)
        if w > 0.0:
            return True
        w = self.s_net.cooccurrence_weight(sig_b, sig_a)
        return w > 0.0

    def _check_fragment_formation(
        self,
        old_active: set[str],
        new_active: set[str],
    ) -> None:
        """被清出的能指如果构成连贯语段 → 存入 internal_speech_buffer."""
        dropped = old_active - new_active
        if len(dropped) < 2:
            return

        fragment_signifiers = self._find_connected_subset(dropped)
        if fragment_signifiers and len(fragment_signifiers) >= 2:
            connectives = self._extract_connectives(fragment_signifiers)
            source_concepts = self._get_concept_refs(fragment_signifiers)
            fragment = InternalSpeechFragment(
                signifiers=tuple(sorted(fragment_signifiers)),
                connective_patterns=tuple(connectives),
                formed_at_step=self.current_step,
                source_concepts=tuple(source_concepts),
            )
            self.internal_speech_buffer.append(fragment)

    def _find_connected_subset(self, signifiers: set[str]) -> set[str]:
        """从能指集合中找到最大的组合轴连通子集.

        使用 BFS 从第一个能指开始，沿组合轴边扩展。
        """
        if not signifiers:
            return set()

        sig_list = sorted(signifiers)
        visited: set[str] = set()
        queue = [sig_list[0]]
        visited.add(sig_list[0])

        while queue:
            current = queue.pop(0)
            for other in sig_list:
                if other in visited:
                    continue
                if self._has_syntagmatic_edge(current, other):
                    visited.add(other)
                    queue.append(other)

        return visited

    def _extract_connectives(self, signifiers: set[str]) -> list[str]:
        """提取能指集合中组合轴边的 evidence 字符串."""
        connectives: list[str] = []
        sig_list = sorted(signifiers)
        for i, sig_a in enumerate(sig_list):
            for sig_b in sig_list[i + 1:]:
                for edge in self.s_net.syntagmatic_neighbors(sig_a):
                    if edge.target == sig_b and edge.evidence:
                        connectives.append(edge.evidence)
                        break
        return connectives

    def _get_concept_refs(self, signifiers: set[str]) -> list[str]:
        """获取能指集合对应的 K_active 概念 ID."""
        refs: list[str] = []
        for sig in sorted(signifiers):
            concepts = self._sig_to_concepts.get(sig, [])
            refs.extend(concepts[:1])  # 每个能指取第一个概念
        return refs

    def get_resonating(self) -> set[str]:
        """返回当前激活态能指的所有组合轴邻居（共振区域）.

        共振区域 = 激活态能指的组合轴邻居集合 - 已激活能指自身。
        """
        resonating: set[str] = set()
        for sig in self.currently_active:
            for edge in self.s_net.syntagmatic_neighbors(sig):
                if edge.target not in self.currently_active:
                    resonating.add(edge.target)
        return resonating

    def get_resonating_concepts(self) -> set[str]:
        """返回共振区域对应的 K_active 概念 ID 集合."""
        resonating_sigs = self.get_resonating()
        concepts: set[str] = set()
        for sig in resonating_sigs:
            for cid in self._sig_to_concepts.get(sig, []):
                concepts.add(cid)
        return concepts

    def check_articulation_feedback(self, graph) -> list[EdgeSuggestion]:
        """检查 articulation feedback.

        S_net 中两个激活能指有组合轴连接，但对应概念在 K_active 中
        没有 edge → 注册为 edge_suggestion。
        """
        suggestions: list[EdgeSuggestion] = []
        active_list = sorted(self.currently_active)

        for i, sig_a in enumerate(active_list):
            for sig_b in active_list[i + 1:]:
                if not self._has_syntagmatic_edge(sig_a, sig_b):
                    continue

                # 获取对应概念
                concepts_a = self._sig_to_concepts.get(sig_a, [])
                concepts_b = self._sig_to_concepts.get(sig_b, [])

                if not concepts_a or not concepts_b:
                    continue

                # 检查 K_active 中是否有 edge
                for ca in concepts_a:
                    for cb in concepts_b:
                        if ca == cb:
                            continue
                        # 检查是否已有 edge（任意方向）
                        neighbors_ca = set(graph.neighbors(ca))
                        if cb in neighbors_ca:
                            continue

                        # 收集 evidence patterns
                        patterns: list[str] = []
                        for edge in self.s_net.syntagmatic_neighbors(sig_a):
                            if edge.target == sig_b and edge.evidence:
                                patterns.append(edge.evidence)
                        for edge in self.s_net.syntagmatic_neighbors(sig_b):
                            if edge.target == sig_a and edge.evidence:
                                patterns.append(edge.evidence)

                        suggestion = EdgeSuggestion(
                            source_concept=ca,
                            target_concept=cb,
                            evidence_signifiers=(sig_a, sig_b),
                            evidence_patterns=tuple(patterns[:5]),
                            step=self.current_step,
                        )
                        suggestions.append(suggestion)

        return suggestions

    # ------------------------------------------------------------------
    # 序列化 / 反序列化
    # ------------------------------------------------------------------

    def to_dict(self) -> dict:
        """序列化为可持久化的 dict."""
        return {
            "currently_active": sorted(self.currently_active),
            "dialogue_focus_set": sorted(self._dialogue_focus_set),
            "current_step": self.current_step,
            "internal_speech_buffer": [
                f.to_dict() for f in self.internal_speech_buffer
            ],
            "edge_suggestions": [
                s.to_dict() for s in self.edge_suggestions
            ],
        }

    def restore_from_dict(self, d: dict) -> None:
        """从 dict 恢复状态."""
        self.currently_active = set(d.get("currently_active", []))
        self._dialogue_focus_set = set(d.get("dialogue_focus_set", []))
        self.current_step = d.get("current_step", 0)
        self.internal_speech_buffer = [
            InternalSpeechFragment.from_dict(f)
            for f in d.get("internal_speech_buffer", [])
        ]
        self.edge_suggestions = [
            EdgeSuggestion.from_dict(s)
            for s in d.get("edge_suggestions", [])
        ]

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), ensure_ascii=False, indent=2)
