#!/usr/bin/env python
"""蜂群 spawn 通用工具。任何节点（ceremony 或 teammate）spawn 蜂群时调用。

用法:
  python scripts/ceremony_scan.py                  # 根 ceremony：全量扫描
  python scripts/ceremony_scan.py --skills         # 只输出 required_skills
  python scripts/ceremony_scan.py --workstations "任务A" "任务B"  # 指定业务工位

075号更新：structural_nodes → required_skills（事件驱动 skill 架构）
"""
import json, os, glob, yaml, sys, argparse


def get_required_skills(root):
    """从 dispatch-dag 的 event_skill_map 读取 structural skill 列表。"""
    dag_path = os.path.join(root, ".chanlun/dispatch-dag.yaml")
    skills = []
    if os.path.isfile(dag_path):
        with open(dag_path, encoding="utf-8") as f:
            dag = yaml.safe_load(f)
        for skill in dag.get("event_skill_map", []):
            if skill.get("skill_type") == "structural":
                skills.append({
                    "id": skill["id"],
                    "agent": skill.get("agent", f".claude/agents/{skill['id']}.md"),
                    "triggers": [t.get("event", "") for t in skill.get("triggers", [])]
                })
    return skills


def get_session_workstations(root):
    """从最新 session 提取遗留工位。"""
    workstations = []
    sessions = glob.glob(os.path.join(root, ".chanlun/sessions/*-session.md"))
    session_name = None
    if sessions:
        sessions.sort(key=os.path.getmtime)
        session_name = os.path.basename(sessions[-1])
        with open(sessions[-1], encoding="utf-8") as f:
            content = f.read()
        in_legacy = False
        for line in content.split("\n"):
            if "遗留" in line or "中断" in line:
                in_legacy = True
                continue
            if in_legacy and line.startswith("|") and "P" in line:
                parts = [c.strip() for c in line.split("|") if c.strip()]
                if len(parts) >= 3 and parts[2] != "已修复" and "✅" not in parts[2]:
                    workstations.append({"priority": parts[0], "name": parts[1], "status": parts[2]})
            elif in_legacy and line.startswith("#"):
                in_legacy = False
    # pending 谱系
    for p in glob.glob(os.path.join(root, ".chanlun/genealogy/pending/*.md")):
        workstations.append({"priority": "P0", "name": f"pending:{os.path.basename(p)}", "status": "pending"})
    return session_name, workstations


def main():
    parser = argparse.ArgumentParser(description="蜂群 spawn 通用工具")
    parser.add_argument("--skills", action="store_true", help="只输出 required_skills")
    # 保留 --structural 作为 --skills 的别名（向后兼容）
    parser.add_argument("--structural", action="store_true", help="(deprecated) 等同于 --skills")
    parser.add_argument("--workstations", nargs="*", help="指定业务工位名称")
    args = parser.parse_args()

    root = os.getcwd()
    result = {"required_skills": get_required_skills(root)}

    if args.skills or args.structural:
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return

    if args.workstations:
        result["workstations"] = [{"name": w, "priority": "P1", "status": "pending"} for w in args.workstations]
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return

    # 根 ceremony 模式：全量扫描
    session_name, workstations = get_session_workstations(root)
    result["mode"] = "warm_start" if session_name else "cold_start"
    result["session"] = session_name
    result["workstations"] = workstations

    # 定义/谱系计数
    defs_path = os.path.join(root, "definitions.yaml")
    if os.path.isfile(defs_path):
        with open(defs_path, encoding="utf-8") as f:
            defs = yaml.safe_load(f)
        if isinstance(defs, dict):
            entities = defs.get("entities", defs.get("definitions", []))
            result["definitions"] = len(entities) if isinstance(entities, list) else 0
    result["pending"] = len(glob.glob(os.path.join(root, ".chanlun/genealogy/pending/*.md")))
    result["settled"] = len(glob.glob(os.path.join(root, ".chanlun/genealogy/settled/*.md")))

    # 二阶反馈：下游推论执行审计
    try:
        from downstream_audit import audit as downstream_audit
        da = downstream_audit(root)
        if da["total_actions"] > 0:
            result["downstream_actions"] = {
                "total": da["total_actions"],
                "unresolved": da["unresolved"],
                "execution_rate": da["execution_rate"],
            }
    except Exception:
        pass

    try:
        import subprocess
        result["head"] = subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"], cwd=root, text=True).strip()
    except Exception:
        pass

    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
