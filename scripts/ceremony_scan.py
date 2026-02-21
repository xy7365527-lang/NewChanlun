#!/usr/bin/env python
"""ceremony Phase 1-2: 确定性状态推导，输出 JSON 指令。"""
import json, os, glob, yaml, sys

def main():
    root = os.getcwd()
    result = {"mode": "cold_start", "definitions": 0, "pending": 0, "settled": 0,
              "head": "", "session": None, "workstations": [], "structural_nodes": []}

    # --- 最新 session（按修改时间排序） ---
    sessions = glob.glob(os.path.join(root, ".chanlun/sessions/*-session.md"))
    if sessions:
        sessions.sort(key=os.path.getmtime)
        result["mode"] = "warm_start"
        result["session"] = os.path.basename(sessions[-1])
        with open(sessions[-1], encoding="utf-8") as f:
            content = f.read()
        # 提取遗留项
        in_legacy = False
        for line in content.split("\n"):
            if "遗留" in line or "中断" in line:
                in_legacy = True
                continue
            if in_legacy and line.startswith("|") and "P" in line:
                parts = [c.strip() for c in line.split("|") if c.strip()]
                if len(parts) >= 3:
                    result["workstations"].append({
                        "priority": parts[0],
                        "name": parts[1],
                        "status": parts[2] if len(parts) > 2 else "pending"
                    })
            elif in_legacy and line.startswith("#"):
                in_legacy = False

    # --- definitions.yaml ---
    defs_path = os.path.join(root, "definitions.yaml")
    if os.path.isfile(defs_path):
        with open(defs_path, encoding="utf-8") as f:
            defs = yaml.safe_load(f)
        if isinstance(defs, dict):
            entities = defs.get("entities", defs.get("definitions", []))
            result["definitions"] = len(entities) if isinstance(entities, list) else 0

    # --- pending 谱系 ---
    pending = glob.glob(os.path.join(root, ".chanlun/genealogy/pending/*.md"))
    result["pending"] = len(pending)
    for p in pending:
        result["workstations"].append({
            "priority": "P0",
            "name": f"pending:{os.path.basename(p)}",
            "status": "pending"
        })

    # --- settled 计数 ---
    settled = glob.glob(os.path.join(root, ".chanlun/genealogy/settled/*.md"))
    result["settled"] = len(settled)

    # --- HEAD ---
    try:
        import subprocess
        head = subprocess.check_output(["git", "rev-parse", "--short", "HEAD"],
                                       cwd=root, text=True).strip()
        result["head"] = head
    except Exception:
        pass

    # --- dispatch-dag mandatory structural nodes ---
    dag_path = os.path.join(root, ".chanlun/dispatch-dag.yaml")
    if os.path.isfile(dag_path):
        with open(dag_path, encoding="utf-8") as f:
            dag = yaml.safe_load(f)
        structural = dag.get("nodes", {}).get("structural", [])
        for node in structural:
            if node.get("mandatory", False):
                result["structural_nodes"].append({
                    "id": node["id"],
                    "agent": node.get("agent", f".claude/agents/{node['id']}.md")
                })

    # --- 过滤已完成的工位 ---
    result["workstations"] = [w for w in result["workstations"] if w.get("status") != "已修复" and "✅" not in w.get("status", "")]

    print(json.dumps(result, ensure_ascii=False, indent=2))

if __name__ == "__main__":
    main()
