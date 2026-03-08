"""signifier_net_ingest.py — 辞典摄入 + 文本段落摄入 Pipeline.

辞典摄入（原有）：
从 JSONL 辞典文件向 S_net 注入结构化关系：
  - synonyms → 聚合轴边 (paradigmatic, relation=synonym)
  - contrasts → 聚合轴边 (paradigmatic, relation=contrast)
  - definition → 组合轴边 (syntagmatic, evidence=definition context)

辞典格式 (每行一个 JSON):
  {"term": str, "domain": str, "definition": str,
   "synonyms": [str, ...], "contrasts": [{"term": str, "differential": str}, ...]}

文本段落摄入（v198-swarm/text-ingest-pipeline 新增）：
从原文段落中提取已知术语共现关系和 surface forms，丰富 S_net 的语言材料。
不提取 vertices/edges，不修改 K_active。

认识论等级：L0（辞典是手工编纂的定义；文本摄入是确定性字符串匹配，不涉及经验假设）

谱系引用：辞典摄入由 v198-swarm/dict-ingest 创建。文本段落摄入由 v198-swarm/text-ingest-pipeline 创建。
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from signifier_net import (
    SNet, Signifier, SignifierEdge, AxisType,
    Morpheme, MorphemeStructure,
)


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
        entry_lang = entry.get("lang", "").strip()

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
                    lang=entry_lang,
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


# ---------------------------------------------------------------------------
# 文本段落摄入：从原文中提取术语共现 + surface forms
# ---------------------------------------------------------------------------

def _split_paragraphs(text: str) -> list[str]:
    """将文本按空行分割为段落。

    连续的非空行合并为一个段落。
    过滤掉过短的段落（< 10 字符）。

    认识论等级：L0（确定性字符串操作）
    """
    paragraphs: list[str] = []
    current: list[str] = []

    for line in text.split("\n"):
        stripped = line.strip()
        if not stripped:
            if current:
                para = "\n".join(current).strip()
                if len(para) >= 10:
                    paragraphs.append(para)
                current = []
        else:
            current.append(stripped)

    if current:
        para = "\n".join(current).strip()
        if len(para) >= 10:
            paragraphs.append(para)

    return paragraphs


def _find_surface_form(text: str, term: str) -> str | None:
    """提取术语在原文中的实际出现形式。

    如果术语直接出现在文本中，返回包含该术语的最短子句作为 surface form。
    子句以中文/英文标点分割。

    认识论等级：L0（确定性字符串操作）
    """
    pos = text.find(term)
    if pos < 0:
        return None

    # 向前找句边界
    start = pos
    for i in range(pos - 1, max(pos - 30, -1), -1):
        if i < 0:
            start = 0
            break
        if text[i] in ("，", "。", "；", "、", ",", ".", ";", "\n"):
            start = i + 1
            break
    else:
        start = max(0, pos - 30)

    # 向后找句边界
    end = pos + len(term)
    for i in range(end, min(end + 30, len(text))):
        if text[i] in ("，", "。", "；", "、", ",", ".", ";", "\n"):
            end = i
            break
    else:
        end = min(len(text), end + 30)

    form = text[start:end].strip()
    if len(form) > 80:
        form = form[:80]

    return form if form else None


def ingest_text_passage(
    snet: SNet,
    text: str,
    domain: str,
    source: str,
) -> tuple[SNet, list[dict]]:
    """从原文段落中提取 connective_patterns 并写入 S_net。

    不提取 vertices/edges！只提取：
    1. 术语共现（同一段落中出现的已知术语对）→ 组合轴边的 connective_patterns
    2. 术语的 surface forms（该术语在原文中的实际出现形式）
    3. 段落上下文（用于后续 articulation feedback 共振时参照）

    此函数不修改 K_active——它只丰富 S_net 的语言材料。
    K_active 的修改由 articulation feedback 在穿越中完成。

    参数：
      snet:    当前 S_net 实例（不会被修改）
      text:    原文段落文本
      domain:  语料域标记（如 "hegel", "marx"）
      source:  来源标记（如 "phenomenology_of_spirit.md"）

    返回：
      (new_snet, log_entries)
      - new_snet: 包含新增共现边和 surface forms 的 SNet
      - log_entries: 摄入日志条目列表

    认识论等级：L0（确定性字符串匹配，不涉及经验假设）
    """
    if not text or not snet.signifiers:
        return snet, []

    # 构建白名单（S_net 中所有能指 id）
    whitelist = set(snet.signifiers.keys())

    # 匹配段落中的已知术语（使用本模块的白名单匹配函数）
    matched_terms = phi_L_whitelist_match(text, whitelist)
    if len(matched_terms) < 2:
        # 不足两个术语，无法产生共现对
        # 但仍可提取 surface forms
        if matched_terms:
            term = matched_terms[0]
            sf = _find_surface_form(text, term)
            if sf:
                existing = snet.signifiers.get(term)
                if existing and sf not in existing.surface_forms:
                    new_forms = existing.surface_forms + (sf,)
                    snet = snet.add_signifier(Signifier(
                        id=existing.id,
                        surface_forms=new_forms,
                        source=existing.source,
                    ))
        return snet, []

    new_snet = snet
    log_entries: list[dict] = []
    evidence_tag = f"corpus:{domain}:{source}"

    # 1. 提取 surface forms 并更新 signifiers
    for term in matched_terms:
        sf = _find_surface_form(text, term)
        if not sf:
            continue
        existing = new_snet.signifiers.get(term)
        if existing and sf not in existing.surface_forms:
            # 限制 surface_forms 数量，避免内存膨胀
            if len(existing.surface_forms) < 20:
                new_forms = existing.surface_forms + (sf,)
                new_snet = new_snet.add_signifier(Signifier(
                    id=existing.id,
                    surface_forms=new_forms,
                    source=existing.source,
                ))

    # 2. 提取术语共现对 → 组合轴边
    for i in range(len(matched_terms)):
        for j in range(i + 1, len(matched_terms)):
            term_a = matched_terms[i]
            term_b = matched_terms[j]

            if not new_snet.has_signifier(term_a) or not new_snet.has_signifier(term_b):
                continue

            # 提取两个术语之间的连接模式
            pattern = extract_pattern(text, term_a, term_b)
            evidence = pattern if pattern else evidence_tag

            new_snet = new_snet.add_edge(SignifierEdge(
                source=term_a,
                target=term_b,
                axis=AxisType.SYNTAGMATIC,
                weight=1.0,
                evidence=evidence,
            ))

            log_entries.append({
                "type": "cooccurrence",
                "source": term_a,
                "target": term_b,
                "pattern": pattern,
                "domain": domain,
                "corpus_ref": evidence_tag,
            })

    # 合并同键边（weight 求和）
    if log_entries:
        new_snet = new_snet.merge_edge_weights()

    return new_snet, log_entries


# ---------------------------------------------------------------------------
# 双语辞典摄入
# ---------------------------------------------------------------------------

def ingest_bilingual_dict(
    snet: SNet,
    dict_path: str | Path,
) -> tuple[SNet, dict]:
    """从双语 JSONL 辞典文件向 S_net 注入跨语言翻译关系。

    JSONL 格式（每行一个 JSON）：
      {"term_a": str, "lang_a": str, "term_b": str, "lang_b": str,
       "domain": str, "differential": str, "source": str}

    处理逻辑：
      1. 为 term_a 和 term_b 创建 Signifier（带 lang），若已存在则保留
      2. 创建 PARADIGMATIC 边（relation="translation", differential=entry 的 differential）

    参数：
      snet:      当前 S_net 实例
      dict_path: 双语辞典 JSONL 文件路径

    返回：
      (new_snet, stats) — stats 包含摄入统计

    认识论等级：L0（辞典是手工编纂的定义）
    """
    path = Path(dict_path)
    if not path.exists():
        raise FileNotFoundError(f"双语辞典文件不存在: {path}")

    stats = {
        "file": str(path.name),
        "type": "bilingual",
        "entries_total": 0,
        "translations_added": 0,
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
        term_a = entry.get("term_a", "").strip()
        term_b = entry.get("term_b", "").strip()
        lang_a = entry.get("lang_a", "").strip()
        lang_b = entry.get("lang_b", "").strip()
        domain = entry.get("domain", "").strip()
        differential = entry.get("differential", "").strip()
        source = entry.get("source", "").strip()

        if not term_a or not term_b:
            continue

        # 创建 Signifier（若不存在）
        if not snet.has_signifier(term_a):
            snet = snet.add_signifier(Signifier(
                id=term_a,
                surface_forms=(),
                source="bilingual_dictionary",
                lang=lang_a,
                domain=domain,
            ))
            stats["signifiers_created"] += 1

        if not snet.has_signifier(term_b):
            snet = snet.add_signifier(Signifier(
                id=term_b,
                surface_forms=(),
                source="bilingual_dictionary",
                lang=lang_b,
                domain=domain,
            ))
            stats["signifiers_created"] += 1

        # 创建翻译边（双向）
        evidence = f"bilingual:{source}" if source else f"bilingual:{path.stem}"
        snet = snet.add_edge(SignifierEdge(
            source=term_a,
            target=term_b,
            axis=AxisType.PARADIGMATIC,
            weight=0.9,
            evidence=evidence,
            relation="translation",
            differential=differential,
        ))
        snet = snet.add_edge(SignifierEdge(
            source=term_b,
            target=term_a,
            axis=AxisType.PARADIGMATIC,
            weight=0.9,
            evidence=evidence,
            relation="translation",
            differential=differential,
        ))
        stats["translations_added"] += 1

    # 合并同键边
    snet = snet.merge_edge_weights()

    return snet, stats


# ---------------------------------------------------------------------------
# 语素辞典摄入
# ---------------------------------------------------------------------------

def ingest_morpheme_dict(
    snet: SNet,
    dict_path: str | Path,
) -> tuple[SNet, dict]:
    """从语素 JSONL 辞典文件向 S_net 注入语素分解结构。

    JSONL 格式（每行一个 JSON）：
      {"signifier_id": str, "lang": str,
       "morphemes": [{"form": str, "meaning": str}, ...],
       "etymology": str}

    处理逻辑：
      1. 创建 MorphemeStructure 并添加到 S_net
      2. 对 shared_with 中的能指对创建 MORPHEME 边

    参数：
      snet:      当前 S_net 实例
      dict_path: 语素辞典 JSONL 文件路径

    返回：
      (new_snet, stats) — stats 包含摄入统计

    认识论等级：L0（辞典是手工编纂的定义）
    """
    path = Path(dict_path)
    if not path.exists():
        raise FileNotFoundError(f"语素辞典文件不存在: {path}")

    stats = {
        "file": str(path.name),
        "type": "morpheme",
        "entries_total": 0,
        "structures_added": 0,
        "morpheme_edges_added": 0,
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
        sig_id = entry.get("signifier_id", "").strip()
        lang = entry.get("lang", "").strip()
        etymology = entry.get("etymology", "").strip()
        raw_morphemes = entry.get("morphemes", [])

        if not sig_id or not raw_morphemes:
            continue

        # 构建 Morpheme tuple
        morphemes: list[Morpheme] = []
        for m in raw_morphemes:
            form = m.get("form", "").strip()
            meaning = m.get("meaning", "").strip()
            shared = tuple(s.strip() for s in m.get("shared_with", []) if s.strip())
            if form:
                morphemes.append(Morpheme(
                    form=form,
                    meaning=meaning,
                    lang=lang,
                    shared_with=shared,
                ))

        if not morphemes:
            continue

        # 创建 MorphemeStructure
        ms = MorphemeStructure(
            signifier_id=sig_id,
            morphemes=tuple(morphemes),
            etymology=etymology,
        )
        snet = snet.add_morpheme_structure(ms)
        stats["structures_added"] += 1

        # 对 shared_with 中的能指对创建 MORPHEME 边
        for morph in morphemes:
            for other_id in morph.shared_with:
                if other_id != sig_id and snet.has_signifier(other_id):
                    snet = snet.add_edge(SignifierEdge(
                        source=sig_id,
                        target=other_id,
                        axis=AxisType.MORPHEME,
                        weight=0.7,
                        evidence=f"shared morpheme: {morph.form} ({morph.meaning})",
                        relation="morpheme_link",
                    ))
                    stats["morpheme_edges_added"] += 1

    # 合并同键边
    snet = snet.merge_edge_weights()

    return snet, stats
