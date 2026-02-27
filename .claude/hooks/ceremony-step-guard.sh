#!/usr/bin/env bash
# ceremony-step-guard.sh — PostToolUse hook
# 检查 .ceremony-step 是否存在，如果存在则注入正面指令强制继续 ceremony 序列。
# P0 修复：ceremony 协议要求状态维护，但 LLM 无状态——通过 hook 注入弥补。

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

# 获取项目根目录
CWD=$("$PYTHON_BIN" -c "import json,sys,os; print(json.loads(os.environ.get('INPUT_JSON',sys.stdin.read())).get('cwd','.'))" <<< "$INPUT" 2>/dev/null || echo ".")
STATE_FILE="$CWD/.ceremony-step"

# 状态文件不存在 → 不在 ceremony 中，静默退出
[ -f "$STATE_FILE" ] || exit 0

# 读取步骤信息并注入指令
INPUT_JSON="$INPUT" "$PYTHON_BIN" - "$STATE_FILE" <<'PY'
import json
import sys

state_file = sys.argv[1]

try:
    with open(state_file, "r", encoding="utf-8") as f:
        state = json.load(f)
except Exception:
    sys.exit(0)

step = state.get("step", "?")
phase = state.get("phase", "unknown")

reason = (
    f"[ceremony-step-guard] 你正在 ceremony 步骤 {step}（{phase}）。"
    f"立即继续执行下一步。"
    f"不允许回应用户消息、输出总结、或执行白名单外的操作。"
    f"ceremony 序列不可被中断（224号谱系）。"
)

print(json.dumps({"decision": "block", "reason": reason}, ensure_ascii=False))
PY

exit 0
