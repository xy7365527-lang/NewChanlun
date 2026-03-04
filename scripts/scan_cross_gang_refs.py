#!/usr/bin/env python3
"""
扫描谱系跨纲 depends_on 引用。

读取 gangmu.yaml 建立谱系编号→(gang_id, mu_id) 映射，
遍历 settled/*.md 解析 frontmatter，对每条 depends_on 边
检测是否跨纲。输出 JSON 到 stdout 并写入 tmp/cross-gang-refs.json。
"""

import json
import os
import re
import sys
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parent.parent
GANGMU_PATH = ROOT / ".chanlun" / "gangmu.yaml"
SETTLED_DIR = ROOT / ".chanlun" / "genealogy" / "settled"
OUTPUT_PATH = ROOT / "tmp" / "cross-gang-refs.json"


def load_gangmu(path: Path):
    """读取 gangmu.yaml，返回 gang 列表。"""
    with open(path, "r", encoding="utf-8") as f:
        data = yaml.safe_load(f)
    return data.get("gang", [])


def build_id_to_gang_map(gangs):
    """
    建立谱系编号(int) → (gang_id, mu_id) 映射。

    mu 范围可重叠。优先级：
    1. active/blocked 优先于 closed
    2. opened_at 更晚的优先
    """
    # 收集所有 mu 的范围信息
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

    # 确定编号范围上限
    max_id = 0
    for entry in mu_entries:
        max_id = max(max_id, entry["opened_at"])
        if entry["closed_at"] is not None:
            max_id = max(max_id, entry["closed_at"])
    max_id = max(max_id, 500)  # 留余量

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
                # active/blocked: 无 closed_at，只需 >= opened_at
                if n >= lo:
                    candidates.append(entry)
        if not candidates:
            continue

        # 排序优先级：active/blocked > closed，然后 opened_at 降序
        def sort_key(e):
            is_active = 1 if e["status"] in ("active", "blocked") else 0
            return (is_active, e["opened_at"])

        candidates.sort(key=sort_key, reverse=True)
        best = candidates[0]
        mapping[n] = (best["gang_id"], best["mu_id"])

    return mapping


def parse_frontmatter(filepath: Path):
    """解析 markdown 文件的 YAML frontmatter，返回 dict 或 None。"""
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    # frontmatter 在文件开头的 --- 和 --- 之间
    match = re.match(r"^---\s*\n(.*?)\n---", content, re.DOTALL)
    if not match:
        return None
    try:
        return yaml.safe_load(match.group(1))
    except yaml.YAMLError:
        return None


def normalize_id(raw_id) -> str:
    """将 id 值规范化为不带前导零的字符串。"""
    if raw_id is None:
        return ""
    s = str(raw_id).strip().strip("'\"")
    # 去掉前导零，但保留 "0"
    try:
        return str(int(s))
    except ValueError:
        return s


def main():
    gangs = load_gangmu(GANGMU_PATH)
    id_to_gang = build_id_to_gang_map(gangs)

    # 收集所有谱系文件的 frontmatter
    genealogy_files = sorted(SETTLED_DIR.glob("*.md"))
    total_genealogy = len(genealogy_files)

    # id → title 映射（用于输出）
    id_to_title = {}
    # id → depends_on 列表
    id_to_deps = {}

    for fp in genealogy_files:
        fm = parse_frontmatter(fp)
        if fm is None:
            continue
        raw_id = fm.get("id", "")
        nid = normalize_id(raw_id)
        if not nid:
            continue
        id_to_title[nid] = fm.get("title", "(无标题)")
        deps = fm.get("depends_on", [])
        if deps is None:
            deps = []
        id_to_deps[nid] = [normalize_id(d) for d in deps if normalize_id(d)]

    # 统计边
    total_edges = 0
    cross_gang_edges = 0
    same_gang_edges = 0
    unmapped_edges = 0
    cross_gang_details = []
    gang_pair_counter = {}

    for from_id, deps in id_to_deps.items():
        for to_id in deps:
            total_edges += 1
            from_num = int(from_id) if from_id.isdigit() else None
            to_num = int(to_id) if to_id.isdigit() else None

            from_info = id_to_gang.get(from_num) if from_num is not None else None
            to_info = id_to_gang.get(to_num) if to_num is not None else None

            if from_info is None or to_info is None:
                unmapped_edges += 1
                # 仍然记录，标记为 unmapped
                detail = {
                    "from_id": from_id,
                    "from_title": id_to_title.get(from_id, "?"),
                    "from_gang": from_info[0] if from_info else "unmapped",
                    "from_mu": from_info[1] if from_info else "unmapped",
                    "to_id": to_id,
                    "to_title": id_to_title.get(to_id, "?"),
                    "to_gang": to_info[0] if to_info else "unmapped",
                    "to_mu": to_info[1] if to_info else "unmapped",
                }
                if from_info and to_info and from_info[0] != to_info[0]:
                    cross_gang_details.append(detail)
                continue

            from_gang = from_info[0]
            to_gang = to_info[0]

            if from_gang == to_gang:
                same_gang_edges += 1
            else:
                cross_gang_edges += 1
                pair_key = f"{from_gang} -> {to_gang}"
                gang_pair_counter[pair_key] = gang_pair_counter.get(pair_key, 0) + 1
                cross_gang_details.append({
                    "from_id": from_id,
                    "from_title": id_to_title.get(from_id, "?"),
                    "from_gang": from_gang,
                    "from_mu": from_info[1],
                    "to_id": to_id,
                    "to_title": id_to_title.get(to_id, "?"),
                    "to_gang": to_gang,
                    "to_mu": to_info[1],
                })

    # 排序输出
    cross_gang_details.sort(key=lambda x: (x["from_gang"], x["to_gang"], x["from_id"]))
    gang_pair_summary = dict(sorted(gang_pair_counter.items(), key=lambda x: -x[1]))

    result = {
        "total_genealogy": total_genealogy,
        "total_depends_on_edges": total_edges,
        "cross_gang_edges": cross_gang_edges,
        "same_gang_edges": same_gang_edges,
        "unmapped_edges": unmapped_edges,
        "cross_gang_details": cross_gang_details,
        "gang_pair_summary": gang_pair_summary,
    }

    # 确保 tmp/ 目录存在
    OUTPUT_PATH.parent.mkdir(parents=True, exist_ok=True)

    # 写入文件
    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)

    # 输出到 stdout
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
