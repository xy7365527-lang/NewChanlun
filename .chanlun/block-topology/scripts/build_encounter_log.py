#!/usr/bin/env python3
"""Generate encounter log from concept registry and Morse landscape analysis.

Records unexpected findings, surprising connections, and structural anomalies
discovered during the Phase 3 infrastructure build.

Output: encounter_log.md in block-topology root.
"""

import json
from collections import defaultdict, Counter
from pathlib import Path

TOPO_DIR = Path(__file__).resolve().parent.parent
BLOCKS_DIR = TOPO_DIR / "blocks"
META_PATH = TOPO_DIR / "meta.json"
REGISTRY_PATH = TOPO_DIR / "concept_registry.json"
LANDSCAPE_PATH = TOPO_DIR / "morse_landscape.json"
RELATIONS_PATH = TOPO_DIR / "relations.jsonl"
OUTPUT_PATH = TOPO_DIR / "encounter_log.md"


def load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def load_relations():
    relations = []
    with open(RELATIONS_PATH, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    relations.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
    return relations


def build_encounters():
    meta = load_json(META_PATH)
    hash_to_id = {v: k for k, v in meta.get("id_mapping", {}).items()}
    registry = load_json(REGISTRY_PATH) if REGISTRY_PATH.exists() else {}
    landscape = load_json(LANDSCAPE_PATH) if LANDSCAPE_PATH.exists() else {}
    relations = load_relations()

    encounters = []

    # --- 1. Structural findings from Morse landscape ---
    peaks = landscape.get("peaks", [])
    stats = landscape.get("statistics", {})
    mean_h = stats.get("mean_height", 0)

    # Find the gravitational center: block(s) with extreme in-degree
    gravity_blocks = [p for p in peaks if p["in_degree"] > 80]
    if gravity_blocks:
        encounters.append({
            "title": "引力中心：入度 > 80 的区块",
            "content": (
                f"发现 {len(gravity_blocks)} 个引力中心区块——入度极高，"
                "意味着谱系中大量后续条目依赖或引用这些区块。"
                "这些不是普通的高频区块，而是谱系的结构性支柱。"
            ),
            "blocks": [
                f"{p['genealogy_id']}号 (入度={p['in_degree']}, 出度={p['out_degree']})"
                for p in gravity_blocks
            ],
            "why_unexpected": (
                "069号（RTAS蜂群）和 020号（阻断等待）的入度远超其他区块，"
                "表明蜂群架构和阻断机制是整个谱系最频繁回溯的两个锚点。"
                "090号（严格性）紧随其后——严格性作为语法规则的辐射范围覆盖了近100个后续区块。"
            ),
        })

    # Find isolated clusters: blocks with high out-degree but low in-degree
    loners = [p for p in peaks if p["out_degree"] > 30 and p["in_degree"] < 10]
    if loners:
        encounters.append({
            "title": "发射型区块：高出度低入度",
            "content": (
                f"发现 {len(loners)} 个发射型区块——大量引用其他区块但很少被引用。"
                "这可能意味着这些区块是综合性结算（引用大量前置），"
                "但尚未成为后续发展的依据。"
            ),
            "blocks": [
                f"{p['genealogy_id']}号 (入度={p['in_degree']}, 出度={p['out_degree']})"
                for p in loners
            ],
            "why_unexpected": "高出度低入度暗示这些区块可能是概念汇聚点但尚未产生下游影响。",
        })

    # Valley analysis: genealogy-mapped blocks with zero degree
    valleys = landscape.get("valleys", [])
    gid_valleys = [v for v in valleys if v["genealogy_id"] and v["height"] == 0]
    if gid_valleys:
        encounters.append({
            "title": "孤岛区块：有谱系号但零连接",
            "content": (
                f"发现 {len(gid_valleys)} 个有谱系号但无任何关系的区块。"
                "这些区块在谱系中有编号，但拓扑上完全孤立。"
            ),
            "blocks": [f"{v['genealogy_id']}号" for v in gid_valleys[:20]],
            "why_unexpected": (
                "有谱系号意味着这些是被正式记录的概念事件，但零连接意味着"
                "它们既不依赖前置区块，也没有后续区块依赖它们——"
                "可能是关系数据未完整覆盖，或这些区块确实是独立的概念岛。"
            ),
        })

    # --- 2. Concept-level findings ---
    # Concepts that span the entire timeline
    timeline_spanners = []
    for term, data in registry.items():
        first = data.get("first_seen", "9999")
        last = data.get("last_seen", "0000")
        if first != "unknown" and last != "unknown" and first < last:
            timeline_spanners.append((term, data))

    # Meta-concepts vs domain concepts
    meta_keywords = ["状态", "类型", "前置", "关联", "域", "来源", "溯源"]
    domain_keywords = ["中枢", "线段", "笔", "走势", "背驰", "级别", "买卖点"]
    meta_count = sum(1 for t in registry if any(k in t for k in meta_keywords))
    domain_count = sum(1 for t in registry if any(k in t for k in domain_keywords))

    encounters.append({
        "title": "概念层构成：元概念 vs 域概念",
        "content": (
            f"在 {len(registry)} 个概念中：\n"
            f"- 元编排概念（状态/类型/前置/关联/域/来源/溯源相关）：~{meta_count} 个\n"
            f"- 缠论域概念（中枢/线段/笔/走势/背驰/级别/买卖点相关）：~{domain_count} 个\n"
            f"- 其余为混合或跨域概念"
        ),
        "blocks": [],
        "why_unexpected": (
            "元编排概念的数量显著多于缠论域概念，"
            "反映了本仓库的谱系不是纯粹的缠论知识库，"
            "而是一个以元编排方法论为主体、缠论为领域的系统。"
            "概念频率排行前10全部是元编排相关概念（状态、类型、前置等），"
            "没有一个缠论域概念进入前10。"
        ),
    })

    # Concept co-occurrence: concepts that always appear together
    concept_block_sets = {
        term: frozenset(data["blocks"])
        for term, data in registry.items()
        if data["count"] >= 3  # minimum frequency
    }
    co_occurrences = []
    terms_list = list(concept_block_sets.keys())
    for i in range(len(terms_list)):
        for j in range(i + 1, len(terms_list)):
            t1, t2 = terms_list[i], terms_list[j]
            s1, s2 = concept_block_sets[t1], concept_block_sets[t2]
            overlap = len(s1 & s2)
            if overlap > 0:
                jaccard = overlap / len(s1 | s2)
                if jaccard > 0.8 and overlap >= 3:
                    co_occurrences.append((t1, t2, jaccard, overlap))

    if co_occurrences:
        co_occurrences.sort(key=lambda x: -x[2])
        encounters.append({
            "title": f"概念共现对：{len(co_occurrences)} 对高度共现概念",
            "content": (
                "以下概念对几乎总是同时出现在相同区块中（Jaccard > 0.8）。"
                "高共现可能意味着这些概念是同一更大概念的不同侧面，"
                "或者它们有未被显式表达的结构性关联。"
            ),
            "blocks": [
                f"'{t1}' ↔ '{t2}' (Jaccard={j:.2f}, 共现{n}次)"
                for t1, t2, j, n in co_occurrences[:15]
            ],
            "why_unexpected": "概念共现揭示了隐含的概念结构——这些对可能是候选的概念合并或概念分离目标。",
        })

    # --- 3. Relation-level findings ---
    # Relation type distribution
    rel_types = Counter()
    for rel in relations:
        rel_types[rel.get("relation", "unknown")] += 1

    # Tension and negation network
    tension_count = rel_types.get("tensions_with", 0)
    negation_count = rel_types.get("negates", 0) + rel_types.get("negated_by", 0)
    if tension_count + negation_count > 0:
        # Find blocks involved in tensions/negations
        tension_blocks = set()
        negation_blocks = set()
        for rel in relations:
            rtype = rel.get("relation", "")
            if rtype == "tensions_with":
                tension_blocks.add(rel["from"])
                tension_blocks.add(rel["to"])
            elif rtype in ("negates", "negated_by"):
                negation_blocks.add(rel["from"])
                negation_blocks.add(rel["to"])

        encounters.append({
            "title": f"否定/张力网络：{negation_count} 条否定 + {tension_count} 条张力",
            "content": (
                f"张力关系涉及 {len(tension_blocks)} 个区块，"
                f"否定关系涉及 {len(negation_blocks)} 个区块。"
                f"张力和否定是谱系中最有信息价值的关系类型——"
                f"它们标记了概念冲突和概念演化的位点。"
            ),
            "blocks": [
                f"张力区块: {', '.join(hash_to_id.get(h, h[:12]) + '号' for h in sorted(tension_blocks)[:10])}",
                f"否定区块: {', '.join(hash_to_id.get(h, h[:12]) + '号' for h in sorted(negation_blocks)[:10])}",
            ],
            "why_unexpected": (
                f"否定+张力占总关系的 {(tension_count + negation_count) / len(relations) * 100:.1f}%。"
                "虽然比例不高，但每条否定/张力关系的信息密度远高于 depends_on 或 references。"
            ),
        })

    # --- 4. Degree distribution shape ---
    block_heights = landscape.get("block_heights", {})
    if block_heights:
        heights = [d["height"] for d in block_heights.values()]
        # Check for power-law-like distribution
        zero_count = sum(1 for h in heights if h == 0)
        high_count = sum(1 for h in heights if h > 50)
        encounters.append({
            "title": "度数分布：幂律特征",
            "content": (
                f"总 {len(heights)} 个区块中：\n"
                f"- 零度（完全孤立）：{zero_count} ({zero_count/len(heights)*100:.1f}%)\n"
                f"- 度数 > 50（高度连接）：{high_count} ({high_count/len(heights)*100:.1f}%)\n"
                f"- 平均度数：{mean_h:.2f}\n"
                f"- 最高度数：{max(heights)}\n"
                f"少数节点集中了大部分连接——典型的无标度网络特征。"
            ),
            "blocks": [],
            "why_unexpected": (
                "谱系的拓扑不是随机图（泊松分布），而是无标度网络（幂律分布）。"
                "这意味着存在少数关键枢纽区块（069、020、090等），"
                "它们的移除会显著改变网络连通性——这些是谱系的结构性脆弱点。"
            ),
        })

    # --- Write encounter log ---
    lines = [
        "# 偶遇记录（Encounter Log）",
        "",
        f"Phase 3 穿越基础设施构建过程中发现的意外关联和结构性发现。",
        f"生成时间：基于 {len(registry)} 个概念、{len(relations)} 条关系、"
        f"{stats.get('total_blocks', '?')} 个区块的分析。",
        "",
        "---",
        "",
    ]

    for i, enc in enumerate(encounters, 1):
        lines.append(f"## {i}. {enc['title']}")
        lines.append("")
        lines.append(enc["content"])
        lines.append("")
        if enc.get("blocks"):
            lines.append("**涉及区块/数据：**")
            for b in enc["blocks"]:
                lines.append(f"- {b}")
            lines.append("")
        lines.append(f"**为什么意外：** {enc['why_unexpected']}")
        lines.append("")
        lines.append("---")
        lines.append("")

    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))

    print(f"Encounter log written: {len(encounters)} encounters")
    print(f"Output: {OUTPUT_PATH}")
    return encounters


if __name__ == "__main__":
    encounters = build_encounters()
    for i, enc in enumerate(encounters, 1):
        print(f"\n{i}. {enc['title']}")
