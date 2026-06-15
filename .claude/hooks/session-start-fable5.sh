#!/usr/bin/env bash
# SessionStart Hook — 自动注入 Fable 5 系统提示词
#
# 每次会话开始时读取项目根目录的 CLAUDE-FABLE-5.md，
# 通过 hookSpecificOutput.additionalContext 注入到 session 上下文。
#
# 为什么用 additionalContext 而非 systemMessage：
#   - systemMessage 只显示给用户（横幅提示），不进模型上下文
#   - additionalContext 才会被拼接进 session 的系统提示
#
# 与 session-start-ceremony.sh 解耦：单一职责，互不影响。

set -uo pipefail

# stdin 含 hook 输入 JSON（cwd 等），保存到临时文件供 python 解析
HOOK_TMPFILE=$(mktemp)
trap 'rm -f "$HOOK_TMPFILE"' EXIT
timeout 3 cat > "$HOOK_TMPFILE" 2>/dev/null || echo "{}" > "$HOOK_TMPFILE"

HOOK_INPUT="$HOOK_TMPFILE" python3 << 'PYEOF'
import json, os, sys

tmpfile = os.environ['HOOK_INPUT']
try:
    with open(tmpfile) as f:
        data = json.load(f)
except (OSError, ValueError):
    data = {}

cwd = data.get('cwd', '.')
try:
    os.chdir(cwd)
except OSError:
    pass

fable_path = 'CLAUDE-FABLE-5.md'

# 文件不存在 → 静默放行（不阻断 session 启动）
if not os.path.isfile(fable_path):
    print(json.dumps({'continue': True, 'suppressOutput': True}, ensure_ascii=False))
    sys.exit(0)

try:
    with open(fable_path, encoding='utf-8') as f:
        fable_content = f.read()
except OSError:
    print(json.dumps({'continue': True, 'suppressOutput': True}, ensure_ascii=False))
    sys.exit(0)

additional_context = (
    "以下是 Claude Fable 5 系统提示词（来自项目根目录 CLAUDE-FABLE-5.md，"
    "每次 session 启动时自动注入）。作为当前 session 的先验行为基底内化：\n\n"
    + fable_content
)

print(json.dumps({
    'continue': True,
    'suppressOutput': True,
    'hookSpecificOutput': {
        'hookEventName': 'SessionStart',
        'additionalContext': additional_context,
    },
}, ensure_ascii=False))
PYEOF
