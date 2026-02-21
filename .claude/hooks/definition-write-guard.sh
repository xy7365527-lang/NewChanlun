#!/usr/bin/env bash
# definition-write-guard.sh — PreToolUse hook for Write/Edit
# 验证 .chanlun/definitions/*.md 的写入格式（allow + 警告，不阻止）
# 蜂群能修改一切，安全靠 git + ESC + 产出物验证
#
# 触发：PreToolUse(Write), PreToolUse(Edit)
# 验证：
#   - Write: content 中应有 status（生成态/已结算）和 version 字段
#   - Edit: 如果 new_string 将 status 从生成态改为已结算，检查对应谱系

set -euo pipefail

INPUT=$(cat)

# ─── 提取 tool_name ───
TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

# 只处理 Write 和 Edit
if [ "$TOOL_NAME" != "Write" ] && [ "$TOOL_NAME" != "Edit" ]; then
  exit 0
fi

# ─── 提取 file_path ───
FILE_PATH=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_input', {}).get('file_path', ''))
" 2>/dev/null || echo "")

# 只拦截 .chanlun/definitions/*.md
# 支持绝对路径和相对路径
if ! echo "$FILE_PATH" | grep -qE '(^|/)\.chanlun/definitions/[^/]+\.md$'; then
  exit 0
fi

# ─── 提取 cwd ───
CWD=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('cwd', '.'))
" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || exit 0

# ─── 熔断机制 ───
COUNTER_FILE=".chanlun/.def-guard-counter"
LAST_FILE_MARKER=".chanlun/.def-guard-last-file"

BLOCK_COUNT=0
LAST_FILE=""

if [ -f "$LAST_FILE_MARKER" ]; then
  LAST_FILE=$(cat "$LAST_FILE_MARKER" 2>/dev/null || echo "")
fi
if [ -f "$COUNTER_FILE" ]; then
  BLOCK_COUNT=$(cat "$COUNTER_FILE" 2>/dev/null || echo "0")
  BLOCK_COUNT=$((BLOCK_COUNT + 0))
fi

# 同一文件连续 block 3 次 → 熔断放行 + 警告
if [ "$FILE_PATH" = "$LAST_FILE" ] && [ "$BLOCK_COUNT" -ge 3 ]; then
  rm -f "$COUNTER_FILE" "$LAST_FILE_MARKER" 2>/dev/null || true
  python -c "
import json
print(json.dumps({
    'decision': 'allow',
    'reason': '[definition-write-guard] 熔断放行：同一文件已连续被 block 3 次。请事后检查该定义文件的完整性。'
}, ensure_ascii=False))
"
  exit 0
fi

# 如果切换到不同文件，重置计数器
if [ "$FILE_PATH" != "$LAST_FILE" ]; then
  BLOCK_COUNT=0
fi

