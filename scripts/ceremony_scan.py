#!/usr/bin/env python
"""蜂群 spawn 通用工具。任何节点（ceremony 或 teammate）spawn 蜂群时调用。

用法:
  python scripts/ceremony_scan.py                  # 根 ceremony：全量扫描
  python scripts/ceremony_scan.py --skills         # 只输出 required_skills
  python scripts/ceremony_scan.py --workstations "任务A" "任务B"  # 指定业务工位

075号更新：structural_nodes → required_skills（事件驱动 skill 架构）
079号更新：background_noise 降级 + 业务层任务发现（no_work_fallback）
081号更新：roadmap.yaml 扫描（最高优先级任务来源）+ 终止逻辑修正
"""
import json, os, glob, yaml, sys, argparse, subprocess


BACKGROUND_NOISE_STATUSES = {"background_noise", "观察项", "背景噪音"}
TERMINAL_STATUSES = {"已修复", "resolved", "background_noise"}


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


def get_roadmap_workstations(root):
    """从 .chanlun/roadmap.yaml 读取待执行的业务目标（最高优先级任务来源）。

    081号谱系：roadmap 是结构化业务目标载体，ceremony_scan 的最高优先级来源。
    只返回 status=active 的任务。
    """
    roadmap_path = os.path.join(root, ".chanlun/roadmap.yaml")
    tasks = []
    if not os.path.isfile(roadmap_path):
        return tasks
    try:
        with open(roadmap_path, encoding="utf-8") as f:
            data = yaml.safe_load(f)
        for task in data.get("tasks", []):
            if task.get("status") == "active":
                tasks.append({
                    "priority": task.get("priority", "P2"),
                    "name": task.get("id", task.get("title", "未命名")),
                    "status": "roadmap:active",
                    "source": "roadmap",
                    "description": task.get("description", ""),
                    "title": task.get("title", ""),
                })
    except Exception as exc:
        # roadmap 格式错误时不阻塞扫描，但记录错误
        tasks.append({
            "priority": "P0",
            "name": "roadmap.yaml 解析错误",
            "status": f"error: {exc}",
            "source": "roadmap_error",
        })
    return tasks


def get_session_workstations(root):
    """从最新 session 提取遗留工位，过滤 background_noise。"""
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
                if len(parts) >= 3:
                    status = parts[2]
                    # 079号：过滤 background_noise 和已终结状态
                    if status in TERMINAL_STATUSES or "✅" in status or "—" in parts[0]:
                        continue
                    # 过滤含 background_noise 关键词的状态
                    if any(kw in status.lower() for kw in BACKGROUND_NOISE_STATUSES):
                        continue
                    workstations.append({
                        "priority": parts[0],
                        "name": parts[1],
                        "status": status,
                    })
            elif in_legacy and line.startswith("#"):
                in_legacy = False
    # pending 谱系
    for p in glob.glob(os.path.join(root, ".chanlun/genealogy/pending/*.md")):
        workstations.append({
            "priority": "P0",
            "name": f"pending:{os.path.basename(p)}",
            "status": "pending",
        })
    return session_name, workstations


def discover_business_tasks(root):
    """no_work_fallback：扫描测试失败、spec 合规等业务层任务。

    对应 dispatch-dag ceremony_sequence.no_work_fallback：
    "扫描 TODO/覆盖率/spec合规/谱系张力，产出至少一个工位"
    """
    tasks = []

    # 1. 测试失败扫描
    try:
        result = subprocess.run(
            [sys.executable, "-m", "pytest",
             "--ignore=tests/test_cli_gateway_plot.py",
             "--ignore=tests/test_data_databento.py",
             "--ignore=tests/test_mcp_bridge.py",
             "--tb=no", "-q"],
            cwd=root, capture_output=True, text=True, timeout=120,
        )
        output = result.stdout + result.stderr
        # 解析 "N failed" 行
        for line in output.split("\n"):
            if "failed" in line and ("passed" in line or "error" in line):
                parts = line.split(",")
                for part in parts:
                    part = part.strip()
                    if "failed" in part:
                        count = part.split()[0]
                        tasks.append({
                            "priority": "P1",
                            "name": f"修复 {count} 个测试失败",
                            "status": "pytest 发现",
                            "source": "test_failures",
                        })
                        break
    except Exception:
        pass

    # 2. 测试覆盖率（如果 pytest-cov 可用）
    # 暂不实现，避免 scan 耗时过长

    return tasks


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
    # 扫描顺序（优先级递减）：
    # 1. roadmap.yaml（最高优先级，结构化业务目标）
    # 2. session 遗留项 + pending 谱系
    # 3. no_work_fallback（测试失败等）
    # 081号：只有 roadmap 为空 AND session 遗留为空 AND fallback 为空，才是真阴性干净终止

    session_name, workstations = get_session_workstations(root)
    result["mode"] = "warm_start" if session_name else "cold_start"
    result["session"] = session_name

    # 1. 最高优先级：roadmap.yaml 中的 active 任务
    roadmap_tasks = get_roadmap_workstations(root)
    if roadmap_tasks:
        # roadmap 任务插入到 workstations 最前（P2 优先级，高于 P3 long_term）
        workstations = roadmap_tasks + workstations
        result["roadmap_tasks_found"] = len(roadmap_tasks)

    # 2. 079号：如果 session 遗留工位为空或全部是背景噪音，执行 no_work_fallback
    # 081号修正：no_work_fallback 仅在 roadmap 也为空时才触发（roadmap 是主工作来源）
    if not workstations:
        workstations = discover_business_tasks(root)
        result["fallback_triggered"] = True

    result["workstations"] = workstations

    # 081号：清晰报告干净终止条件
    # 真阴性干净终止 = roadmap 为空 AND session 遗留为空 AND fallback 为空 AND pending 谱系为空
    pending_count = len(glob.glob(os.path.join(root, ".chanlun/genealogy/pending/*.md")))
    result["clean_terminate"] = (
        len(roadmap_tasks) == 0
        and len(workstations) == 0
        and pending_count == 0
    )

    # 定义/谱系计数
    defs_path = os.path.join(root, "definitions.yaml")
    if os.path.isfile(defs_path):
        with open(defs_path, encoding="utf-8") as f:
            defs = yaml.safe_load(f)
        if isinstance(defs, dict):
            entities = defs.get("entities", defs.get("definitions", []))
            result["definitions"] = len(entities) if isinstance(entities, list) else 0
    result["pending"] = pending_count
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
    except Exception as exc:
        # Keep scan resilient, but do not hide failures.
        result["downstream_actions_error"] = f"{type(exc).__name__}: {exc}"

    try:
        result["head"] = subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"], cwd=root, text=True).strip()
    except Exception:
        pass

    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
