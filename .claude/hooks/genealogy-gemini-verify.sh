#!/usr/bin/env bash
# PostToolUse Hook — Gemini 谱系异质审查触发提示
#
# 触发：PostToolUse on Write/Edit
# 逻辑：
#   1. 只处理 .chanlun/genealogy/**/*.md 的写入/编辑
#   2. 调用 scripts/gemini_genealogy_verify_prompt.py 生成结构化审查上下文
#   3. 输出 advisory 提示，建议 agent 触发 Gemini challenge verify 模式
#
# 约束4（异质验证必要性）：同质系统盲点不可自检 → 谱系写入后建议异质审查
# 原则0：只建议，不阻断。

set -uo pipefail

INPUT=$(cat)

PYTHON_BIN=""
if command -v python >/dev/null 2>&1; then
    PYTHON_BIN="python"
elif command -v python3 >/dev/null 2>&1; then
    PYTHON_BIN="python3"
else
    exit 0
fi

# Extract tool_name
TOOL_NAME=$(echo "$INPUT" | "$PYTHON_BIN" -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

if [ "$TOOL_NAME" != "Write" ] && [ "$TOOL_NAME" != "Edit" ]; then
    exit 0
fi

# Extract file_path
FILE_PATH=$(echo "$INPUT" | "$PYTHON_BIN" -c "
import sys, json
data = json.loads(sys.stdin.read())
ti = data.get('tool_input', {})
for key in ('file_path', 'path', 'target_file'):
    v = ti.get(key, '')
    if isinstance(v, str) and v:
        print(v)
        break
else:
    print('')
" 2>/dev/null || echo "")

if [ -z "$FILE_PATH" ]; then
    exit 0
fi

# Extract cwd
CWD=$(echo "$INPUT" | "$PYTHON_BIN" -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('cwd', '.'))
" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || true

# Convert to relative path
REL_PATH=$("$PYTHON_BIN" -c "
import sys, os
try:
    rel = os.path.relpath(sys.argv[1], sys.argv[2])
except ValueError:
    rel = sys.argv[1]
print(rel.replace(os.sep, '/'))
" "$FILE_PATH" "$CWD" 2>/dev/null || echo "$FILE_PATH")

# Only process .chanlun/genealogy/(pending|settled)/*.md
MATCH=$("$PYTHON_BIN" -c "
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

# Run the verify prompt generator
VERIFY_JSON=$("$PYTHON_BIN" scripts/gemini_genealogy_verify_prompt.py "$FILE_PATH" 2>/dev/null || echo "")

if [ -z "$VERIFY_JSON" ]; then
    exit 0
fi

# Check for error in output
HAS_ERROR=$("$PYTHON_BIN" -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print('yes' if 'error' in d else 'no')
except:
    print('yes')
" <<< "$VERIFY_JSON" 2>/dev/null || echo "yes")

if [ "$HAS_ERROR" = "yes" ]; then
    exit 0
fi

# Extract summary fields and produce advisory message
GEMINI_VERIFY_JSON="$VERIFY_JSON" "$PYTHON_BIN" - <<'PY' 2>/dev/null || true
import json, os, sys

raw = os.environ.get("GEMINI_VERIFY_JSON", "")
if not raw.strip():
    raise SystemExit(0)

try:
    verify = json.loads(raw)
except json.JSONDecodeError:
    raise SystemExit(0)

number = verify.get("genealogy_number", "???")
entry_type = verify.get("type", "未知")
conclusion = verify.get("conclusion_summary", "")
prereqs = verify.get("prerequisites", [])
boundary = verify.get("boundary_conditions", "")

summary_parts = [f"编号={number}", f"类型={entry_type}"]
if prereqs:
    summary_parts.append("前置=" + ";".join(prereqs))
if conclusion:
    short_conclusion = conclusion[:80] + "..." if len(conclusion) > 80 else conclusion
    summary_parts.append(f"结论摘要={short_conclusion}")
if boundary:
    summary_parts.append("有边界条件")
else:
    summary_parts.append("缺边界条件")

summary = ", ".join(summary_parts)

msg = (
    f"[genealogy-gemini-verify] 新谱系 {number} 已写入。"
    f"建议触发 Gemini verify 审查（约束4：异质验证必要性）。"
    f" verify 上下文：{summary}"
)

print(json.dumps({"decision": "allow", "reason": msg}, ensure_ascii=False))
PY

exit 0
