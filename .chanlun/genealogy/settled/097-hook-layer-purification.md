---
id: "097"
title: "hook 层纯化——三层存在形式的边界精化"
status: 已结算
type: 选择（Gemini decide 选项D）
negation_source: gemini-3.1-pro-preview
created: "2026-02-22"
---

## 决断背景

096号确立蜂群规则的三层分布式存在形式：CLAUDE.md（声明层）+ hooks（强制层）+ skills（知识层）。

确立后，编排者 INTERRUPT："如果是严格地这样存在，那么 team-structural-inject.sh 这个 hook 做提示/索引是不是就不够优雅和干净了？"

`team-structural-inject.sh` 是 PostToolUse hook（TeamCreate 后触发），功能是输出文字告诉 agent "必须读取 sub-swarm-ceremony skill"。

该功能的问题：
- 不是"阻断"（allow 了 TeamCreate）
- 不是"声明"（CLAUDE.md 已有原则15）
- 不是"执行"（不含可执行流程）

它是夹在三层之间的"提示/索引"第四层——违反三层纯粹边界。

## 四个选项

| 选项 | 描述 |
|------|------|
| A | 删除 hook——三层已自足 |
| B | 保留但改为纯列表输出（知识索引） |
| C | 改为 PreToolUse 验证阻断 |
| D | 删除 hook，将 skill 路径写入 CLAUDE.md |

## Gemini 决断：选项 D

**推理链**：
1. hook 层的动作语义必须是"阻断/放行"，绝不能是"提示/教学"
2. CLAUDE.md 作为声明层，不仅应声明"是什么"，也理应包含指向"怎么做"的入口（skill 路径）——类比基因序列中的启动子（Promoter），指引转录过程
3. 将 skill 索引归入 CLAUDE.md，彻底消除夹在三层之间的"第四层（提示层）"

**边界条件**（Gemini 提供）：如果未来 skills 数量激增导致 CLAUDE.md 中 skill 索引列表过长，此决策应被推翻——届时引入"路由 skill"或"目录文件"接管索引，CLAUDE.md 只保留指向目录的唯一指针。

**风险**（Gemini 提供）：agent 可能忽略 CLAUDE.md 中附带的 skill 路径，导致子蜂群仪式被遗漏。

## 执行结果

- 删除：`.claude/hooks/team-structural-inject.sh`
- 修改：`CLAUDE.md` 原则15，更新三层存在形式描述，明确 hook 只做阻断/放行，CLAUDE.md 包含 skill 路径索引（`.claude/skills/sub-swarm-ceremony/SKILL.md`）

## 推导链

096号（三层存在形式确立）→ 编排者 INTERRUPT（hook 提示/索引不纯粹）→ Gemini decide 选项D → 097号（hook 层纯化）

## 谱系链接

- 前置：096号（三层存在形式的确立）
- 关联：090号（严格性是蜂群语法规则）——hook 的动作语义必须严格
- 参照：005b号（语法规则的先例——对象否定对象）

## 影响声明

- 删除 `.claude/hooks/team-structural-inject.sh`（1个文件删除）
- 修改 `CLAUDE.md` 原则15（三层存在形式描述精化，加 097号谱系引用）
- 三层存在形式从"096号确立"进入"097号精化"状态：hook 层纯粹性得到强制保障
