"""snet_bootstrap.py — S_net 三层初始化.

Layer A: K_active 投影 → 骨架（每个概念顶点 → 一个能指节点）
Layer B: surface_forms 语料 → 组合轴（PMI 权重，syntagmatic edges）
Layer C: 聚合轴（缠论聚合类种子 + 手动扩展接口）

认识论等级声明（形式化有效域规则）：
  Layer A: L0（代数定义，从图顶点机械投影）
  Layer B: L1→L2（语料来自真实缠论博文，PMI 校正高频偏差；
           但"PMI > threshold ∝ 真实组合强度"假设未 L2 检验）
           当前标注为 L1：管线正确性已验证，但有效域未独立验证
  Layer C: L0（手工种子，聚合关系来自缠论定义，非从语料归纳）

边界条件：
  - Layer A 只投影有 content 的顶点（纯 id 顶点不产生能指节点）
  - Layer B 的 PMI 权重依赖"句子"边界定义（当前以换行分割上下文）
  - Layer B PMI 过滤：默认阈值 0.0（正 PMI = 共现超过独立期望）
  - Layer C 种子为硬编码，聚合对两端若不在 snet 则跳过（不自动添加）
  - term_a == term_b 的记录（自指共现）跳过，不产生 syntagmatic edge

变更记录：
  v2（Gemini Round 2 反馈）:
    - Layer B: raw co-occurrence → PMI 过滤，修复高频术语垄断问题
    - Layer C: 占位 → 填充缠论聚合类种子（中枢/趋势/买点/背驰/线段/分型）
    - bootstrap_snet: 增加 pmi_threshold 参数
    - report_snet: 增加 PMI 过滤统计（过滤前后边数、高 PMI 对）
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path
from typing import Optional

try:
    from engine import Graph, VertexStatus
    from signifier_net import SNet, Signifier, SignifierEdge, AxisType
except ImportError as e:
    print(f"[snet_bootstrap] Import error: {e}", file=sys.stderr)
    raise


# ---------------------------------------------------------------------------
# Layer A: K_active 投影
# ---------------------------------------------------------------------------

def bootstrap_layer_a(graph: Graph) -> SNet:
    """从 K_active 投影骨架 S_net。

    规则：
      - 每个活跃顶点 (status != FOLDED) 且 content 非空 → 一个 Signifier
      - Signifier.id = vertex.content（规范术语形式）
      - Signifier.source = 'k_active_projection'
      - 顶点 id 作为 Signifier.surface_forms 的第一项（保留溯源）

    认识论等级：L0
    """
    snet = SNet()
    seen_content: dict[str, list[str]] = {}  # content -> [vertex_ids]

    # 收集所有活跃顶点的 content
    for vid in graph.active_vertex_ids():
        v = graph.vertices[vid]
        if not v.content:
            continue
        content = v.content.strip()
        if not content:
            continue
        seen_content.setdefault(content, []).append(vid)

    # 为每个唯一 content 创建 Signifier
    for content, vids in seen_content.items():
        sig = Signifier(
            id=content,
            surface_forms=tuple(vids),   # 顶点 id 列表作为 source trace
            source="k_active_projection",
        )
        snet = snet.add_signifier(sig)

    return snet


# ---------------------------------------------------------------------------
# PMI 计算辅助
# ---------------------------------------------------------------------------

def _compute_pmi(
    raw_edges: dict[tuple[str, str], tuple[int, str]],
    term_sentence_counts: dict[str, int],
    total_sentences: int,
) -> dict[tuple[str, str], float]:
    """计算每个术语对的 PMI 值。

    PMI(a, b) = log2(P(a,b) / (P(a) * P(b)))
             = log2(cooccur_ab * total / (count_a * count_b))

    其中：
      - cooccur_ab = 包含 (a, b) 的句子数（由 raw_edges 的 count 近似）
      - count_a = 包含 a 的句子数
      - count_b = 包含 b 的句子数
      - total = 总句子数

    注意：raw_edges 的 count 是共现"记录"数（一个句子可能产生多条记录）。
    这里用记录数近似共现句子数。有效域声明：此近似在同一上下文切割下
    单调保序，不影响过滤方向，但绝对 PMI 值有偏。

    认识论等级：L1（同 bootstrap_layer_b）
    """
    pmi_scores: dict[tuple[str, str], float] = {}
    if total_sentences == 0:
        return pmi_scores

    for (term_a, term_b), (count_ab, _) in raw_edges.items():
        count_a = term_sentence_counts.get(term_a, 0)
        count_b = term_sentence_counts.get(term_b, 0)
        if count_a == 0 or count_b == 0:
            pmi_scores[(term_a, term_b)] = -float("inf")
            continue
        # P(a,b) / (P(a) * P(b)) = (count_ab/N) / ((count_a/N) * (count_b/N))
        #                         = count_ab * N / (count_a * count_b)
        ratio = (count_ab * total_sentences) / (count_a * count_b)
        pmi_scores[(term_a, term_b)] = math.log2(ratio) if ratio > 0 else -float("inf")

    return pmi_scores


# ---------------------------------------------------------------------------
# Layer B: 从 surface_forms 语料填充组合轴（PMI 过滤）
# ---------------------------------------------------------------------------

def bootstrap_layer_b(
    snet: SNet,
    surface_forms_path: str | Path,
    whitelist: frozenset[str] | None = None,
    add_missing_signifiers: bool = True,
    pmi_threshold: float = 0.0,
) -> tuple[SNet, dict]:
    """从 chanlun_surface_forms.jsonl 填充 S_net 组合轴（PMI 权重）。

    参数：
      snet:                   Layer A 已初始化的 SNet（或空 SNet）
      surface_forms_path:     jsonl 文件路径
      whitelist:              仅处理在白名单内的术语对（None = 不限制）
      add_missing_signifiers: 若 term_a/term_b 不在 snet 中，是否自动添加
                              （True 时来源标记为 'corpus'）
      pmi_threshold:          PMI 过滤阈值（默认 0.0 = 正 PMI，即共现超过独立期望）
                              设为 -inf 可禁用过滤（回退到 raw co-occurrence）

    每条记录格式：
      {"term_a": str, "term_b": str, "surface": str, "context": str, ...}

    产出：
      - (term_a, term_b) 组合轴边，weight = PMI 值（过滤后）
      - evidence = 第一次出现的 surface 片段
      - 同时返回过滤统计字典：
        {edges_before, edges_after, filtered, high_pmi_pairs: [(a, b, pmi), ...]}

    认识论等级：L1（语料来自真实博文，但 PMI ∝ 组合强度假设未 L2 验证）
    """
    path = Path(surface_forms_path)
    if not path.exists():
        raise FileNotFoundError(f"surface_forms 文件不存在: {path}")

    # 两遍扫描：
    # 第1遍：统计 term_sentence_counts（每个术语出现的记录数）
    #        同时收集 raw_edges（共现记录数 + 首次 evidence）
    # 第2遍：计算 PMI，过滤

    raw_edges: dict[tuple[str, str], tuple[int, str]] = {}
    term_sentence_counts: dict[str, int] = {}
    missing_signifiers: set[str] = set()
    total_records = 0

    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue

            term_a = record.get("term_a", "").strip()
            term_b = record.get("term_b", "").strip()
            surface = record.get("surface", "").strip()

            if not term_a or not term_b:
                continue
            if term_a == term_b:
                continue

            if whitelist is not None:
                if term_a not in whitelist or term_b not in whitelist:
                    continue

            total_records += 1

            # 统计每个术语的出现次数（用于 PMI 分母）
            term_sentence_counts[term_a] = term_sentence_counts.get(term_a, 0) + 1
            term_sentence_counts[term_b] = term_sentence_counts.get(term_b, 0) + 1

            # 记录缺失的能指
            if not snet.has_signifier(term_a):
                missing_signifiers.add(term_a)
            if not snet.has_signifier(term_b):
                missing_signifiers.add(term_b)

            # 累积共现计数（evidence 取第一条质量好的 surface）
            key = (term_a, term_b)
            if key in raw_edges:
                count, first_ev = raw_edges[key]
                if len(first_ev) < 6 and len(surface) >= 6:
                    raw_edges[key] = (count + 1, surface)
                else:
                    raw_edges[key] = (count + 1, first_ev)
            else:
                raw_edges[key] = (1, surface)

    edges_before = len(raw_edges)

    # 添加缺失能指
    if add_missing_signifiers:
        for term in missing_signifiers:
            if not snet.has_signifier(term):
                snet = snet.add_signifier(Signifier(
                    id=term,
                    surface_forms=(),
                    source="corpus",
                ))

    # 计算 PMI
    pmi_scores = _compute_pmi(raw_edges, term_sentence_counts, total_records)

    # PMI 过滤 + 批量构建边列表
    all_edges = list(snet.edges)
    filtered_count = 0
    high_pmi_pairs: list[tuple[str, str, float]] = []

    for (src, tgt), (count, first_ev) in raw_edges.items():
        if not (snet.has_signifier(src) and snet.has_signifier(tgt)):
            continue
        pmi = pmi_scores.get((src, tgt), -float("inf"))
        if pmi < pmi_threshold:
            filtered_count += 1
            continue
        all_edges.append(SignifierEdge(
            source=src,
            target=tgt,
            axis=AxisType.SYNTAGMATIC,
            weight=pmi,          # PMI 作为边权重（替换 raw count）
            evidence=first_ev,
        ))
        high_pmi_pairs.append((src, tgt, pmi))

    edges_after = len(all_edges) - len(snet.edges)

    # top-10 高 PMI 对（用于报告）
    high_pmi_pairs.sort(key=lambda x: -x[2])
    high_pmi_top = high_pmi_pairs[:10]

    stats = {
        "edges_before": edges_before,
        "edges_after": edges_after,
        "filtered": filtered_count,
        "pmi_threshold": pmi_threshold,
        "total_records": total_records,
        "high_pmi_pairs": [(a, b, round(p, 3)) for a, b, p in high_pmi_top],
    }

    new_snet = SNet(snet.signifiers, all_edges)
    return new_snet, stats


# ---------------------------------------------------------------------------
# Layer C: 聚合轴（缠论聚合类种子）
# ---------------------------------------------------------------------------

# 缠论聚合类种子：每组 (canonical_term, alternative_term, weight, differential)
# canonical → alternative 表示"alternative 可替换 canonical，但有语义差异"
# differential 记录差异（用于下游 ConstraintSet 和断裂检测）
# weight: 1.0 = 完全替换，< 1.0 = 部分替换（保留信息差）

_CHANLUN_PARADIGMATIC_SEEDS: list[tuple[str, str, float, str]] = [
    # 中枢类
    ("中枢", "走势中枢", 0.9, "走势中枢强调级别归属，中枢更泛用"),
    ("中枢", "震荡区间", 0.5, "震荡区间丢失级别递归含义，仅描述价格范围"),
    # 趋势类
    ("趋势", "上涨", 0.7, "上涨是趋势的方向特化（上涨趋势）"),
    ("趋势", "下跌", 0.7, "下跌是趋势的方向特化（下跌趋势）"),
    ("走势类型", "趋势", 0.8, "趋势是走势类型的一个子类（另一子类是盘整）"),
    ("走势类型", "盘整", 0.8, "盘整是走势类型的一个子类（另一子类是趋势）"),
    # 买点类
    ("买点", "第一类买点", 0.8, "第一类买点是背驰后的买点，是买点的特定形态"),
    ("买点", "第二类买点", 0.8, "第二类买点是中枢回调后的买点，是买点的特定形态"),
    ("买点", "第三类买点", 0.8, "第三类买点是中枢突破后回踩的买点，是买点的特定形态"),
    # 卖点类
    ("卖点", "第一类卖点", 0.8, "第一类卖点与第一类买点对称"),
    ("卖点", "第二类卖点", 0.8, "第二类卖点与第二类买点对称"),
    ("卖点", "第三类卖点", 0.8, "第三类卖点与第三类买点对称"),
    # 背驰类
    ("背驰", "盘整背驰", 0.8, "盘整背驰不改变走势方向，仅结束当前盘整段"),
    ("背驰", "趋势背驰", 0.8, "趋势背驰可能改变走势方向，是更强的背驰信号"),
    # 线段/笔类
    ("线段", "段", 0.95, "段是线段的简称，指同一概念"),
    ("笔", "bi", 0.9, "bi 是笔的拼音简写，用于代码 ID"),
    # 分型类
    ("分型", "顶分型", 0.8, "顶分型是分型的方向特化（价格高点处）"),
    ("分型", "底分型", 0.8, "底分型是分型的方向特化（价格低点处）"),
]


def bootstrap_layer_c(
    snet: SNet,
    extra_pairs: list[tuple[str, str, float]] | None = None,
) -> SNet:
    """聚合轴初始化：缠论聚合类种子 + 可选额外对。

    种子来源：缠论知识库 + 编排者手工标注（见 _CHANLUN_PARADIGMATIC_SEEDS）
    认识论等级：L0（手工种子，聚合关系来自定义，非语料归纳）

    extra_pairs: [(term_a, term_b, weight), ...] — 额外同义/替换关系
    """
    # 写入缠论种子（只添加两端都在 snet 中的对）
    for canonical, alternative, weight, differential in _CHANLUN_PARADIGMATIC_SEEDS:
        if not snet.has_signifier(canonical) or not snet.has_signifier(alternative):
            continue
        snet = snet.add_edge(SignifierEdge(
            source=canonical,
            target=alternative,
            axis=AxisType.PARADIGMATIC,
            weight=weight,
            evidence=differential,
        ))

    # 额外对（外部传入）
    if extra_pairs:
        for term_a, term_b, weight in extra_pairs:
            if not snet.has_signifier(term_a) or not snet.has_signifier(term_b):
                continue
            snet = snet.add_edge(SignifierEdge(
                source=term_a,
                target=term_b,
                axis=AxisType.PARADIGMATIC,
                weight=weight,
                evidence="",
            ))

    return snet


# ---------------------------------------------------------------------------
# 一体化 bootstrap：从 K_active + 语料 -> 完整 SNet
# ---------------------------------------------------------------------------

def bootstrap_snet(
    graph: Graph,
    surface_forms_path: str | Path | None = None,
    whitelist: frozenset[str] | None = None,
    pmi_threshold: float = 0.0,
    extra_paradigmatic_pairs: list[tuple[str, str, float]] | None = None,
) -> tuple[SNet, dict]:
    """主入口：三层 bootstrap，返回初始化完成的 SNet + 统计报告。

    步骤：
      1. Layer A: 从 graph 投影骨架
      2. Layer B: 从 surface_forms_path 填充组合轴（PMI 过滤，若提供）
      3. Layer C: 写入缠论聚合类种子 + 可选额外对
      4. merge_edge_weights(): 合并同键边，weight 求和（组合轴）

    参数：
      pmi_threshold: Layer B PMI 过滤阈值（默认 0.0 = 正 PMI）
      extra_paradigmatic_pairs: 额外聚合对，传给 Layer C

    返回：
      (snet, stats) — stats 包含 Layer B 过滤统计

    认识论等级：
      - 仅 Layer A: L0
      - Layer A + B: L1（PMI 权重，未 L2 验证组合轴有效性）
      - Layer A + B + C: L1（聚合轴种子为 L0，不升降 Layer B 等级）
    """
    stats: dict = {"pmi_threshold": pmi_threshold}

    # Layer A
    snet = bootstrap_layer_a(graph)

    # Layer B
    if surface_forms_path is not None:
        snet, layer_b_stats = bootstrap_layer_b(
            snet,
            surface_forms_path=surface_forms_path,
            whitelist=whitelist,
            add_missing_signifiers=True,
            pmi_threshold=pmi_threshold,
        )
        stats["layer_b"] = layer_b_stats
    else:
        stats["layer_b"] = None

    # Layer C
    snet = bootstrap_layer_c(snet, extra_pairs=extra_paradigmatic_pairs)

    # 合并权重（组合轴 PMI 值相加，聚合轴保持不变）
    snet = snet.merge_edge_weights()

    # Layer C 统计（merge 后计数）
    par_edges = [e for e in snet.edges if e.axis == AxisType.PARADIGMATIC]
    stats["layer_c"] = {"paradigmatic_edges": len(par_edges)}

    return snet, stats


# ---------------------------------------------------------------------------
# bootstrap 结果报告
# ---------------------------------------------------------------------------

def report_snet(snet: SNet, top_n: int = 10, layer_b_stats: dict | None = None) -> str:
    """生成 SNet 状态报告（用于调试和审计）。

    layer_b_stats: bootstrap_layer_b 返回的统计字典（可选，用于报告 PMI 过滤情况）
    """
    lines: list[str] = []
    sigs = snet.signifiers
    edges = snet.edges

    syn_edges = [e for e in edges if e.axis == AxisType.SYNTAGMATIC]
    par_edges = [e for e in edges if e.axis == AxisType.PARADIGMATIC]

    n_nodes = len(sigs)
    max_possible_edges = n_nodes * (n_nodes - 1)
    actual_edges = len(syn_edges)
    density = actual_edges / max_possible_edges if max_possible_edges > 0 else 0.0

    lines.append("=== S_net 状态报告 ===")
    lines.append(f"能指节点: {n_nodes}")
    lines.append(f"组合轴边: {actual_edges}  (最大可能: {max_possible_edges}, 密度: {density:.1%})")
    lines.append(f"聚合轴边: {len(par_edges)}")
    lines.append("")

    # PMI 过滤统计
    if layer_b_stats:
        eb = layer_b_stats.get("edges_before", "?")
        ea = layer_b_stats.get("edges_after", "?")
        filtered = layer_b_stats.get("filtered", "?")
        thresh = layer_b_stats.get("pmi_threshold", "?")
        lines.append(f"-- PMI 过滤统计 (阈值={thresh}) --")
        lines.append(f"  过滤前边数: {eb}")
        lines.append(f"  过滤后边数: {ea}  (移除: {filtered})")
        high_pairs = layer_b_stats.get("high_pmi_pairs", [])
        if high_pairs:
            lines.append(f"  top-{min(5, len(high_pairs))} 高 PMI 对:")
            for a, b, pmi in high_pairs[:5]:
                lines.append(f"    {a} ↔ {b}  (PMI={pmi:.3f})")
        lines.append("")

    # 来源统计
    source_counts: dict[str, int] = {}
    for sig in sigs.values():
        source_counts[sig.source] = source_counts.get(sig.source, 0) + 1
    lines.append("-- 能指来源 --")
    for src, cnt in sorted(source_counts.items(), key=lambda x: -x[1]):
        lines.append(f"  {src}: {cnt}")
    lines.append("")

    # top-N 高权重组合轴边（PMI 值）
    if syn_edges:
        top_syn = sorted(syn_edges, key=lambda e: -e.weight)[:top_n]
        lines.append(f"-- 组合轴 top-{top_n} (按 PMI 权重) --")
        for e in top_syn:
            ev_preview = e.evidence[:30] + "…" if len(e.evidence) > 30 else e.evidence
            lines.append(f"  {e.source} → {e.target}  (PMI={e.weight:.3f})  「{ev_preview}」")
        lines.append("")

    # top-N 高连接度能指（组合轴）
    degree: dict[str, int] = {}
    for e in syn_edges:
        degree[e.source] = degree.get(e.source, 0) + 1
        degree[e.target] = degree.get(e.target, 0) + 1
    if degree:
        top_degree = sorted(degree.items(), key=lambda x: -x[1])[:top_n]
        lines.append(f"-- 组合轴度数 top-{top_n} --")
        for sid, d in top_degree:
            lines.append(f"  {sid}: {d}")
        lines.append("")

    # 聚合轴摘要
    if par_edges:
        lines.append(f"-- 聚合轴（{len(par_edges)} 条）--")
        for e in sorted(par_edges, key=lambda x: -x.weight)[:top_n]:
            lines.append(f"  {e.source} → {e.target}  (w={e.weight:.2f})  「{e.evidence[:40]}」")

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _demo(surface_forms_path: str | None = None, pmi_threshold: float = 0.0) -> None:
    """演示：从最小图 + 可选语料 bootstrap SNet。"""
    from engine import Graph, Vertex, Edge, EdgeType

    # 最小 K_active（缠论核心概念）
    g = Graph()
    concepts = [
        ("bi", "笔"), ("duan", "段"), ("zhongshu", "中枢"),
        ("zoushi", "走势"), ("maichadian", "买点"), ("faxing", "分型"),
        ("zoushileixing", "走势类型"), ("zhanzheng", "趋势"), ("panzheng", "盘整"),
        ("beichu", "背驰"), ("pandzhengbeichu", "盘整背驰"), ("qushileixing", "趋势背驰"),
        ("xianxduan", "线段"), ("tezhengxulie", "特征序列"), ("dinggfenxing", "顶分型"),
        ("digufenxing", "底分型"),
    ]
    for vid, content in concepts:
        g = g.add_vertex(Vertex(id=vid, content=content))

    deps = [("bi", "duan"), ("duan", "zhongshu"), ("zhongshu", "zoushi")]
    for src, tgt in deps:
        g = g.add_edge(Edge(source=src, target=tgt, edge_type=EdgeType.DEPENDENCY))

    # Bootstrap
    from vertex_cleaning import CHANLUN_WHITELIST

    snet, stats = bootstrap_snet(
        graph=g,
        surface_forms_path=surface_forms_path,
        whitelist=CHANLUN_WHITELIST if surface_forms_path else None,
        pmi_threshold=pmi_threshold,
    )

    layer_b_stats = stats.get("layer_b")
    print(report_snet(snet, top_n=10, layer_b_stats=layer_b_stats))

    print()
    print("-- 中枢 的组合轴邻居 (PMI 降序) --")
    for e in snet.syntagmatic_neighbors("中枢")[:5]:
        print(f"  → {e.target}  (PMI={e.weight:.3f})  「{e.evidence[:40]}」")

    print()
    print("-- 中枢 的聚合轴替换项 --")
    for e in snet.paradigmatic_alternatives("中枢")[:5]:
        print(f"  → {e.target}  (w={e.weight:.2f})  「{e.evidence[:50]}」")


if __name__ == "__main__":
    import os
    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    # 尝试加载真实语料
    sf_path = Path(script_dir) / "data" / "chanlun_surface_forms.jsonl"
    if sf_path.exists():
        print(f"加载语料: {sf_path}")
        _demo(str(sf_path), pmi_threshold=0.0)
    else:
        print("未找到语料文件，仅演示 Layer A + Layer C")
        _demo(None, pmi_threshold=0.0)
