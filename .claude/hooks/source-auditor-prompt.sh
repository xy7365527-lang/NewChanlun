#!/bin/bash
# PostToolUse Hook — source-auditor 提示
# 087号谱系修复（A选项）：实现 docs/** file_write 事件检测
# 消除 dispatch-dag.yaml 中 source-auditor 的声明-能力缺口
#
# 触发：PostToolUse on "Write" | "Edit" tool
# 逻辑：检测写入路径是否匹配 docs/**，若匹配则输出提示（D策略：hooks 提示 + Lead 认领）
# 不阻断，只提示

set -uo pipefail

INPUT=$(cat)

TOOL_NAME=$(python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" << 'EOF'
PLACEHOLDER
EOF
)
TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

if [ "$TOOL_NAME" != "Write" ] && [ "$TOOL_NAME" != "Edit" ]; then
    exit 0
fi

FILE_PATH=$(echo "$INPUT" | python -c "
import sys, json, os
data = json.loads(sys.stdin.read())
fp = data.get('tool_input', {}).get('file_path', '')
cwd = data.get('cwd', '.')
if not fp:
    print('')
else:
    try:
        rel = os.path.relpath(fp, cwd)
    except ValueError:
        rel = fp
    print(rel.replace(os.sep, '/'))
" 2>/dev/null || echo "")

# 检查是否匹配 docs/**
if ! echo "$FILE_PATH" | grep -qE '^docs/'; then
    exit 0
fi

# 匹配：输出提示（advisory，不阻断）
python -c "
import json
fp = '$FILE_PATH'
print(json.dumps({
    'continue': True,
    'systemMessage': f'[source-auditor advisory] 检测到 docs/ 变更（{fp}）。Lead 可选择：读取 .claude/agents/source-auditor.md 对本次变更执行溯源标签验证（三级权威链合规性检查）。'
}, ensure_ascii=False))
" 2>/dev/null || true

exit 0
