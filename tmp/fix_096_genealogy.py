#!/usr/bin/env python
import sys

path = '.chanlun/genealogy/settled/096-distributed-rules-no-exception.md'
content = open(path, encoding='utf-8').read()

# 修正1：team-structural-inject 描述从"信息性提示"改为"skill 索引器"
old1 = '### 2. team-structural-inject.sh\n降为纯信息性提示：删除规则传递内容（"095号真递归架构要求"等）。规则已在 CLAUDE.md 中，无需通过 prompt 重复注入。'
new1 = '### 2. team-structural-inject.sh\n修正为 skill 索引器：hook 语义从"信息性提示"提升为"强制导向 sub-swarm-ceremony skill"。\n规则的三层存在形式：CLAUDE.md（原则）+ hooks（语法守卫/Skill索引器）+ skills（操作流程结晶）。\nhook 输出必须明确指向 .claude/skills/sub-swarm-ceremony/SKILL.md，强制 agent 读取。\n（096号修正——Gemini decide，选项C，2026-02-22）'

if old1 in content:
    content = content.replace(old1, new1)
    print('OK: 修正 team-structural-inject 描述')
else:
    print('WARN: 未找到 old1，跳过')

# 修正2：影响声明
old2 = '- `.claude/hooks/team-structural-inject.sh`：删除规则传递，保留信息性提示'
new2 = '- `.claude/hooks/team-structural-inject.sh`：修正为 skill 索引器（强制导向 sub-swarm-ceremony skill，096号修正）\n- `.chanlun/dispatch-dag.yaml` knowledge_templates：注册 sub-swarm-ceremony skill（096号修正）'

if old2 in content:
    content = content.replace(old2, new2)
    print('OK: 修正影响声明')
else:
    print('WARN: 未找到 old2，跳过')

open(path, 'w', encoding='utf-8').write(content)
print('写入完成')
