"""从谱系文件 frontmatter 生成 dag.yaml"""

import re
from datetime import datetime
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent.parent.parent / ".chanlun" / "genealogy"
DIRS = ["settled", "pending"]


def parse_frontmatter(path: Path) -> dict | None:
    text = path.read_text(encoding="utf-8")
    m = re.match(r"^---\n(.+?)\n---", text, re.DOTALL)
    if not m:
        return None
    data = yaml.safe_load(m.group(1))
    data["_file"] = f"{path.parent.name}/{path.name}"
    return data


def build_dag() -> dict:
    entries = []
    for d in DIRS:
        for p in sorted((ROOT / d).glob("*.md")):
            fm = parse_frontmatter(p)
            if fm and "id" in fm:
                entries.append(fm)

    nodes = [
        {"id": e["id"], "title": e.get("title", ""),
         "status": e.get("status", e.get("状态", "")),
         "type": e.get("type", e.get("类型", "")),
         "file": e["_file"]}
        for e in entries
    ]

    # 有向边: from/source → to/target
    DIRECTED = ["depends_on", "triggered", "derived", "negates"]
    # 无向边: 双向去重
    UNDIRECTED = ["related", "tensions_with"]
    # 反向引用边: target ← by
    REVERSE = ["negated_by"]

    directed = {k: [] for k in DIRECTED}
    undirected = {k: set() for k in UNDIRECTED}
    reverse = {k: [] for k in REVERSE}

    for e in entries:
        eid = e["id"]
        for k in DIRECTED:
            for v in e.get(k) or []:
                directed[k].append({"from": eid, "to": str(v)})
        for k in UNDIRECTED:
            for v in e.get(k) or []:
                undirected[k].add(tuple(sorted([eid, str(v)])))
        for k in REVERSE:
            for v in e.get(k) or []:
                reverse[k].append({"target": eid, "by": str(v)})

    edges = {}
    for k in DIRECTED:
        edges[k] = directed[k]
    for k in UNDIRECTED:
        edges[k] = [{"between": list(p)} for p in sorted(undirected[k])]
    for k in REVERSE:
        edges[k] = reverse[k]

    return {"nodes": nodes, "edges": edges}


def main():
    dag = build_dag()
    now = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")
    header = (
        f"# 谱系 DAG — 自动生成，勿手动编辑\n"
        f"# 生成时间: {now}\n"
        f"# 来源: .chanlun/genealogy/{{settled,pending}}/*.md frontmatter\n\n"
    )
    out = ROOT / "dag.yaml"
    out.write_text(
        header + yaml.dump(dag, allow_unicode=True, default_flow_style=False, sort_keys=False),
        encoding="utf-8",
    )
    print(f"生成完成: {out}")
    print(f"  节点数: {len(dag['nodes'])}")
    for k, v in dag["edges"].items():
        print(f"  {k} 边: {len(v)}")


if __name__ == "__main__":
    main()
