#!/usr/bin/env bash
# spec-write-guard.sh — 核心配置文件写入验证（从block改为validate+allow）
# 蜂群能修改一切，安全靠 git + ESC + 产出物验证
# 触发：PreToolUse(Write), PreToolUse(Edit)

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

# 检查是否是核心配置文件 → 验证+记录+放行（不阻止）
python -c "
import re, json, sys, os
path = sys.argv[1]
cwd = sys.argv[2]
core_patterns = [
    (r'(^|/)CLAUDE\.md$', 'CLAUDE.md'),
    (r'(^|/)\.chanlun/dispatch-dag\.yaml$', 'dispatch-dag.yaml'),
    (r'(^|/)\.chanlun/dispatch-dag\.yaml$', 'dispatch-dag.yaml'),
    (r'(^|/)\.chanlun/genealogy/settled/.*\.md$', '已结算谱系'),
    (r'(^|/)\.chanlun/definitions/.*\.md$', '定义文件'),
    (r'(^|/)\.claude/agents/.*\.md$', 'agent定义'),
    (r'(^|/)\.claude/hooks/.*\.sh$', 'hook脚本'),
    (r'(^|/)\.claude/commands/.*\.md$', '命令定义'),
    (r'(^|/)scripts/ceremony_scan\.py$', 'ceremony脚本'),
]
for pat, label in core_patterns:
    if re.search(pat, path):
        full = os.path.join(cwd, path) if not os.path.isabs(path) else path
        if os.path.exists(full):
            print(json.dumps({
                'decision': 'allow',
                'reason': f'[spec-write-guard] 核心文件修改: {path} ({label})。异步自指：t时刻修改，t+1时刻生效。git追踪，ESC可中断。'
            }, ensure_ascii=False))
            sys.exit(0)
" "$REL_PATH" "$CWD"
exit 0
