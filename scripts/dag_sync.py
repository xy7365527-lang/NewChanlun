#!/usr/bin/env python3
"""dag_sync.py — 谱系文件 → dag.yaml 自动同步

读取 .chanlun/genealogy/{settled,pending}/ 下所有 .md 文件的 YAML frontmatter，
与 .chanlun/genealogy/dag.yaml 对比，自动补全缺失的节点和边。

确定性操作，不需要 LLM。由 PostToolUse hook 或手动调用。
"""

import sys
import os
import re
import yaml
from pathlib import Path


def parse_frontmatter(filepath: Path) -> dict | None:
    """解析 markdown 文件的 YAML frontmatter。"""
    text = filepath.read_text(encoding="utf-8")
    match = re.match(r"^---\n(.*?\n)---", text, re.DOTALL)
    if not match:
        return None
    try:
        return yaml.safe_load(match.group(1))
    except yaml.YAMLError:
        return None


def load_dag(dag_path: Path) -> dict:
    """加载 dag.yaml。"""
    text = dag_path.read_text(encoding="utf-8")
    return yaml.safe_load(text) or {"nodes": [], "edges": {"depends_on": [], "tensions_with": [], "negates": [], "related": []}}


def save_dag(dag_path: Path, dag: dict):
    """写回 dag.yaml。"""
    dag_path.write_text(
        yaml.dump(dag, allow_unicode=True, default_flow_style=False, sort_keys=False),
        encoding="utf-8",
    )


def get_existing_node_ids(dag: dict) -> set:
    return {str(n["id"]) for n in dag.get("nodes", [])}


def get_existing_edges(dag: dict, edge_type: str) -> set:
    """返回边集合。depends_on/negates/related 用 (from,to)，tensions_with 用 frozenset。"""
    edges = dag.get("edges", {}).get(edge_type, []) or []
    result = set()
    for e in edges:
        if "from" in e and "to" in e:
            result.add((str(e["from"]), str(e["to"])))
        elif "between" in e:
            pair = [str(x) for x in e["between"]]
            result.add(frozenset(pair))
    return result


def scan_genealogy_files(base: Path) -> list[tuple[Path, dict]]:
    """扫描所有谱系 .md 文件，返回 (路径, frontmatter) 列表。"""
    results = []
    for subdir in ["settled", "pending"]:
        d = base / subdir
        if not d.exists():
            continue
        for f in sorted(d.glob("*.md")):
            fm = parse_frontmatter(f)
            if fm and "id" in fm:
                results.append((f, fm))
    return results


def sync(root: Path) -> dict:
    """执行同步，返回变更摘要。"""
    genealogy_base = root / ".chanlun" / "genealogy"
    dag_path = genealogy_base / "dag.yaml"

    if not dag_path.exists():
        return {"error": "dag.yaml not found"}

    dag = load_dag(dag_path)
    files = scan_genealogy_files(genealogy_base)
    existing_ids = get_existing_node_ids(dag)

    changes = {"nodes_added": [], "edges_added": [], "total_files": len(files), "total_dag_nodes": len(existing_ids)}

    # 边类型映射：frontmatter 字段 → dag edges 键
    edge_type_map = {
        "depends_on": "depends_on",
        "tensions_with": "tensions_with",
        "negates": "negates",
        "related": "related",
    }

    for filepath, fm in files:
        node_id = str(fm["id"])

        # 补节点
        if node_id not in existing_ids:
            relative = filepath.relative_to(genealogy_base).as_posix()
            new_node = {
                "id": node_id,
                "title": fm.get("title", ""),
                "status": fm.get("status", ""),
                "type": fm.get("type", ""),
                "file": relative,
            }
            dag["nodes"].append(new_node)
            existing_ids.add(node_id)
            changes["nodes_added"].append(node_id)

        # 补边
        for fm_key, dag_key in edge_type_map.items():
            targets = fm.get(fm_key, []) or []
            if isinstance(targets, str):
                targets = [targets]

            existing_edges = get_existing_edges(dag, dag_key)

            for target in targets:
                # tensions_with 可能是 dict（含 valid_until 等字段）
                if isinstance(target, dict):
                    target_id = str(target.get("id", target.get("node", "")))
                else:
                    target_id = str(target)

                if not target_id:
                    continue

                # tensions_with 和 related 用 between 格式（无向），depends_on/negates 用 from/to（有向）
                if dag_key in ("tensions_with", "related"):
                    edge_key = frozenset([node_id, target_id])
                    if edge_key not in existing_edges:
                        new_edge = {"between": sorted([node_id, target_id])}
                        # 保留 valid_until 等额外字段
                        if isinstance(target, dict):
                            for k, v in target.items():
                                if k not in ("id", "node"):
                                    new_edge[k] = v
                        if dag_key not in dag.get("edges", {}):
                            dag.setdefault("edges", {})[dag_key] = []
                        dag["edges"][dag_key].append(new_edge)
                        changes["edges_added"].append(f"{dag_key}: {node_id}↔{target_id}")
                        existing_edges.add(edge_key)
                else:
                    edge_tuple = (node_id, target_id)
                    if edge_tuple not in existing_edges:
                        new_edge = {"from": node_id, "to": target_id}
                        if dag_key not in dag.get("edges", {}):
                            dag.setdefault("edges", {})[dag_key] = []
                        dag["edges"][dag_key].append(new_edge)
                        changes["edges_added"].append(f"{dag_key}: {node_id}→{target_id}")
                        existing_edges.add(edge_tuple)

    if changes["nodes_added"] or changes["edges_added"]:
        save_dag(dag_path, dag)

    changes["total_dag_nodes_after"] = len(existing_ids)
    return changes


def main():
    root = Path(__file__).resolve().parent.parent
    changes = sync(root)

    if "error" in changes:
        print(f"ERROR: {changes['error']}", file=sys.stderr)
        sys.exit(1)

    if not changes["nodes_added"] and not changes["edges_added"]:
        print(f"[dag-sync] 已同步。{changes['total_files']} 文件 = {changes['total_dag_nodes']} 节点，无变更。")
    else:
        print(f"[dag-sync] 同步完成：")
        if changes["nodes_added"]:
            print(f"  节点新增: {', '.join(changes['nodes_added'])}")
        if changes["edges_added"]:
            for e in changes["edges_added"]:
                print(f"  边新增: {e}")
        print(f"  dag 节点数: {changes['total_dag_nodes']} → {changes['total_dag_nodes_after']}")


if __name__ == "__main__":
    main()
