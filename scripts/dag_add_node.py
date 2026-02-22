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

    with open(dag_path, "w", encoding="utf-8") as f:
        yaml.dump(dag, f, allow_unicode=True, default_flow_style=False, sort_keys=False)

    print(f"dag.yaml updated: {len(dag['nodes'])} nodes")

if __name__ == "__main__":
    main()
