#!/usr/bin/env python3
"""
偶遇实时检测脚本——穿越基础设施组件二（ceremony 集成）

ceremony 写入谱系后调用。检测新增谱系的跨纲 depends_on 边
是否满足341号偶遇标准：迫使修正已结算理解的关联。

用法:
  python scripts/check_encounter.py <genealogy_id>
  python scripts/check_encounter.py 343           # 检测343号谱系
  python scripts/check_encounter.py --since 340   # 检测340号之后的所有谱系

产出:
  JSON 到 stdout，包含 cross_gang_edges 和初步偶遇候选。
  如果发现候选，追加到 .chanlun/encounter-records.yaml。

341号偶遇标准：
  - 偶遇 = 迫使修正已结算理解的关联（不绑定时间性）
  - 设计内引用不是偶遇（元观察观察所有纲、K4研究链跨纲等）
  - 类比性连接不是偶遇（语义相近但不迫使修正）

设计内引用的七个模式（v150 Layer 1 结晶）：
  1. 元观察引用（标题含"元观察"）
  2. K4研究链跨纲（from/to 涉及 k4-* 目）
  3. 操作方法论→K4本体论
  4. 区块拓扑映射（from block-topology-eng）
  5. 折叠实验→折叠理论
  6. 语法规则结晶
  7. 元观察链
"""

import argparse
import json
import os
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parent.parent
GANGMU_PATH = ROOT / ".chanlun" / "gangmu.yaml"
SETTLED_DIR = ROOT / ".chanlun" / "genealogy" / "settled"
ENCOUNTER_RECORDS_PATH = ROOT / ".chanlun" / "encounter-records.yaml"


def load_gangmu():
    """读取 gangmu.yaml，返回 gang 列表。"""
    with open(GANGMU_PATH, encoding="utf-8") as f:
        data = yaml.safe_load(f)
    return data.get("gang", [])


def build_id_to_gang_map(gangs):
    """建立谱系编号(int) → (gang_id, mu_id) 映射。"""
    mu_entries = []
    for gang in gangs:
        gang_id = gang["id"]
        for mu in gang.get("mu", []):
            mu_id = mu["id"]
            status = mu.get("status", "active")
            opened_at = int(mu["opened_at"])
            closed_at = int(mu["closed_at"]) if "closed_at" in mu else None
            mu_entries.append({
                "gang_id": gang_id,
                "mu_id": mu_id,
                "status": status,
                "opened_at": opened_at,
                "closed_at": closed_at,
            })

    max_id = max(
        (e["closed_at"] or e["opened_at"] for e in mu_entries),
        default=500,
    )
    max_id = max(max_id, 500)

    mapping = {}
    for n in range(1, max_id + 1):
        candidates = []
        for entry in mu_entries:
            lo = entry["opened_at"]
            hi = entry["closed_at"]
            if hi is not None:
                if lo <= n <= hi:
                    candidates.append(entry)
            else:
                if n >= lo:
                    candidates.append(entry)
        if not candidates:
            continue

        def sort_key(e):
            is_active = 1 if e["status"] in ("active", "blocked") else 0
            return (is_active, e["opened_at"])

        candidates.sort(key=sort_key, reverse=True)
        best = candidates[0]
        mapping[n] = (best["gang_id"], best["mu_id"])

    return mapping


def parse_frontmatter(filepath):
    """解析 markdown 文件的 YAML frontmatter。"""
    with open(filepath, encoding="utf-8") as f:
        content = f.read()
    match = re.match(r"^---\s*\n(.*?)\n---", content, re.DOTALL)
    if not match:
        return None
    try:
        return yaml.safe_load(match.group(1))
    except yaml.YAMLError:
        return None


def normalize_id(raw_id):
    """将 id 值规范化为不带前导零的字符串。"""
    if raw_id is None:
        return ""
    s = str(raw_id).strip().strip("'\"")
    try:
        return str(int(s))
    except ValueError:
        return s


