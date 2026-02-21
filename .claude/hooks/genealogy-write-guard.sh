#!/usr/bin/env bash
# PreToolUse Hook — 谱系写入守卫（genealogy-write-guard）
#
# 触发：PreToolUse on "Write" tool
# 逻辑：
#   1. 只处理 Write 工具对 .chanlun/genealogy/**/*.md 的写入
#   2. 检查强制字段存在性：类型、状态、日期、前置
#   3. 检查前置引用的谱系编号是否存在于 settled/ 或 pending/
#   4. 二阶反馈回路检查：验证活跃蜂群中存在 meta-observer
#   5. 验证不通过时：allow + 警告（不阻止写入）
#
# 原则0：蜂群能修改一切。守卫只验证，不阻断。

set -euo pipefail

INPUT=$(cat)

# 提取 tool_name
TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

# 只拦截 Write 和 Edit 工具
if [ "$TOOL_NAME" != "Write" ] && [ "$TOOL_NAME" != "Edit" ]; then
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

# ── 字段验证 ──────────────────────────────────────────────
VALIDATION_RESULT=$(echo "$INPUT" | python -c "
import sys, json, re, os, glob

data = json.loads(sys.stdin.read())
tool_name = data.get('tool_name', '')
tool_input = data.get('tool_input', {})
if tool_name == 'Edit':
    file_path = tool_input.get('file_path', '')
    old_str = tool_input.get('old_string', '')
    new_str = tool_input.get('new_string', '')
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        content = content.replace(old_str, new_str, 1)
    except:
        content = ''
else:
    content = tool_input.get('content', '')
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

# ── 二阶反馈回路检查：谱系写入时 meta-observer 必须在场 ──
META_OBS_WARN=""
META_OBS_CHECK=$(python -c "
import json, os, glob
team_dir = os.path.join(os.path.expanduser('~'), '.claude', 'teams')
if not os.path.isdir(team_dir):
    print('no_team')
else:
    found = False
    for cfg in glob.glob(os.path.join(team_dir, '*/config.json')):
        with open(cfg, encoding='utf-8') as f:
            tc = json.load(f)
        names = [m.get('name', '') for m in tc.get('members', [])]
        if any('meta-observer' in n for n in names):
            found = True
            break
    print('ok' if found else 'missing')
" 2>/dev/null || echo "skip")

if [ "$META_OBS_CHECK" = "missing" ]; then
    META_OBS_WARN="二阶反馈回路未激活：活跃蜂群中缺少 meta-observer，谱系写入无元观察覆盖"
elif [ "$META_OBS_CHECK" = "no_team" ]; then
    META_OBS_WARN="二阶反馈回路未激活：无活跃蜂群，meta-observer 未 spawn"
fi

# 如果没有任何问题，放行
if [ -z "$MISSING" ] && [ -z "$INVALID_REFS" ] && [ -z "$META_OBS_WARN" ]; then
    exit 0
fi

# ── 有问题：allow + 警告 ──────────────────────────────────
python -c "
import json, sys
missing = sys.argv[1]
invalid = sys.argv[2]
meta_warn = sys.argv[3]

parts = []
if missing:
    fields = missing.split(',')
    parts.append('缺失强制字段: ' + ', '.join(fields))
if invalid:
    refs = invalid.split(',')
    parts.append('前置引用的谱系编号不存在于 settled/ 或 pending/: ' + ', '.join(refs))
if meta_warn:
    parts.append(meta_warn)

detail = '; '.join(parts)
msg = (
    '[genealogy-write-guard] 警告：' + detail + '。'
    ' 请在后续 commit 前修复。'
)
print(json.dumps({
    'decision': 'allow',
    'reason': msg
}, ensure_ascii=False))
" "$MISSING" "$INVALID_REFS" "$META_OBS_WARN"
