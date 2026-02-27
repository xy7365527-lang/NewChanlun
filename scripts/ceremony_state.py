"""ceremony_state.py — ceremony 步骤状态文件管理。

提供 write_step / read_step / clear_step / is_in_ceremony 四个操作，
状态持久化到项目根目录 .ceremony-step（JSON）。
"""

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

# 状态文件路径：项目根目录下 .ceremony-step
_STATE_FILE = Path(__file__).resolve().parent.parent / ".ceremony-step"


def write_step(step: int, phase: str) -> None:
    """写入当前 ceremony 步骤。"""
    payload = {
        "step": step,
        "phase": phase,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }
    _STATE_FILE.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")


def read_step() -> dict | None:
    """读取当前步骤，文件不存在返回 None。"""
    if not _STATE_FILE.exists():
        return None
    try:
        return json.loads(_STATE_FILE.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return None


def clear_step() -> None:
    """清除状态文件。"""
    try:
        _STATE_FILE.unlink(missing_ok=True)
    except OSError:
        pass


def is_in_ceremony() -> bool:
    """状态文件存在 = 在 ceremony 中。"""
    return _STATE_FILE.exists()


# CLI 入口：供 hook 脚本调用
if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: ceremony_state.py <write|read|clear|check> [step] [phase]")
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == "write":
        if len(sys.argv) < 4:
            print("Usage: ceremony_state.py write <step> <phase>")
            sys.exit(1)
        write_step(int(sys.argv[2]), sys.argv[3])
    elif cmd == "read":
        data = read_step()
        if data is None:
            sys.exit(1)
        print(json.dumps(data, ensure_ascii=False))
    elif cmd == "clear":
        clear_step()
    elif cmd == "check":
        sys.exit(0 if is_in_ceremony() else 1)
    else:
        print(f"Unknown command: {cmd}")
        sys.exit(1)
