#!/usr/bin/env bash
# flow-continuity-guard.sh — PostToolUse hook for Bash (git commit)
# 触发：Bash 执行成功的 git commit 后，注入继续执行提示。

set -euo pipefail

INPUT=$(cat)

resolve_python() {
  local bin
  for bin in python3 python; do
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

if data.get("tool_name", "") != "Bash":
    sys.exit(0)

command = data.get("tool_input", {}).get("command", "") or ""
exit_code = data.get("tool_result", {}).get("exit_code", 1)

if "git commit" not in command:
    sys.exit(0)

if str(exit_code) != "0":
    sys.exit(0)

if "[FINAL]" in command.upper():
    sys.exit(0)

reason = (
    "[028号谱系 · 运行时强制] Commit 成功。不允许停顿或输出总结段落。你必须立即：\n"
    "1. 输出 '→ 接下来：[具体动作]'\n"
    "2. 紧跟 tool 调用执行该动作\n\n"
    "如果确实无事可做，执行扫描（TODO/覆盖率/spec合规/谱系张力）。\n"
    "如果所有工作已完成，输出 '→ 接下来：写 session 记录' 并执行。"
)

print(json.dumps({"decision": "block", "reason": reason}, ensure_ascii=False))
PY

exit 0
