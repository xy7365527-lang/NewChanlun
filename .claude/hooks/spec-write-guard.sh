#!/usr/bin/env bash
# spec-write-guard.sh — 核心配置文件写入验证（validate + allow）
# 触发：PreToolUse(Write), PreToolUse(Edit)

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
if [ -z "$PYTHON_BIN" ]; then
  exit 0
fi

INPUT_JSON="$INPUT" "$PYTHON_BIN" - <<'PY'
import json
import os
import re
import sys

try:
    data = json.loads(os.environ.get("INPUT_JSON", "{}"))
except Exception:
    sys.exit(0)

tool_name = data.get("tool_name", "")
if tool_name not in ("Write", "Edit"):
    sys.exit(0)

file_path = data.get("tool_input", {}).get("file_path", "")
cwd = data.get("cwd", ".")
if not file_path:
    sys.exit(0)

try:
    rel_path = os.path.relpath(file_path, cwd).replace(os.sep, "/")
except Exception:
    rel_path = str(file_path).replace("\\", "/")

core_patterns = [
    (r"(^|/)CLAUDE\.md$", "CLAUDE.md"),
    (r"(^|/)\.chanlun/dispatch-dag\.yaml$", "dispatch-dag.yaml"),
    (r"(^|/)\.chanlun/genealogy/settled/.*\.md$", "已结算谱系"),
    (r"(^|/)\.chanlun/definitions/.*\.md$", "定义文件"),
    (r"(^|/)\.claude/agents/.*\.md$", "agent定义"),
    (r"(^|/)\.claude/hooks/.*\.sh$", "hook脚本"),
    (r"(^|/)\.claude/commands/.*\.md$", "命令定义"),
    (r"(^|/)scripts/ceremony_scan\.py$", "ceremony脚本"),
]

for pattern, label in core_patterns:
    if not re.search(pattern, rel_path):
        continue

    full_path = file_path if os.path.isabs(file_path) else os.path.join(cwd, rel_path.replace("/", os.sep))
    if not os.path.exists(full_path):
        break

    reason = (
        f"[spec-write-guard] 核心文件修改: {rel_path} ({label})。"
        "异步自指：t时刻修改，t+1时刻生效。git追踪，ESC可中断。"
    )
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow",
                    "permissionDecisionReason": reason,
                }
            },
            ensure_ascii=False,
        )
    )
    break
PY

exit 0
