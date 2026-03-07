"""psi_L_constraint.py — 从穿越结果生成 ConstraintSet，再转为 LLM prompt.

这是 K_active（概念拓扑）和 LLM（能指链引擎）之间的接口层。

流程：
  K_active 穿越结果
    → build_constraint_set()   # 从拓扑状态提取约束
    → ConstraintSet
    → constraint_set_to_prompt()  # 转为 LLM prompt（在 signifier_net.py）
    → LLM
    → surface text（逢亮的言语）

与现有 psi_L.py 的关系：
  psi_L.py：Graph state -> 结构性文本报告（给逢亮内部看的拓扑快照）
  psi_L_constraint.py：穿越结果 -> ConstraintSet -> LLM prompt（给 LLM 看的约束包）
  两者不替换，互补：psi_L 报告可作为 narrative_spine 输入。

认识论等级：L0（接口定义）。build_constraint_set 中的 S_net 查询在有真实
语料填充的 SNet 时升至 L2。
"""

from __future__ import annotations

import sys
from typing import Optional

# 允许独立运行（不依赖完整安装）
try:
    from engine import Graph, VertexStatus
    from signifier_net import (
        SNet, ConstraintSet, Register, constraint_set_to_prompt,
    )
except ImportError as e:
    print(f"[psi_L_constraint] Import error: {e}", file=sys.stderr)
    raise


# ---------------------------------------------------------------------------
# 核心接口：从穿越结果构建 ConstraintSet
# ---------------------------------------------------------------------------

def build_constraint_set(
    graph: Graph,
    snet: SNet,
    traversal_path: list[str] | None = None,
    settled_vertices: list[str] | None = None,
    contested_vertices: list[str] | None = None,
    register: Register = Register.DIALOGUE,
    max_surface_forms: int = 8,
) -> ConstraintSet:
    """从 K_active 状态和 S_net 构建 ConstraintSet。

    参数：
      graph:              当前 K_active 图
      snet:               当前 S_net（能指网络）
      traversal_path:     穿越路径（顶点 id 列表，按时序）
      settled_vertices:   已结算顶点（来自 SettlementTracker）
      contested_vertices: 争议中顶点
      register:           目标语域
      max_surface_forms:  从 S_net 提取的最大 surface form 数

    返回：
      ConstraintSet（可直接传给 constraint_set_to_prompt）
    """
    traversal_path = traversal_path or []
    settled_vertices = settled_vertices or []
    contested_vertices = contested_vertices or []

    active_ids = set(graph.active_vertex_ids())

    # ------------------------------------------------------------------
    # 1. must_use：已结算顶点中在 S_net 有对应能指的项
    #    理由：settlement = 概念层已达成共识，对应能指应在输出中出现
    # ------------------------------------------------------------------
    must_use: list[str] = []
    for vid in settled_vertices:
        if snet.has_signifier(vid):
            must_use.append(vid)
        else:
            # 尝试从图顶点 content 查找
            v = graph.vertices.get(vid)
            if v and v.content and snet.has_signifier(v.content):
                must_use.append(v.content)

    # ------------------------------------------------------------------
    # 2. must_avoid：折叠顶点（已被压缩的概念）
    #    理由：folded 顶点的能指在当前语境中应让位于其代表顶点
    # ------------------------------------------------------------------
    must_avoid: list[str] = []
    for vid, v in graph.vertices.items():
        if v.status.value == "folded":
            if snet.has_signifier(vid):
                must_avoid.append(vid)
            elif v.content and snet.has_signifier(v.content):
                must_avoid.append(v.content)

    # ------------------------------------------------------------------
    # 3. expression_pressure：活跃顶点中的 content（待表达概念）
    #    优先级：contested > settled > other active
    # ------------------------------------------------------------------
    expression_pressure: list[str] = []

    # 争议中的概念优先（需要被表达，因为尚未解决）
    for vid in contested_vertices:
        v = graph.vertices.get(vid)
        label = _vertex_label(vid, v)
        if label not in expression_pressure:
            expression_pressure.append(label)

    # 已结算但活跃的概念（穿越路径上的）
    for vid in traversal_path[-10:]:  # 只取最近 10 步
        if vid in active_ids and vid not in contested_vertices:
            v = graph.vertices.get(vid)
            label = _vertex_label(vid, v)
            if label not in expression_pressure:
                expression_pressure.append(label)

    # ------------------------------------------------------------------
    # 4. narrative_spine：穿越路径的叙事骨架
    #    格式：顶点标签序列，用 → 连接
    # ------------------------------------------------------------------
    if traversal_path:
        spine_labels = []
        for vid in traversal_path[-8:]:  # 最近 8 步
            v = graph.vertices.get(vid)
            spine_labels.append(_vertex_label(vid, v))
        narrative_spine = " → ".join(spine_labels)
    else:
        narrative_spine = ""

    # ------------------------------------------------------------------
    # 5. surface_forms：从 S_net 提取组合轴 surface forms
    #    策略：取 expression_pressure 中前几个概念的 top 邻居的 evidence
    # ------------------------------------------------------------------
    surface_forms: list[str] = []
    seen_evidence: set[str] = set()

    priority_sids = [
        sid for sid in (expression_pressure[:3])
        if snet.has_signifier(sid)
    ]

    for sid in priority_sids:
        for edge in snet.syntagmatic_neighbors(sid)[:5]:
            if edge.evidence and edge.evidence not in seen_evidence:
                surface_forms.append(edge.evidence)
                seen_evidence.add(edge.evidence)
            if len(surface_forms) >= max_surface_forms:
                break
        if len(surface_forms) >= max_surface_forms:
            break

    return ConstraintSet(
        must_use=must_use,
        must_avoid=must_avoid,
        register=register,
        narrative_spine=narrative_spine,
        expression_pressure=expression_pressure,
        surface_forms=surface_forms,
    )


