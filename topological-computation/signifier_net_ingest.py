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
from cooccurrence_hyperedge import CooccurrenceHyperedge


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

def _build_head_term_index(snet: SNet) -> dict[str, str]:
    """从 S_net 的所有 signifier ID 构建 head-term → signifier_id 索引。

    Layer A 创建的 signifier ID 是复合描述短语（如 "culture industry — mass deception"）。
    辞典术语是规范短语（如 "culture industry"）。需要一个桥接索引。

    提取规则：
      - "X — Y" 模式：head = X（em-dash 前部分）
      - "X (Y)" 模式：head = X（括号前部分）
      - 纯短语：head = 完整 ID（已经是规范形式）

    冲突解决：当多个 signifier ID 映射到同一个 head term 时，保留最短的
    signifier ID（最接近规范形式）。

    认识论等级：L0（确定性字符串操作）
    """
    index: dict[str, str] = {}

    for sid in snet.signifiers:
        # 完整 ID 已经在 snet 中，无需索引
        # 索引的是 head term → 完整 sid 的映射

        head = sid  # 默认：完整 ID

        # "X — Y" 模式
        if " — " in sid:
            head = sid.split(" — ")[0].strip()
        # "X (Y)" 模式（不含 em-dash 的情况）
        elif "(" in sid:
            head = sid[:sid.index("(")].strip()

        if head == sid:
            # 完整 ID 就是 head term，无需索引（has_signifier 已覆盖）
            continue

        if not head:
            continue

        # 冲突解决：保留最短 signifier ID
        if head in index:
            if len(sid) < len(index[head]):
                index[head] = sid
        else:
            index[head] = sid

    return index


