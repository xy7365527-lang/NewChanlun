#!/bin/bash
# PostToolUse Hook — 结晶守卫（crystallization-guard）
#
# 触发：PostToolUse on "Bash" tool
# 逻辑：
#   1. 检查 Bash 命令是否为 git commit（session/chore 类型）
#   2. 读取 .chanlun/.crystallization-debt.json
#   3. 如果有 status=pending 的债务记录，阻断 commit
#   4. [Ad-hoc] 标记的模式跳过
#
# Gemini 审计结论：断裂在"显存→持久化"写入阀门。
# 结晶靠自觉所以不发生。需要运行时强制。

set -euo pipefail

INPUT=$(cat)

# 只处理 Bash 工具
TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

if [ "$TOOL_NAME" != "Bash" ]; then
    exit 0
fi

# 获取命令内容
COMMAND=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_input', {}).get('command', ''))
" 2>/dev/null || echo "")

# 只拦截 git commit 命令
if ! echo "$COMMAND" | grep -q "git commit"; then
    exit 0
fi

# 只拦截 session 或 chore 类型的 commit
IS_SESSION_OR_CHORE=$(echo "$COMMAND" | python -c "
import sys
cmd = sys.stdin.read().lower()
keywords = ['session', 'chore:']
print('yes' if any(kw in cmd for kw in keywords) else 'no')
" 2>/dev/null || echo "no")

if [ "$IS_SESSION_OR_CHORE" != "yes" ]; then
    exit 0
fi

# 提取 cwd
CWD=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('cwd', '.'))
" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || true

DEBT_FILE=".chanlun/.crystallization-debt.json"

# 如果债务文件不存在，无债务，放行
if [ ! -f "$DEBT_FILE" ]; then
    exit 0
fi

# 检查是否有 pending 债务（排除 [Ad-hoc] 标记的）
PENDING_DEBTS=$(python -c "
import json, sys

with open('$DEBT_FILE', 'r', encoding='utf-8') as f:
    debts = json.load(f)

if not isinstance(debts, list):
    debts = []

pending = []
for d in debts:
    if d.get('status') != 'pending':
        continue
    desc = d.get('desc', '')
    if '[Ad-hoc]' in desc or '[ad-hoc]' in desc:
        continue
    pending.append(d)

if not pending:
    print('')
else:
    lines = []
    for p in pending:
        lines.append(f\"  - [{p.get('id','?')}] {p.get('desc','(no desc)')}\")
    print('\n'.join(lines))
" 2>/dev/null || echo "")

# 无 pending 债务，继续检查 pattern-buffer
if [ -z "$PENDING_DEBTS" ]; then
    # 检查 pattern-buffer 中是否有 frequency >= 3 的未处理模式（043号谱系：自生长回路）
    PATTERN_BUFFER=".chanlun/pattern-buffer.yaml"
    if [ -f "$PATTERN_BUFFER" ]; then
        UNPROCESSED_PATTERNS=$(python -c "
import sys, re

with open('$PATTERN_BUFFER', 'r', encoding='utf-8') as f:
    content = f.read()

# 简单 YAML 解析：提取 frequency >= 3 且 status == pending 的模式
patterns = []
current = {}
for line in content.split('\n'):
    stripped = line.strip()
    if stripped.startswith('- id:'):
        if current.get('frequency', 0) >= 3 and current.get('status') == 'pending':
            patterns.append(current)
        current = {'id': stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")}
    elif stripped.startswith('frequency:'):
        try:
            current['frequency'] = int(stripped.split(':', 1)[1].strip())
        except ValueError:
            pass
    elif stripped.startswith('status:'):
        current['status'] = stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")
    elif stripped.startswith('description:'):
        current['description'] = stripped.split(':', 1)[1].strip().strip('\"').strip(\"'\")

# 最后一个
if current.get('frequency', 0) >= 3 and current.get('status') == 'pending':
    patterns.append(current)

if not patterns:
    print('')
else:
    lines = []
    for p in patterns:
        lines.append(f\"  - [{p.get('id','?')}] freq={p.get('frequency',0)} {p.get('description','(no desc)')}\")
    print('\n'.join(lines))
" 2>/dev/null || echo "")

        if [ -n "$UNPROCESSED_PATTERNS" ]; then
            python -c "
import json, sys

patterns = sys.argv[1]
msg = (
    '[crystallization-guard] 阻断：pattern-buffer 中存在 frequency >= 3 的未处理模式。'
    ' 这些模式已达到结晶阈值，commit 前必须先完成结晶（promote 为 skill）或标记为 rejected。\n'
    '未处理模式：\n'
    + patterns + '\n'
    '处理方式：\n'
    '  1. 为达标模式创建对应的 skill 文件，将 status 改为 promoted\n'
    '  2. 如果模式不值得结晶，将 status 改为 rejected\n'
    '  3. pattern-buffer 文件: .chanlun/pattern-buffer.yaml'
)
print(json.dumps({
    'decision': 'block',
    'reason': msg
}, ensure_ascii=False))
" "$UNPROCESSED_PATTERNS"
            exit 0
        fi
    fi
    exit 0
fi

# 有未结晶的债务，阻断 commit
python -c "
import json, sys

debts = sys.argv[1]
msg = (
    '[crystallization-guard] 阻断：检测到未结晶的稳定模式债务。'
    ' session/chore commit 前必须先完成结晶（创建 skill 文件）或标记为 resolved。\n'
    '未结晶债务：\n'
    + debts + '\n'
    '处理方式：\n'
    '  1. 为每个 pending 模式创建对应的 skill 文件，然后将债务标记为 resolved\n'
    '  2. 如果模式是临时性的，在 desc 中添加 [Ad-hoc] 标记\n'
    '  3. 债务文件: .chanlun/.crystallization-debt.json'
)
print(json.dumps({
    'decision': 'block',
    'reason': msg
}, ensure_ascii=False))
" "$PENDING_DEBTS"
