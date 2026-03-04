#!/bin/bash
# test_hook_input.sh — 打印完整 INPUT_JSON 到日志文件
# 用于调查 Claude Code PostToolUse hook 的 INPUT_JSON 是否携带 agent 标识
# 228号谱系下游推论2 调查工具
#
# 使用方法：
#   将此脚本注册为 PostToolUse hook，触发任意工具后查看日志文件
#   日志路径：tmp/hook_input_dump.log

set -euo pipefail

INPUT=$(cat)
LOGFILE="$(cd "$(dirname "$0")/.." && pwd)/tmp/hook_input_dump.log"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date +"%Y-%m-%dT%H:%M:%S")

{
  echo "=== $TIMESTAMP ==="
  echo "--- INPUT_JSON (stdin) ---"
  echo "$INPUT" | python -c "
import sys, json, os

raw = sys.stdin.read()
try:
    data = json.loads(raw)
    # 打印所有顶层 key
    print('Top-level keys:', sorted(data.keys()))
    print()
    # 逐个打印（排除 tool_result 中的大 content）
    for k, v in sorted(data.items()):
        if k == 'tool_result':
            # 只打印 tool_result 的 key，不打印完整内容（太大）
            if isinstance(v, dict):
                print(f'{k}: (dict with keys: {sorted(v.keys())})')
            else:
                print(f'{k}: (type={type(v).__name__})')
        else:
            print(f'{k}: {json.dumps(v, ensure_ascii=False, indent=2)}')
    print()
    # 特别检查 agent 相关字段
    agent_fields = [k for k in data.keys() if 'agent' in k.lower()]
    print(f'Agent-related fields in INPUT_JSON: {agent_fields if agent_fields else \"NONE\"}')
    print()
    # 检查环境变量中的 agent 标识
    agent_env = {k: v for k, v in os.environ.items() if 'agent' in k.lower() or 'claude' in k.lower()}
    print(f'Agent-related env vars: {agent_env if agent_env else \"NONE\"}')
except Exception as e:
    print(f'Parse error: {e}')
    print(f'Raw input (first 500 chars): {raw[:500]}')
" 2>&1
  echo ""
} >> "$LOGFILE"

# 静默退出，不影响正常 hook 流程
exit 0
