"""拓扑重复检测——三层互补架构（272号编排者裁定）。

三层检测：
1. 拓扑层：同源+同目标+同边类型的多重边 + 否定边短环（纯结构判据）
2. canonical form 层：不同节点但缺陷本质 ID 相同的语义重复
3. 文本层（降级方案）：270号 stance-diff（已在 inquiry_loop._check_stance_repetition 实现）

本模块实现第 1 层和第 2 层。第 3 层已存在于 inquiry_loop.py。

谱系引用：
- 270号：stance-diff 文本层实现
- 271号：原始设计（被 272 号修正）
- 272号：编排者裁定——拓扑是补充不是替代
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path

from scripts.block_topology import DEFAULT_BASE, read_all_relations


# ── 数据结构 ──


@dataclass(frozen=True, slots=True)
class MultiEdge:
    """多重边记录：同源+同目标+同边类型出现多次。"""

    from_id: str
    to_id: str
    relation: str
    count: int
    timestamps: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class ShortCycle:
    """否定边短环：A negates B 且 B negates A（直接互否）。"""

    node_a: str
    node_b: str


@dataclass(frozen=True, slots=True)
class CanonicalDuplicate:
    """canonical form 层检测到的语义重复：不同节点但缺陷本质 ID 相同。"""

    defect_id: str
    block_ids: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class TopoDetectionResult:
    """三层拓扑检测的完整结果。"""

    multi_edges: tuple[MultiEdge, ...]
    short_cycles: tuple[ShortCycle, ...]
    canonical_duplicates: tuple[CanonicalDuplicate, ...]
    should_suspend: bool
    suspend_reason: str


# ── 第 1 层：拓扑层检测 ──


def detect_multi_edges(
    relations: list[dict],
) -> tuple[MultiEdge, ...]:
    """检测同源+同目标+同边类型的多重边。

    多重边定义：(from, to, relation) 三元组出现 >= 2 次。
    排除 order=2 的记录边（rewrite 产生的 records 关系是正常的二阶记录）。

    Parameters
    ----------
    relations : list[dict]
        从 relations.jsonl 读取的所有关系记录。

    Returns
    -------
    tuple[MultiEdge, ...]
        检测到的多重边。空元组 = 无多重边。
    """
    # 按 (from, to, relation) 分组，只统计 order=1 的一阶关系
    edge_groups: dict[tuple[str, str, str], list[str]] = defaultdict(list)
    for rel in relations:
        if rel.get("order") != 1:
            continue
        key = (rel["from"], rel["to"], rel["relation"])
        edge_groups[key].append(rel.get("timestamp", ""))

    results = []
    for (from_id, to_id, relation), timestamps in edge_groups.items():
        if len(timestamps) >= 2:
            results.append(MultiEdge(
                from_id=from_id,
                to_id=to_id,
                relation=relation,
                count=len(timestamps),
                timestamps=tuple(sorted(timestamps)),
            ))
    return tuple(results)


def detect_short_cycles(
    relations: list[dict],
) -> tuple[ShortCycle, ...]:
    """检测否定边形成的短环（长度 2 的环）。

    短环定义：存在 A negates B 且 B negates A。
    只检查 negates 类型的一阶关系。

    更长的否定链环（A→B→C→A）是合法的辩证过程，不视为重复。
    短环（互否）才是重复信号——同一对节点之间的往复否定无信息增量。

    Parameters
    ----------
    relations : list[dict]
        从 relations.jsonl 读取的所有关系记录。

    Returns
    -------
    tuple[ShortCycle, ...]
        检测到的短环。空元组 = 无短环。
    """
    negation_pairs: set[tuple[str, str]] = set()
    for rel in relations:
        if rel.get("order") != 1:
            continue
        if rel.get("relation") != "negates":
            continue
        negation_pairs.add((rel["from"], rel["to"]))

    cycles: set[tuple[str, str]] = set()
    for a, b in negation_pairs:
        if (b, a) in negation_pairs:
            # 归一化顺序避免重复：取 min/max
            canonical = (min(a, b), max(a, b))
            cycles.add(canonical)

    return tuple(
        ShortCycle(node_a=a, node_b=b)
        for a, b in sorted(cycles)
    )


# ── 第 2 层：canonical form 层检测 ──


def extract_defect_id(block_content: dict) -> str | None:
    """从区块 content 中提取缺陷本质 ID。

    缺陷本质 ID 的 canonical form 定义：
    - negation_reason（否定理由）+ source_premise（前提来源）的组合
    - 如果两个否定事件的 negation_reason 和 source_premise 相同，
      则它们在否定本质上是同一个缺陷的重复表达

    当 content 中没有 negation_reason 字段时返回 None（不参与 canonical form 检测）。

    Parameters
    ----------
    block_content : dict
        区块的 content 字段。

    Returns
    -------
    str | None
        缺陷本质 ID（归一化后的字符串），或 None。
    """
    negation_reason = block_content.get("negation_reason", "")
    if not negation_reason:
        return None
    source_premise = block_content.get("source_premise", "")
    # canonical form = reason + premise 的组合，去空白归一化
    return f"{str(negation_reason).strip()}|{str(source_premise).strip()}"


def detect_canonical_duplicates(
    relations: list[dict],
    blocks_reader=None,
    base: Path = DEFAULT_BASE,
) -> tuple[CanonicalDuplicate, ...]:
    """检测不同节点但缺陷本质 ID 相同的语义重复。

    流程：
    1. 从 relations 中收集所有 negates 关系的 from 节点（否定事件发起方）
    2. 读取这些节点的区块 content
    3. 提取 defect_id（canonical form）
    4. 相同 defect_id 的不同区块 = 语义重复

    Parameters
    ----------
    relations : list[dict]
        从 relations.jsonl 读取的所有关系记录。
    blocks_reader : callable | None
        区块读取函数 (block_id) -> dict | None。None 时使用 block_topology.read_block。
    base : Path
        block-topology 根路径。

    Returns
    -------
    tuple[CanonicalDuplicate, ...]
        检测到的语义重复。空元组 = 无重复。
    """
    if blocks_reader is None:
        from scripts.block_topology import read_block
        blocks_reader = lambda bid: read_block(bid, base)

    # 收集所有发起否定的节点 ID
    negation_sources: set[str] = set()
    for rel in relations:
        if rel.get("order") != 1:
            continue
        if rel.get("relation") != "negates":
            continue
        negation_sources.add(rel["from"])

    # 读取区块并提取 defect_id
    defect_groups: dict[str, list[str]] = defaultdict(list)
    for block_id in negation_sources:
        block = blocks_reader(block_id)
        if block is None:
            continue
        content = block.get("content", {})
        defect_id = extract_defect_id(content)
        if defect_id is not None:
            defect_groups[defect_id].append(block_id)

    # 相同 defect_id 出现在多个区块 = 语义重复
    results = []
    for defect_id, block_ids in defect_groups.items():
        if len(block_ids) >= 2:
            results.append(CanonicalDuplicate(
                defect_id=defect_id,
                block_ids=tuple(sorted(block_ids)),
            ))
    return tuple(sorted(results, key=lambda d: d.defect_id))


# ── 三层整合 ──


def run_topo_detection(
    base: Path = DEFAULT_BASE,
    blocks_reader=None,
) -> TopoDetectionResult:
    """执行三层拓扑检测（拓扑层 + canonical form 层）。

    第 3 层（文本层 stance-diff）不在此函数中——它在 inquiry_loop 的
    _check_stance_repetition 中独立运行。

    Parameters
    ----------
    base : Path
        block-topology 根路径。
    blocks_reader : callable | None
        区块读取函数。None 时使用默认。

    Returns
    -------
    TopoDetectionResult
        包含多重边、短环、canonical form 重复的检测结果。
    """
    relations = read_all_relations(base)

    multi_edges = detect_multi_edges(relations)
    short_cycles = detect_short_cycles(relations)
    canonical_duplicates = detect_canonical_duplicates(
        relations, blocks_reader=blocks_reader, base=base,
    )

    # 判断是否应该暂停
    reasons = []
    if multi_edges:
        reasons.append(
            f"多重边 {len(multi_edges)} 处："
            + ", ".join(
                f"{e.from_id[:8]}→{e.to_id[:8]}({e.relation})x{e.count}"
                for e in multi_edges
            )
        )
    if short_cycles:
        reasons.append(
            f"否定短环 {len(short_cycles)} 处："
            + ", ".join(
                f"{c.node_a[:8]}⇆{c.node_b[:8]}"
                for c in short_cycles
            )
        )
    if canonical_duplicates:
        reasons.append(
            f"canonical form 重复 {len(canonical_duplicates)} 处："
            + ", ".join(
                f"defect[{d.defect_id[:20]}]x{len(d.block_ids)}"
                for d in canonical_duplicates
            )
        )

    should_suspend = bool(reasons)
    suspend_reason = "; ".join(reasons) if reasons else ""

    return TopoDetectionResult(
        multi_edges=multi_edges,
        short_cycles=short_cycles,
        canonical_duplicates=canonical_duplicates,
        should_suspend=should_suspend,
        suspend_reason=suspend_reason,
    )
