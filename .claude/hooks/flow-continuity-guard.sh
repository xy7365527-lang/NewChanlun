#!/usr/bin/env bash
# flow-continuity-guard.sh — PostToolUse hook for Bash (git commit)
# 028号谱系 + post-commit-flow 的运行时强制层（016号）
#
# 触发条件：Bash 工具调用完成后
# 检查：是否是 git commit 且成功
# 动作：注入 systemMessage 强制 Lead 输出下一步行动
#
# Gemini 编排者代理决策：方案A（PostToolUse hook 注入）
# 边界条件：commit message 含 [FINAL] 时允许停顿

set -euo pipefail

# 读取 tool input
INPUT=$(cat)

# 只处理 Bash 工具
TOOL_NAME=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('tool_name',''))" 2>/dev/null || echo "")
if [ "$TOOL_NAME" != "Bash" ]; then
  exit 0
fi

# 获取命令内容和退出码
COMMAND=$(echo "$INPUT" | python -c "import sys,json; d=json.load(sys.stdin); print(d.get('tool_input',{}).get('command',''))" 2>/dev/null || echo "")
EXIT_CODE=$(echo "$INPUT" | python -c "import sys,json; print(json.load(sys.stdin).get('tool_result',{}).get('exit_code',1))" 2>/dev/null || echo "1")

# 只处理 git commit 成功的情况
if ! echo "$COMMAND" | grep -q "git commit"; then
  exit 0
fi

if [ "$EXIT_CODE" != "0" ]; then
  exit 0
fi

# 边界条件：[FINAL] 标记允许停顿
if echo "$COMMAND" | grep -qi "\[FINAL\]"; then
  exit 0
fi

# 注入 systemMessage 强制继续
cat <<'HOOK_OUTPUT'
{
  "decision": "block",
  "reason": "[028号谱系 · 运行时强制] Commit 成功。不允许停顿或输出总结段落。你必须立即：\n1. 输出 '→ 接下来：[具体动作]'\n2. 紧跟 tool 调用执行该动作\n\n如果确实无事可做，执行扫描（TODO/覆盖率/spec合规/谱系张力）。\n如果所有工作已完成，输出 '→ 接下来：写 session 记录' 并执行。"
}
HOOK_OUTPUT
