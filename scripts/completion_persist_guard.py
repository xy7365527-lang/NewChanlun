#!/usr/bin/env python3
"""completion_persist_guard.py — 检测 completion→session 写入耦合

Codex 持存诊断缓解措施：
- 假设1（RLHF偏置）：通过 hook 运行时提醒，对抗"先收集再输出"
- 假设5（指令不够机械化）：将"每条 completion 到达时增量写 session"
  转化为机械检测——如果 shutdown_request 发送但近期无 session 追加，注入提醒

用法（由 hook 调用）：
    python scripts/completion_persist_guard.py check_append <session_file>
    → exit 0 + JSON stdout 表示 session 近期有追加
    → exit 0 + JSON stdout 带 warning 表示缺少追加

    python scripts/completion_persist_guard.py mark_append <session_file>
    → 写时间戳标记到 .chanlun/.last-session-append

设计约束：
- 纯函数，不依赖全局状态
- 时间窗口 = 120 秒（两分钟内无追加 = 可能批量写）
- 零 LLM 交互：hook 自动执行
"""

import json
import os
import sys
import time
from pathlib import Path

MARKER_FILE = ".chanlun/.last-session-append"
STALE_THRESHOLD_SECONDS = 120


def get_root() -> Path:
    return Path(__file__).resolve().parent.parent


def mark_append(root: Path) -> dict:
    """标记 session 追加时间戳。"""
    marker = root / MARKER_FILE
    marker.parent.mkdir(parents=True, exist_ok=True)
    marker.write_text(str(time.time()), encoding="utf-8")
    return {"action": "mark_append", "timestamp": time.time()}


def check_append(root: Path, session_file: str) -> dict:
    """检查最近是否有 session 追加。

    Returns:
        dict with keys:
        - has_recent_append: bool
        - seconds_since_last: float or None
        - warning: str or None (如果需要提醒)
    """
    marker = root / MARKER_FILE
    if not marker.exists():
        return {
            "has_recent_append": False,
            "seconds_since_last": None,
            "warning": (
                "[增量持存·假设1/5缓解] 本 ceremony 中尚无 session 增量写入记录。"
                "请在处理每个工位 completion 后立即调用: "
                "bash scripts/session_append.sh \"工位名: 产出摘要\""
            ),
        }

    try:
        last_ts = float(marker.read_text(encoding="utf-8").strip())
    except (ValueError, OSError):
        return {
            "has_recent_append": False,
            "seconds_since_last": None,
            "warning": "[增量持存] .last-session-append 标记文件损坏，请重新写入。",
        }

    elapsed = time.time() - last_ts
    if elapsed > STALE_THRESHOLD_SECONDS:
        return {
            "has_recent_append": False,
            "seconds_since_last": elapsed,
            "warning": (
                f"[增量持存·假设1缓解] 距上次 session 追加已 {elapsed:.0f} 秒"
                f"（阈值 {STALE_THRESHOLD_SECONDS}s）。"
                "你可能在批量处理多个 completion 而未增量写入。"
                "请立即调用: bash scripts/session_append.sh \"工位名: 产出摘要\""
            ),
        }

    return {
        "has_recent_append": True,
        "seconds_since_last": elapsed,
        "warning": None,
    }


def clear_marker(root: Path) -> dict:
    """清除追加标记（ceremony 结束时调用）。"""
    marker = root / MARKER_FILE
    if marker.exists():
        marker.unlink()
    return {"action": "clear_marker"}


def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "用法: completion_persist_guard.py <check_append|mark_append|clear> [session_file]"}))
        sys.exit(1)

    action = sys.argv[1]
    root = get_root()

    if action == "mark_append":
        result = mark_append(root)
    elif action == "check_append":
        session_file = sys.argv[2] if len(sys.argv) > 2 else ""
        result = check_append(root, session_file)
    elif action == "clear":
        result = clear_marker(root)
    else:
        result = {"error": f"未知动作: {action}"}

    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
