#!/usr/bin/env bash
# SessionStart Hook — Agent Team 自动 bootstrap（持久化 team 拓扑重建）
#
# 触发：SessionStart（在 session-start-ceremony.sh 之后）
# 逻辑：
#   1. 读取 .claude/team-topology.json 获取持久化拓扑定义
#   2. 生成 systemMessage 指令让 Lead 在 ceremony 第一步创建 Agent Team
#   3. 拓扑定义通过 git 持久化，team 运行时实例通过 ceremony 重建
#
# 谱系依据：
#   095号：Agent Team 真递归是默认模式
#   096号：无例外——所有工位 spawn 通过 Agent tool（带 name = teammate）
#   562号：结构=teammate，ceremony 必 spawn 6 常设结构工位
#   (c)裁决（2026-06-23，编排者）：递归在任务结构里（TaskCreate 子任务），Lead 只编排。
#     harness 硬约束：teammate 不能 spawn teammate（flat roster）→ 子蜂群=子任务，非子 teammate。
#   本号：team 拓扑持久化——git 中的拓扑定义 → ceremony 时自动重建运行时 team

set -uo pipefail

# 将 stdin 保存到临时文件，通过环境变量传递给 python
HOOK_TMPFILE=$(mktemp)
trap 'rm -f "$HOOK_TMPFILE"' EXIT
timeout 3 cat > "$HOOK_TMPFILE" 2>/dev/null || echo "{}" > "$HOOK_TMPFILE"

# 全部逻辑在 python 中完成（通过环境变量传递临时文件路径）
HOOK_INPUT="$HOOK_TMPFILE" python << 'PYEOF'
import json, sys, os

tmpfile = os.environ['HOOK_INPUT']
with open(tmpfile) as f:
    data = json.load(f)

cwd = data.get('cwd', '.')
try:
    os.chdir(cwd)
except OSError:
    pass

topology_path = '.claude/team-topology.json'

if not os.path.isfile(topology_path):
    print(json.dumps({
        'continue': True,
        'suppressOutput': True
    }, ensure_ascii=False))
    sys.exit(0)

with open(topology_path) as f:
    topo = json.load(f)

prefix = topo.get('team_name_prefix', 'swarm')

structural = topo.get('structural_agents', {}).get('agents', [])
auto_agents = [a for a in structural if a.get('auto_spawn', False)]
on_demand = topo.get('on_demand_agents', {}).get('agents', [])

if not auto_agents:
    print(json.dumps({
        'continue': True,
        'suppressOutput': True
    }, ensure_ascii=False))
    sys.exit(0)

lines = []
lines.append('[Agent Team Bootstrap] 蜂群拓扑持久化重建指令（095/096/562号 + (c)裁决）')
lines.append('')
lines.append('=== 隐式 team（harness 已演化，2026-06-23 实测）===')
lines.append('单一隐式 team，首次 Agent spawn 自动加入。无需 TeamCreate，team_name 已废弃不传。')
lines.append('')
lines.append('=== 强制执行：结构工位（常设，(c)循环的常设部分）===')
lines.append('ceremony 完成后的第一个动作：并行 spawn ' + str(len(auto_agents)) + ' 个结构工位 teammates。')
lines.append('缺任一 → Stop-Guard 检查 1.5 机制性 block（562号）。')
lines.append('')

for a in auto_agents:
    lines.append('  - Agent(name="' + a['name'] + '", subagent_type="' + a['name']
                 + '", run_in_background=true)  # ' + a['role'])

lines.append('')
lines.append('=== 按需工位（task-triggered，(c)循环的按需部分）===')
lines.append('按需工位不预启动。(c) 机制：工位识别到需要某类能力时，TaskCreate 一个子任务，')
lines.append('在 metadata.agent_type 标注所需 subagent_type；Lead 的循环 spawn 该类型工位。')
lines.append('可用 ' + str(len(on_demand)) + ' 类按需工位：')
for a in on_demand:
    trigger = a.get('trigger', '按需')
    lines.append('  - ' + a['name'] + '（' + a['role'] + '）← ' + trigger)

lines.append('')
lines.append('=== (c) 持久循环（编排者裁决 2026-06-23）===')
lines.append('Lead 只编排，不做实质工作。循环：')
lines.append('  1. TaskList 扫任务列表')
lines.append('  2. 为每个 owner 为空且未阻塞的 pending 任务 spawn 一个工位 Agent')
lines.append('     （subagent_type 读 metadata.agent_type，缺省 general-purpose）')
lines.append('  3. 工位执行 + 识别子工作时自己 TaskCreate 子任务（=向下递归，非 spawn 子 teammate）')
lines.append('  4. re-scan（TaskList）→ 回到 2')
lines.append('  5. 无无主/未阻塞/in_progress 任务 → 不动点终止')
lines.append('')
lines.append('=== 约束 ===')
lines.append('- (c)裁决：teammate 不 spawn teammate（flat roster）；递归=TaskCreate 子任务，Lead spawn')
lines.append('- 拓扑定义来源：.claude/team-topology.json（git 持久化）')
lines.append('- 运行时 team 实例：session 级别，每次 ceremony 自动重建')

msg = '\n'.join(lines)

# 通道修复（同 session-start-ceremony.sh）：team 重建指令是可执行行为内容，
# 必须经 hookSpecificOutput.additionalContext 进入模型 context（systemMessage 只给用户横幅、
# 不进 context、被 compact summary 的 "Resume directly" 压过 → 095/096 在 compact 后失效根因）。
print(json.dumps({
    'continue': True,
    'suppressOutput': False,
    'hookSpecificOutput': {
        'hookEventName': 'SessionStart',
        'additionalContext': msg
    },
    'systemMessage': '[Agent Team Bootstrap] 拓扑重建指令已注入 context（'
                     + str(len(auto_agents)) + ' 结构工位 + ' + str(len(on_demand)) + ' 按需）'
}, ensure_ascii=False))
PYEOF
