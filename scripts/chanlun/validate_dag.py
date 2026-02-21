"""DAG validation for .chanlun/genealogy/dag.yaml"""
import sys
from pathlib import Path
from collections import defaultdict

import yaml

ROOT = Path(__file__).resolve().parent.parent.parent
DAG_PATH = ROOT / ".chanlun" / "genealogy" / "dag.yaml"
GENEALOGY_DIR = ROOT / ".chanlun" / "genealogy"


def load_dag():
    with open(DAG_PATH, encoding="utf-8") as f:
        return yaml.safe_load(f)


def validate(dag):
    errors = []
    nodes = dag.get("nodes", [])
    edges_section = dag.get("edges", {})

    node_ids = {str(n["id"]) for n in nodes}

    # 1. Reference integrity + collect directed edges for cycle check
    UNDIRECTED = {"related", "tensions_with"}
    directed_edges = []
    total_edge_count = 0
    for edge_type, edge_list in edges_section.items():
        if not isinstance(edge_list, list):
            continue
        for e in edge_list:
            total_edge_count += 1
            if "between" in e:
                for r in e["between"]:
                    if str(r) not in node_ids:
                        errors.append(f"edge ({edge_type}) references unknown node: {r}")
            else:
                src, dst = str(e["from"]), str(e["to"])
                if src not in node_ids:
                    errors.append(f"edge ({edge_type}) references unknown node: {src}")
                if dst not in node_ids:
                    errors.append(f"edge ({edge_type}) references unknown node: {dst}")
                if edge_type not in UNDIRECTED:
                    directed_edges.append((src, dst))

    # 2. Acyclicity (topological sort on directed edges only)
    adj = defaultdict(set)
    in_deg = {nid: 0 for nid in node_ids}
    for src, dst in directed_edges:
        if dst not in adj[src]:
            adj[src].add(dst)
            in_deg[dst] = in_deg.get(dst, 0) + 1

    queue = [n for n in node_ids if in_deg[n] == 0]
    visited = 0
    while queue:
        n = queue.pop()
        visited += 1
        for nb in adj[n]:
            in_deg[nb] -= 1
            if in_deg[nb] == 0:
                queue.append(nb)
    if visited != len(node_ids):
        errors.append(f"cycle detected: topological sort visited {visited}/{len(node_ids)} nodes")

    # 3. File existence
    for n in nodes:
        fpath = GENEALOGY_DIR / n["file"]
        if not fpath.exists():
            errors.append(f"node {n['id']}: file not found: {n['file']}")

    # 4. Node count = genealogy file count
    md_files = set()
    for subdir in ("settled", "pending"):
        d = GENEALOGY_DIR / subdir
        if d.exists():
            md_files.update(d.glob("*.md"))
    if len(nodes) != len(md_files):
        errors.append(f"node count mismatch: dag has {len(nodes)} nodes, filesystem has {len(md_files)} .md files")

    return errors, len(nodes), total_edge_count


def main():
    if not DAG_PATH.exists():
        print(f"FAIL: {DAG_PATH} not found", file=sys.stderr)
        sys.exit(1)

    dag = load_dag()
    errors, n_nodes, n_edges = validate(dag)

    if errors:
        print(f"DAG validation FAILED ({len(errors)} errors):", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        sys.exit(1)

    print(f"DAG validation passed: {n_nodes} nodes, {n_edges} edges")


if __name__ == "__main__":
    main()
