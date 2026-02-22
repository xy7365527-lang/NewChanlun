#!/usr/bin/env bash
set -uo pipefail

# PostToolUse hook: validate dag.yaml when genealogy files are modified
# Triggers on Write/Edit to .chanlun/genealogy/

INPUT=$(cat)

TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

if [ "$TOOL_NAME" != "Write" ] && [ "$TOOL_NAME" != "Edit" ]; then
  exit 0
fi

REL_PATH=$(echo "$INPUT" | python -c "
import sys, json, os
data = json.loads(sys.stdin.read())
fp = data.get('tool_input', {}).get('file_path', '')
cwd = data.get('cwd', '.')
if not fp:
    print('')
else:
    try:
        rel = os.path.relpath(fp, cwd)
    except ValueError:
        rel = fp
    print(rel.replace(os.sep, '/'))
" 2>/dev/null || echo "")

if ! echo "$REL_PATH" | grep -qE '(^|/)\.chanlun/genealogy/'; then
  exit 0
fi

CWD=$(echo "$INPUT" | python -c "
import sys, json
print(json.loads(sys.stdin.read()).get('cwd', '.'))
" 2>/dev/null || echo ".")
cd "$CWD" 2>/dev/null || true

SCRIPT="scripts/chanlun/validate_dag.py"
DAG=".chanlun/genealogy/dag.yaml"

# Skip if dag.yaml or validation script doesn't exist yet
if [ ! -f "$DAG" ] || [ ! -f "$SCRIPT" ]; then
  exit 0
fi

OUTPUT=$(python "$SCRIPT" 2>&1) || true
if echo "$OUTPUT" | grep -q "DAG validation passed"; then
  exit 0
fi

# Auto-fix: run dag_sync.py to resolve node/edge mismatches
SYNC_SCRIPT="scripts/dag_sync.py"
if [ -f "$SYNC_SCRIPT" ]; then
  SYNC_OUTPUT=$(python "$SYNC_SCRIPT" 2>&1) || true
  # Re-validate after sync
  OUTPUT2=$(python "$SCRIPT" 2>&1) || true
  if echo "$OUTPUT2" | grep -q "DAG validation passed"; then
    exit 0
  fi
  # Still failing after auto-fix — block with both outputs
  python -c "
import json, sys
output = sys.argv[1]
sync = sys.argv[2]
print(json.dumps({
    'decision': 'block',
    'reason': f'[dag-validation-guard] DAG 验证失败（dag_sync 自动修复后仍失败）: {output[:200]} | sync: {sync[:100]}'
}, ensure_ascii=False))
" "$OUTPUT2" "$SYNC_OUTPUT"
  exit 0
fi

python -c "
import json, sys
output = sys.argv[1]
print(json.dumps({
    'decision': 'block',
    'reason': f'[dag-validation-guard] DAG 验证失败: {output[:300]}'
}, ensure_ascii=False))
" "$OUTPUT"
exit 0
