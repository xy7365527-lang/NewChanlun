"""ceremony_state.py — ceremony 步骤状态文件管理。

提供 write_step / read_step / clear_step / is_in_ceremony 四个操作，
状态持久化到项目根目录 .ceremony-step（JSON）。

270号更新：suspended 工位管理——suspend_workstation / unsuspend_workstation / get_suspended_workstations。
状态持久化到 .ceremony-suspended（JSON）。
"""

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

# 状态文件路径：项目根目录下 .ceremony-step
_STATE_FILE = Path(__file__).resolve().parent.parent / ".ceremony-step"

# suspended 工位文件路径：项目根目录下 .ceremony-suspended
_SUSPENDED_FILE = Path(__file__).resolve().parent.parent / ".ceremony-suspended"


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


def _read_suspended_data() -> dict:
    """读取 suspended 文件，返回 {name: {reason, suspended_at}} 字典。"""
    if not _SUSPENDED_FILE.exists():
        return {}
    try:
        data = json.loads(_SUSPENDED_FILE.read_text(encoding="utf-8"))
        if isinstance(data, dict):
            return data
        return {}
    except (json.JSONDecodeError, OSError):
        return {}


def _write_suspended_data(data: dict) -> None:
    """写入 suspended 数据到文件。"""
    _SUSPENDED_FILE.write_text(
        json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8",
    )


def suspend_workstation(name: str, reason: str) -> None:
    """将工位标记为 suspended。

    Args:
        name: 工位名称（与 ceremony_scan workstations 中的 name 字段匹配）
        reason: 挂起原因（如 "stance-repetition"）
    """
    data = _read_suspended_data()
    data[name] = {
        "reason": reason,
        "suspended_at": datetime.now(timezone.utc).isoformat(),
    }
    _write_suspended_data(data)


def unsuspend_workstation(name: str) -> bool:
    """恢复 suspended 工位。返回 True 表示成功移除，False 表示工位不在 suspended 列表中。"""
    data = _read_suspended_data()
    if name not in data:
        return False
    del data[name]
    _write_suspended_data(data)
    return True


def get_suspended_workstations() -> dict:
    """获取所有 suspended 工位。返回 {name: {reason, suspended_at}} 字典。"""
    return _read_suspended_data()


# CLI 入口：供 hook 脚本调用
if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: ceremony_state.py <write|read|clear|check|suspend|unsuspend|list-suspended> [args...]")
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
    elif cmd == "suspend":
        if len(sys.argv) < 4:
            print("Usage: ceremony_state.py suspend <name> <reason>")
            sys.exit(1)
        suspend_workstation(sys.argv[2], sys.argv[3])
    elif cmd == "unsuspend":
        if len(sys.argv) < 3:
            print("Usage: ceremony_state.py unsuspend <name>")
            sys.exit(1)
        removed = unsuspend_workstation(sys.argv[2])
        if not removed:
            print(f"工位 '{sys.argv[2]}' 不在 suspended 列表中")
            sys.exit(1)
    elif cmd == "list-suspended":
        suspended = get_suspended_workstations()
        print(json.dumps(suspended, ensure_ascii=False, indent=2))
    else:
        print(f"Unknown command: {cmd}")
        sys.exit(1)
