#!/usr/bin/env bash
# PreToolUse Hook — Agent Team 强制（095号谱系，096号无例外修正）
#
# 触发：PreToolUse on "Task" tool
# 逻辑：
#   1. 检查 Task 调用是否包含 team_name 参数
#   2. 如果没有 team_name → block + 要求使用 Agent Team
#   3. 无例外（096号谱系）：Task(Explore) 同样需要 team_name
#      搜索任务改用 Glob/Grep/Read 直接工具，或在 team 内 spawn 搜索 teammate
#   4. [spec-gap-audit] 检查 spawn prompt 是否包含三基因（073a号：topo_address/depth_budget/parent_callback）
#      缺失时 advisory 警告（不阻断——避免032号死锁重演）
#
# 095号谱系：严格使用 Agent Team，不使用孤立 subagent
# 096号谱系：无例外——规则是语法规则，例外使规则降级为软性建议
# 016号谱系：规则没有代码强制就不会被执行
# 073a号谱系：spawn 三基因——topo_address, depth_budget, parent_callback

set -uo pipefail

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

python() { command "$PYTHON_BIN" "$@"; }

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
        'hookSpecificOutput': {
            'hookEventName': 'PreToolUse',
            'permissionDecision': 'deny',
            'permissionDecisionReason': reason
        }
    }, ensure_ascii=False))
else:
    # --- 三基因检查（073a号谱系，spec-gap-audit 修复） ---
    # Task 有 team_name → 放行，但检查 prompt 是否包含三基因
    # 仅对非 Explore 类型检查（Explore 是搜索，不是 spawn）
    subagent_type = tool_input.get('subagent_type', '')
    if subagent_type != 'Explore':
        prompt = tool_input.get('prompt', '') or tool_input.get('description', '') or ''
        missing_genes = []
        if 'topo_address' not in prompt.lower() and '拓扑坐标' not in prompt:
            missing_genes.append('topo_address')
        if 'depth_budget' not in prompt.lower() and '递归深度' not in prompt and '深度预算' not in prompt:
            missing_genes.append('depth_budget')
        if 'parent_callback' not in prompt.lower() and '父节点回调' not in prompt:
            missing_genes.append('parent_callback')

        # 递归判断块检测（ceremony 步骤5 强制注入）
        has_recursion_block = ('递归判断' in prompt or 'sub-swarm-ceremony' in prompt.lower())
        missing_recursion = not has_recursion_block

        messages = []
        if missing_genes:
            messages.append(
                f'[073a号 spawn 三基因] Task prompt 缺少基因: {", ".join(missing_genes)}。'
                f'dispatch-dag task_template 要求每个衍生节点携带三基因：'
                f'topo_address（拓扑坐标）, depth_budget（递归深度预算）, parent_callback（父节点回调）。'
                f'请在 prompt 中注入这些信息（见 .claude/commands/ceremony.md 步骤5 递归判断块）。')
        if missing_recursion:
            messages.append(
                f'[递归判断缺失] Task prompt 未包含递归判断块。'
                f'ceremony 步骤5 要求每个工位 prompt 开头包含递归判断指令（评估任务是否可分解为子蜂群）。'
                f'原则15：真递归是默认模式，扁平执行是退化特例。'
                f'请在 prompt 开头注入递归判断块（见 .claude/commands/ceremony.md 步骤5）。')
        if messages:
            print(json.dumps({'systemMessage': ' | '.join(messages)}, ensure_ascii=False))
        else:
            sys.exit(0)
    else:
        sys.exit(0)
" <<< "$INPUT"
