#!/usr/bin/env bash
# spec-write-guard.sh — 核心配置文件写入守卫（061号 A3+A4）
# 拦截对 dispatch-spec.yaml 和 CLAUDE.md 的修改
# 触发：PreToolUse(Write), PreToolUse(Edit)

set -uo pipefail

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

# ─── 提取 file_path + cwd，规范化路径 ───
PARSE_RESULT=$(echo "$INPUT" | python -c "
import sys, json, os
data = json.loads(sys.stdin.read())
fp = data.get('tool_input', {}).get('file_path', '')
cwd = data.get('cwd', '.')
if not fp:
    print('')
    print(cwd)
else:
    try:
        rel = os.path.relpath(fp, cwd)
    except ValueError:
        rel = fp
    print(rel.replace(os.sep, '/'))
    print(cwd)
" 2>/dev/null || echo "")

REL_PATH=$(echo "$PARSE_RESULT" | head -1)
CWD=$(echo "$PARSE_RESULT" | tail -1)

if [ -z "$REL_PATH" ]; then
  exit 0
fi

# ─── 放行：生成态谱系允许直接写入 ───
if echo "$REL_PATH" | grep -qE '(^|/)\.chanlun/genealogy/pending/'; then
  exit 0
fi

# ─── 检查是否是核心配置文件（阻断） ───
python -c "
import re, json, sys, os
path = sys.argv[1]
cwd = sys.argv[2]
core_patterns = [
    (r'(^|/)CLAUDE\.md$', 'CLAUDE.md'),
    (r'(^|/)\.chanlun/dispatch-spec\.yaml$', 'dispatch-spec.yaml'),
    (r'(^|/)\.chanlun/genealogy/settled/.*\.md$', '已结算谱系'),
    (r'(^|/)\.chanlun/definitions/.*\.md$', '定义文件'),
]
for pat, label in core_patterns:
    if re.search(pat, path):
        # 新建文件允许（谱系新增不是修改核心文件）
        full = os.path.join(cwd, path) if not os.path.isabs(path) else path
        if not os.path.exists(full):
            sys.exit(0)
        print(json.dumps({
            'decision': 'block',
            'reason': f'[spec-write-guard] 核心文件修改需走提案模式（069号谱系）: {path} ({label})'
        }, ensure_ascii=False))
        sys.exit(0)
" "$REL_PATH" "$CWD"
exit 0
