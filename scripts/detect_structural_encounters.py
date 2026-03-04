#!/usr/bin/env python3
"""
Layer 2 偶遇检测辅助脚本

读取 gangmu.yaml 和 cross-gang-refs.json，识别最稀疏的纲对，
列出值得深度阅读的谱系文件路径。

341号偶遇标准：
  - 偶遇 = 迫使修正已有结构理解的关联（不绑定时间性）
  - 不是类比性连接（"语义相近"不够）
  - 必须"迫使修正某条已结算谱系的结论"

产出：稀疏纲对分析 + 推荐深度阅读列表
"""

import json
import os
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GANGMU = ROOT / ".chanlun" / "gangmu.yaml"
CROSS_REFS = ROOT / "tmp" / "cross-gang-refs.json"
DAG = ROOT / ".chanlun" / "genealogy" / "dag.yaml"
SETTLED_DIR = ROOT / ".chanlun" / "genealogy" / "settled"
OUTPUT = ROOT / "tmp" / "structural-encounters.json"


def load_yaml(path):
    """Minimal YAML loader for our specific format."""
    import yaml
    with open(path, encoding="utf-8") as f:
        return yaml.safe_load(f)


def load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def get_gang_ids(gangmu_data):
    """Extract all gang IDs from gangmu."""
    return [g["id"] for g in gangmu_data["gang"]]


def get_mu_ranges(gangmu_data):
    """Extract mu ID -> (gang_id, mu_id, opened_at) mapping."""
    result = {}
    for gang in gangmu_data["gang"]:
        for mu in gang.get("mu", []):
            result[mu["id"]] = {
                "gang": gang["id"],
                "mu_id": mu["id"],
                "mu_name": mu["name"],
                "status": mu["status"],
                "opened_at": mu.get("opened_at", "?"),
            }
    return result


def build_pair_matrix(cross_data, gang_ids):
    """Build full pair matrix including zeros."""
    pair_counts = Counter()
    for e in cross_data["cross_gang_details"]:
        key = (e["from_gang"], e["to_gang"])
        pair_counts[key] += 1

    # Also count concept-only (exclude meta-observations)
    concept_counts = Counter()
    for e in cross_data["cross_gang_details"]:
        if "元观察" not in e["from_title"] and "元观察" not in e["to_title"]:
            key = (e["from_gang"], e["to_gang"])
            concept_counts[key] += 1

    # Build undirected totals
    undirected = {}
    for i, g1 in enumerate(gang_ids):
        for j, g2 in enumerate(gang_ids):
            if j > i:
                a = pair_counts.get((g1, g2), 0)
                b = pair_counts.get((g2, g1), 0)
                ca = concept_counts.get((g1, g2), 0)
                cb = concept_counts.get((g2, g1), 0)
                undirected[(g1, g2)] = {
                    "total": a + b,
                    "concept_only": ca + cb,
                }
    return undirected


def find_sparse_pairs(pair_matrix, threshold=5):
    """Find pairs below threshold, sorted by sparsity."""
    sparse = []
    for (g1, g2), counts in pair_matrix.items():
        if counts["concept_only"] <= threshold:
            sparse.append({
                "gang_a": g1,
                "gang_b": g2,
                "total_connections": counts["total"],
                "concept_connections": counts["concept_only"],
            })
    sparse.sort(key=lambda x: x["concept_connections"])
    return sparse


def read_frontmatter(filepath):
    """Read YAML frontmatter from a genealogy file."""
    with open(filepath, encoding="utf-8") as f:
        content = f.read()
    m = re.match(r"---\n(.*?)\n---", content, re.DOTALL)
    if not m:
        return {}
    import yaml
    return yaml.safe_load(m.group(1)) or {}


def find_genealogies_for_gang_mu(dag_data, gang_id, gangmu_data):
    """Find genealogies that belong to a specific gang.

    Strategy: use mu opened_at ranges and cross-gang edge mappings.
    """
    mu_ranges = get_mu_ranges(gangmu_data)
    gang_mus = {k: v for k, v in mu_ranges.items() if v["gang"] == gang_id}

    # For each mu, find genealogies in the opened_at range
    results = []
    for mu_id, info in gang_mus.items():
        opened = int(info["opened_at"]) if info["opened_at"] != "?" else 0
        # Heuristic: genealogies from opened_at to next mu's opened_at
        for node in dag_data["nodes"]:
            nid = int(node["id"]) if node["id"].isdigit() else 0
            if nid >= opened:
                results.append({
                    "id": node["id"],
                    "title": node["title"],
                    "type": node.get("type", "?"),
                    "status": node.get("status", "?"),
                    "mu_id": mu_id,
                    "file": node.get("file", ""),
                })
    return results


