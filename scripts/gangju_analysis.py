#!/usr/bin/env python
"""纲举目张分析脚本（185号谱系下游推论）。

RTAS 循环步骤 7.5：commit+push 之后、ceremony_scan 之前执行。
从纲（总方针核心条件）推导新目，输出 JSON 到 stdout。

输入：
  1. 总方针纲（core-principles SKILL.md）
  2. block-topology 统计（.chanlun/block-topology/）
  3. 最新谱系状态（.chanlun/genealogy/settled/）

输出：JSON，包含 gang / filled_mu / empty_mu / new_mu / block_stats / audit_needed
"""
import json
import os
import glob
import re
import sys
import yaml


# ---------------------------------------------------------------------------
# 纲提取
# ---------------------------------------------------------------------------

GANG_PATH = os.path.join(".claude", "skills", "core-principles", "SKILL.md")


def extract_gang(root):
    """从总方针 SKILL.md 提取纲的摘要（编号原则列表）。"""
    path = os.path.join(root, GANG_PATH)
    if not os.path.isfile(path):
        return []
    with open(path, encoding="utf-8") as f:
        content = f.read()
    # 匹配 "N. **xxx**" 格式的原则行
    gang = []
    for m in re.finditer(
        r"^(\d+)\.\s+\*\*(.+?)\*\*", content, re.MULTILINE
    ):
        gang.append({"id": int(m.group(1)), "text": m.group(2).strip()})
    return gang


# ---------------------------------------------------------------------------
# block-topology 统计
# ---------------------------------------------------------------------------

