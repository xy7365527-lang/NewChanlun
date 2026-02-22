#!/usr/bin/env bash
# PostToolUse hook: TeamCreate 后强制导向 sub-swarm-ceremony skill
# 075号更新：不再注入结构工位 spawn 指令，改为 skill 事件驱动架构
# 096号（修正版）：三层存在形式——CLAUDE.md（原则）+ hooks（语法/索引）+ skills（流程结晶）
#   hook 的语义是"Skill 索引器"：强制将 agent 注意力导向 sub-swarm-ceremony skill 文件
#   不是"信息提示"，是"规则存在形式的第三层——结晶知识的入口"

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

# 096号修正（Gemini decide，选项C）：
# hook = Skill 索引器，不是信息提示
# 子蜂群行为规则的完整操作流程在 skill 文件中结晶，必须读取
python -c "
import json, sys

team = '$team_name'
skill_path = '.claude/skills/sub-swarm-ceremony/SKILL.md'

msg = '[096号修正——三层存在形式] 蜂群 ' + team + ' 已创建。\n\n子蜂群创建规则以 skill 形式结晶（原则11：skill = 知识维度结晶）。\n你必须读取并遵循：' + skill_path + '\n\nskill 包含：\n- 何时触发（子任务可分解时）\n- 完整创建步骤（TeamCreate → TaskList → spawn → 监控 → 清理）\n- 四特征（拓扑 + 异步自指 + 结晶 + 状态管理）\n\n这不是建议，是蜂群规则存在方式的第三层（CLAUDE.md 原则 → hooks 语法守卫 → skill 操作流程）。'

print(json.dumps({'decision': 'allow', 'reason': msg}, ensure_ascii=False))
"
