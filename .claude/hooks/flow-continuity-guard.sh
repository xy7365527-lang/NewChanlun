#!/usr/bin/env bash
# flow-continuity-guard.sh — PostToolUse hook for Bash (git commit / git push / ceremony_scan.py)
# 触发：Bash 执行成功的 git commit、git push、ceremony_scan.py 后，注入继续执行提示。
# 224号谱系：push→rescan 原子性断裂修复——扩展覆盖 git push。
# 225号谱系：rescan→evaluate 原子性断裂修复——扩展覆盖 ceremony_scan.py。

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

if data.get("tool_name", "") != "Bash":
    sys.exit(0)

command = data.get("tool_input", {}).get("command", "") or ""
exit_code = data.get("tool_result", {}).get("exit_code", 1)

if str(exit_code) != "0":
    sys.exit(0)

if "[FINAL]" in command.upper():
    sys.exit(0)

is_commit = "git commit" in command
is_push = "git push" in command
is_ceremony_scan = "ceremony_scan.py" in command

if not is_commit and not is_push and not is_ceremony_scan:
    sys.exit(0)

if is_ceremony_scan:
    reason = (
        "[225号谱系 · 运行时强制] ceremony_scan.py 执行完成。立即处理扫描结果：\n"
        "1. 解析 JSON 输出\n"
        "2. 如果 workstations 非空且与上轮不同 → 回到步骤5 spawn 新工位\n"
        "3. 如果 workstations 为空或与上轮相同（不动点）→ TeamDelete → 停止\n\n"
        "rescan→evaluate 是原子操作，中间不允许插入任何其他操作（包括回应编排者消息）。\n"
        "ceremony 序列不可被用户消息中断（224号）。用户消息在 ceremony 到达终止条件后统一回应。"
    )
elif is_push:
    reason = (
        "[224号谱系 · 运行时强制] Push 成功。立即执行 rescan：\n"
        "python scripts/ceremony_scan.py --phase rescan\n\n"
        "push→rescan 是原子操作，中间不允许插入任何其他操作（包括回应编排者消息）。\n"
        "如果不在 ceremony 序列中，执行扫描（TODO/覆盖率/spec合规/谱系张力）。"
    )
elif is_commit:
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
