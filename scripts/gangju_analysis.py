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

def check_residue_empty(root):
    """检查所有 residue 区块的 content 是否实质为空。

    "实质为空" = gemini_conceded + codex_conceded + reasons 全为空列表/空字典。
    """
    blocks_dir = os.path.join(root, ".chanlun", "block-topology", "blocks")
    if not os.path.isdir(blocks_dir):
        return True  # 无区块目录，视为空

    for fp in glob.glob(os.path.join(blocks_dir, "*.json")):
        try:
            with open(fp, encoding="utf-8") as f:
                block = json.load(f)
            if block.get("type") != "residue":
                continue
            content = block.get("content", {})
            # 非空判断：任何一个字段有实质内容就不算空
            if content.get("gemini_conceded"):
                return False
            if content.get("codex_conceded"):
                return False
            reasons = content.get("reasons", {})
            if reasons and any(reasons.values()):
                return False
        except Exception:
            pass
    return True


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

def derive_mu(block_stats, genealogy_stats, root):
    """从纲的逻辑必然性推导 filled_mu / empty_mu / new_mu。"""
    filled_mu = []
    empty_mu = []
    new_mu = []

    type_counts = block_stats.get("type_counts", {})
    delta_blocks = block_stats.get("delta_blocks", 0)

    # 规则1：consensus > 0 但 residue 内容全为空 → 新目"多轮质询管道"
    consensus_count = type_counts.get("consensus", 0)
    if consensus_count > 0:
        residue_all_empty = check_residue_empty(root)
        if residue_all_empty:
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

    # 补充规则：谱系类型分布检测
    recent_types = genealogy_stats.get("recent_type_distribution", {})
    if recent_types:
        # 如果最近谱系全是 语法记录 类型，缺少 矛盾发现 → 质询深度不足信号
        contradiction_types = sum(
            v for k, v in recent_types.items() if "矛盾" in k
        )
        total_recent = sum(recent_types.values())
        if total_recent >= 10 and contradiction_types == 0:
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

    return filled_mu, empty_mu, new_mu


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main():
    root = os.getcwd()

    gang = extract_gang(root)
    block_stats = compute_block_stats(root)
    genealogy_stats = compute_genealogy_stats(root)
    filled_mu, empty_mu, new_mu = derive_mu(block_stats, genealogy_stats, root)

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
        "audit_needed": len(new_mu) > 0,
    }

    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
