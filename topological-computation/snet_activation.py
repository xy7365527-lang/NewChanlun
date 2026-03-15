"""snet_activation.py — S_net 激活态管理 + 耦合振荡.

K_active 每步进一步，S_net 同步共振。S_net 的共振反馈给 K_active
的下一步选择（pull，不是 override）。

核心类：
  SNetActivation — 管理 S_net 的激活态（当前处于激活状态的能指节点集合）

数据结构：
  InternalSpeechFragment — 被清出的能指如果构成连贯语段，存入内部言语缓冲区
  EdgeSuggestion — S_net 中两个激活能指有组合轴连接，但对应概念在 K_active 中
                   没有 edge → 注册为 edge_suggestion（articulation feedback）
  OrphanExplorationHint — 共振激活无 concept_ref 的 signifier 且与有
                   concept_ref 的 signifier 有组合轴连接时，将 anchor_concept
                   推入优先探索队列，引导穿越（纯导航，不建议创建节点）

认识论等级：L0（数据结构 + 拓扑操作，无经验假设）
"""

from __future__ import annotations

import json
import re
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


@dataclass(frozen=True, slots=True)
class OrphanExplorationHint:
    """orphan_exploration_hint: 无 concept_ref 的 signifier 通过共振激活,
    且与有 concept_ref 的 signifier 有组合轴连接时，将 anchor_concept
    推入优先探索队列引导穿越（纯导航机制，不建议创建节点）.

    orphan_signifier:    无 concept_ref 的能指 ID
    anchor_concept:      有 concept_ref 的锚定概念 ID (K_active vertex id)
    anchor_signifier:    锚定能指 ID (有 concept_ref 的那个)
    evidence_patterns:   组合轴边的 evidence 字符串列表
    syntagmatic_weight:  组合轴连接强度
    step:                生成时的穿越步数
    reviewed:            是否已被导航访问
    """
    orphan_signifier: str
    anchor_concept: str
    anchor_signifier: str
    evidence_patterns: tuple[str, ...] = ()
    syntagmatic_weight: float = 0.0
    step: int = 0
    reviewed: bool = False

    def to_dict(self) -> dict:
        return {
            "orphan_signifier": self.orphan_signifier,
            "anchor_concept": self.anchor_concept,
            "anchor_signifier": self.anchor_signifier,
            "evidence_patterns": list(self.evidence_patterns),
            "syntagmatic_weight": self.syntagmatic_weight,
            "step": self.step,
            "reviewed": self.reviewed,
        }

    @staticmethod
    def from_dict(d: dict) -> "OrphanExplorationHint":
        return OrphanExplorationHint(
            orphan_signifier=d.get("orphan_signifier", ""),
            anchor_concept=d.get("anchor_concept", ""),
            anchor_signifier=d.get("anchor_signifier", ""),
            evidence_patterns=tuple(d.get("evidence_patterns", [])),
            syntagmatic_weight=d.get("syntagmatic_weight", 0.0),
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
    verts = graph.vertices
    for vid in graph._active_ids:
        v = verts.get(vid)
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
    verts = graph.vertices
    for vid in graph._active_ids:
        v = verts.get(vid)
        if v is None or not v.content:
            continue
        content = v.content.strip()
        if content and s_net.has_signifier(content):
            mapping.setdefault(content, []).append(vid)
    return mapping


def _expand_mappings(
    s_net: SNet,
    graph,
    concept_to_sig: dict[str, str],
    sig_to_concepts: dict[str, list[str]],
) -> tuple[dict[str, str], dict[str, list[str]], dict]:
    """扩展 concept↔signifier 映射，通过聚合轴传播和子串匹配.

    三阶段扩展：
      1. 聚合轴传播（迭代至收敛）：沿 paradigmatic/translation/synonym 边
         将 concept_ref 从有映射的 signifier 传播到无映射的 signifier
      2. 子串匹配：对仍无映射的 signifier，检查其 ID 是否作为词边界匹配
         出现在某个 vertex.content 中
      3. 反向填充 concept_to_sig：新发现的 signifier→concept 映射反向注入

    参数：
      s_net:           已 bootstrap 的 SNet（含 dictionary/bilingual ingest）
      graph:           K_active 图
      concept_to_sig:  基础 concept→signifier 映射（exact match）
      sig_to_concepts: 基础 signifier→concept 映射（exact match）

    返回：
      (expanded_c2s, expanded_s2c, expansion_stats)

    认识论等级：L0（拓扑操作 + 字符串匹配，无经验假设）
    """
    expanded_s2c: dict[str, list[str]] = {k: list(v) for k, v in sig_to_concepts.items()}
    expanded_c2s: dict[str, str] = dict(concept_to_sig)
    stats: dict = {"before": len(sig_to_concepts)}

    # --- 阶段1：聚合轴传播 ---
    # 收集所有聚合轴边（含反向）的邻接索引
    # SNetLazy: 用 query_edges_by_axis 避免全量加载
    # SNet: 遍历 edges 属性
    par_neighbors: dict[str, list[str]] = {}

    par_edges: list
    if hasattr(s_net, '_persistence') and s_net._persistence is not None:
        par_edges = s_net._persistence.query_edges_by_axis("paradigmatic")
    else:
        par_edges = [e for e in s_net.edges if e.axis == AxisType.PARADIGMATIC]

    for edge in par_edges:
        par_neighbors.setdefault(edge.source, []).append(edge.target)
        par_neighbors.setdefault(edge.target, []).append(edge.source)

    paradigmatic_added = 0
    max_rounds = 3
    for round_idx in range(max_rounds):
        changed = False
        for sid in list(s_net._signifiers.keys()):
            if sid in expanded_s2c:
                # 传播给无映射的邻居
                for neighbor in par_neighbors.get(sid, []):
                    if neighbor not in expanded_s2c and s_net.has_signifier(neighbor):
                        expanded_s2c[neighbor] = list(expanded_s2c[sid])
                        paradigmatic_added += 1
                        changed = True
            else:
                # 从有映射的邻居获取
                for neighbor in par_neighbors.get(sid, []):
                    if neighbor in expanded_s2c:
                        expanded_s2c[sid] = list(expanded_s2c[neighbor])
                        paradigmatic_added += 1
                        changed = True
                        break
        if not changed:
            break

    stats["paradigmatic_added"] = paradigmatic_added
    stats["paradigmatic_rounds"] = round_idx + 1

    # --- 阶段2：子串匹配（反转搜索：从少量 content 出发匹配 signifiers）---
    # 构建 content 索引（小写化）
    content_index: dict[str, str] = {}  # vid -> lowercase content
    verts2 = graph.vertices
    for vid in graph._active_ids:
        v = verts2.get(vid)
        if v and v.content:
            content_index[vid] = v.content.strip().lower()

    substring_added = 0
    still_unmapped = [sid for sid in s_net._signifiers if sid not in expanded_s2c]

    # 反转搜索：从 content 的单词出发，在 unmapped set 中查找
    # content_index 通常只有 ~200 条目，每条目几十个单词
    # 总操作数 = M × W（M=content数, W=平均单词数）+ set 查找 O(1)
    unmapped_lower_to_sid: dict[str, str] = {}  # lowercase → original id
    for sid in still_unmapped:
        sid_lower = sid.lower()
        if len(sid_lower) >= 3:
            unmapped_lower_to_sid[sid_lower] = sid

    # 分离单词和多词术语
    unmapped_single_lower: set[str] = set()
    unmapped_multi_lower: list[str] = []
    for sid_lower in unmapped_lower_to_sid:
        if ' ' in sid_lower:
            unmapped_multi_lower.append(sid_lower)
        else:
            unmapped_single_lower.add(sid_lower)

    for vid, content_lower in content_index.items():
        content_words = set(content_lower.split())

        # 单词术语：set 交集 O(min(|words|, |unmapped|))
        matched_words = content_words & unmapped_single_lower
        for sid_lower in matched_words:
            sid = unmapped_lower_to_sid[sid_lower]
            if sid not in expanded_s2c:
                expanded_s2c[sid] = [vid]
                substring_added += 1
            elif len(expanded_s2c[sid]) < 3:
                expanded_s2c[sid].append(vid)

        # 多词术语：子串匹配（数量通常不大）
        for sid_lower in unmapped_multi_lower:
            sid = unmapped_lower_to_sid[sid_lower]
            if sid in expanded_s2c and len(expanded_s2c[sid]) >= 3:
                continue
            if sid_lower in content_lower:
                if sid not in expanded_s2c:
                    expanded_s2c[sid] = [vid]
                    substring_added += 1
                elif len(expanded_s2c[sid]) < 3:
                    expanded_s2c[sid].append(vid)

    stats["substring_added"] = substring_added

    # --- 阶段3：反向填充 c2s ---
    # 对新增的 s2c 映射，如果某个 concept 还没有 c2s 条目，添加一个
    reverse_added = 0
    for sid, concepts in expanded_s2c.items():
        for cid in concepts:
            if cid not in expanded_c2s:
                expanded_c2s[cid] = sid
                reverse_added += 1
    stats["reverse_added"] = reverse_added
    stats["after"] = len(expanded_s2c)
    stats["total_signifiers"] = len(s_net._signifiers)

    return expanded_c2s, expanded_s2c, stats


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
        self.orphan_exploration_hints: list[OrphanExplorationHint] = []
        self.current_step: int = 0

    def activate(self, concept_id: str) -> None:
        """K_active 步进到新概念时调用.

        激活对应能指，清理失去组合轴连接的旧激活。
        超边消费：到达节点 A 时，查询 snet.hyperedges_containing(A) 获取完整语境，
        超边中的共现邻居也保留在激活态。
        """
        new_signifier = self._concept_to_sig.get(concept_id)
        if not new_signifier:
            return

        new_active: set[str] = {new_signifier}

        # 超边中与新能指共现的所有术语也保留激活
        hyperedge_cooccurrents = self._hyperedge_context(new_signifier)

        # 保留与新能指有组合轴连接或同一超边中的旧激活
        for sig in self.currently_active:
            if sig == new_signifier:
                continue
            if sig in hyperedge_cooccurrents or self._has_syntagmatic_edge(sig, new_signifier):
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

    def _hyperedge_context(self, signifier_id: str) -> set[str]:
        """返回与 signifier_id 在同一超边中的所有术语（不含自身）."""
        result: set[str] = set()
        for he in self.s_net.hyperedges_containing(signifier_id):
            result.update(he.vertices)
        result.discard(signifier_id)
        return result

    def context_vertices(self, concept_id: str) -> set[str]:
        """返回与 concept_id 在同一超边中的所有术语的 concept IDs.

        查询路径：concept_id → signifier_id → hyperedges_containing → 其他 vertices → concept IDs。
        "同时在场"信息是超边保留而成对边丢失的关键价值。
        """
        signifier_id = self._concept_to_sig.get(concept_id)
        if not signifier_id:
            return set()

        cooccurrents = self._hyperedge_context(signifier_id)
        result: set[str] = set()
        for sig in cooccurrents:
            for cid in self._sig_to_concepts.get(sig, []):
                if cid != concept_id:
                    result.add(cid)
        return result

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

        S_net 中两个激活能指有组合轴连接或在同一超边中，但对应概念在 K_active 中
        没有 edge → 注册为 edge_suggestion。

        超边信息增强：如果 A 和 B 在同一超边中但 K_active 无边 → EdgeSuggestion
        附带超边上下文（evidence_patterns 中包含 hyperedge evidence_tag）。

        当一个能指有 concept_ref 而另一个没有（孤立 signifier）时，
        生成桥接建议而非跳过（articulation_bridge）。

        同时：当共振激活一个无 concept_ref 的 signifier，且该 signifier
        与某个有 concept_ref 的 signifier 有组合轴连接时，注册
        orphan_exploration_hint（导航提示）。
        """
        suggestions: list[EdgeSuggestion] = []
        bridge_suggestions: list[dict] = []
        exploration_hints: list[OrphanExplorationHint] = []
        active_list = sorted(self.currently_active)

        # 已注册过的 orphan signifier（防止重复注册 orphan_exploration_hint）
        known_orphans: set[str] = {
            s.orphan_signifier for s in self.orphan_exploration_hints
        }

        for i, sig_a in enumerate(active_list):
            for sig_b in active_list[i + 1:]:
                has_syntagmatic = self._has_syntagmatic_edge(sig_a, sig_b)
                shared_hyperedges = self.s_net.cooccurrence_context(sig_a, sig_b)
                if not has_syntagmatic and not shared_hyperedges:
                    continue

                # 获取对应概念
                concepts_a = self._sig_to_concepts.get(sig_a, [])
                concepts_b = self._sig_to_concepts.get(sig_b, [])

                # 收集 evidence patterns（桥接和标准路径都需要）
                syntagmatic_evidence: list[str] = []
                syntagmatic_weight = 0.0
                for edge in self.s_net.syntagmatic_neighbors(sig_a):
                    if edge.target == sig_b:
                        if edge.evidence:
                            syntagmatic_evidence.append(edge.evidence)
                        syntagmatic_weight = max(syntagmatic_weight, edge.weight)
                for edge in self.s_net.syntagmatic_neighbors(sig_b):
                    if edge.target == sig_a:
                        if edge.evidence:
                            syntagmatic_evidence.append(edge.evidence)
                        syntagmatic_weight = max(syntagmatic_weight, edge.weight)

                # 超边 evidence 附加
                for he in shared_hyperedges:
                    if he.evidence_tag and he.evidence_tag not in syntagmatic_evidence:
                        syntagmatic_evidence.append(he.evidence_tag)

                # 桥接路径：一个有 concept_ref，另一个没有
                if concepts_a and not concepts_b:
                    bridge_suggestions.append({
                        "anchor_concept": concepts_a[0],
                        "orphan_signifier": sig_b,
                        "evidence_signifiers": (sig_a, sig_b),
                        "evidence_patterns": tuple(syntagmatic_evidence[:5]),
                    })
                    # orphan_exploration_hint
                    if sig_b not in known_orphans:
                        exploration_hints.append(OrphanExplorationHint(
                            orphan_signifier=sig_b,
                            anchor_concept=concepts_a[0],
                            anchor_signifier=sig_a,
                            evidence_patterns=tuple(syntagmatic_evidence[:5]),
                            syntagmatic_weight=syntagmatic_weight,
                            step=self.current_step,
                        ))
                        known_orphans.add(sig_b)
                    continue
                elif concepts_b and not concepts_a:
                    bridge_suggestions.append({
                        "anchor_concept": concepts_b[0],
                        "orphan_signifier": sig_a,
                        "evidence_signifiers": (sig_a, sig_b),
                        "evidence_patterns": tuple(syntagmatic_evidence[:5]),
                    })
                    # orphan_exploration_hint
                    if sig_a not in known_orphans:
                        exploration_hints.append(OrphanExplorationHint(
                            orphan_signifier=sig_a,
                            anchor_concept=concepts_b[0],
                            anchor_signifier=sig_b,
                            evidence_patterns=tuple(syntagmatic_evidence[:5]),
                            syntagmatic_weight=syntagmatic_weight,
                            step=self.current_step,
                        ))
                        known_orphans.add(sig_a)
                    continue

                # 两个都没有 concept_ref → 不创建（避免大量孤立对注入）
                if not concepts_a or not concepts_b:
                    continue

                # 标准路径：两个都有 concept_ref
                # 检查 K_active 中是否有 edge
                for ca in concepts_a:
                    for cb in concepts_b:
                        if ca == cb:
                            continue
                        # 检查是否已有 edge（任意方向）
                        neighbors_ca = set(graph.neighbors(ca))
                        if cb in neighbors_ca:
                            continue

                        suggestion = EdgeSuggestion(
                            source_concept=ca,
                            target_concept=cb,
                            evidence_signifiers=(sig_a, sig_b),
                            evidence_patterns=tuple(syntagmatic_evidence[:5]),
                            step=self.current_step,
                        )
                        suggestions.append(suggestion)

        # 处理桥接建议
        bridge_edges = self._bridge_orphan_signifiers(bridge_suggestions, graph)
        suggestions.extend(bridge_edges)

        # 存入 orphan_exploration_hints 导航队列
        self.orphan_exploration_hints.extend(exploration_hints)

        return suggestions

    def _bridge_orphan_signifiers(
        self,
        bridge_suggestions: list[dict],
        graph,
    ) -> list[EdgeSuggestion]:
        """为孤立 signifier 创建桥接 EdgeSuggestion.

        每步最多桥接 MAX_BRIDGE_PER_STEP 个（防止膨胀）。
        桥接建议的 target_concept 使用 __bridge__ 前缀标记，
        由 traversal.py 的消费逻辑创建实际顶点。
        """
        MAX_BRIDGE_PER_STEP = 3
        results: list[EdgeSuggestion] = []
        bridged = 0

        for bridge in bridge_suggestions:
            if bridged >= MAX_BRIDGE_PER_STEP:
                break

            orphan_sig = bridge["orphan_signifier"]
            anchor = bridge["anchor_concept"]

            # 检查是否已被桥接（避免重复创建）
            if self._sig_to_concepts.get(orphan_sig):
                continue

            results.append(EdgeSuggestion(
                source_concept=anchor,
                target_concept=f"__bridge__{orphan_sig}",
                evidence_signifiers=bridge["evidence_signifiers"],
                evidence_patterns=bridge.get("evidence_patterns", ()),
                origin="articulation_bridge",
                step=self.current_step,
            ))
            bridged += 1

        return results

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
            "orphan_exploration_hints": [
                s.to_dict() for s in self.orphan_exploration_hints
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
        # 向后兼容：旧 key "concept_creation_suggestions" 也识别
        hints_raw = d.get("orphan_exploration_hints",
                          d.get("concept_creation_suggestions", []))
        self.orphan_exploration_hints = [
            OrphanExplorationHint.from_dict(s)
            for s in hints_raw
        ]

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), ensure_ascii=False, indent=2)
