#!/bin/bash
# PostToolUse Hook — 函数行数守卫（原名 recursive-guard，092号审计重命名）
# 实际功能：检查重构任务完成后函数是否仍超过行数阈值
# 与递归深度检测无关——名称修正以消除声明-能力缺口（084号模式）
#
# 触发：PostToolUse on "TaskUpdate" tool
# 逻辑：
#   1. 检查被完成的 task 是否涉及代码重构（description 关键词判断）
#   2. 如果是重构任务，扫描相关文件中的函数行数
#   3. 如果有函数仍 > 阈值，注入 systemMessage 要求 worker 继续拆分
#   4. 最大重试 3 次后放行并输出警告
#   5. #[atomic] 标记的任务跳过检查
#
# 不硬编码阈值——从 dispatch-dag.yaml 读取或使用默认值 50

set -euo pipefail

INPUT=$(cat)

# 提取 tool_name
TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

# 只拦截 TaskUpdate
if [ "$TOOL_NAME" != "TaskUpdate" ]; then
    exit 0
fi

# 提取 tool_input
TOOL_INPUT_JSON=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
ti = data.get('tool_input', {})
print(json.dumps(ti, ensure_ascii=False))
" 2>/dev/null || echo "{}")

# 检查 status 是否为 completed
STATUS=$(echo "$TOOL_INPUT_JSON" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('status', ''))
" 2>/dev/null || echo "")

if [ "$STATUS" != "completed" ]; then
    exit 0
fi

# 提取 taskId
TASK_ID=$(echo "$TOOL_INPUT_JSON" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('taskId', ''))
" 2>/dev/null || echo "")

if [ -z "$TASK_ID" ]; then
    exit 0
fi

# 提取 tool_result（包含 task 的 description/subject）
TASK_DESC=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
result = data.get('tool_result', {})
# tool_result 可能是字符串或对象
if isinstance(result, str):
    print(result)
else:
    desc = result.get('description', '')
    subj = result.get('subject', '')
    print(subj + ' ' + desc)
" 2>/dev/null || echo "")

