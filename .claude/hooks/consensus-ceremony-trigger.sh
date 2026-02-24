#!/usr/bin/env bash
# PostToolUse Hook — 共识仪式触发提示（advisory）
#
# 触发：PostToolUse on Write/Edit
# 逻辑：
#   1. 只处理 .chanlun/review-results/*.md 的写入
#   2. 检测文件是否表明质询循环已收敛
#   3. 输出 advisory 提示，建议触发共识仪式
#
# 总方针 §21: 每次质询循环收敛时，Serena 强制执行共识仪式
# D策略（082号）：hooks 提示 + Lead 认领，不阻断

set -uo pipefail

INPUT=$(cat)

resolve_python() {
    local bin
    for bin in python3 python; do
        if command -v "$bin" >/dev/null 2>&1 && "$bin" -c "import sys" >/dev/null 2>&1; then
            echo "$bin"
            return 0
        fi
    done
    return 1
}

PYTHON_BIN="$(resolve_python || true)"
if [ -z "$PYTHON_BIN" ]; then
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

# Only process .chanlun/review-results/*.md
MATCH=$("$PYTHON_BIN" -c "
import re, sys
path = sys.argv[1]
if re.match(r'^\.chanlun/review-results/[^/]+\.md$', path):
    print('yes')
else:
    print('no')
" "$REL_PATH" 2>/dev/null || echo "no")

if [ "$MATCH" != "yes" ]; then
    exit 0
fi

# Check convergence and generate advisory message
REVIEW_FILE="$FILE_PATH" "$PYTHON_BIN" - <<'PY' 2>/dev/null || true
import json, os, sys
from pathlib import Path

review_file = Path(os.environ.get("REVIEW_FILE", ""))
if not review_file.exists():
    raise SystemExit(0)

# Inline convergence detection (avoid import issues in hook context)
text = review_file.read_text(encoding="utf-8")

plan_review_converged = any(
    marker in text
    for marker in ("确认满意", "共识达成", "方案定稿", "[plan-reviewed]")
)

import re
gemini_converged = bool(
    re.search(r"^result:\s*(pass|fail|escalate)", text, re.MULTILINE)
)

if not (plan_review_converged or gemini_converged):
    raise SystemExit(0)

# Determine scenario
fname = review_file.name
if fname.startswith("plan-review-"):
    scenario = "plan-review 多轮对审"
elif fname.startswith("gemini-genealogy-review-"):
    scenario = "Gemini 概念层谱系质询"
elif fname.startswith("codex-"):
    scenario = "Codex 代码层审查"
else:
    scenario = "质询循环"

msg = (
    f"[consensus-ceremony-trigger] {scenario}已收敛（{review_file.name}）。"
    f"建议触发共识仪式（§21: 三区块原子写入 consensus/residue/tension）。"
    f"调用: scripts/consensus_trigger.py 中的 trigger_ceremony()。"
)

print(json.dumps({"systemMessage": msg}, ensure_ascii=False))
PY

exit 0
