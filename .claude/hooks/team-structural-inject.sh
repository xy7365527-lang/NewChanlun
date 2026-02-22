#!/usr/bin/env bash
# PostToolUse hook: TeamCreate 后输出 skill 可用性提示
# 075号更新：不再注入结构工位 spawn 指令，改为提示 skill 事件驱动架构
# 096号更新：删除规则注入（子蜂群行为规则已在 CLAUDE.md 基因组中，无需 prompt 重复传递）
# 从 dispatch-dag.yaml 的 event_skill_map 读取 structural skill

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

# 从 dispatch-dag.yaml 读取 event_skill_map 中的 structural skill
python -c "
import json, yaml, sys

dag_path = '.chanlun/dispatch-dag.yaml'
try:
    with open(dag_path, encoding='utf-8') as f:
        dag = yaml.safe_load(f)
except Exception:
    sys.exit(0)

skills = dag.get('event_skill_map', [])
structural = [s for s in skills if s.get('skill_type') == 'structural']

if not structural:
    sys.exit(0)

lines = []
for i, s in enumerate(structural, 1):
    sid = s['id']
    triggers = ', '.join(t.get('event', '?') for t in s.get('triggers', []))
    lines.append(f'{i}. {sid} — 触发事件: [{triggers}]')

skill_list = chr(10).join(lines)
count = len(structural)

# 096号：仅信息性提示，不注入规则（规则在 CLAUDE.md 基因组中，分布式自动加载）
print(json.dumps({
    'decision': 'allow',
    'reason': f'[075号 skill 架构] 蜂群 {team_name} 已创建。{count} 个 structural skill 由事件自动触发（无需 spawn teammate）：\n{skill_list}\n直接 spawn 业务工位即可。'
}, ensure_ascii=False))
"