def classify_cross_gang_edge(from_id, from_title, from_gang, from_mu,
                             to_id, to_title, to_gang, to_mu):
    """分类一条跨纲边。

    返回 (classification, pattern_name)。
    classification: "design_internal" | "encounter_candidate"
    pattern_name: 匹配的设计内模式名（如果是设计内引用）
    """
    # 模式1：元观察引用（标题含"元观察"）
    if "元观察" in from_title or "元观察" in to_title:
        return "design_internal", "meta_observation_refs"

    # 模式2：K4研究链跨纲（from/to 涉及 k4-* 目）
    if (from_mu.startswith("k4-") and to_mu.startswith("k4-")) or \
       (from_mu.startswith("k4-") and to_mu == "k4-independence-fiber") or \
       (from_mu == "k4-independence-fiber" and to_mu.startswith("k4-")):
        return "design_internal", "k4_research_chain_cross_gang"

    # 模式3：操作方法论→K4本体论
    if from_mu == "operational-methodology" and to_mu.startswith("k4-"):
        return "design_internal", "operational_methodology_to_k4"

    # 模式4：区块拓扑映射
    if from_mu == "block-topology-eng" and from_gang == "block-topology":
        return "design_internal", "block_topology_mapping"

    # 模式5：折叠实验→折叠理论（block-topology → shipen-bihuan 涉及折叠）
    if ("折叠" in from_title or "fold" in from_title.lower()) and \
       ("折叠" in to_title or "fold" in to_title.lower()):
        return "design_internal", "fold_experiment_to_theory"

    # 模式6：语法规则结晶（from 涉及规则/有效域/形式化）
    if any(kw in from_title for kw in ("规则", "有效域", "形式化")):
        return "design_internal", "grammar_rule_crystallization"

    # 模式7：K4体制分析跨纲
    if from_mu == "k4-regime-analysis" or to_mu == "k4-regime-analysis":
        if to_mu.startswith("k4-") or from_mu.startswith("k4-") or \
           to_mu == "operational-methodology" or from_mu == "operational-methodology":
            return "design_internal", "k4_regime_cross_gang"

    # 模式8：蜂群架构跨纲引用（swarm-infra 相关的架构规则被引用）
    if to_gang == "swarm-infra" or from_gang == "swarm-infra":
        if "架构" in from_title or "架构" in to_title or \
           "Lead" in from_title or "Lead" in to_title or \
           "ceremony" in from_title or "ceremony" in to_title or \
           "并行" in from_title or "并行" in to_title:
            return "design_internal", "swarm_architecture_refs"

    # 不匹配任何已知设计内模式 → 候选偶遇
    return "encounter_candidate", None


def find_genealogy_file(genealogy_id):
    """根据谱系编号找到文件路径。"""
    nid = normalize_id(genealogy_id)
    for fp in SETTLED_DIR.iterdir():
        if fp.suffix == ".md" and fp.name.startswith(f"{nid}-"):
            return fp
    return None


def check_single_genealogy(genealogy_id, id_to_gang, id_to_title):
    """检测单个谱系的跨纲边。

    返回 dict: {genealogy_id, cross_gang_edges, encounter_candidates, design_internal}
    """
    nid = normalize_id(genealogy_id)
    fp = find_genealogy_file(nid)
    if fp is None:
        return {"genealogy_id": nid, "error": f"文件未找到: {nid}-*.md"}

    fm = parse_frontmatter(fp)
    if fm is None:
        return {"genealogy_id": nid, "error": "frontmatter 解析失败"}

    from_title = fm.get("title", "(无标题)")
    deps = fm.get("depends_on", [])
    if deps is None:
        deps = []

    from_num = int(nid) if nid.isdigit() else None
    from_info = id_to_gang.get(from_num) if from_num is not None else None

    if from_info is None:
        return {
            "genealogy_id": nid,
            "title": from_title,
            "error": f"谱系 {nid} 未映射到纲目",
        }

    from_gang, from_mu = from_info
    cross_gang_edges = []
    encounter_candidates = []
    design_internal = []

    for dep_raw in deps:
        dep_id = normalize_id(dep_raw)
        if not dep_id:
            continue

        dep_num = int(dep_id) if dep_id.isdigit() else None
        dep_info = id_to_gang.get(dep_num) if dep_num is not None else None
        if dep_info is None:
            continue

        dep_gang, dep_mu = dep_info
        if dep_gang == from_gang:
            continue  # 同纲边，跳过

        dep_title = id_to_title.get(dep_id, "(无标题)")

        edge = {
            "from_id": nid,
            "from_title": from_title,
            "from_gang": from_gang,
            "from_mu": from_mu,
            "to_id": dep_id,
            "to_title": dep_title,
            "to_gang": dep_gang,
            "to_mu": dep_mu,
        }
        cross_gang_edges.append(edge)

        classification, pattern = classify_cross_gang_edge(
            nid, from_title, from_gang, from_mu,
            dep_id, dep_title, dep_gang, dep_mu,
        )

        if classification == "design_internal":
            edge_classified = {**edge, "pattern": pattern}
            design_internal.append(edge_classified)
        else:
            encounter_candidates.append(edge)

    return {
        "genealogy_id": nid,
        "title": from_title,
        "gang": from_gang,
        "mu": from_mu,
        "total_deps": len(deps),
        "cross_gang_edges": len(cross_gang_edges),
        "design_internal_count": len(design_internal),
        "encounter_candidate_count": len(encounter_candidates),
        "design_internal": design_internal,
        "encounter_candidates": encounter_candidates,
    }


