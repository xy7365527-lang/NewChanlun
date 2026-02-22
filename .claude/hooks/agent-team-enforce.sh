#!/usr/bin/env bash
# PreToolUse Hook — Agent Team 强制（095号谱系，096号无例外修正）
#
# 触发：PreToolUse on "Task" tool
# 逻辑：
#   1. 检查 Task 调用是否包含 team_name 参数
#   2. 如果没有 team_name → block + 要求使用 Agent Team
#   3. 无例外（096号谱系）：Task(Explore) 同样需要 team_name
#      搜索任务改用 Glob/Grep/Read 直接工具，或在 team 内 spawn 搜索 teammate
#
# 095号谱系：严格使用 Agent Team，不使用孤立 subagent
# 096号谱系：无例外——规则是语法规则，例外使规则降级为软性建议
# 016号谱系：规则没有代码强制就不会被执行

set -uo pipefail

INPUT=$(cat)

TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
data = json.loads(sys.stdin.read())
print(data.get('tool_name', ''))
" 2>/dev/null || echo "")

# 只拦截 Task 工具
if [ "$TOOL_NAME" != "Task" ]; then
    exit 0
fi

# 检查 team_name（无例外）
python -c "
import sys, json

data = json.loads(sys.stdin.buffer.read().decode('utf-8'))
tool_input = data.get('tool_input', {})

team_name = tool_input.get('team_name', '')

# 如果没有 team_name → 阻断（无例外，096号谱系）
if not team_name:
    subagent_type = tool_input.get('subagent_type', '')
    if subagent_type == 'Explore':
        reason = ('[096号 无例外 Agent Team 强制] Task(Explore) 不允许作为孤立 subagent。'
                  '需要搜索时：(1) 简单搜索 → 直接使用 Glob/Grep/Read 工具；'
                  '(2) 多轮自主搜索 → 在当前 team 内 spawn 搜索 teammate（这是真正的子任务，属于 team 范畴）。'
                  '规则来源：096号谱系（蜂群规则的分布式存在形式——无例外）。')
    else:
        reason = ('[096号 无例外 Agent Team 强制] Task 调用缺少 team_name 参数。'
                  '所有 Task 调用必须通过 Agent Team（TeamCreate + Task with team_name）管理，'
                  '不允许孤立 subagent。请先 TeamCreate 创建 team，然后使用 Task(team_name=xxx) spawn teammate。'
                  '无例外——包括 Explore 类型（096号谱系）。')
    print(json.dumps({
        'decision': 'block',
        'reason': reason
    }, ensure_ascii=False))
else:
    sys.exit(0)
" <<< "$INPUT"
