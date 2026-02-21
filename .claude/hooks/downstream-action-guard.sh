#!/usr/bin/env bash
# PostToolUse Hook — 二阶反馈：下游推论追踪守卫
#
# 触发：PostToolUse on Write/Edit
# 逻辑：写入 .chanlun/genealogy/settled/*.md 后，提取下游推论条目数量，
#        并运行 downstream_audit.py 报告全局未解决下游行动。
# 原则0：只警告，不阻断。

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

# 只处理 settled/ 下的谱系文件
MATCH=$(python -c "
import re, sys
path = sys.argv[1]
if re.match(r'^\.chanlun/genealogy/settled/[^/]+\.md$', path):
    print('yes')
else:
    print('no')
" "$REL_PATH" 2>/dev/null || echo "no")

if [ "$MATCH" != "yes" ]; then
    exit 0
fi

# 运行审计脚本，提取未解决下游行动
AUDIT=$(python scripts/downstream_audit.py --summary 2>/dev/null || echo "")

if [ -z "$AUDIT" ]; then
    exit 0
fi

# 检查当前文件的下游推论条目数
LOCAL_COUNT=$(python -c "
import sys, re
try:
    with open(sys.argv[1], encoding='utf-8') as f:
        content = f.read()
    m = re.search(r'##\s*下游推论\s*\n(.*?)(?=\n##\s|\Z)', content, re.DOTALL)
    if m:
        items = re.findall(r'^\d+\.', m.group(1), re.MULTILINE)
        print(len(items))
    else:
        print(0)
except:
    print(0)
" "$FILE_PATH" 2>/dev/null || echo "0")

if [ "$LOCAL_COUNT" = "0" ] && echo "$AUDIT" | grep -q "0 未解决"; then
    exit 0
fi

python -c "
import json, sys
local = sys.argv[1]
audit = sys.argv[2]
parts = []
if int(local) > 0:
    parts.append(f'本谱系含 {local} 条下游推论，需后续追踪执行')
parts.append(f'全局状态: {audit}')
msg = '[downstream-action-guard] 二阶反馈: ' + '。'.join(parts)
print(json.dumps({'decision': 'allow', 'reason': msg}, ensure_ascii=False))
" "$LOCAL_COUNT" "$AUDIT"