def load_encounter_records():
    """加载已有偶遇记录。"""
    if not ENCOUNTER_RECORDS_PATH.is_file():
        return {"version": "1.0", "records": [], "last_checked": None}
    with open(ENCOUNTER_RECORDS_PATH, encoding="utf-8") as f:
        data = yaml.safe_load(f) or {}
    if "records" not in data:
        data["records"] = []
    if "version" not in data:
        data["version"] = "1.0"
    return data


def save_encounter_records(data):
    """保存偶遇记录。"""
    ENCOUNTER_RECORDS_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(ENCOUNTER_RECORDS_PATH, "w", encoding="utf-8") as f:
        yaml.dump(data, f, allow_unicode=True, default_flow_style=False, sort_keys=False)


def append_candidates(candidates, genealogy_id):
    """将偶遇候选追加到记录文件。"""
    if not candidates:
        return
    records = load_encounter_records()
    now = datetime.now(timezone.utc).isoformat()
    for candidate in candidates:
        records["records"].append({
            "detected_at": now,
            "genealogy_id": genealogy_id,
            "status": "candidate",
            "edge": candidate,
        })
    records["last_checked"] = genealogy_id
    save_encounter_records(records)


def build_title_map():
    """从已结算谱系构建 id→title 映射。"""
    id_to_title = {}
    for fp in SETTLED_DIR.iterdir():
        if fp.suffix != ".md":
            continue
        fm = parse_frontmatter(fp)
        if fm is None:
            continue
        raw_id = fm.get("id", "")
        nid = normalize_id(raw_id)
        if nid:
            id_to_title[nid] = fm.get("title", "(无标题)")
    return id_to_title


def main():
    parser = argparse.ArgumentParser(description="偶遇实时检测")
    parser.add_argument("genealogy_id", nargs="?", help="要检测的谱系编号")
    parser.add_argument("--since", type=int, help="检测此编号之后的所有谱系")
    parser.add_argument("--record", action="store_true", default=True,
                        help="将候选追加到 encounter-records.yaml（默认开启）")
    parser.add_argument("--no-record", action="store_false", dest="record",
                        help="不写入记录文件（仅输出 JSON）")
    args = parser.parse_args()

    if args.genealogy_id is None and args.since is None:
        parser.error("请指定 genealogy_id 或 --since")

    gangs = load_gangmu()
    id_to_gang = build_id_to_gang_map(gangs)
    id_to_title = build_title_map()

    results = []

    if args.since is not None:
        # 检测 --since 之后的所有谱系
        for fp in sorted(SETTLED_DIR.iterdir()):
            if fp.suffix != ".md":
                continue
            m = re.match(r"^(\d+)", fp.name)
            if m and int(m.group(1)) > args.since:
                result = check_single_genealogy(m.group(1), id_to_gang, id_to_title)
                results.append(result)
                if args.record and result.get("encounter_candidate_count", 0) > 0:
                    append_candidates(result["encounter_candidates"], result["genealogy_id"])
    else:
        result = check_single_genealogy(args.genealogy_id, id_to_gang, id_to_title)
        results.append(result)
        if args.record and result.get("encounter_candidate_count", 0) > 0:
            append_candidates(result["encounter_candidates"], result["genealogy_id"])

    # 汇总
    total_cross = sum(r.get("cross_gang_edges", 0) for r in results)
    total_candidates = sum(r.get("encounter_candidate_count", 0) for r in results)
    total_design = sum(r.get("design_internal_count", 0) for r in results)

    output = {
        "checked_count": len(results),
        "total_cross_gang_edges": total_cross,
        "total_design_internal": total_design,
        "total_encounter_candidates": total_candidates,
        "results": results,
    }

    print(json.dumps(output, ensure_ascii=False, indent=2))
    return 0 if total_candidates == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
