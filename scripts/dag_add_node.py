#!/usr/bin/env python
"""向 dag.yaml 添加节点和边。避免 Lead 读取整个 55KB+ 的 dag.yaml。

用法:
  python scripts/dag_add_node.py --id 143 --title "标题" --type "定理" \
    --file "settled/143-xxx.md" \
    --depends_on 140,141 \
    --related 069,093
"""
import argparse, yaml, os, sys

def main():
    parser = argparse.ArgumentParser(description="向 dag.yaml 添加节点和边")
    parser.add_argument("--id", required=True, help="节点 ID")
    parser.add_argument("--title", required=True, help="节点标题")
    parser.add_argument("--type", default="定理", help="节点类型")
    parser.add_argument("--file", required=True, help="谱系文件路径（相对于 genealogy/）")
    parser.add_argument("--depends_on", default="", help="逗号分隔的依赖节点 ID")
    parser.add_argument("--related", default="", help="逗号分隔的相关节点 ID")
    parser.add_argument("--tensions_with", default="", help="逗号分隔的张力节点 ID")
    parser.add_argument("--negates", default="", help="逗号分隔的否定节点 ID")
    parser.add_argument("--topo_effect", default="", help="拓扑操作 格式: type:target:scope (freeze|split|sever)")
    args = parser.parse_args()

    dag_path = ".chanlun/genealogy/dag.yaml"
    if not os.path.isfile(dag_path):
        print(f"Error: {dag_path} not found", file=sys.stderr)
        sys.exit(1)

    with open(dag_path, encoding="utf-8") as f:
        dag = yaml.safe_load(f)

    node_id = args.id
    existing_ids = {str(n["id"]) for n in dag["nodes"]}
    if node_id in existing_ids:
        print(f"Node {node_id} already exists, updating edges only")
    else:
        new_node = {
            "id": node_id,
            "title": args.title,
            "status": "已结算",
            "type": args.type,
            "file": args.file,
        }
        dag["nodes"].append(new_node)
        print(f"Added node {node_id}: {args.title}")

    edges = dag.setdefault("edges", {})

    def add_edges(edge_type, ids_str, directed=True):
        if not ids_str:
            return
        section = edges.setdefault(edge_type, [])
        existing = set()
        for e in section:
            if directed:
                existing.add((str(e.get("from", "")), str(e.get("to", ""))))
            else:
                pair = tuple(sorted([str(e["between"][0]), str(e["between"][1])]))
                existing.add(pair)
        for target_id in ids_str.split(","):
            target_id = target_id.strip()
            if not target_id:
                continue
            if directed:
                key = (node_id, target_id)
                if key not in existing:
                    section.append({"from": node_id, "to": target_id})
                    print(f"  Added {edge_type}: {node_id} -> {target_id}")
            else:
                pair = tuple(sorted([node_id, target_id]))
                if pair not in existing:
                    section.append({"between": [pair[0], pair[1]]})
                    print(f"  Added {edge_type}: {pair[0]} <-> {pair[1]}")

    add_edges("depends_on", args.depends_on, directed=True)
    add_edges("related", args.related, directed=False)
    add_edges("tensions_with", args.tensions_with, directed=False)
    add_edges("negates", args.negates, directed=True)

    # 147号：拓扑操作（freeze/split/sever）
    if args.topo_effect:
        parts = args.topo_effect.split(":")
        if len(parts) != 3:
            print(f"Error: --topo_effect 格式应为 type:target:scope, got '{args.topo_effect}'", file=sys.stderr)
            sys.exit(1)
        effect_type, target_id, scope = parts
        if effect_type not in ("freeze", "split", "sever"):
            print(f"Error: topo_effect type must be freeze|split|sever, got '{effect_type}'", file=sys.stderr)
            sys.exit(1)

        if effect_type == "freeze":
            # 在目标节点上添加 frozen 标记
            for n in dag["nodes"]:
                if str(n["id"]) == target_id:
                    n["frozen"] = True
                    n["frozen_by"] = node_id
                    print(f"  Froze node {target_id} (by {node_id})")
                    break
            else:
                print(f"  Warning: target node {target_id} not found for freeze", file=sys.stderr)
            # 如果 scope=downstream，冻结所有以 target 为起点的 depends_on 边
            if scope == "downstream":
                for e in edges.get("depends_on", []):
                    if str(e.get("to", "")) == target_id:
                        e["frozen_by"] = node_id
                        print(f"  Froze edge: {e.get('from')} -> {target_id}")

        elif effect_type == "sever":
            # 在目标节点相关的边上添加 severed 标记
            for edge_type in ("depends_on", "negates"):
                for e in edges.get(edge_type, []):
                    if str(e.get("from", "")) == target_id or str(e.get("to", "")) == target_id:
                        e["severed_by"] = node_id
                        print(f"  Severed {edge_type} edge involving {target_id}")
            for edge_type in ("related", "tensions_with"):
                for e in edges.get(edge_type, []):
                    pair = e.get("between", [])
                    if target_id in [str(p) for p in pair]:
                        e["severed_by"] = node_id
                        print(f"  Severed {edge_type} edge involving {target_id}")

        elif effect_type == "split":
            # 节点分裂：创建 target-a 和 target-b
            original = None
            for i, n in enumerate(dag["nodes"]):
                if str(n["id"]) == target_id:
                    original = n
                    break
            if original:
                node_a = {**original, "id": f"{target_id}-a", "split_from": target_id, "split_by": node_id}
                node_b = {**original, "id": f"{target_id}-b", "split_from": target_id, "split_by": node_id}
                original["split_into"] = [f"{target_id}-a", f"{target_id}-b"]
                original["split_by"] = node_id
                dag["nodes"].append(node_a)
                dag["nodes"].append(node_b)
                print(f"  Split node {target_id} into {target_id}-a and {target_id}-b (by {node_id})")
            else:
                print(f"  Warning: target node {target_id} not found for split", file=sys.stderr)

    with open(dag_path, "w", encoding="utf-8") as f:
        yaml.dump(dag, f, allow_unicode=True, default_flow_style=False, sort_keys=False)

    print(f"dag.yaml updated: {len(dag['nodes'])} nodes")

if __name__ == "__main__":
    main()