# 也检查 tool_input 中的 description/subject（TaskUpdate 可能携带）
TASK_INPUT_DESC=$(echo "$TOOL_INPUT_JSON" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
desc = data.get('description', '')
subj = data.get('subject', '')
print(subj + ' ' + desc)
" 2>/dev/null || echo "")

COMBINED_DESC="${TASK_DESC} ${TASK_INPUT_DESC}"

# 检查 #[atomic] 标记——跳过检查
if echo "$COMBINED_DESC" | grep -q '#\[atomic\]'; then
    exit 0
fi

# 检查是否为重构任务（关键词匹配）
IS_REFACTOR=$(echo "$COMBINED_DESC" | python -c "
import sys
text = sys.stdin.read().lower()
keywords = ['refactor', 'refactoring', '重构', '拆分', 'extract', 'split',
            'decompose', '分解', 'break down', 'simplify']
found = any(kw in text for kw in keywords)
print('yes' if found else 'no')
" 2>/dev/null || echo "no")

if [ "$IS_REFACTOR" != "yes" ]; then
    exit 0
fi

# 提取 cwd
CWD=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('cwd', '.'))
" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || true

# 从 dispatch-dag.yaml 读取函数行数阈值（默认 50）
SPEC_FILE=".chanlun/dispatch-dag.yaml"
MAX_LINES=50
if [ -f "$SPEC_FILE" ]; then
    PARSED_MAX=$(python -c "
import re
with open('$SPEC_FILE', 'r', encoding='utf-8') as f:
    content = f.read()
m = re.search(r'max_function_lines:\s*(\d+)', content)
if m:
    print(m.group(1))
else:
    print('')
" 2>/dev/null || echo "")
    if [ -n "$PARSED_MAX" ]; then
        MAX_LINES="$PARSED_MAX"
    fi
fi

# 从 task description 提取文件路径
TARGET_FILES=$(echo "$COMBINED_DESC" | python -c "
import sys, re
text = sys.stdin.read()
# 匹配常见文件路径模式：xxx.py, xxx.ts, xxx.js 等
patterns = re.findall(r'[\w/\\\\._-]+\.(?:py|ts|js|tsx|jsx|rs|go|java|rb|sh)', text)
# 去重
seen = set()
result = []
for p in patterns:
    clean = p.strip()
    if clean not in seen:
        seen.add(clean)
        result.append(clean)
print('\n'.join(result))
" 2>/dev/null || echo "")

if [ -z "$TARGET_FILES" ]; then
    exit 0
fi

# 重试次数检查（通过临时文件跟踪）
RETRY_DIR="/tmp/recursive-guard"
mkdir -p "$RETRY_DIR"
RETRY_FILE="${RETRY_DIR}/task-${TASK_ID}.count"

RETRY_COUNT=0
if [ -f "$RETRY_FILE" ]; then
    RETRY_COUNT=$(cat "$RETRY_FILE" 2>/dev/null || echo "0")
fi

MAX_RETRIES=3
if [ "$RETRY_COUNT" -ge "$MAX_RETRIES" ]; then
    # 超过最大重试次数，放行并警告
    rm -f "$RETRY_FILE"
    python -c "
import json
msg = (
    '[recursive-guard] 警告：task ${TASK_ID} 已重试 ${MAX_RETRIES} 次仍有超长函数，'
    '强制放行。请人工检查代码质量。'
)
print(json.dumps({
    'decision': 'allow',
    'reason': msg
}, ensure_ascii=False))
"
    exit 0
fi

# 扫描文件中的函数行数
VIOLATIONS=$(echo "$TARGET_FILES" | while IFS= read -r filepath; do
    [ -z "$filepath" ] && continue
    # 尝试找到文件（可能是相对路径）
    if [ ! -f "$filepath" ]; then
        continue
    fi
    python -c "
import sys, re

filepath = sys.argv[1]
max_lines = int(sys.argv[2])

with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
    lines = f.readlines()

# 检测函数定义（Python: def, JS/TS: function/=>/class method, etc.）
violations = []
func_start = None
func_name = None
func_indent = None

for i, line in enumerate(lines):
    stripped = line.rstrip()
    # Python 函数
    m = re.match(r'^(\s*)def\s+(\w+)', stripped)
    if not m:
        # JS/TS 函数
        m = re.match(r'^(\s*)(?:async\s+)?(?:function\s+(\w+)|(\w+)\s*(?:=|:)\s*(?:async\s+)?(?:function|\(.*\)\s*=>))', stripped)
    if m:
        # 如果有前一个函数，检查行数
        if func_start is not None:
            length = i - func_start
            if length > max_lines:
                violations.append(f'  {filepath}:{func_start+1} {func_name}() = {length} lines')
        func_start = i
        func_name = m.group(2) if m.group(2) else (m.group(3) if len(m.groups()) >= 3 and m.group(3) else 'anonymous')
        func_indent = len(m.group(1))

# 检查最后一个函数
if func_start is not None:
    length = len(lines) - func_start
    if length > max_lines:
        violations.append(f'  {filepath}:{func_start+1} {func_name}() = {length} lines')

for v in violations:
    print(v)
" "$filepath" "$MAX_LINES" 2>/dev/null || true
done)

if [ -z "$VIOLATIONS" ]; then
    # 全部达标，清除重试计数，放行
    rm -f "$RETRY_FILE"
    exit 0
fi

# 有违规——递增重试计数
NEW_COUNT=$((RETRY_COUNT + 1))
echo "$NEW_COUNT" > "$RETRY_FILE"

# 注入 systemMessage 要求继续拆分
python -c "
import json, sys

violations = sys.argv[1]
task_id = sys.argv[2]
max_lines = sys.argv[3]
retry = sys.argv[4]
max_retry = sys.argv[5]

msg = (
    '[recursive-guard] 037号递归蜂群检查未通过（重试 ' + retry + '/' + max_retry + '）。'
    ' task ' + task_id + ' 标记为 completed，但以下函数仍超过 ' + max_lines + ' 行阈值：\n'
    + violations + '\n'
    '请继续拆分这些函数，或 spawn 子工位处理。'
    ' 如果此任务确实不需要进一步拆分，在 description 中添加 #[atomic] 标记。'
)
print(json.dumps({
    'decision': 'block',
    'reason': msg
}, ensure_ascii=False))
" "$VIOLATIONS" "$TASK_ID" "$MAX_LINES" "$NEW_COUNT" "$MAX_RETRIES"
