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
#   096号：无例外——所有 Task 调用必须通过 Agent Team
#   本号：team 拓扑持久化——git 中的拓扑定义 → ceremony 时自动重建运行时 team

set -uo pipefail

# 将 stdin 保存到临时文件，通过环境变量传递给 python
HOOK_TMPFILE=$(mktemp)
trap 'rm -f "$HOOK_TMPFILE"' EXIT
cat > "$HOOK_TMPFILE"

# 全部逻辑在 python 中完成（通过环境变量传递临时文件路径）
HOOK_INPUT="$HOOK_TMPFILE" python3 << 'PYEOF'
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
lines.append('[Agent Team Bootstrap] 蜂群拓扑持久化重建指令（095/096号谱系）')
lines.append('')
lines.append('团队名称：' + prefix)
lines.append('')
lines.append('=== 强制执行 ===')
lines.append('ceremony 完成后的第一个动作：并行 spawn 结构工位 teammates。')
lines.append('所有 Task 调用使用 team_name="' + prefix + '"。')
lines.append('首次 Task 调用时 team 自动创建（无需显式 TeamCreate）。')
lines.append('')
lines.append('自动 spawn ' + str(len(auto_agents)) + ' 个结构工位（并行）：')

for a in auto_agents:
    lines.append('  - Task(team_name="' + prefix + '", subagent_type="' + a['name'] + '", description="' + a['role'] + '")')

lines.append('')
lines.append('按需 spawn ' + str(len(on_demand)) + ' 个工位（不预启动）：')
for a in on_demand:
    trigger = a.get('trigger', '按需')
    lines.append('  - ' + a['name'] + '（' + a['role'] + '）← ' + trigger)

lines.append('')
lines.append('=== 约束 ===')
lines.append('- 096号谱系：所有 Task 调用必须携带 team_name，无例外')
lines.append('- 拓扑定义来源：.claude/team-topology.json（git 持久化）')
lines.append('- 运行时 team 实例：session 级别，每次 ceremony 自动重建')

msg = '\n'.join(lines)

print(json.dumps({
    'continue': True,
    'suppressOutput': False,
    'systemMessage': msg
}, ensure_ascii=False))
PYEOF
