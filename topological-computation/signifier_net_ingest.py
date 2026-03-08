"""signifier_net_ingest.py — 辞典摄入 Pipeline.

从 JSONL 辞典文件向 S_net 注入结构化关系：
  - synonyms → 聚合轴边 (paradigmatic, relation=synonym)
  - contrasts → 聚合轴边 (paradigmatic, relation=contrast)
  - definition → 组合轴边 (syntagmatic, evidence=definition context)

辞典格式 (每行一个 JSON):
  {"term": str, "domain": str, "definition": str,
   "synonyms": [str, ...], "contrasts": [{"term": str, "differential": str}, ...]}

认识论等级：L0（辞典是手工编纂的定义，不涉及经验假设）

谱系引用：此模块由 v198-swarm/dict-ingest 创建。
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from signifier_net import SNet, Signifier, SignifierEdge, AxisType


# ---------------------------------------------------------------------------
# 白名单匹配：从定义文本中提取已知术语
# ---------------------------------------------------------------------------

def phi_L_whitelist_match(
    text: str,
    whitelist: set[str],
) -> list[str]:
    """从文本中匹配白名单中的术语。

    简单的子串匹配（按长度降序贪心），避免短术语被长术语包含时重复报告。
    例如"走势类型"匹配后，不再单独匹配"走势"和"类型"。

    认识论等级：L0（确定性字符串匹配）
    """
    if not text or not whitelist:
        return []

    # 按长度降序排列，长术语优先匹配
    sorted_terms = sorted(whitelist, key=len, reverse=True)
    matched: list[str] = []
    remaining = text

    for term in sorted_terms:
        if term in remaining:
            matched.append(term)
            # 移除已匹配的术语（防止子串重复匹配）
            remaining = remaining.replace(term, " " * len(term))

    return matched


def extract_pattern(
    definition: str,
    term_a: str,
    term_b: str,
) -> str:
    """从定义文本中提取两个术语之间的连接模式。

    尝试提取包含两个术语的最短子句。
    如果找不到同时包含两个术语的片段，返回空字符串。

    认识论等级：L0（确定性文本操作）
    """
    if term_a not in definition or term_b not in definition:
        return ""

    # 找到两个术语在文本中的位置范围
    pos_a = definition.find(term_a)
    pos_b = definition.find(term_b)

    start = min(pos_a, pos_b)
    end_a = pos_a + len(term_a)
    end_b = pos_b + len(term_b)
    end = max(end_a, end_b)

    # 扩展到句边界（中文逗号/句号/分号）
    sent_start = start
    sent_end = end
    for delim in ("，", "。", "；", ",", ".", ";"):
        prev = definition.rfind(delim, 0, start)
        if prev != -1 and prev > sent_start - 20:
            sent_start = prev + 1

        nxt = definition.find(delim, end)
        if nxt != -1 and nxt < sent_end + 20:
            sent_end = nxt

    pattern = definition[sent_start:sent_end].strip()
    # 限制长度
    if len(pattern) > 100:
        pattern = pattern[:100]

    return pattern


# ---------------------------------------------------------------------------
# 辞典摄入主函数
# ---------------------------------------------------------------------------

def ingest_dictionary(
    snet: SNet,
    dict_path: str | Path,
    domain_whitelist: set[str] | None = None,
) -> tuple[SNet, dict]:
    """从 JSONL 辞典文件向 S_net 注入关系。

    处理逻辑：
      1. synonyms → 聚合轴边 (relation=synonym)
         - 若 synonym 不在 S_net 中，创建新 signifier (source="dictionary")
         - synonym 边是非对称的（canonical → synonym）
      2. contrasts → 聚合轴边 (relation=contrast)
         - 仅当 contrast term 已在 S_net 中时创建边
         - contrast 边是对称的（双向）
      3. definition → 组合轴边
         - 从 definition 文本中匹配已知术语
         - 为匹配到的术语对创建/增强组合轴边

    参数：
      snet:             当前 S_net 实例
      dict_path:        辞典 JSONL 文件路径
      domain_whitelist: 用于 definition 匹配的术语白名单
                        （None 时使用 S_net 中所有 signifier id）

    返回：
      (new_snet, stats) — stats 包含摄入统计

    认识论等级：L0（辞典是手工编纂的定义）
    """
    path = Path(dict_path)
    if not path.exists():
        raise FileNotFoundError(f"辞典文件不存在: {path}")

    # 如果没有提供白名单，使用 S_net 中所有 signifier id
    if domain_whitelist is None:
        domain_whitelist = set(snet.signifiers.keys())

    stats = {
        "file": str(path.name),
        "entries_total": 0,
        "entries_matched": 0,
        "synonyms_added": 0,
        "contrasts_added": 0,
        "syntagmatic_added": 0,
        "signifiers_created": 0,
    }

    entries: list[dict] = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                entries.append(json.loads(line))
            except json.JSONDecodeError:
                continue

    stats["entries_total"] = len(entries)

    for entry in entries:
        term = entry.get("term", "").strip()
        domain = entry.get("domain", "").strip()

        if not term:
            continue

        # 主术语必须已在 S_net 中（辞典不创建主节点，只丰富已有节点）
        if not snet.has_signifier(term):
            continue

        stats["entries_matched"] += 1

        # 1. synonyms → 聚合轴
        for syn in entry.get("synonyms", []):
            syn = syn.strip()
            if not syn or syn == term:
                continue

            # 若 synonym 不在 S_net 中，创建新 signifier
            if not snet.has_signifier(syn):
                snet = snet.add_signifier(Signifier(
                    id=syn,
                    surface_forms=(),
                    source="dictionary",
                ))
                stats["signifiers_created"] += 1

            # 添加聚合轴边（synonym 关系）
            snet = snet.add_edge(SignifierEdge(
                source=term,
                target=syn,
                axis=AxisType.PARADIGMATIC,
                weight=0.85,
                evidence=f"辞典近义项 ({domain})",
            ))
            stats["synonyms_added"] += 1

        # 2. contrasts → 聚合轴
        for contrast in entry.get("contrasts", []):
            c_term = contrast.get("term", "").strip()
            c_diff = contrast.get("differential", "").strip()
            if not c_term or c_term == term:
                continue

            # contrast 边仅当对方已在 S_net 中时创建
            if not snet.has_signifier(c_term):
                continue

            # 添加对称的 contrast 边（A→B 和 B→A）
            snet = snet.add_edge(SignifierEdge(
                source=term,
                target=c_term,
                axis=AxisType.PARADIGMATIC,
                weight=0.6,
                evidence=c_diff or f"辞典对比项 ({domain})",
            ))
            snet = snet.add_edge(SignifierEdge(
                source=c_term,
                target=term,
                axis=AxisType.PARADIGMATIC,
                weight=0.6,
                evidence=c_diff or f"辞典对比项 ({domain})",
            ))
            stats["contrasts_added"] += 1

        # 3. definition → 组合轴
        definition = entry.get("definition", "").strip()
        if not definition:
            continue

        mentioned = phi_L_whitelist_match(definition, domain_whitelist)
        for other_term in mentioned:
            if other_term == term:
                continue
            if not snet.has_signifier(other_term):
                continue

            pattern = extract_pattern(definition, term, other_term)
            evidence = pattern if pattern else f"dict:{term}:{domain}"

            snet = snet.add_edge(SignifierEdge(
                source=term,
                target=other_term,
                axis=AxisType.SYNTAGMATIC,
                weight=1.0,  # 辞典定义中的共现，固定权重
                evidence=evidence,
            ))
            stats["syntagmatic_added"] += 1

    # 合并同键边（weight 求和）
    snet = snet.merge_edge_weights()

    return snet, stats


# ---------------------------------------------------------------------------
# 批量摄入：从目录加载所有辞典
# ---------------------------------------------------------------------------

def ingest_all_dictionaries(
    snet: SNet,
    dict_dir: str | Path,
    domain_whitelist: set[str] | None = None,
) -> tuple[SNet, list[dict]]:
    """从目录中加载所有 dict_*.jsonl 文件并摄入。

    参数：
      snet:             当前 S_net 实例
      dict_dir:         辞典目录路径
      domain_whitelist: 术语白名单（None 时使用 S_net 中所有 signifier id）

    返回：
      (new_snet, all_stats) — all_stats 是每个文件的摄入统计列表
    """
    dir_path = Path(dict_dir)
    if not dir_path.is_dir():
        return snet, []

    all_stats: list[dict] = []

    # 按文件名排序（确定性顺序）
    dict_files = sorted(dir_path.glob("dict_*.jsonl"))

    for dict_file in dict_files:
        try:
            snet, stats = ingest_dictionary(
                snet,
                dict_file,
                domain_whitelist=domain_whitelist,
            )
            all_stats.append(stats)
        except Exception as exc:
            print(
                f"辞典摄入失败 ({dict_file.name}): {exc}",
                file=sys.stderr,
            )
            all_stats.append({
                "file": str(dict_file.name),
                "error": str(exc),
            })

    return snet, all_stats


# ---------------------------------------------------------------------------
# 摄入报告
# ---------------------------------------------------------------------------

def format_ingest_report(all_stats: list[dict]) -> str:
    """格式化摄入统计报告。"""
    lines: list[str] = ["=== 辞典摄入报告 ==="]

    total_matched = 0
    total_syn = 0
    total_contrast = 0
    total_syntag = 0
    total_created = 0

    for stats in all_stats:
        if "error" in stats:
            lines.append(f"  {stats['file']}: ERROR - {stats['error']}")
            continue

        matched = stats.get("entries_matched", 0)
        total = stats.get("entries_total", 0)
        syn = stats.get("synonyms_added", 0)
        contrast = stats.get("contrasts_added", 0)
        syntag = stats.get("syntagmatic_added", 0)
        created = stats.get("signifiers_created", 0)

        total_matched += matched
        total_syn += syn
        total_contrast += contrast
        total_syntag += syntag
        total_created += created

        lines.append(
            f"  {stats['file']}: "
            f"{matched}/{total} 条匹配, "
            f"+{syn} synonym, +{contrast} contrast, "
            f"+{syntag} syntagmatic, +{created} new signifiers"
        )

    lines.append(
        f"  合计: {total_matched} 条匹配, "
        f"+{total_syn} synonym, +{total_contrast} contrast, "
        f"+{total_syntag} syntagmatic, +{total_created} new signifiers"
    )

    return "\n".join(lines)