def _resolve_term(
    term: str,
    snet: SNet,
    head_index: dict[str, str],
) -> str | None:
    """解析辞典术语到 S_net signifier ID。

    优先级：
      1. 精确匹配（term 是 signifier ID）
      2. Head-term 索引匹配（term 是某个复合 signifier 的 head）
      3. 不区分大小写的 head-term 索引匹配

    返回 signifier ID 或 None。
    """
    # 1. 精确匹配
    if snet.has_signifier(term):
        return term

    # 2. Head-term 索引
    if term in head_index:
        return head_index[term]

    # 3. 大小写不敏感
    term_lower = term.lower()
    for head, sid in head_index.items():
        if head.lower() == term_lower:
            return sid

    return None


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

    术语解析：辞典术语通过 head-term 索引桥接到 Layer A 的复合 signifier ID。
    例如辞典术语 "culture industry" 桥接到 signifier ID "culture industry — mass deception"。

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

    # 构建 head-term 索引用于术语解析
    head_index = _build_head_term_index(snet)

    # 如果没有提供白名单，使用 S_net 中所有 signifier id
    if domain_whitelist is None:
        domain_whitelist = set(snet.signifiers.keys())

    stats = {
        "file": str(path.name),
        "entries_total": 0,
        "entries_matched": 0,
        "entries_resolved_via_head": 0,
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

        # 主术语解析：精确匹配 → head-term 索引
        resolved_id = _resolve_term(term, snet, head_index)
        if resolved_id is None:
            continue

        if resolved_id != term:
            stats["entries_resolved_via_head"] += 1

        stats["entries_matched"] += 1

        # 1. synonyms → 聚合轴
        for syn in entry.get("synonyms", []):
            syn = syn.strip()
            if not syn or syn == resolved_id:
                continue

            # synonym 也做术语解析
            syn_resolved = _resolve_term(syn, snet, head_index)
            if syn_resolved is None:
                # synonym 不在 S_net 中，创建新 signifier
                snet = snet.add_signifier(Signifier(
                    id=syn,
                    surface_forms=(),
                    source="dictionary",
                    lang=entry_lang,
                ))
                stats["signifiers_created"] += 1
                syn_resolved = syn

            # 添加聚合轴边（synonym 关系）
            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=syn_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.85,
                evidence=f"辞典近义项 ({domain})",
            ))
            stats["synonyms_added"] += 1

        # 2. contrasts → 聚合轴
        for contrast in entry.get("contrasts", []):
            # 兼容两种格式：{"term": ..., "differential": ...} 或纯字符串
            if isinstance(contrast, str):
                c_term = contrast.strip()
                c_diff = ""
            else:
                c_term = contrast.get("term", "").strip()
                c_diff = contrast.get("differential", "").strip()
            if not c_term or c_term == resolved_id:
                continue

            # contrast 也做术语解析
            c_resolved = _resolve_term(c_term, snet, head_index)
            if c_resolved is None:
                continue

            # 添加对称的 contrast 边（A→B 和 B→A）
            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=c_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.6,
                evidence=c_diff or f"辞典对比项 ({domain})",
            ))
            snet = snet.add_edge(SignifierEdge(
                source=c_resolved,
                target=resolved_id,
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
            if other_term == resolved_id:
                continue

            # definition 中提到的术语也走解析
            other_resolved = _resolve_term(other_term, snet, head_index)
            if other_resolved is None:
                if not snet.has_signifier(other_term):
                    continue
                other_resolved = other_term

            pattern = extract_pattern(definition, term, other_term)
            evidence = pattern if pattern else f"dict:{term}:{domain}"

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=other_resolved,
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

    # 按文件名排序（确定性顺序）——dict_ 为词典，text_ 为论述文本
    dict_files = sorted(
        list(dir_path.glob("dict_*.jsonl")) + list(dir_path.glob("text_*.jsonl"))
    )

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


def _build_augmented_whitelist(
    snet: SNet,
) -> tuple[set[str], dict[str, str]]:
    """构建增强白名单：signifier ID + head terms。

    返回：
      (whitelist, head_to_sid)
      - whitelist: 用于 phi_L_whitelist_match 的增强白名单
      - head_to_sid: head term → signifier ID 的映射（用于将匹配结果映射回 signifier ID）

    对于文本匹配，full signifier ID "culture industry — mass deception" 几乎不会在
    原文中出现。但 head term "culture industry" 经常出现。增强白名单同时包含两者，
    phi_L_whitelist_match 的长度降序贪心保证 full ID 优先匹配（如果碰巧出现的话）。
    """
    whitelist = set(snet.signifiers.keys())
    head_to_sid: dict[str, str] = {}

    for sid in snet.signifiers:
        head = None
        if " — " in sid:
            head = sid.split(" — ")[0].strip()
        elif "(" in sid:
            head = sid[:sid.index("(")].strip()

        if head and head != sid and head not in whitelist:
            whitelist.add(head)
            # 冲突解决：保留最短的 signifier ID
            if head in head_to_sid:
                if len(sid) < len(head_to_sid[head]):
                    head_to_sid[head] = sid
            else:
                head_to_sid[head] = sid

    return whitelist, head_to_sid


def ingest_text_passage(
    snet: SNet,
    text: str,
    domain: str,
    source: str,
) -> tuple[SNet, list[dict]]:
    """从原文段落中提取共现超边并写入 S_net。

    基本事件是"段落 P 包含 {A, B, C, ...}"，产出 1 个 CooccurrenceHyperedge。
    不再展开为 C(N,2) 成对边——成对边是超边的 lazy 派生物。

    此函数不修改 K_active——它只丰富 S_net 的语言材料。
    K_active 的修改由 articulation feedback 在穿越中完成。

    参数：
      snet:    当前 S_net 实例（不会被修改）
      text:    原文段落文本
      domain:  语料域标记（如 "hegel", "marx"）
      source:  来源标记（如 "phenomenology_of_spirit.md"）

    返回：
      (new_snet, log_entries)
      - new_snet: 包含新增超边和 surface forms 的 SNet
      - log_entries: 摄入日志条目列表

    认识论等级：L0（确定性字符串匹配，不涉及经验假设）
    """
    import time as _time

    if not text or not snet.signifiers:
        return snet, []

    # 构建增强白名单（signifier IDs + head terms）
    whitelist, head_to_sid = _build_augmented_whitelist(snet)

    # 匹配段落中的已知术语（使用本模块的白名单匹配函数）
    raw_matches = phi_L_whitelist_match(text, whitelist)

    # 将 head term 匹配结果映射回 signifier ID
    matched_terms = []
    for m in raw_matches:
        resolved = head_to_sid.get(m, m)  # head term → sid, 或保持原样
        if resolved not in matched_terms:  # 去重
            matched_terms.append(resolved)

    if len(matched_terms) < 2:
        # 不足两个术语，无法产生超边
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

    # 2. 产出 1 个超边（不是 C(N,2) 成对边）
    vertices = frozenset(
        t for t in matched_terms if new_snet.has_signifier(t)
    )
    if len(vertices) >= 2:
        he = CooccurrenceHyperedge(
            vertices=vertices,
            source=source,
            domain=domain,
            timestamp=str(_time.time()),
            evidence_tag=evidence_tag,
        )
        new_snet = new_snet.add_hyperedge(he)

        log_entries.append({
            "type": "hyperedge",
            "vertices": sorted(vertices),
            "domain": domain,
            "corpus_ref": evidence_tag,
        })

    return new_snet, log_entries


def ingest_text_passage_batch(
    snet: SNet,
    paragraphs: list[str],
    domain: str,
    source: str,
) -> tuple[SNet, list[dict]]:
    """批量摄入多个段落——性能优化版本。

    每段落产出 1 个 CooccurrenceHyperedge（vertices = matched_terms 的 frozenset），
    不再生成 C(N,2) 成对 SignifierEdge。

    性能优化：
      1. 增强白名单只构建一次（而非每段落一次）
      2. 预排序白名单只构建一次
      3. signifier 更新在内存中收集，最终一次性写入 SNet
      4. 超边批量添加

    参数与 ingest_text_passage 相同，但 text -> paragraphs（段落列表）。

    认识论等级：L0（确定性字符串匹配，不涉及经验假设）
    """
    import time as _time

    if not paragraphs or not snet.signifiers:
        return snet, []

    # 构建增强白名单——一次性
    whitelist, head_to_sid = _build_augmented_whitelist(snet)
    # 预排序白名单（按长度降序），避免 phi_L_whitelist_match 每次重新排序
    sorted_terms = sorted(whitelist, key=len, reverse=True)

    # 收集所有变更，最终一次性应用到 SNet
    pending_hyperedges: list[CooccurrenceHyperedge] = []
    # signifier 更新：id → updated Signifier（新 surface_forms）
    sig_updates: dict[str, Signifier] = {}
    all_log_entries: list[dict] = []
    evidence_tag = f"corpus:{domain}:{source}"

    # 快照当前 signifier 的 surface_forms，用于去重
    current_surface_forms: dict[str, set[str]] = {}
    for sid, sig in snet.signifiers.items():
        current_surface_forms[sid] = set(sig.surface_forms)

    for text in paragraphs:
        if not text:
            continue

        # 匹配段落中的已知术语（内联白名单匹配，避免重复排序）
        matched: list[str] = []
        remaining = text
        for term in sorted_terms:
            if term in remaining:
                matched.append(term)
                remaining = remaining.replace(term, " " * len(term))

        # 将 head term 匹配结果映射回 signifier ID
        matched_terms = []
        for m in matched:
            resolved = head_to_sid.get(m, m)
            if resolved not in matched_terms:
                matched_terms.append(resolved)

        # 提取 surface forms（即使 <2 个术语）
        for term in matched_terms:
            sf = _find_surface_form(text, term)
            if not sf:
                continue
            # 检查是否已有此 surface form
            existing_forms = current_surface_forms.get(term, set())
            if sf in existing_forms:
                continue
            if len(existing_forms) >= 20:
                continue
            # 记录新增 surface form
            existing_forms.add(sf)
            current_surface_forms[term] = existing_forms
            # 构建更新后的 Signifier
            existing_sig = snet.signifiers.get(term)
            if existing_sig:
                # 合并已有的 pending 更新
                if term in sig_updates:
                    prev = sig_updates[term]
                    new_forms = prev.surface_forms + (sf,)
                else:
                    new_forms = existing_sig.surface_forms + (sf,)
                sig_updates[term] = Signifier(
                    id=existing_sig.id,
                    surface_forms=new_forms,
                    source=existing_sig.source,
                )

        if len(matched_terms) < 2:
            continue

        # 产出 1 个超边（不是 C(N,2) 成对边）
        vertices = frozenset(
            t for t in matched_terms if snet.has_signifier(t)
        )
        if len(vertices) >= 2:
            he = CooccurrenceHyperedge(
                vertices=vertices,
                source=source,
                domain=domain,
                timestamp=str(_time.time()),
                evidence_tag=evidence_tag,
            )
            pending_hyperedges.append(he)

            all_log_entries.append({
                "type": "hyperedge",
                "vertices": sorted(vertices),
                "domain": domain,
                "corpus_ref": evidence_tag,
            })

    # 一次性应用所有变更
    new_snet = snet
    if sig_updates:
        new_snet = new_snet.add_signifiers(list(sig_updates.values()))
    if pending_hyperedges:
        new_snet = new_snet.add_hyperedges(pending_hyperedges)

    return new_snet, all_log_entries


# ---------------------------------------------------------------------------
# 双语辞典摄入
# ---------------------------------------------------------------------------

def _normalize_bilingual_entry(entry: dict) -> dict:
    """将双语辞典条目归一化为内部格式。

    支持两种 JSONL 字段名：
      格式A（原始）: term_a/lang_a/term_b/lang_b
      格式B（translation）: source_term/source_lang/target_term/target_lang

    格式B 额外支持 pos 字段（词性标注）。

    认识论等级：L0（确定性字段映射）
    """
    # 格式B → 格式A 映射
    term_a = entry.get("term_a", "") or entry.get("source_term", "")
    term_b = entry.get("term_b", "") or entry.get("target_term", "")
    lang_a = entry.get("lang_a", "") or entry.get("source_lang", "")
    lang_b = entry.get("lang_b", "") or entry.get("target_lang", "")

    return {
        "term_a": term_a.strip() if term_a else "",
        "term_b": term_b.strip() if term_b else "",
        "lang_a": lang_a.strip() if lang_a else "",
        "lang_b": lang_b.strip() if lang_b else "",
        "domain": (entry.get("domain", "") or "").strip(),
        "differential": (entry.get("differential", "") or "").strip(),
        "source": (entry.get("source", "") or "").strip(),
        "pos": (entry.get("pos", "") or "").strip(),
    }


def ingest_bilingual_dict(
    snet: SNet,
    dict_path: str | Path,
) -> tuple[SNet, dict]:
    """从双语 JSONL 辞典文件向 S_net 注入跨语言翻译关系。

    支持两种 JSONL 格式：
      格式A: {"term_a": str, "lang_a": str, "term_b": str, "lang_b": str,
              "domain": str, "differential": str, "source": str}
      格式B: {"source_term": str, "source_lang": str, "target_term": str,
              "target_lang": str, "differential": str, "source": str,
              "domain": str, "pos": str}

    处理逻辑：
      1. 为两端术语创建 Signifier（带 lang + domain），若已存在则保留
      2. 创建双向 PARADIGMATIC 边（relation="translation"）
      3. translation 边是双向的——A→B 和 B→A 只创建一条对称边对
      4. differential 记录翻译间的语义差异（S_net 的独特价值）

    注意：同一概念在不同语言中各有一个 Signifier（lang 字段区分）。
    跨语言用 translation 边连接，paradigmatic_class 内部是同一语言的同义词。

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
        "skipped_duplicate": 0,
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

    # 去重：同一对术语只创建一条翻译边（双向）
    seen_pairs: set[tuple[str, str]] = set()
    pending_signifiers: list[Signifier] = []
    pending_edges: list[SignifierEdge] = []
    existing_sig_ids: set[str] = set(snet.signifiers.keys())

    for raw_entry in entries:
        entry = _normalize_bilingual_entry(raw_entry)
        term_a = entry["term_a"]
        term_b = entry["term_b"]
        lang_a = entry["lang_a"]
        lang_b = entry["lang_b"]
        domain = entry["domain"]
        differential = entry["differential"]
        source = entry["source"]

        if not term_a or not term_b:
            continue

        # 去重：(A,B) 和 (B,A) 视为同一对
        pair_key = (min(term_a, term_b), max(term_a, term_b))
        if pair_key in seen_pairs:
            stats["skipped_duplicate"] += 1
            continue
        seen_pairs.add(pair_key)

        # 收集 Signifier（若不存在）
        if term_a not in existing_sig_ids:
            pending_signifiers.append(Signifier(
                id=term_a,
                surface_forms=(),
                source="bilingual_dictionary",
                lang=lang_a,
                domain=domain,
            ))
            existing_sig_ids.add(term_a)
            stats["signifiers_created"] += 1

        if term_b not in existing_sig_ids:
            pending_signifiers.append(Signifier(
                id=term_b,
                surface_forms=(),
                source="bilingual_dictionary",
                lang=lang_b,
                domain=domain,
            ))
            existing_sig_ids.add(term_b)
            stats["signifiers_created"] += 1

        # 收集翻译边（双向）
        evidence = f"bilingual:{source}" if source else f"bilingual:{path.stem}"
        pending_edges.append(SignifierEdge(
            source=term_a,
            target=term_b,
            axis=AxisType.PARADIGMATIC,
            weight=0.9,
            evidence=evidence,
            relation="translation",
            differential=differential,
        ))
        pending_edges.append(SignifierEdge(
            source=term_b,
            target=term_a,
            axis=AxisType.PARADIGMATIC,
            weight=0.9,
            evidence=evidence,
            relation="translation",
            differential=differential,
        ))
        stats["translations_added"] += 1

    # 批量写入（一次性重建 SNet，避免 211K 次逐条重建）
    if pending_signifiers:
        snet = snet.add_signifiers(pending_signifiers)
    if pending_edges:
        snet = snet.add_edges(pending_edges)
        snet = snet.merge_edge_weights()

    return snet, stats


# ingest_translation_dict 是 ingest_bilingual_dict 的别名（格式B兼容）
ingest_translation_dict = ingest_bilingual_dict


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


# ---------------------------------------------------------------------------
# 同义词辞典摄入（synonym_*.jsonl）
# ---------------------------------------------------------------------------

def ingest_synonym_dict(
    snet: SNet,
    dict_path: str | Path,
) -> tuple[SNet, dict]:
    """从同义词 JSONL 文件向 S_net 注入聚合轴关系。

    JSONL 格式（每行一个 JSON）：
      {"term": str, "lang": str, "synonyms": [str, ...],
       "near_synonyms": [str, ...], "domain": str,
       "differential": str}

    处理逻辑：
      - synonyms → 聚合轴边 (weight=0.85)
      - near_synonyms → 聚合轴边 (weight=0.6)
      - 主术语通过 head-term 索引解析到 signifier ID

    认识论等级：L0（辞典是手工编纂的定义）
    """
    path = Path(dict_path)
    if not path.exists():
        raise FileNotFoundError(f"同义词辞典文件不存在: {path}")

    head_index = _build_head_term_index(snet)

    stats = {
        "file": str(path.name),
        "type": "synonym",
        "entries_total": 0,
        "entries_matched": 0,
        "synonyms_added": 0,
        "near_synonyms_added": 0,
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
        lang = entry.get("lang", "").strip()
        differential = entry.get("differential", "").strip()

        if not term:
            continue

        resolved_id = _resolve_term(term, snet, head_index)
        if resolved_id is None:
            continue

        stats["entries_matched"] += 1

        # synonyms → 聚合轴 (higher weight)
        for syn in entry.get("synonyms", []):
            syn = syn.strip()
            if not syn or syn == resolved_id:
                continue

            syn_resolved = _resolve_term(syn, snet, head_index)
            if syn_resolved is None:
                snet = snet.add_signifier(Signifier(
                    id=syn, surface_forms=(), source="synonym_dict",
                    lang=lang, domain=domain,
                ))
                stats["signifiers_created"] += 1
                syn_resolved = syn

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=syn_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.85,
                evidence=differential or f"synonym:{domain}",
                relation="synonym",
            ))
            stats["synonyms_added"] += 1

        # near_synonyms → 聚合轴 (lower weight)
        for ns in entry.get("near_synonyms", []):
            ns = ns.strip()
            if not ns or ns == resolved_id:
                continue

            ns_resolved = _resolve_term(ns, snet, head_index)
            if ns_resolved is None:
                snet = snet.add_signifier(Signifier(
                    id=ns, surface_forms=(), source="synonym_dict",
                    lang=lang, domain=domain,
                ))
                stats["signifiers_created"] += 1
                ns_resolved = ns

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=ns_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.6,
                evidence=differential or f"near_synonym:{domain}",
                relation="synonym",
            ))
            stats["near_synonyms_added"] += 1

    snet = snet.merge_edge_weights()
    return snet, stats


# ---------------------------------------------------------------------------
# 搭配辞典摄入（collocations_*.jsonl）
# ---------------------------------------------------------------------------

def ingest_collocation_dict(
    snet: SNet,
    dict_path: str | Path,
) -> tuple[SNet, dict]:
    """从搭配 JSONL 文件向 S_net 注入组合轴关系。

    JSONL 格式（每行一个 JSON）：
      {"term": str, "lang": str, "domain": str,
       "collocations": {"adj": [str], "verb_subject": [str],
                        "verb_object": [str], "noun_prep": [str],
                        "common_phrases": [str]}}

    处理逻辑：
      - common_phrases → 组合轴边 evidence（用于 connective_pattern 积累）
      - adj/verb/noun 搭配中出现的已知术语 → 组合轴边

    认识论等级：L0（辞典是手工编纂的搭配）
    """
    path = Path(dict_path)
    if not path.exists():
        raise FileNotFoundError(f"搭配辞典文件不存在: {path}")

    head_index = _build_head_term_index(snet)

    stats = {
        "file": str(path.name),
        "type": "collocation",
        "entries_total": 0,
        "entries_matched": 0,
        "syntagmatic_added": 0,
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

    # 构建增强白名单用于搭配文本中的术语匹配
    whitelist, head_to_sid = _build_augmented_whitelist(snet)

    for entry in entries:
        term = entry.get("term", "").strip()
        domain = entry.get("domain", "").strip()

        if not term:
            continue

        resolved_id = _resolve_term(term, snet, head_index)
        if resolved_id is None:
            continue

        stats["entries_matched"] += 1

        collocations = entry.get("collocations", {})

        # 从所有搭配文本中提取已知术语
        all_collocation_texts: list[str] = []
        for key in ("adj", "verb_subject", "verb_object", "noun_prep", "common_phrases"):
            items = collocations.get(key, [])
            all_collocation_texts.extend(items)

        for text in all_collocation_texts:
            # 在搭配文本中匹配已知术语
            matched = phi_L_whitelist_match(text, whitelist)
            for m in matched:
                other_id = head_to_sid.get(m, m)
                if other_id == resolved_id:
                    continue
                if not snet.has_signifier(other_id):
                    continue

                snet = snet.add_edge(SignifierEdge(
                    source=resolved_id,
                    target=other_id,
                    axis=AxisType.SYNTAGMATIC,
                    weight=1.0,
                    evidence=text,
                ))
                stats["syntagmatic_added"] += 1

    snet = snet.merge_edge_weights()
    return snet, stats


# ---------------------------------------------------------------------------
# 词类典摄入（thesaurus_*.jsonl）
# ---------------------------------------------------------------------------

def ingest_thesaurus_dict(
    snet: SNet,
    dict_path: str | Path,
) -> tuple[SNet, dict]:
    """从 thesaurus JSONL 文件向 S_net 注入聚合轴关系。

    支持四种 thesaurus schema（通过字段存在性自适应）：
      - roget:         related_terms, philosophical_usage, roget_category
      - wordnet (en):  synonyms, antonyms, hypernyms, hyponyms, definitions
      - openthesaurus: synonyms
      - wordnet (fr):  fr_lemmas

    所有关系映射到聚合轴（PARADIGMATIC）：
      - synonyms / related_terms / fr_lemmas → weight=0.85, relation=synonym
      - antonyms                             → weight=0.6,  relation=contrast
      - hypernyms                            → weight=0.7,  relation=hypernym
      - hyponyms                             → weight=0.7,  relation=hyponym

    philosophical_usage / definitions → 组合轴边（从文本中匹配已知术语）

    认识论等级：L0（辞典是手工编纂的定义）
    """
    path = Path(dict_path)
    if not path.exists():
        raise FileNotFoundError(f"Thesaurus 文件不存在: {path}")

    head_index = _build_head_term_index(snet)
    whitelist, head_to_sid = _build_augmented_whitelist(snet)

    stats = {
        "file": str(path.name),
        "type": "thesaurus",
        "entries_total": 0,
        "entries_matched": 0,
        "synonyms_added": 0,
        "antonyms_added": 0,
        "hypernyms_added": 0,
        "hyponyms_added": 0,
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
        lang = entry.get("lang", "").strip()

        if not term:
            continue

        resolved_id = _resolve_term(term, snet, head_index)
        if resolved_id is None:
            continue

        stats["entries_matched"] += 1

        # 统一收集所有 synonym-like 字段
        synonym_items: list[str] = []
        for key in ("synonyms", "related_terms", "fr_lemmas"):
            items = entry.get(key, [])
            if items:
                synonym_items.extend(items)

        for syn in synonym_items:
            if not isinstance(syn, str):
                continue
            syn = syn.strip()
            if not syn or syn == resolved_id:
                continue

            syn_resolved = _resolve_term(syn, snet, head_index)
            if syn_resolved is None:
                snet = snet.add_signifier(Signifier(
                    id=syn, surface_forms=(), source="thesaurus",
                    lang=lang,
                ))
                stats["signifiers_created"] += 1
                syn_resolved = syn

            category = entry.get("roget_category", "").strip()
            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=syn_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.85,
                evidence=f"thesaurus:{category}" if category else f"thesaurus:{path.stem}",
                relation="synonym",
            ))
            stats["synonyms_added"] += 1

        # antonyms → contrast 边（对称）
        for ant in entry.get("antonyms", []):
            if not isinstance(ant, str):
                continue
            ant = ant.strip()
            if not ant or ant == resolved_id:
                continue

            ant_resolved = _resolve_term(ant, snet, head_index)
            if ant_resolved is None:
                snet = snet.add_signifier(Signifier(
                    id=ant, surface_forms=(), source="thesaurus",
                    lang=lang,
                ))
                stats["signifiers_created"] += 1
                ant_resolved = ant

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=ant_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.6,
                evidence=f"thesaurus:{path.stem}",
                relation="contrast",
            ))
            snet = snet.add_edge(SignifierEdge(
                source=ant_resolved,
                target=resolved_id,
                axis=AxisType.PARADIGMATIC,
                weight=0.6,
                evidence=f"thesaurus:{path.stem}",
                relation="contrast",
            ))
            stats["antonyms_added"] += 1

        # hypernyms → 上位关系（不创建新 signifier）
        for hyper in entry.get("hypernyms", []):
            if not isinstance(hyper, str):
                continue
            hyper = hyper.strip()
            if not hyper or hyper == resolved_id:
                continue

            hyper_resolved = _resolve_term(hyper, snet, head_index)
            if hyper_resolved is None:
                continue

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=hyper_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.7,
                evidence=f"thesaurus:{path.stem}",
                relation="hypernym",
            ))
            stats["hypernyms_added"] += 1

        # hyponyms → 下位关系（不创建新 signifier）
        for hypo in entry.get("hyponyms", []):
            if not isinstance(hypo, str):
                continue
            hypo = hypo.strip()
            if not hypo or hypo == resolved_id:
                continue

            hypo_resolved = _resolve_term(hypo, snet, head_index)
            if hypo_resolved is None:
                continue

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=hypo_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.7,
                evidence=f"thesaurus:{path.stem}",
                relation="hyponym",
            ))
            stats["hyponyms_added"] += 1

        # philosophical_usage / definitions → 组合轴
        usage_texts: list[str] = []
        usage = entry.get("philosophical_usage", "").strip()
        if usage:
            usage_texts.append(usage)
        usage_texts.extend(entry.get("definitions", []))

        for text in usage_texts:
            if not isinstance(text, str):
                continue
            text = text.strip()
            if not text:
                continue

            matched = phi_L_whitelist_match(text, whitelist)
            for m in matched:
                other_id = head_to_sid.get(m, m)
                if other_id == resolved_id:
                    continue
                if not snet.has_signifier(other_id):
                    continue

                pattern = extract_pattern(text, term, m)
                snet = snet.add_edge(SignifierEdge(
                    source=resolved_id,
                    target=other_id,
                    axis=AxisType.SYNTAGMATIC,
                    weight=1.0,
                    evidence=pattern if pattern else f"thesaurus_usage:{term}",
                ))
                stats["syntagmatic_added"] += 1

    snet = snet.merge_edge_weights()
    return snet, stats


# ---------------------------------------------------------------------------
# 维基词典摄入（wiktionary_*.jsonl）
# ---------------------------------------------------------------------------

def ingest_wiktionary_dict(
    snet: SNet,
    dict_path: str | Path,
) -> tuple[SNet, dict]:
    """从 wiktionary JSONL 文件向 S_net 注入关系。

    支持两种 schema（通过字段存在性自适应）：
      - en/de/fr: {"term", "lang", "definitions", "etymology", "source"}
      - zh:       {"headword", "definitions", "synonyms", "antonyms",
                   "related", "etymology", "source"}

    处理逻辑：
      - definitions → 从定义文本中匹配已知术语 → 组合轴边
      - etymology → 同上（词源中常有关联术语）
      - synonyms (zh) → 聚合轴边
      - antonyms (zh) → 聚合轴边 (contrast)
      - related (zh) → 聚合轴边 (weight=0.7)

    认识论等级：L0（辞典是社区编纂的词条）
    """
    path = Path(dict_path)
    if not path.exists():
        raise FileNotFoundError(f"Wiktionary 文件不存在: {path}")

    head_index = _build_head_term_index(snet)
    whitelist, head_to_sid = _build_augmented_whitelist(snet)

    stats = {
        "file": str(path.name),
        "type": "wiktionary",
        "entries_total": 0,
        "entries_matched": 0,
        "syntagmatic_added": 0,
        "paradigmatic_added": 0,
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
        # zh 版用 headword，其他用 term
        term = entry.get("term", "") or entry.get("headword", "")
        term = term.strip()
        lang = entry.get("lang", "").strip()

        if not term:
            continue

        resolved_id = _resolve_term(term, snet, head_index)
        if resolved_id is None:
            continue

        stats["entries_matched"] += 1

        # definitions + etymology → 组合轴（从文本中提取共现术语）
        all_texts: list[str] = list(entry.get("definitions", []))
        etymology = entry.get("etymology", "").strip()
        if etymology:
            all_texts.append(etymology)

        for text in all_texts:
            if not isinstance(text, str):
                continue
            text = text.strip()
            if not text:
                continue

            matched = phi_L_whitelist_match(text, whitelist)
            for m in matched:
                other_id = head_to_sid.get(m, m)
                if other_id == resolved_id:
                    continue
                if not snet.has_signifier(other_id):
                    continue

                pattern = extract_pattern(text, term, m)
                snet = snet.add_edge(SignifierEdge(
                    source=resolved_id,
                    target=other_id,
                    axis=AxisType.SYNTAGMATIC,
                    weight=1.0,
                    evidence=pattern if pattern else f"wiktionary:{path.stem}:def",
                ))
                stats["syntagmatic_added"] += 1

        # synonyms (zh) → 聚合轴
        for syn in entry.get("synonyms", []):
            if not isinstance(syn, str):
                continue
            syn = syn.strip()
            if not syn or syn == resolved_id:
                continue

            syn_resolved = _resolve_term(syn, snet, head_index)
            if syn_resolved is None:
                snet = snet.add_signifier(Signifier(
                    id=syn, surface_forms=(), source="wiktionary",
                    lang=lang,
                ))
                stats["signifiers_created"] += 1
                syn_resolved = syn

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=syn_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.85,
                evidence=f"wiktionary:{path.stem}",
                relation="synonym",
            ))
            stats["paradigmatic_added"] += 1

        # antonyms (zh) → 聚合轴 contrast
        for ant in entry.get("antonyms", []):
            if not isinstance(ant, str):
                continue
            ant = ant.strip()
            if not ant or ant == resolved_id:
                continue

            ant_resolved = _resolve_term(ant, snet, head_index)
            if ant_resolved is None:
                continue

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=ant_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.6,
                evidence=f"wiktionary:{path.stem}",
                relation="contrast",
            ))
            stats["paradigmatic_added"] += 1

        # related (zh) → 聚合轴 (weaker)
        for rel in entry.get("related", []):
            if not isinstance(rel, str):
                continue
            rel = rel.strip()
            if not rel or rel == resolved_id:
                continue

            rel_resolved = _resolve_term(rel, snet, head_index)
            if rel_resolved is None:
                continue

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=rel_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.7,
                evidence=f"wiktionary:{path.stem}",
                relation="related",
            ))
            stats["paradigmatic_added"] += 1

    snet = snet.merge_edge_weights()
    return snet, stats


# ---------------------------------------------------------------------------
# 成语/固定短语摄入（idioms_*.jsonl）
# ---------------------------------------------------------------------------

def ingest_idiom_dict(
    snet: SNet,
    dict_path: str | Path,
) -> tuple[SNet, dict]:
    """从 idioms JSONL 文件向 S_net 注入组合轴关系 + surface forms。

    JSONL 格式：
      {"term": str, "lang": str, "domain": str,
       "idiomatic_uses": [str, ...], "fixed_phrases": [str, ...],
       "classical_references": [str, ...] (optional, zh only),
       "source": str}

    处理逻辑：
      - idiomatic_uses → surface_forms 积累 + 组合轴边（从习语中提取共现术语）
      - fixed_phrases → surface_forms 积累 + 组合轴边

    认识论等级：L0（辞典是手工编纂的惯用语）
    """
    path = Path(dict_path)
    if not path.exists():
        raise FileNotFoundError(f"Idioms 文件不存在: {path}")

    head_index = _build_head_term_index(snet)
    whitelist, head_to_sid = _build_augmented_whitelist(snet)

    stats = {
        "file": str(path.name),
        "type": "idiom",
        "entries_total": 0,
        "entries_matched": 0,
        "syntagmatic_added": 0,
        "surface_forms_added": 0,
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

        resolved_id = _resolve_term(term, snet, head_index)
        if resolved_id is None:
            continue

        stats["entries_matched"] += 1

        # 收集所有习语文本
        idiom_texts: list[str] = (
            list(entry.get("idiomatic_uses", []))
            + list(entry.get("fixed_phrases", []))
        )

        for text in idiom_texts:
            if not isinstance(text, str) or not text.strip():
                continue
            text = text.strip()

            # 添加为 surface form（限制数量）
            existing = snet.signifiers.get(resolved_id)
            if existing and text not in existing.surface_forms:
                if len(existing.surface_forms) < 20:
                    new_forms = existing.surface_forms + (text,)
                    snet = snet.add_signifier(Signifier(
                        id=existing.id,
                        surface_forms=new_forms,
                        source=existing.source,
                    ))
                    stats["surface_forms_added"] += 1

            # 从习语文本中提取共现术语
            matched = phi_L_whitelist_match(text, whitelist)
            for m in matched:
                other_id = head_to_sid.get(m, m)
                if other_id == resolved_id:
                    continue
                if not snet.has_signifier(other_id):
                    continue

                snet = snet.add_edge(SignifierEdge(
                    source=resolved_id,
                    target=other_id,
                    axis=AxisType.SYNTAGMATIC,
                    weight=1.0,
                    evidence=f"idiom:{domain}:{text[:60]}",
                ))
                stats["syntagmatic_added"] += 1

    snet = snet.merge_edge_weights()
    return snet, stats


# ---------------------------------------------------------------------------
# 词汇场摄入（wortschatz_*.jsonl）
# ---------------------------------------------------------------------------

def ingest_wortschatz_dict(
    snet: SNet,
    dict_path: str | Path,
) -> tuple[SNet, dict]:
    """从 Wortschatz JSONL 文件向 S_net 注入词场和构词关系。

    JSONL 格式：
      {"term": str, "lang": str, "word_field": [str, ...],
       "compounds": [str, ...], "derivations": [str, ...],
       "register": str, "source": str}

    处理逻辑：
      - word_field → 聚合轴边（同一词场 = paradigmatic 关系, weight=0.65）
      - compounds → 语素轴边（复合词 = morpheme 关系, weight=0.7）
      - derivations → 语素轴边（派生词 = morpheme 关系, weight=0.6）

    认识论等级：L0（辞典是手工编纂的词场分类）
    """
    path = Path(dict_path)
    if not path.exists():
        raise FileNotFoundError(f"Wortschatz 文件不存在: {path}")

    head_index = _build_head_term_index(snet)

    stats = {
        "file": str(path.name),
        "type": "wortschatz",
        "entries_total": 0,
        "entries_matched": 0,
        "word_field_added": 0,
        "compound_edges_added": 0,
        "derivation_edges_added": 0,
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
        lang = entry.get("lang", "").strip()

        if not term:
            continue

        resolved_id = _resolve_term(term, snet, head_index)
        if resolved_id is None:
            continue

        stats["entries_matched"] += 1

        # word_field → 聚合轴
        for wf in entry.get("word_field", []):
            if not isinstance(wf, str):
                continue
            wf = wf.strip()
            if not wf or wf == resolved_id:
                continue

            wf_resolved = _resolve_term(wf, snet, head_index)
            if wf_resolved is None:
                snet = snet.add_signifier(Signifier(
                    id=wf, surface_forms=(), source="wortschatz",
                    lang=lang,
                ))
                stats["signifiers_created"] += 1
                wf_resolved = wf

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=wf_resolved,
                axis=AxisType.PARADIGMATIC,
                weight=0.65,
                evidence=f"word_field:{term}",
                relation="word_field",
            ))
            stats["word_field_added"] += 1

        # compounds → 语素轴
        for comp in entry.get("compounds", []):
            if not isinstance(comp, str):
                continue
            comp = comp.strip()
            if not comp or comp == resolved_id:
                continue

            comp_resolved = _resolve_term(comp, snet, head_index)
            if comp_resolved is None:
                snet = snet.add_signifier(Signifier(
                    id=comp, surface_forms=(), source="wortschatz",
                    lang=lang,
                ))
                stats["signifiers_created"] += 1
                comp_resolved = comp

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=comp_resolved,
                axis=AxisType.MORPHEME,
                weight=0.7,
                evidence=f"compound:{term}->{comp}",
                relation="compound",
            ))
            stats["compound_edges_added"] += 1

        # derivations → 语素轴
        for deriv in entry.get("derivations", []):
            if not isinstance(deriv, str):
                continue
            deriv = deriv.strip()
            if not deriv or deriv == resolved_id:
                continue

            deriv_resolved = _resolve_term(deriv, snet, head_index)
            if deriv_resolved is None:
                snet = snet.add_signifier(Signifier(
                    id=deriv, surface_forms=(), source="wortschatz",
                    lang=lang,
                ))
                stats["signifiers_created"] += 1
                deriv_resolved = deriv

            snet = snet.add_edge(SignifierEdge(
                source=resolved_id,
                target=deriv_resolved,
                axis=AxisType.MORPHEME,
                weight=0.6,
                evidence=f"derivation:{term}->{deriv}",
                relation="derivation",
            ))
            stats["derivation_edges_added"] += 1

    snet = snet.merge_edge_weights()
    return snet, stats


# ---------------------------------------------------------------------------
# 代码辞典摄入：从 code_dict_*.jsonl 注入代码概念 signifier
# ---------------------------------------------------------------------------

def ingest_code_dict(
    snet: SNet,
    dict_path: str | Path,
    graph=None,
) -> tuple[SNet, dict]:
    """从代码辞典 JSONL 向 S_net 注入代码概念 signifier。

    代码辞典由 generate_code_dict.py 通过 AST 分析自动生成，格式：
      {"term": str, "domain": "code_project", "definition": str,
       "synonyms": [str], "contrasts": [str],
       "connective_patterns": [str], "concept_ids": [str]}

    与 ingest_dictionary() 的区别：
      1. concept_ids 字段：存放 K_active vertex ID，用于直接建立 concept_ref 映射
      2. 每个 term 创建短形式 signifier（如 "Graph"），并与 Layer A 的长形式
         signifier（如 "[domain:self] class Graph..."）通过 synonym 边连接
      3. synonyms 中的同名函数（不同模块）创建聚合轴边
      4. contrasts 中的对比概念创建聚合轴边（contrast 关系）
      5. connective_patterns 中的调用关系创建组合轴边（evidence 标注关系类型）

    参数：
      snet:      当前 S_net 实例
      dict_path: 代码辞典 JSONL 文件路径
      graph:     K_active 图（用于查找 vertex content → signifier ID 映射）

    返回：
      (new_snet, stats)

    认识论等级：L0（AST 是确定性解析，辞典条目是确定性数据）
    """
    path = Path(dict_path)
    if not path.exists():
        raise FileNotFoundError(f"代码辞典文件不存在: {path}")

    stats = {
        "file": str(path.name),
        "entries_total": 0,
        "signifiers_created": 0,
        "concept_bridges": 0,
        "synonyms_added": 0,
        "contrasts_added": 0,
        "syntagmatic_added": 0,
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

    # Build concept_id -> signifier_id mapping from existing S_net
    # (Layer A signifiers have vertex content as ID, vertex ID in surface_forms)
    vertex_id_to_sig: dict[str, str] = {}
    if graph is not None:
        for vid in graph.active_vertex_ids():
            v = graph.vertices.get(vid)
            if v is None or not v.content:
                continue
            content = v.content.strip()
            if content and snet.has_signifier(content):
                vertex_id_to_sig[vid] = content

    # Also build reverse: check surface_forms for vertex IDs
    for sid, sig in snet.signifiers.items():
        for sf in sig.surface_forms:
            if sf not in vertex_id_to_sig:
                vertex_id_to_sig[sf] = sid

    # Track created code signifiers for inter-entry edge building
    code_term_to_sig: dict[str, str] = {}

    for entry in entries:
        term = entry.get("term", "").strip()
        if not term:
            continue

        # Create signifier for this code concept if not already in S_net
        if not snet.has_signifier(term):
            snet = snet.add_signifier(Signifier(
                id=term,
                surface_forms=tuple(entry.get("concept_ids", [])),
                source="code_dictionary",
                domain="code_project",
            ))
            stats["signifiers_created"] += 1
        code_term_to_sig[term] = term

        # Bridge to Layer A signifiers via concept_ids
        for concept_id in entry.get("concept_ids", []):
            if concept_id in vertex_id_to_sig:
                layer_a_sig = vertex_id_to_sig[concept_id]
                if layer_a_sig != term:
                    # Create synonym edge: short-form ↔ long-form
                    snet = snet.add_edge(SignifierEdge(
                        source=term,
                        target=layer_a_sig,
                        axis=AxisType.PARADIGMATIC,
                        weight=1.0,
                        evidence=f"code_bridge:{concept_id}",
                        relation="synonym",
                    ))
                    snet = snet.add_edge(SignifierEdge(
                        source=layer_a_sig,
                        target=term,
                        axis=AxisType.PARADIGMATIC,
                        weight=1.0,
                        evidence=f"code_bridge:{concept_id}",
                        relation="synonym",
                    ))
                    stats["concept_bridges"] += 1

    # Second pass: build inter-entry edges now that all signifiers exist
    for entry in entries:
        term = entry.get("term", "").strip()
        if not term or not snet.has_signifier(term):
            continue

        # Synonyms → paradigmatic (same-name in different modules)
        for syn in entry.get("synonyms", []):
            syn = syn.strip()
            if not syn or syn == term:
                continue
            # Try to resolve: could be a module.name format
            syn_short = syn.split(".")[-1] if "." in syn else syn
            syn_target = None
            if snet.has_signifier(syn):
                syn_target = syn
            elif snet.has_signifier(syn_short):
                syn_target = syn_short
            if syn_target is None:
                continue

            snet = snet.add_edge(SignifierEdge(
                source=term,
                target=syn_target,
                axis=AxisType.PARADIGMATIC,
                weight=0.7,
                evidence="code_synonym",
                relation="synonym",
            ))
            stats["synonyms_added"] += 1

        # Contrasts → paradigmatic (sibling classes/methods)
        for contrast in entry.get("contrasts", []):
            contrast = contrast.strip()
            if not contrast or contrast == term:
                continue
            if not snet.has_signifier(contrast):
                continue

            snet = snet.add_edge(SignifierEdge(
                source=term,
                target=contrast,
                axis=AxisType.PARADIGMATIC,
                weight=0.5,
                evidence="code_contrast",
                relation="contrast",
            ))
            snet = snet.add_edge(SignifierEdge(
                source=contrast,
                target=term,
                axis=AxisType.PARADIGMATIC,
                weight=0.5,
                evidence="code_contrast",
                relation="contrast",
            ))
            stats["contrasts_added"] += 1

        # Connective patterns → syntagmatic (calls, defined_in, imports)
        for pattern in entry.get("connective_patterns", []):
            pattern = pattern.strip()
            if not pattern:
                continue

            # Parse pattern to extract relationship and target
            evidence = pattern
            target_term = None

            if " calls " in pattern:
                callee = pattern.split(" calls ", 1)[1].strip()
                callee_short = callee.split(".")[-1] if "." in callee else callee
                if snet.has_signifier(callee):
                    target_term = callee
                elif snet.has_signifier(callee_short):
                    target_term = callee_short
                evidence = f"calls:{callee}"

            elif "defined in " in pattern:
                parent = pattern.split("defined in ", 1)[1].strip()
                if snet.has_signifier(parent):
                    target_term = parent
                evidence = f"defined_in:{parent}"

            elif pattern.startswith("import "):
                mod = pattern.split("import ", 1)[1].strip()
                if snet.has_signifier(mod):
                    target_term = mod
                evidence = f"imports:{mod}"

            elif "." in pattern:
                # Class.method pattern
                parts = pattern.split(".", 1)
                method_name = parts[1] if len(parts) > 1 else None
                if method_name and snet.has_signifier(pattern):
                    target_term = pattern
                    evidence = f"member:{pattern}"

            if target_term and target_term != term:
                snet = snet.add_edge(SignifierEdge(
                    source=term,
                    target=target_term,
                    axis=AxisType.SYNTAGMATIC,
                    weight=0.8,
                    evidence=evidence,
                ))
                stats["syntagmatic_added"] += 1

    snet = snet.merge_edge_weights()
    return snet, stats


# ---------------------------------------------------------------------------
# Unified dict-type ingest loop (shared by daemon and corpus_ingest_batch)
# ---------------------------------------------------------------------------


def ingest_all_dict_types(
    snet: SNet,
    dict_dir: str | Path,
    graph=None,
    output=sys.stderr,
) -> SNet:
    """Ingest all dictionary types from dict_dir into snet.

    Runs ingest_all_dictionaries (dict_*.jsonl) first, then each specialized
    dict type (bilingual, morpheme, synonym, etc.), and finally code_dict.

    This is the single authoritative dict ingest loop -- both daemon startup
    and corpus_ingest_batch.py should call this instead of duplicating the loop.

    Args:
        snet:     Current SNet instance (not mutated).
        dict_dir: Path to signifier_net/dictionaries/.
        graph:    Optional K_active graph (needed for code_dict bridge edges).
        output:   File-like for progress output (default: stderr).

    Returns:
        New SNet with all dictionary entries ingested.
    """
    dict_dir = Path(dict_dir)
    if not dict_dir.is_dir():
        return snet

    # 1. Monolingual dictionaries (dict_*.jsonl) and text passages (text_*.jsonl)
    snet, all_stats = ingest_all_dictionaries(snet, dict_dir)
    if all_stats:
        report = format_ingest_report(all_stats)
        print(report, file=output)

    # 2-9: Specialized dict types
    _dict_types: list[tuple[str, str, callable]] = [
        ("bilingual_*.jsonl", "bilingual", ingest_bilingual_dict),
        ("morpheme_*.jsonl", "morpheme", ingest_morpheme_dict),
        ("synonym_*.jsonl", "synonym", ingest_synonym_dict),
        ("collocations_*.jsonl", "collocation", ingest_collocation_dict),
        ("thesaurus_*.jsonl", "thesaurus", ingest_thesaurus_dict),
        ("wiktionary_*.jsonl", "wiktionary", ingest_wiktionary_dict),
        ("idioms_*.jsonl", "idiom", ingest_idiom_dict),
        ("wortschatz_*.jsonl", "wortschatz", ingest_wortschatz_dict),
    ]
    for glob_pattern, label, ingest_fn in _dict_types:
        for fpath in sorted(dict_dir.glob(glob_pattern)):
            try:
                snet, _ = ingest_fn(snet, fpath)
                print(f"  {label} {fpath.name}: OK", file=output)
            except Exception as exc:
                print(f"  {label} {fpath.name}: ERROR - {exc}", file=output)

    # 10. Code dictionaries (code_dict_*.jsonl) -- needs graph for bridge edges
    for cdf in sorted(dict_dir.glob("code_dict_*.jsonl")):
        try:
            snet, _ = ingest_code_dict(snet, cdf, graph=graph)
            print(f"  code_dict {cdf.name}: OK", file=output)
        except Exception as exc:
            print(f"  code_dict {cdf.name}: ERROR - {exc}", file=output)

    return snet
