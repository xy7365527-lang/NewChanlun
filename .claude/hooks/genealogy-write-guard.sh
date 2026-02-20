#!/usr/bin/env bash
# PreToolUse Hook — 谱系写入守卫（genealogy-write-guard）
#
# 触发：PreToolUse on "Write" tool
# 逻辑：
#   1. 只处理 Write 工具对 .chanlun/genealogy/**/*.md 的写入
#   2. 检查强制字段存在性：类型、状态、日期、前置
#   3. 检查前置引用的谱系编号是否存在于 settled/ 或 pending/
#   4. 熔断：连续 block 3 次同一文件 → 放行 + 警告
#
# 熔断状态文件：
#   .chanlun/.gen-guard-counter   — 连续 block 计数
#   .chanlun/.gen-guard-last-file — 上次被 block 的文件路径

set -euo pipefail

INPUT=$(cat)

# 提取 tool_name
TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

# 只拦截 Write 工具
if [ "$TOOL_NAME" != "Write" ]; then
    exit 0
fi

# 提取 file_path 和 content
FILE_PATH=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_input', {}).get('file_path', ''))
" 2>/dev/null || echo "")

# 提取 cwd
CWD=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('cwd', '.'))
" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || true

# 将绝对路径转为相对路径（相对于 cwd）以便匹配
REL_PATH=$(python -c "
import sys, os
file_path = sys.argv[1]
cwd = sys.argv[2]
# 尝试转为相对路径
try:
    rel = os.path.relpath(file_path, cwd)
except ValueError:
    rel = file_path
# 统一为正斜杠
print(rel.replace(os.sep, '/'))
" "$FILE_PATH" "$CWD" 2>/dev/null || echo "$FILE_PATH")

# 只处理 .chanlun/genealogy/pending/*.md 或 .chanlun/genealogy/settled/*.md
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

# ── 熔断检查 ──────────────────────────────────────────────
COUNTER_FILE=".chanlun/.gen-guard-counter"
LAST_FILE=".chanlun/.gen-guard-last-file"

FUSE_BLOWN="no"
if [ -f "$COUNTER_FILE" ] && [ -f "$LAST_FILE" ]; then
    LAST=$(cat "$LAST_FILE" 2>/dev/null || echo "")
    COUNT=$(cat "$COUNTER_FILE" 2>/dev/null || echo "0")
    if [ "$LAST" = "$REL_PATH" ] && [ "$COUNT" -ge 3 ] 2>/dev/null; then
        FUSE_BLOWN="yes"
    fi
fi

# ── 字段验证 ──────────────────────────────────────────────
VALIDATION_RESULT=$(echo "$INPUT" | python -c "
import sys, json, re, os, glob

data = json.loads(sys.stdin.read())
content = data.get('tool_input', {}).get('content', '')
rel_path = '$REL_PATH'

# 强制字段检查（支持 **字段**: 和 字段: 两种格式）
required_fields = {
    '类型': r'(?:\*\*类型\*\*|类型)\s*[:：]',
    '状态': r'(?:\*\*状态\*\*|状态)\s*[:：]',
    '日期': r'(?:\*\*日期\*\*|日期)\s*[:：]',
    '前置': r'(?:\*\*前置\*\*|前置)\s*[:：]',
}

missing = []
for name, pattern in required_fields.items():
    if not re.search(pattern, content):
        missing.append(name)

# 前置引用验证（只在前置字段存在时检查）
invalid_refs = []
if '前置' not in missing:
    # 提取前置字段的值
    m = re.search(r'(?:\*\*前置\*\*|前置)\s*[:：]\s*(.*)', content)
    if m:
        prereq_value = m.group(1).strip()
        # 跳过空值、无、N/A 等
        if prereq_value and prereq_value not in ('无', 'N/A', '-', '（无）', '(无)', '(none)', 'none'):
            # 提取所有谱系编号引用（如 053-xxx, 001-yyy）
            refs = re.findall(r'(\d{3}-[a-zA-Z0-9_-]+)', prereq_value)
            for ref in refs:
                # 检查 settled/ 和 pending/ 目录
                found = False
                for subdir in ['settled', 'pending']:
                    pattern_glob = os.path.join('.chanlun', 'genealogy', subdir, ref + '*.md')
                    matches = glob.glob(pattern_glob)
                    if matches:
                        found = True
                        break
                if not found:
                    invalid_refs.append(ref)

result = {'missing': missing, 'invalid_refs': invalid_refs}
print(json.dumps(result, ensure_ascii=False))
" 2>/dev/null || echo '{"missing":[],"invalid_refs":[]}')

# 解析验证结果
MISSING=$(echo "$VALIDATION_RESULT" | python -c "
import sys, json
d = json.loads(sys.stdin.read())
print(','.join(d.get('missing', [])))
" 2>/dev/null || echo "")

INVALID_REFS=$(echo "$VALIDATION_RESULT" | python -c "
import sys, json
d = json.loads(sys.stdin.read())
print(','.join(d.get('invalid_refs', [])))
" 2>/dev/null || echo "")

# 如果没有问题，放行并重置熔断计数
if [ -z "$MISSING" ] && [ -z "$INVALID_REFS" ]; then
    # 重置熔断状态
    rm -f "$COUNTER_FILE" "$LAST_FILE" 2>/dev/null || true
    exit 0
fi

# ── 有问题：检查熔断 ──────────────────────────────────────
if [ "$FUSE_BLOWN" = "yes" ]; then
    # 熔断触发：放行 + 警告 + 重置计数
    rm -f "$COUNTER_FILE" "$LAST_FILE" 2>/dev/null || true
    python -c "
import json, sys
missing = sys.argv[1]
invalid = sys.argv[2]
parts = []
if missing:
    parts.append('缺失字段: ' + missing)
if invalid:
    parts.append('无效前置引用: ' + invalid)
detail = '; '.join(parts)
msg = (
    '[genealogy-write-guard] 熔断放行（连续 3 次 block 同一文件）。'
    ' 仍存在问题：' + detail + '。'
    ' 请在后续 commit 前修复。'
)
print(json.dumps({
    'decision': 'allow',
    'reason': msg
}, ensure_ascii=False))
" "$MISSING" "$INVALID_REFS"
    exit 0
fi

# ── 阻断 + 更新熔断计数 ──────────────────────────────────
# 更新计数
if [ -f "$LAST_FILE" ]; then
    LAST=$(cat "$LAST_FILE" 2>/dev/null || echo "")
else
    LAST=""
fi

if [ "$LAST" = "$REL_PATH" ] && [ -f "$COUNTER_FILE" ]; then
    COUNT=$(cat "$COUNTER_FILE" 2>/dev/null || echo "0")
    NEW_COUNT=$((COUNT + 1))
else
    NEW_COUNT=1
fi

# 确保目录存在
mkdir -p .chanlun 2>/dev/null || true
echo "$NEW_COUNT" > "$COUNTER_FILE"
echo "$REL_PATH" > "$LAST_FILE"

REMAINING=$((3 - NEW_COUNT))

python -c "
import json, sys
missing = sys.argv[1]
invalid = sys.argv[2]
remaining = sys.argv[3]

parts = []
if missing:
    fields = missing.split(',')
    parts.append('缺失强制字段: ' + ', '.join(fields))
if invalid:
    refs = invalid.split(',')
    parts.append('前置引用的谱系编号不存在于 settled/ 或 pending/: ' + ', '.join(refs))

detail = '\n  '.join(parts)
msg = (
    '[genealogy-write-guard] 阻断：谱系文件缺少强制字段或引用无效。\n'
    '  ' + detail + '\n'
    '要求：每个谱系文件必须包含 类型、状态、日期、前置 四个字段（前置可为空但字段必须存在）。\n'
    '前置引用的谱系编号必须在 .chanlun/genealogy/settled/ 或 pending/ 中存在。\n'
    '熔断提示：再连续 block ' + remaining + ' 次将自动放行。'
)
print(json.dumps({
    'decision': 'block',
    'reason': msg
}, ensure_ascii=False))
" "$MISSING" "$INVALID_REFS" "$REMAINING"
