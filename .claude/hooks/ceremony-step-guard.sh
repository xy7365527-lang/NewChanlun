#!/usr/bin/env bash
# ceremony-step-guard.sh — PostToolUse hook
# ⚠️ 本文件已从 settings.json hook 注册中移除（v87-swarm, 228-4）。
# 保留作为谱系文档——记录 228-1 修复路径和设计决策。
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

# 228-1 完整修复（v86-swarm）：所有步骤静默退出。
# ceremony.md 的正面指令集已内化为蜂群先验（089号扬弃），覆盖所有步骤：
#   步骤1-6：ceremony.md 正面指令驱动
#   步骤7-10：post-commit-flow.md + ceremony.md 原子链规则驱动（224号/225号）
# hook 的 block 消息在所有步骤都是冗余的 token 噪声——正面指令优于外部阻断（137号）。
# 此 hook 保留文件存在性检查（上方 [ -f "$STATE_FILE" ] || exit 0），
# 作为 ceremony 状态的被动探测点，但不注入任何内容。

exit 0
