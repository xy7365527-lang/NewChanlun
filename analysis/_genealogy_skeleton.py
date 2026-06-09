#!/usr/bin/env python3
"""谱系骨架提取——从 dag.yaml 解析 nodes/edges，构建反向依赖图、根/叶节点、扬弃/否定/张力链。

纯技术性产出（数据提取），结果供下游 genealogy_map.md 综合使用。
"""
import yaml
import json
from collections import defaultdict
from pathlib import Path

DAG = Path(".chanlun/genealogy/dag.yaml")
OUT = Path("analysis/_genealogy_skeleton.json")


def main():
    data = yaml.safe_load(DAG.read_text(encoding="utf-8"))
    nodes = data.get("nodes", [])
    edges = data.get("edges", {}) or {}

    # 节点表
    node_index = {}
    for n in nodes:
        nid = str(n.get("id"))
        node_index[nid] = {
            "id": nid,
            "title": n.get("title", ""),
            "status": n.get("status", ""),
            "type": n.get("type", ""),
            "file": n.get("file", ""),
        }

    # 边分类
    edge_types = list(edges.keys())
    edge_lists = {}
    for et in edge_types:
        lst = edges.get(et) or []
        norm = []
        for e in lst:
            if isinstance(e, dict):
                norm.append({"from": str(e.get("from")), "to": str(e.get("to")),
                             **{k: v for k, v in e.items() if k not in ("from", "to")}})
        edge_lists[et] = norm

    # 依赖图（depends_on: from 依赖 to）
    dep = edge_lists.get("depends_on", [])
    forward = defaultdict(list)   # node -> 它依赖谁 (to)
    reverse = defaultdict(list)   # node -> 谁依赖它 (from)
    for e in dep:
        forward[e["from"]].append(e["to"])
        reverse[e["to"]].append(e["from"])

    all_ids = set(node_index.keys())
    have_deps = set(forward.keys())          # 有出边（依赖别人）
    are_depended = set(reverse.keys())       # 有入边（被依赖）

    # 根节点：没有依赖别人的（在 depends_on 中不作为 from） 但被别人依赖
    roots = sorted(are_depended - have_deps)
    # 叶节点：依赖别人但没人依赖它
    leaves = sorted(have_deps - are_depended)
    # 孤立节点：既不依赖也不被依赖（在 depends_on 图中）
    isolated = sorted(all_ids - have_deps - are_depended)

    # 入度排行（被依赖最多 = 谱系枢纽）
    indeg = sorted(((len(v), k) for k, v in reverse.items()), reverse=True)

    skeleton = {
        "node_count": len(node_index),
        "edge_types": {et: len(v) for et, v in edge_lists.items()},
        "nodes": node_index,
        "edges": edge_lists,
        "reverse_depends": {k: sorted(v) for k, v in reverse.items()},
        "forward_depends": {k: sorted(v) for k, v in forward.items()},
        "roots_depended_no_outdep": roots,
        "leaves_outdep_no_indep": leaves,
        "isolated_in_dep_graph": isolated,
        "top_indegree": indeg[:30],
    }
    OUT.write_text(json.dumps(skeleton, ensure_ascii=False, indent=2), encoding="utf-8")

    # 控制台摘要
    print(f"节点数: {len(node_index)}")
    print(f"边类型: {skeleton['edge_types']}")
    print(f"根节点(被依赖,无出依赖) {len(roots)}: {roots}")
    print(f"叶节点(有出依赖,无入依赖) {len(leaves)}: {leaves[:40]}{'...' if len(leaves)>40 else ''}")
    print(f"dep图孤立节点 {len(isolated)}: {isolated[:40]}{'...' if len(isolated)>40 else ''}")
    print("\n被依赖最多的枢纽 (入度 top15):")
    for c, k in indeg[:15]:
        print(f"  {k}: 入度{c} | {node_index.get(k,{}).get('title','?')[:40]}")
    print("\n非 depends_on 边:")
    for et in edge_types:
        if et != "depends_on":
            print(f"  [{et}] {len(edge_lists[et])} 条:")
            for e in edge_lists[et]:
                extra = {k: v for k, v in e.items() if k not in ("from", "to")}
                print(f"    {e['from']} -> {e['to']} {extra if extra else ''}")


if __name__ == "__main__":
    main()
