#!/usr/bin/env bash
# PostToolUse hook: TeamCreate 后自动注入结构工位提示
# 从 dispatch-dag.yaml 动态读取 mandatory structural nodes
# 不硬编码——结构工位数量和名称由 DAG 定义决定

set -uo pipefail

input=$(cat)
tool_name=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('tool_name',''))" 2>/dev/null || echo "")

[ "$tool_name" != "TeamCreate" ] && exit 0

cwd=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('cwd','.'))" 2>/dev/null || echo ".")
cd "$cwd" 2>/dev/null || true

team_name=$(echo "$input" | python -c "
import sys, json, re
d = json.loads(sys.stdin.read())
r = d.get('tool_result', '')
if isinstance(r, dict):
    print(r.get('team_name', '')); exit()
if isinstance(r, str):
    m = re.search(r'\"team_name\":\s*\"([^\"]+)\"', r)
    if m: print(m.group(1))
" 2>/dev/null || echo "")

[ -z "$team_name" ] && exit 0

# 从 dispatch-dag.yaml 动态读取 mandatory structural nodes
python -c "
import json, yaml, sys

dag_path = '.chanlun/dispatch-dag.yaml'
try:
    with open(dag_path, encoding='utf-8') as f:
        dag = yaml.safe_load(f)
except:
    # DAG 不存在时静默放行
    sys.exit(0)

nodes = dag.get('nodes', {})
structural = nodes.get('structural', [])
mandatory = [n for n in structural if n.get('mandatory', False)]

if not mandatory:
    sys.exit(0)

lines = []
for i, n in enumerate(mandatory, 1):
    nid = n['id']
    agent = n.get('agent', f'.claude/agents/{nid}.md')
    purpose = n.get('purpose', '')
    lines.append(f'{i}. {nid}（{agent}）— {purpose}')

node_list = chr(10).join(lines)
count = len(mandatory)

print(json.dumps({
    'decision': 'allow',
    'reason': f'[蜂群拓扑强制] 蜂群 $team_name 已创建。dispatch-dag.yaml 定义了 {count} 个 mandatory structural nodes，必须立即 spawn：\n{node_list}\n每个使用 Task tool: subagent_type=general-purpose, team_name=$team_name, run_in_background=true, mode=bypassPermissions。\n在 spawn 业务 teammates 之前完成。Stop hook 将验证——缺少任何一个，蜂群无法停止。'
}, ensure_ascii=False))
"