def recommend_reading(sparse_pairs, gangmu_data, dag_data, cross_data):
    """For each sparse pair, recommend genealogies for deep reading."""
    recommendations = []

    # Build gang->genealogy mapping from cross-gang edges
    gang_genealogies = defaultdict(set)
    for e in cross_data["cross_gang_details"]:
        gang_genealogies[e["from_gang"]].add(e["from_id"])
        gang_genealogies[e["to_gang"]].add(e["to_id"])

    # Build title map
    title_map = {n["id"]: n["title"] for n in dag_data["nodes"]}
    type_map = {n["id"]: n.get("type", "?") for n in dag_data["nodes"]}
    file_map = {n["id"]: n.get("file", "") for n in dag_data["nodes"]}

    mu_ranges = get_mu_ranges(gangmu_data)

    for pair in sparse_pairs[:5]:  # Top 5 sparsest
        ga, gb = pair["gang_a"], pair["gang_b"]

        # For each gang in the pair, find key genealogies
        for gang_id in [ga, gb]:
            gang_mus = {k: v for k, v in mu_ranges.items() if v["gang"] == gang_id}
            active_mus = {k: v for k, v in gang_mus.items() if v["status"] == "active"}

            # Prefer active mu, fallback to all
            target_mus = active_mus if active_mus else gang_mus

            key_ids = []
            for mu_id, info in target_mus.items():
                opened = int(info["opened_at"]) if info["opened_at"] != "?" else 0
                # Find non-meta-observation genealogies in this range
                candidates = []
                for n in dag_data["nodes"]:
                    nid = int(n["id"]) if n["id"].isdigit() else 0
                    if nid >= opened and "元观察" not in n["title"]:
                        candidates.append(n)

                # Take up to 5 most recent non-meta candidates
                candidates.sort(key=lambda x: int(x["id"]) if x["id"].isdigit() else 0, reverse=True)
                for c in candidates[:5]:
                    key_ids.append({
                        "id": c["id"],
                        "title": c["title"],
                        "type": c.get("type", "?"),
                        "file": c.get("file", ""),
                        "mu": mu_id,
                    })

            pair.setdefault("recommended_reading", {})[gang_id] = key_ids

        recommendations.append(pair)

    return recommendations


def main():
    gangmu = load_yaml(GANGMU)
    cross = load_json(CROSS_REFS)
    dag = load_yaml(DAG)

    gang_ids = get_gang_ids(gangmu)
    pair_matrix = build_pair_matrix(cross, gang_ids)
    sparse_pairs = find_sparse_pairs(pair_matrix)

    print("=== Sparse Gang Pairs (concept connections <= 5) ===")
    for p in sparse_pairs:
        print(f"  {p['gang_a']} <-> {p['gang_b']}: "
              f"total={p['total_connections']}, concept={p['concept_connections']}")

    print(f"\n=== Top {min(5, len(sparse_pairs))} sparsest pairs for deep reading ===")
    recommendations = recommend_reading(sparse_pairs, gangmu, dag, cross)

    for rec in recommendations:
        print(f"\n--- {rec['gang_a']} <-> {rec['gang_b']} ---")
        for gang_id, readings in rec.get("recommended_reading", {}).items():
            print(f"  [{gang_id}]:")
            for r in readings[:3]:
                filepath = SETTLED_DIR / r["file"].replace("settled/", "") if r["file"] else "?"
                print(f"    {r['id']}: {r['title']}")
                print(f"       file: {filepath}")

    # Write sparse pairs analysis for downstream
    output = {
        "sparse_pairs": sparse_pairs,
        "recommendations": [
            {
                "gang_a": r["gang_a"],
                "gang_b": r["gang_b"],
                "total_connections": r["total_connections"],
                "concept_connections": r["concept_connections"],
                "recommended_reading": r.get("recommended_reading", {}),
            }
            for r in recommendations
        ],
    }

    with open(OUTPUT, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    print(f"\nSparse pair analysis written to {OUTPUT}")
    return sparse_pairs, recommendations


if __name__ == "__main__":
    main()