def compute_block_stats(root):
    """读取 block-topology，统计各 type/relation/source 分布。"""
    base = os.path.join(root, ".chanlun", "block-topology")
    blocks_dir = os.path.join(base, "blocks")
    relations_path = os.path.join(base, "relations.jsonl")
    meta_path = os.path.join(base, "meta.json")

    # 1. 区块 type 分布
    type_counts = {}
    if os.path.isdir(blocks_dir):
        for fp in glob.glob(os.path.join(blocks_dir, "*.json")):
            try:
                with open(fp, encoding="utf-8") as f:
                    block = json.load(f)
                t = block.get("type", "unknown")
                type_counts[t] = type_counts.get(t, 0) + 1
            except Exception:
                pass

    # 2. 关系 type 分布
    relation_counts = {}
    if os.path.isfile(relations_path):
        try:
            with open(relations_path, encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if line:
                        rel = json.loads(line)
                        r = rel.get("relation", "unknown")
                        relation_counts[r] = relation_counts.get(r, 0) + 1
        except Exception:
            pass

    # 3. source 分布（从 meta.json 无法直接得到，需遍历区块）
    source_counts = {}
    if os.path.isdir(blocks_dir):
        for fp in glob.glob(os.path.join(blocks_dir, "*.json")):
            try:
                with open(fp, encoding="utf-8") as f:
                    block = json.load(f)
                s = block.get("source", "unknown")
                source_counts[s] = source_counts.get(s, 0) + 1
            except Exception:
                pass

    # 4. meta 信息
    migration_block_count = 0
    if os.path.isfile(meta_path):
        try:
            with open(meta_path, encoding="utf-8") as f:
                meta = json.load(f)
            migration_block_count = meta.get("block_count", 0)
        except Exception:
            pass

    total_blocks = sum(type_counts.values())
    total_relations = sum(relation_counts.values())
    delta_blocks = total_blocks - migration_block_count

    return {
        "total_blocks": total_blocks,
        "total_relations": total_relations,
        "migration_block_count": migration_block_count,
        "delta_blocks": delta_blocks,
        "type_counts": type_counts,
        "relation_counts": relation_counts,
        "source_counts": source_counts,
    }


# ---------------------------------------------------------------------------
# 谱系状态
# ---------------------------------------------------------------------------

def compute_genealogy_stats(root):
    """读取谱系状态：settled 数量、最近 N 个的 type 分布、pending 数量。"""
    settled_dir = os.path.join(root, ".chanlun", "genealogy", "settled")
    pending_dir = os.path.join(root, ".chanlun", "genealogy", "pending")

    settled_files = sorted(glob.glob(os.path.join(settled_dir, "*.md")))
    pending_files = glob.glob(os.path.join(pending_dir, "*.md"))

    settled_count = len(settled_files)
    pending_count = len(pending_files)

    # 最近 20 个 settled 的 type 分布
    recent_n = 20
    recent_types = {}
    for fp in settled_files[-recent_n:]:
        try:
            with open(fp, encoding="utf-8") as f:
                head = f.read(1500)
            fm_match = re.match(r"^---\s*\n(.+?)\n---", head, re.DOTALL)
            if fm_match:
                fm = yaml.safe_load(fm_match.group(1))
                if isinstance(fm, dict):
                    t = fm.get("type", "unknown")
                    recent_types[t] = recent_types.get(t, 0) + 1
        except Exception:
            pass

    return {
        "settled_count": settled_count,
        "pending_count": pending_count,
        "recent_type_distribution": recent_types,
    }


# ---------------------------------------------------------------------------
# residue 内容检测
# ---------------------------------------------------------------------------

def _block_has_substantive_concessions(content):
    """判断一个区块的 content 是否包含实质让步内容。"""
    if content.get("gemini_conceded"):
        return True
    if content.get("codex_conceded"):
        return True
    reasons = content.get("reasons", {})
    if reasons and any(reasons.values()):
        return True
    return False


def compute_residue_status(root):
    """计算 residue 状态三态值（183号-3 分界线）。

    返回值：
      - "no_residue": 无 residue 区块
      - "empty_shell": residue 区块存在但让步内容全为空
      - "substantive": 至少一个 residue 区块包含非空让步

    检测范围包括：
      1. residue 区块自身的 gemini_conceded / codex_conceded / reasons
      2. consensus 区块中的 gemini_conceded / codex_conceded（如存在）
    """
    blocks_dir = os.path.join(root, ".chanlun", "block-topology", "blocks")
    if not os.path.isdir(blocks_dir):
        return "no_residue"

    found_residue = False
    for fp in glob.glob(os.path.join(blocks_dir, "*.json")):
        try:
            with open(fp, encoding="utf-8") as f:
                block = json.load(f)
        except Exception:
            continue

        block_type = block.get("type")
        content = block.get("content", {})

        if block_type == "residue":
            found_residue = True
            if _block_has_substantive_concessions(content):
                return "substantive"

        elif block_type == "consensus":
            # consensus 区块也可能携带 conceded 字段
            if content.get("gemini_conceded") or content.get("codex_conceded"):
                found_residue = True
                if _block_has_substantive_concessions(content):
                    return "substantive"

    return "empty_shell" if found_residue else "no_residue"


def check_residue_empty(root):
    """向后兼容：返回 True 当 residue 内容全为空。"""
    return compute_residue_status(root) != "substantive"


# ---------------------------------------------------------------------------
# async_self_reference 检测
# ---------------------------------------------------------------------------

def has_async_self_reference_blocks(root):
    """检测 block-topology 中是否存在与 async_self_reference 相关的区块。

    通过搜索区块内容中出现 "async_self_reference" 或 "异步自指" 关键词判断。
    """
    blocks_dir = os.path.join(root, ".chanlun", "block-topology", "blocks")
    if not os.path.isdir(blocks_dir):
        return False

    for fp in glob.glob(os.path.join(blocks_dir, "*.json")):
        try:
            with open(fp, encoding="utf-8") as f:
                raw = f.read()
            if "async_self_reference" in raw or "异步自指" in raw:
                return True
        except Exception:
            pass
    return False


# ---------------------------------------------------------------------------
# 最近 ceremony 时间检测
# ---------------------------------------------------------------------------

def get_last_ceremony_info(root):
    """从 session 文件或 block-topology 推断最近一次 ceremony 的存在。

    返回 True 表示距上次 ceremony 有至少一次 RTAS 循环。
    """
    # 通过 session 文件存在性判断
    sessions = glob.glob(
        os.path.join(root, ".chanlun", "sessions", "*-session.md")
    )
    return len(sessions) > 0


# ---------------------------------------------------------------------------
# 纲举目张推导
# ---------------------------------------------------------------------------

def _already_audited_targets(root):
    """扫描 settled + pending 谱系中的 audit_targets 字段，返回已审计过的目标集合。

    192号发现：gangju_analysis 没有记忆已审计过的 new_mu，导致重复生成 pending 谱系。
    """
    audited = set()
    for subdir in ("settled", "pending"):
        dirpath = os.path.join(root, ".chanlun", "genealogy", subdir)
        if not os.path.isdir(dirpath):
            continue
        for fp in glob.glob(os.path.join(dirpath, "*.md")):
            try:
                with open(fp, encoding="utf-8") as f:
                    text = f.read()
                fm_match = re.match(r"^---\s*\n(.*?)\n---", text, re.DOTALL)
                if not fm_match:
                    continue
                fm = yaml.safe_load(fm_match.group(1))
                if fm and isinstance(fm.get("audit_targets"), list):
                    for target in fm["audit_targets"]:
                        audited.add(str(target).strip())
            except Exception:
                continue
    return audited


def derive_mu(block_stats, genealogy_stats, root):
    """从纲的逻辑必然性推导 filled_mu / empty_mu / new_mu。"""
    filled_mu = []
    empty_mu = []
    new_mu = []

    # 192号修复：已审计过的目标不再重复检测
    audited = _already_audited_targets(root)

    type_counts = block_stats.get("type_counts", {})
    delta_blocks = block_stats.get("delta_blocks", 0)

    # 规则1：consensus > 0 但 residue 内容全为空 → 新目"多轮质询管道"
    consensus_count = type_counts.get("consensus", 0)
    residue_status = compute_residue_status(root)
    if consensus_count > 0:
        if residue_status != "substantive":
            if "多轮质询管道" not in audited:
                new_mu.append({
                    "mu": "多轮质询管道",
                    "reason": f"consensus 区块 {consensus_count} 个，但 residue 内容全为空——质询仅走形式未产出实质让步",
                })
        else:
            filled_mu.append({
                "mu": "共识仪式物质证据",
                "evidence": f"consensus={consensus_count}, residue 含实质内容",
            })

    # 规则2：无 async_self_reference 相关区块 → 新目"异步自指实现"
    if not has_async_self_reference_blocks(root):
        if "异步自指实现" not in audited:
            new_mu.append({
                "mu": "异步自指实现",
                "reason": "block-topology 中无 async_self_reference 相关区块——蜂群声明为异步自指拓扑但无物质证据",
            })

    # 规则3：delta_blocks == 0 且有 session（距上次 ceremony > 0）→ empty_mu
    has_ceremony = get_last_ceremony_info(root)
    if delta_blocks == 0 and has_ceremony:
        empty_mu.append({
            "mu": "RTAS 未产出新区块",
            "reason": f"delta_blocks=0, migration_block_count={block_stats['migration_block_count']}, current={block_stats['total_blocks']}",
        })

    # 规则：拓扑标注覆盖率检测（195号）
    topo_modules = [
        "a_inclusion.py", "a_fractal.py", "a_stroke.py",
        "a_segment_v1.py", "a_center_v0.py", "a_trendtype_v0.py",
        "a_recursive_engine.py",
    ]
    topo_annotated = 0
    for mod in topo_modules:
        mod_path = os.path.join(root, "src", "newchan", mod)
        if os.path.isfile(mod_path):
            with open(mod_path, encoding="utf-8") as f:
                if "拓扑语义" in f.read(3000):
                    topo_annotated += 1
    topo_coverage = topo_annotated / len(topo_modules) if topo_modules else 0
    if topo_coverage < 1.0:
        if "拓扑标注覆盖" not in audited:
            new_mu.append({
                "mu": "拓扑标注覆盖",
                "reason": f"A 系统模块拓扑标注 {topo_annotated}/{len(topo_modules)} ({topo_coverage:.0%})——未全覆盖",
            })

    # 规则：转换函数等价验证（195号）
    topo_module = os.path.join(root, "src", "newchan", "a_topology.py")
    if not os.path.isfile(topo_module):
        if "转换函数等价" not in audited:
            new_mu.append({
                "mu": "转换函数等价",
                "reason": "a_topology.py 不存在——分解不唯一(001号)的等价类验证缺失",
            })

    # 规则：Layer 1 审核共识检测
    review_dir = os.path.join(root, ".chanlun", "review-results")
    layer1_approved = False
    if os.path.isdir(review_dir):
        gemini_approved = False
        codex_approved = False
        # 查找最终轮（最大 round 号）的审核文件
        gemini_files = sorted(glob.glob(
            os.path.join(review_dir, "plan-review-layer1-gemini-round*.md"),
        ))
        codex_files = sorted(glob.glob(
            os.path.join(review_dir, "plan-review-layer1-codex-round*.md"),
        ))
        if gemini_files:
            with open(gemini_files[-1], encoding="utf-8") as f:
                if "APPROVED" in f.read():
                    gemini_approved = True
        if codex_files:
            with open(codex_files[-1], encoding="utf-8") as f:
                if "APPROVED" in f.read():
                    codex_approved = True
        layer1_approved = gemini_approved and codex_approved

    if layer1_approved:
        filled_mu.append({
            "mu": "Layer 1 审核共识",
            "evidence": "Gemini + Codex 最终轮均 APPROVED",
        })
    else:
        empty_mu.append({
            "mu": "Layer 1 审核共识",
            "reason": "Gemini/Codex 审核未全部 APPROVED（或审核文件不存在）",
        })

    # 规则：gauge 经验验证检测
    gauge_report_path = os.path.join(
        root, ".chanlun", "review-results",
        "gauge-equivalence-empirical-report.md",
    )
    gauge_verified = False
    gauge_summary = ""
    if os.path.isfile(gauge_report_path):
        with open(gauge_report_path, encoding="utf-8") as f:
            gauge_content = f.read()
        # 检测 成功/失败/总计 行的成功率
        m = re.search(
            r"\*\*成功/失败/总计\*\*:\s*(\d+)/(\d+)/(\d+)",
            gauge_content,
        )
        if m:
            success, fail, total = int(m.group(1)), int(m.group(2)), int(m.group(3))
            if fail == 0 and success == total and total > 0:
                gauge_verified = True
                # 提取保持率摘要
                rates = re.findall(
                    r"\|\s*(\w+)\s*\|.*?\|\s*([\d.]+%)\s*\|",
                    gauge_content,
                )
                gauge_summary = (
                    f"{success}/{total} 全成功"
                    + (f"，保持率: {', '.join(f'{k}={v}' for k, v in rates[:5])}" if rates else "")
                )

    if gauge_verified:
        filled_mu.append({
            "mu": "gauge 经验验证",
            "evidence": gauge_summary,
        })
    else:
        empty_mu.append({
            "mu": "gauge 经验验证",
            "reason": "gauge-equivalence-empirical-report.md 不存在或成功率非 100%",
        })

    # 规则：Layer 2 审核共识检测（先于 Layer 2 就绪/已实现判定）
    layer2_approved = False
    if os.path.isdir(review_dir):
        gemini_l2_files = sorted(glob.glob(
            os.path.join(review_dir, "plan-review-layer2-gemini-round*.md"),
        ))
        codex_l2_files = sorted(glob.glob(
            os.path.join(review_dir, "plan-review-layer2-codex-round*.md"),
        ))
        gemini_l2_ok = False
        codex_l2_ok = False
        if gemini_l2_files:
            with open(gemini_l2_files[-1], encoding="utf-8") as f:
                if "APPROVED" in f.read():
                    gemini_l2_ok = True
        if codex_l2_files:
            with open(codex_l2_files[-1], encoding="utf-8") as f:
                if "APPROVED" in f.read():
                    codex_l2_ok = True
        layer2_approved = gemini_l2_ok and codex_l2_ok

    if layer2_approved:
        filled_mu.append({
            "mu": "Layer 2 审核共识",
            "evidence": "Gemini + Codex 最终轮均 APPROVED（T8 背驰拓扑化）",
        })
    elif layer1_approved and gauge_verified:
        empty_mu.append({
            "mu": "Layer 2 审核共识",
            "reason": "Layer 2 审核文件不存在或未全部 APPROVED",
        })

    # 规则：Layer 2 T8 状态检测（197号谱系：T8 已实现）
    if layer1_approved and gauge_verified:
        if layer2_approved:
            filled_mu.append({
                "mu": "Layer 2 T8 已实现",
                "evidence": "Layer 1 审核共识 + gauge 经验验证 + Layer 2 审核共识均 filled（197号谱系）",
            })
        elif "Layer 2 T8 就绪" not in audited:
            new_mu.append({
                "mu": "Layer 2 T8 就绪",
                "reason": "Layer 1 审核共识 + gauge 经验验证均 filled → T8 背驰拓扑化可启动",
            })

    # 规则：Layer 3 T6 状态检测（204号谱系：T6 可计算近似已实现）
    # 检测 check_cross_level_leray 是否存在于 a_topology.py
    t6_implemented = False
    topology_path = os.path.join(root, "src", "newchan", "a_topology.py")
    if os.path.isfile(topology_path):
        with open(topology_path, encoding="utf-8") as f:
            topo_content = f.read()
        t6_implemented = "def check_cross_level_leray" in topo_content

    # 检测 Layer 3 审核共识
    layer3_approved = False
    l3_gemini_files = glob.glob(
        os.path.join(review_dir, "plan-review-layer3-gemini-round*.md"),
    )
    l3_codex_files = glob.glob(
        os.path.join(review_dir, "plan-review-layer3-codex-round*.md"),
    )
    gemini_l3_ok = False
    codex_l3_ok = False
    if l3_gemini_files:
        with open(sorted(l3_gemini_files)[-1], encoding="utf-8") as f:
            gemini_l3_ok = "APPROVED" in f.read()
    if l3_codex_files:
        with open(sorted(l3_codex_files)[-1], encoding="utf-8") as f:
            codex_l3_ok = "APPROVED" in f.read()
    layer3_approved = gemini_l3_ok and codex_l3_ok

    if t6_implemented:
        if layer3_approved:
            filled_mu.append({
                "mu": "Layer 3 T6 已审核",
                "evidence": "T6 可计算近似已实现 + Gemini×Codex 审核通过（204号谱系）",
            })
        else:
            filled_mu.append({
                "mu": "Layer 3 T6 已实现",
                "evidence": "check_cross_level_leray 已实现（W₁单调+bottleneck有界+KL散度有界），待审核",
            })
            if "Layer 3 T6 待审核" not in audited:
                new_mu.append({
                    "mu": "Layer 3 T6 待审核",
                    "reason": "T6 可计算近似已实现但未经 Gemini×Codex 审核",
                })
    elif layer2_approved:
        if "Layer 3 T6 就绪" not in audited:
            new_mu.append({
                "mu": "Layer 3 T6 就绪",
                "reason": "Layer 2 审核通过 → T6 可计算近似可启动（204号：不搁置）",
            })

    # 补充规则：谱系类型分布检测
    recent_types = genealogy_stats.get("recent_type_distribution", {})
    if recent_types:
        # 如果最近谱系全是 语法记录 类型，缺少 矛盾发现 → 质询深度不足信号
        contradiction_types = sum(
            v for k, v in recent_types.items() if "矛盾" in k
        )
        total_recent = sum(recent_types.values())
        if total_recent >= 10 and contradiction_types == 0:
            if "质询深度不足" not in audited:
                new_mu.append({
                    "mu": "质询深度不足",
                    "reason": f"最近 {total_recent} 个谱系无矛盾发现类型——可能停留在语法记录层，未触及概念张力",
                })

    # 补充规则：pending 谱系积压
    pending_count = genealogy_stats.get("pending_count", 0)
    if pending_count > 0:
        empty_mu.append({
            "mu": "谱系积压",
            "reason": f"{pending_count} 个 pending 谱系未结算",
        })

    # 已填充的目：基于区块拓扑的物质证据
    tension_count = type_counts.get("tension", 0)
    if tension_count > 0:
        filled_mu.append({
            "mu": "张力区块记录",
            "evidence": f"tension 区块 {tension_count} 个",
        })

    relation_counts = block_stats.get("relation_counts", {})
    negates_count = relation_counts.get("negates", 0)
    if negates_count > 0:
        filled_mu.append({
            "mu": "否定关系物质化",
            "evidence": f"negates 关系 {negates_count} 条",
        })

    settled_count = genealogy_stats.get("settled_count", 0)
    if settled_count > 100:
        filled_mu.append({
            "mu": "谱系规模",
            "evidence": f"已结算 {settled_count} 个谱系",
        })

    return filled_mu, empty_mu, new_mu, residue_status


# ---------------------------------------------------------------------------
# 谱系 pending 骨架生成（187号目E）
# ---------------------------------------------------------------------------

def _next_genealogy_id(root):
    """扫描 settled + pending 目录，返回 max(id) + 1。"""
    max_id = 0
    for subdir in ("settled", "pending"):
        dirpath = os.path.join(root, ".chanlun", "genealogy", subdir)
        if not os.path.isdir(dirpath):
            continue
        for fp in glob.glob(os.path.join(dirpath, "*.md")):
            m = re.match(r"^(\d+)-", os.path.basename(fp))
            if m:
                max_id = max(max_id, int(m.group(1)))
    return max_id + 1


def generate_pending_skeleton(root, new_mu):
    """当 audit_needed=true 且 new_mu 非空时，生成谱系 pending 骨架。

    返回生成的文件路径，或 None。
    不调用任何外部 API——只生成 frontmatter + 占位符。
    """
    if not new_mu:
        return None

    from datetime import datetime, timezone

    pending_dir = os.path.join(root, ".chanlun", "genealogy", "pending")
    os.makedirs(pending_dir, exist_ok=True)

    next_id = _next_genealogy_id(root)
    now = datetime.now(timezone.utc)
    timestamp = now.strftime("%Y%m%d-%H%M")

    filename = f"{next_id:03d}-gangju-auto-{timestamp}.md"
    filepath = os.path.join(pending_dir, filename)

    # 从 new_mu 提取目标摘要
    mu_summaries = []
    for item in new_mu:
        mu_summaries.append(f"  - '{item['mu']}'")

    mu_yaml_list = "\n".join(mu_summaries)

    content = f"""---
id: '{next_id}'
title: 纲举目张自动检测——待质询项
type: 待定
status: 生成态
date: {now.strftime("%Y-%m-%d")}
source: gangju_analysis.py
audit_targets:
{mu_yaml_list}
---

# {next_id}号：纲举目张自动检测——待质询项

## 来源标注

[gangju_analysis.py 自动生成] audit_needed=true, new_mu={len(new_mu)} 项

## 待质询目标

"""
    for item in new_mu:
        content += f"### {item['mu']}\n\n"
        content += f"**触发原因**: {item['reason']}\n\n"
        content += "**质询结果**: （待多轮质询填充——此骨架由计算过程生成，内容须由对话过程填充）\n\n"

    content += """## 边界条件

（待质询循环填充）

## 下游推论

（待质询循环填充）
"""

    with open(filepath, "w", encoding="utf-8") as f:
        f.write(content)

    return filepath


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main():
    root = os.getcwd()

    gang = extract_gang(root)
    block_stats = compute_block_stats(root)
    genealogy_stats = compute_genealogy_stats(root)
    filled_mu, empty_mu, new_mu, residue_status = derive_mu(
        block_stats, genealogy_stats, root
    )

    audit_needed = len(new_mu) > 0

    # 187号目E：audit_needed 且 new_mu 非空时生成谱系 pending 骨架
    generated_pending = None
    if audit_needed:
        generated_pending = generate_pending_skeleton(root, new_mu)

    result = {
        "gang": gang,
        "filled_mu": filled_mu,
        "empty_mu": empty_mu,
        "new_mu": new_mu,
        "block_stats": {
            "total_blocks": block_stats["total_blocks"],
            "total_relations": block_stats["total_relations"],
            "delta_blocks": block_stats["delta_blocks"],
            "type_counts": block_stats["type_counts"],
            "relation_counts": block_stats["relation_counts"],
            "source_counts": block_stats["source_counts"],
        },
        "genealogy": {
            "settled": genealogy_stats["settled_count"],
            "pending": genealogy_stats["pending_count"],
            "recent_types": genealogy_stats["recent_type_distribution"],
        },
        "residue_status": residue_status,
        "audit_needed": audit_needed,
        "generated_pending": generated_pending,
    }

    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
