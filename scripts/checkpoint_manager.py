"""checkpoint_manager.py — 工位级 checkpoint 读写（283号缺口A）。

提供 write_checkpoint / read_checkpoint / clear_checkpoints / list_checkpoints 四个操作，
状态持久化到 .chanlun/checkpoints/{team_name}/{agent_name}.json。

原子写入：写到 .tmp 后缀文件 → os.replace() 到目标路径。
"""

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent
_CHECKPOINTS_DIR = _PROJECT_ROOT / ".chanlun" / "checkpoints"


def _atomic_write(path: Path, payload: dict) -> None:
    """原子写入 JSON：tmp→rename。"""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = path.with_suffix(".json.tmp")
    tmp_path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    os.replace(str(tmp_path), str(path))


def write_checkpoint(
    team_name: str,
    agent_name: str,
    phase: str,
    done: list[str],
    pending: list[str],
    next_action: str,
    artifacts: list[dict] | None = None,
) -> None:
    """写入工位 checkpoint。

    路径：.chanlun/checkpoints/{team_name}/{agent_name}.json
    """
    now = datetime.now(timezone.utc)
    op_id = f"{agent_name}-run-{now.strftime('%Y%m%d-%H%M%S')}"
    payload = {
        "agent_name": agent_name,
        "team_name": team_name,
        "phase": phase,
        "done": list(done),
        "pending": list(pending),
        "next_action": next_action,
        "artifacts": list(artifacts) if artifacts else [],
        "op_id": op_id,
        "updated_at": now.isoformat(),
    }
    target = _CHECKPOINTS_DIR / team_name / f"{agent_name}.json"
    _atomic_write(target, payload)


def read_checkpoint(team_name: str, agent_name: str) -> dict | None:
    """读取 checkpoint。文件不存在或损坏返回 None。"""
    target = _CHECKPOINTS_DIR / team_name / f"{agent_name}.json"
    if not target.exists():
        return None
    try:
        return json.loads(target.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None


def clear_checkpoints(team_name: str) -> None:
    """清除整个 team 的 checkpoints（ceremony 终止时调用）。"""
    team_dir = _CHECKPOINTS_DIR / team_name
    if not team_dir.exists():
        return
    for f in team_dir.iterdir():
        if f.is_file():
            f.unlink(missing_ok=True)
    # 尝试移除空目录
    try:
        team_dir.rmdir()
    except OSError:
        pass


def list_checkpoints(team_name: str) -> list[dict]:
    """列出 team 下所有 checkpoints 的摘要。"""
    team_dir = _CHECKPOINTS_DIR / team_name
    if not team_dir.is_dir():
        return []
    result = []
    for f in sorted(team_dir.iterdir()):
        if f.suffix == ".json" and not f.name.endswith(".tmp"):
            try:
                data = json.loads(f.read_text(encoding="utf-8"))
                result.append(data)
            except (json.JSONDecodeError, OSError):
                pass
    return result


# CLI 入口
if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: checkpoint_manager.py <write|read|clear|list> [args...]")
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == "write":
        if len(sys.argv) < 8:
            print("Usage: checkpoint_manager.py write <team> <agent> <phase> <done_json> <pending_json> <next_action> [artifacts_json]")
            sys.exit(1)
        artifacts = json.loads(sys.argv[8]) if len(sys.argv) > 8 else None
        write_checkpoint(
            team_name=sys.argv[2],
            agent_name=sys.argv[3],
            phase=sys.argv[4],
            done=json.loads(sys.argv[5]),
            pending=json.loads(sys.argv[6]),
            next_action=sys.argv[7],
            artifacts=artifacts,
        )
    elif cmd == "read":
        if len(sys.argv) < 4:
            print("Usage: checkpoint_manager.py read <team> <agent>")
            sys.exit(1)
        data = read_checkpoint(sys.argv[2], sys.argv[3])
        if data is None:
            sys.exit(1)
        print(json.dumps(data, ensure_ascii=False, indent=2))
    elif cmd == "clear":
        if len(sys.argv) < 3:
            print("Usage: checkpoint_manager.py clear <team>")
            sys.exit(1)
        clear_checkpoints(sys.argv[2])
    elif cmd == "list":
        if len(sys.argv) < 3:
            print("Usage: checkpoint_manager.py list <team>")
            sys.exit(1)
        checkpoints = list_checkpoints(sys.argv[2])
        print(json.dumps(checkpoints, ensure_ascii=False, indent=2))
    else:
        print(f"Unknown command: {cmd}")
        sys.exit(1)
