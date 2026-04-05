#!/usr/bin/env bash
# completion-session-guard.sh — PostToolUse hook for SendMessage
#
# Codex 持存诊断·运行时缓解
# 触发：每次 SendMessage(type=shutdown_request) 后
# 检测：本轮是否有 session 增量写入（通过 .last-session-append 标记）
# 效果：注入 systemMessage 提醒 Lead 增量写 session
#
# 假设覆盖：
# - 假设1（RLHF偏置）：运行时提醒对抗"先收集再输出"
# - 假设3（Write调用成本）：提示使用 session_append.sh（一行调用）
# - 假设5（指令不够机械化）：机械触发替代语言描述

set -euo pipefail

INPUT=$(timeout 3 cat 2>/dev/null || echo "{}")

resolve_python() {
  local bin
  for bin in python python3; do
    if command -v "$bin" >/dev/null 2>&1 && "$bin" -c "import sys" >/dev/null 2>&1; then
      echo "$bin"
      return 0
    fi
  done
  return 1
}

PYTHON_BIN="$(resolve_python || true)"
[ -n "$PYTHON_BIN" ] || exit 0

INPUT_JSON="$INPUT" "$PYTHON_BIN" - <<'PY'
import json
import os
import sys

try:
    data = json.loads(os.environ.get("INPUT_JSON", "{}"))
except Exception:
    sys.exit(0)

if data.get("tool_name", "") != "SendMessage":
    sys.exit(0)

tool_input = data.get("tool_input", {})
msg_type = tool_input.get("type", "")

# 只在 shutdown_request 时触发检查
if msg_type != "shutdown_request":
    sys.exit(0)

# 检查 session 追加标记
from pathlib import Path

# 定位项目根目录
script_dir = Path(__file__).resolve().parent if "__file__" in dir() else Path(".")
# hook 运行在项目根
cwd = Path(data.get("cwd", ".")).resolve()
marker_file = cwd / ".chanlun" / ".last-session-append"

if marker_file.exists():
    import time
    try:
        last_ts = float(marker_file.read_text(encoding="utf-8").strip())
        elapsed = time.time() - last_ts
        if elapsed < 120:
            # 近期有追加，静默通过
            sys.exit(0)
    except (ValueError, OSError):
        pass

# 未找到近期追加记录 → 注入提醒
recipient = tool_input.get("recipient", "?")
warning = (
    f"[增量持存·运行时强制] 你刚对 {recipient} 发送了 shutdown_request，"
    "但本轮可能尚未增量写入 session。"
    "请在发送 shutdown_request 前或后立即执行：\n"
    f"bash scripts/session_append.sh \"{recipient}: [产出摘要]\"\n\n"
    "增量持存不变量（ceremony.md 步骤6）：每条 completion 到达时写 session，不等 consume_all。"
)

print(json.dumps({"decision": "block", "reason": warning}, ensure_ascii=False))
PY

exit 0