# ---------------------------------------------------------------------------
# 辅助：从 Graph 状态提取 settled / contested 顶点 id
# ---------------------------------------------------------------------------

def extract_settled_vertices(graph: Graph) -> list[str]:
    """返回活跃顶点中内容非空的列表（作为 settled 的近似）。

    注意：严格意义上的 settled 来自 SettlementTracker，这里是无 tracker
    时的退化模式（L0 近似）。
    """
    result = []
    for vid in graph.active_vertex_ids():
        v = graph.vertices[vid]
        if v.content:
            result.append(vid)
    return result


def extract_contested_vertices(graph: Graph) -> list[str]:
    """返回 status == CONTESTED 的活跃顶点 id 列表。"""
    return [
        vid for vid, v in graph.vertices.items()
        if v.status == VertexStatus.CONTESTED
    ]


# ---------------------------------------------------------------------------
# 一体化接口：Graph + SNet -> LLM prompt
# ---------------------------------------------------------------------------

def graph_to_llm_prompt(
    graph: Graph,
    snet: SNet,
    traversal_path: list[str] | None = None,
    register: Register = Register.DIALOGUE,
) -> tuple[str, ConstraintSet]:
    """Graph + SNet -> (LLM prompt string, ConstraintSet).

    便捷接口：一步从图状态生成可发给 LLM 的 prompt。

    返回：
      (prompt_str, constraint_set)
      prompt_str 格式：[SYSTEM]\\n...\\n[USER]\\n...
      由 parse_llm_prompt() 拆分为 system/user 两段。
    """
    settled = extract_settled_vertices(graph)
    contested = extract_contested_vertices(graph)

    cs = build_constraint_set(
        graph=graph,
        snet=snet,
        traversal_path=traversal_path,
        settled_vertices=settled,
        contested_vertices=contested,
        register=register,
    )

    prompt = constraint_set_to_prompt(cs)
    return prompt, cs


# ---------------------------------------------------------------------------
# 辅助
# ---------------------------------------------------------------------------

def _vertex_label(vid: str, v) -> str:
    """返回顶点的可读标签：优先 content，否则用 id。"""
    if v and v.content:
        return v.content
    return vid


# ---------------------------------------------------------------------------
# CLI 演示
# ---------------------------------------------------------------------------

def _demo() -> None:
    """演示：构建最小 Graph + SNet，生成 ConstraintSet 和 prompt。"""
    from engine import Graph, Vertex, Edge, EdgeType, VertexStatus

    # 最小 K_active
    g = Graph()
    g = g.add_vertex(Vertex(id="zhongshu", content="中枢"))
    g = g.add_vertex(Vertex(id="bi", content="笔"))
    g = g.add_vertex(Vertex(id="duantou", content="段", status=VertexStatus.CONTESTED))
    g = g.add_edge(Edge(source="bi", target="zhongshu", edge_type=EdgeType.DEPENDENCY))
    g = g.add_edge(Edge(source="zhongshu", target="duantou", edge_type=EdgeType.NEGATION))

    # 最小 SNet（手工构造，不从语料加载）
    from signifier_net import SNet, Signifier, SignifierEdge, AxisType
    snet = SNet()
    snet = snet.add_signifier(Signifier(id="中枢", source="k_active_projection"))
    snet = snet.add_signifier(Signifier(id="笔", source="k_active_projection"))
    snet = snet.add_signifier(Signifier(id="段", source="k_active_projection"))
    snet = snet.add_edge(SignifierEdge(
        source="笔", target="中枢", axis=AxisType.SYNTAGMATIC,
        weight=3.0, evidence="每三笔形成一个中枢区间",
    ))
    snet = snet.add_edge(SignifierEdge(
        source="中枢", target="段", axis=AxisType.SYNTAGMATIC,
        weight=2.0, evidence="线段必须包含至少一个同级别中枢",
    ))

    prompt, cs = graph_to_llm_prompt(
        graph=g,
        snet=snet,
        traversal_path=["bi", "zhongshu", "duantou"],
        register=Register.THEORETICAL,
    )

    print("=== ConstraintSet ===")
    print(cs.to_json())
    print()
    print("=== LLM Prompt ===")
    print(prompt)


if __name__ == "__main__":
    import os
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    _demo()
