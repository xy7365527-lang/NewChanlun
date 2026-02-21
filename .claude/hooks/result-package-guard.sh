#!/usr/bin/env bash
# PostToolUse Hook — 结果包六要素检查（result-package-guard）
#
# 触发：PostToolUse on Write/Edit tool
# 逻辑：写入 .chanlun/genealogy/**/*.md 后，检查最低三要素（除"结论"外）：
#   - 边界条件 / boundary
#   - 下游推论 / downstream
#   - 影响声明 / impact
# 缺失 → allow + 缺失字段警告（validate但不阻止，原则0）
# 全部存在 → 静默放行

set -uo pipefail

INPUT=$(cat)

TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

if [ "$TOOL_NAME" != "Write" ] && [ "$TOOL_NAME" != "Edit" ]; then
    exit 0
fi

FILE_PATH=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_input', {}).get('file_path', ''))
" 2>/dev/null || echo "")

CWD=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('cwd', '.'))
" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || true

REL_PATH=$(python -c "
import sys, os
try:
    rel = os.path.relpath(sys.argv[1], sys.argv[2])
except ValueError:
    rel = sys.argv[1]
print(rel.replace(os.sep, '/'))
" "$FILE_PATH" "$CWD" 2>/dev/null || echo "$FILE_PATH")

MATCH=$(python -c "
import re, sys
path = sys.argv[1]
if re.match(r'^\.chanlun/genealogy/(pending|settled)/[^/]+\.md$', path):
    print('yes')
else:
    print('no')
" "$REL_PATH" 2>/dev/null || echo "no")

if [ "$MATCH" != "yes" ]; then
    exit 0
fi

# PostToolUse: 文件已写入磁盘，直接读取检查
python -c "
import sys, json, re

file_path = sys.argv[1]
try:
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
except:
    sys.exit(0)

required = {
    '边界条件': r'(?:##\s*边界条件|\*\*边界条件\*\*|boundary)',
    '下游推论': r'(?:##\s*下游推论|\*\*下游推论\*\*|downstream)',
    '影响声明': r'(?:##\s*影响声明|\*\*影响声明\*\*|impact)',
}

missing = [name for name, pat in required.items() if not re.search(pat, content, re.IGNORECASE)]

if not missing:
    sys.exit(0)

print(json.dumps({
    'decision': 'allow',
    'reason': '[result-package-guard] ⚠ 谱系文件缺少结果包字段: ' + ', '.join(missing)
        + '。建议补全：边界条件、下游推论、影响声明（## 标题或 **粗体** 格式）。'
}, ensure_ascii=False))
" "$FILE_PATH"
