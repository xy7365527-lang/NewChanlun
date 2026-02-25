#!/usr/bin/env python3
"""清理僵尸任务：将无对应活跃 agent 的非完成任务标记为 completed。

200号下游推论-1：Plan 阶段创建的 TaskCreate 任务在实现完成后
不会自动标记为 completed，导致 Stop-Guard 误判。

用法:
    python cleanup_zombie_tasks.py --session v52-swarm
    python cleanup_zombie_tasks.py                      # 自动检测最新 session
"""

import argparse
import json
import sys
from pathlib import Path

CLAUDE_DIR = Path.home() / ".claude"
TASKS_DIR = CLAUDE_DIR / "tasks"
TEAMS_DIR = CLAUDE_DIR / "teams"


def detect_latest_session() -> str | None:
    """按修改时间返回最新的 tasks session 目录名。"""
    if not TASKS_DIR.exists():
        return None
    dirs = [d for d in TASKS_DIR.iterdir() if d.is_dir()]
    if not dirs:
        return None
    return max(dirs, key=lambda d: d.stat().st_mtime).name


def load_team_members(session: str) -> set[str]:
    """从 teams config 中提取成员名（排除 team-lead）。"""
    config_path = TEAMS_DIR / session / "config.json"
    if not config_path.exists():
        return set()
    data = json.loads(config_path.read_text(encoding="utf-8"))
    return {
        m["name"]
        for m in data.get("members", [])
        if m.get("name") != "team-lead"
    }


def load_tasks(session: str) -> list[tuple[Path, dict]]:
    """加载 session 下所有任务文件。"""
    task_dir = TASKS_DIR / session
    if not task_dir.exists():
        return []
    results = []
    for f in sorted(task_dir.glob("*.json")):
        try:
            results.append((f, json.loads(f.read_text(encoding="utf-8"))))
        except (json.JSONDecodeError, OSError):
            continue
    return results


def is_zombie(task: dict, active_members: set[str]) -> bool:
    """任务未完成且无对应活跃 agent → 僵尸。"""
    if task.get("status") == "completed":
        return False
    # 匹配：task.owner 或 task.subject 在 active_members 中
    owner = task.get("owner", "")
    subject = task.get("subject", "")
    return owner not in active_members and subject not in active_members


def main():
    parser = argparse.ArgumentParser(description="清理僵尸任务")
    parser.add_argument("--session", default=None, help="session 名称，默认自动检测")
    parser.add_argument("--dry-run", action="store_true", help="仅报告，不修改")
    args = parser.parse_args()

    session = args.session or detect_latest_session()
    if not session:
        json.dump({"cleaned": 0, "tasks": [], "error": "no session found"}, sys.stdout)
        return

    members = load_team_members(session)
    tasks = load_tasks(session)
    cleaned = []

    for path, task in tasks:
        if not is_zombie(task, members):
            continue
        entry = {"id": task.get("id"), "subject": task.get("subject"), "status": task.get("status")}
        if not args.dry_run:
            task["status"] = "completed"
            path.write_text(json.dumps(task, indent=2, ensure_ascii=False), encoding="utf-8")
        cleaned.append(entry)

    json.dump({"cleaned": len(cleaned), "tasks": cleaned}, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