# ─── Write 工具验证 ───
if [ "$TOOL_NAME" = "Write" ]; then
  VALIDATION=$(echo "$INPUT" | python -c "
import sys, json, re

data = json.loads(sys.stdin.read())
content = data.get('tool_input', {}).get('content', '')

missing = []

# 检查 status 字段 — 支持 YAML frontmatter 和 markdown bold 两种格式
# YAML: status: 生成态 / status: 已结算
# Markdown: **状态**: 生成态 / **状态**: 已结算
has_status = False
valid_status_values = ['生成态', '已结算']

# YAML frontmatter 格式
yaml_status = re.search(r'^status:\s*(.+)$', content, re.MULTILINE)
# Markdown bold 格式
md_status = re.search(r'^\*\*状态\*\*:\s*(.+)$', content, re.MULTILINE)

if yaml_status:
    val = yaml_status.group(1).strip()
    if val in valid_status_values:
        has_status = True
    else:
        missing.append(f'status 值无效: \"{val}\"（必须是 生成态 或 已结算）')
elif md_status:
    val = md_status.group(1).strip()
    if val in valid_status_values:
        has_status = True
    else:
        missing.append(f'状态 值无效: \"{val}\"（必须是 生成态 或 已结算）')
else:
    missing.append('缺少 status/状态 字段（必须是 生成态 或 已结算）')

# 检查 version 字段 — 同样支持两种格式
# YAML: version: v1.0
# Markdown: **版本**: v1.0
has_version = False
yaml_version = re.search(r'^version:\s*\S+', content, re.MULTILINE)
md_version = re.search(r'^\*\*版本\*\*:\s*\S+', content, re.MULTILINE)

if yaml_version or md_version:
    has_version = True
else:
    missing.append('缺少 version/版本 字段')

if missing:
    print('BLOCK:' + '；'.join(missing))
else:
    print('OK')
" 2>/dev/null || echo "OK")

  if echo "$VALIDATION" | grep -q "^BLOCK:"; then
    REASON=$(echo "$VALIDATION" | sed 's/^BLOCK://')

    # 更新熔断计数器
    echo "$FILE_PATH" > "$LAST_FILE_MARKER"
    echo $((BLOCK_COUNT + 1)) > "$COUNTER_FILE"

    python -c "
import json, sys
reason = sys.argv[1]
msg = (
    '[definition-write-guard] 阻断：定义文件写入缺少必需字段。\n'
    '目标文件: ' + sys.argv[2] + '\n'
    '缺失项: ' + reason + '\n\n'
    '定义文件必须包含:\n'
    '  - status/状态 字段（值: 生成态 或 已结算）\n'
    '  - version/版本 字段\n\n'
    '格式示例（Markdown）:\n'
    '  **版本**: v1.0\n'
    '  **状态**: 生成态\n\n'
    '连续 block 3 次后自动熔断放行。'
)
print(json.dumps({
    'decision': 'allow',
    'reason': msg
}, ensure_ascii=False))
" "$REASON" "$FILE_PATH"
    exit 0
  fi
fi

# ─── Edit 工具验证 ───
if [ "$TOOL_NAME" = "Edit" ]; then
  VALIDATION=$(echo "$INPUT" | python -c "
import sys, json, re, os, glob

data = json.loads(sys.stdin.read())
new_string = data.get('tool_input', {}).get('new_string', '')
file_path = data.get('tool_input', {}).get('file_path', '')

# 检查 new_string 是否将 status 从生成态改为已结算
# YAML 格式
yaml_settled = re.search(r'^status:\s*已结算', new_string, re.MULTILINE)
# Markdown 格式
md_settled = re.search(r'^\*\*状态\*\*:\s*已结算', new_string, re.MULTILINE)

if not yaml_settled and not md_settled:
    # 不涉及 status 变更为已结算，放行
    print('OK')
    sys.exit(0)

# 从文件路径提取定义名称（不含扩展名）
basename = os.path.basename(file_path).replace('.md', '')

# 检查 .chanlun/genealogy/settled/ 中是否有对应谱系
settled_dir = '.chanlun/genealogy/settled'
if not os.path.isdir(settled_dir):
    print(f'BLOCK:谱系目录 {settled_dir}/ 不存在，无法验证已结算状态')
    sys.exit(0)

# 在 settled 目录中搜索包含该定义名称的谱系文件
found = False
settled_files = glob.glob(os.path.join(settled_dir, '*.md'))
for sf in settled_files:
    sf_basename = os.path.basename(sf).lower()
    if basename.lower() in sf_basename:
        found = True
        break

# 如果按文件名没找到，搜索谱系文件内容中是否引用了该定义
if not found:
    for sf in settled_files:
        try:
            with open(sf, 'r', encoding='utf-8') as f:
                content = f.read()
            # 检查是否引用了该定义文件
            if basename in content or file_path in content:
                found = True
                break
        except Exception:
            continue

if found:
    print('OK')
else:
    print(f'BLOCK:将 {basename}.md 状态改为已结算，但在 {settled_dir}/ 中未找到对应谱系记录。请先创建谱系记录再结算定义。')
" 2>/dev/null || echo "OK")

  if echo "$VALIDATION" | grep -q "^BLOCK:"; then
    REASON=$(echo "$VALIDATION" | sed 's/^BLOCK://')

    # 更新熔断计数器
    echo "$FILE_PATH" > "$LAST_FILE_MARKER"
    echo $((BLOCK_COUNT + 1)) > "$COUNTER_FILE"

    python -c "
import json, sys
reason = sys.argv[1]
msg = (
    '[definition-write-guard] 阻断：定义状态变更缺少谱系支撑。\n'
    '详情: ' + reason + '\n\n'
    '将定义从生成态改为已结算，必须在 .chanlun/genealogy/settled/ 中有对应谱系记录。\n'
    '谱系是定义结算的前置条件（012号谱系：谱系优先于汇总）。\n\n'
    '连续 block 3 次后自动熔断放行。'
)
print(json.dumps({
    'decision': 'allow',
    'reason': msg
}, ensure_ascii=False))
" "$REASON"
    exit 0
  fi
fi

# ─── 验证通过，清理计数器 ───
rm -f "$COUNTER_FILE" "$LAST_FILE_MARKER" 2>/dev/null || true
exit 0
