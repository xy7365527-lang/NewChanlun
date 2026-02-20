#!/usr/bin/env bash
# spec-write-guard.sh — 核心配置文件写入守卫（061号 A3+A4）
# 拦截对 dispatch-spec.yaml 和 CLAUDE.md 的修改
# 触发：PreToolUse(Write), PreToolUse(Edit)

set -euo pipefail

INPUT=$(cat)

# ─── 提取 tool_name ───
TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

if [ "$TOOL_NAME" != "Write" ] && [ "$TOOL_NAME" != "Edit" ]; then
  exit 0
fi

# ─── 提取 file_path ───
FILE_PATH=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_input', {}).get('file_path', ''))
" 2>/dev/null || echo "")

if [ -z "$FILE_PATH" ]; then
  exit 0
fi

# ─── 检查是否是核心配置文件 ───
IS_CORE=false
FILE_TYPE=""

if echo "$FILE_PATH" | grep -qE '(^|/)dispatch-spec\.yaml$'; then
  IS_CORE=true
  FILE_TYPE="dispatch-spec.yaml"
elif echo "$FILE_PATH" | grep -qE '(^|/)CLAUDE\.md$'; then
  IS_CORE=true
  FILE_TYPE="CLAUDE.md"
fi

if [ "$IS_CORE" = false ]; then
  exit 0
fi

# ─── 输出警告（不阻断，确保可见）───
python -c "
import json
print(json.dumps({
    'decision': 'allow',
    'reason': '[spec-write-guard] 核心配置文件修改检测：$FILE_TYPE\n'
              '  此文件属于元编排「必须阻断等待」类别（058号递归节点默认行为）\n'
              '  修改路径：/ritual 仪式门控'
}, ensure_ascii=False))
"
exit 0
