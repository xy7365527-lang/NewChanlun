---
id: "091"
title: "089号后全系统审计——四个拓扑/声明缺口修复"
status: "已结算"
type: "审计修复"
date: "2026-02-21"
depends_on: ["089", "090", "084"]
related: ["075", "073"]
negated_by: []
negates: []
---

# 091 — 089号后全系统审计——四个拓扑/声明缺口修复

- **状态**: 已结算
- **类型**: 审计修复
- **日期**: 2026-02-21
- **negation_source**: Gemini challenge（089号后孤岛扫描 + genome_layer 完整性审计）
- **negation_form**: 异质否定——Gemini 发现四个拓扑/声明不一致
- **前置**: 089, 090, 084
- **关联**: 075, 073

## 命题

089号严格扬弃将 genome_layer 内化后，Gemini 异质否定审计发现四个残留缺口。本次修复消除所有缺口，使 dispatch-dag 与实际文件系统严格一致。

## Gemini 审计四个发现

| # | 严重性 | 发现 | 根因 |
|---|--------|------|------|
| 1 | FATAL | `.claude/skills/`（6个 skill）是完全拓扑孤岛 | dispatch-dag genome_layer 遗漏 knowledge_templates |
| 2 | FATAL | `manifest.yaml` 严重过时——含幽灵 agent、缺 agent、hook 不全 | 历史遗留，未随系统演化同步 |
| 3 | IMPORTANT | genome_layer 遗漏 `.claude/rules/` 目录 | 089号内化时只处理了核心文件，未覆盖语法规则集 |
| 4 | IMPORTANT | `build-error-resolver` 本体论矛盾——有 agent 文件但 dispatch-dag 声明为虚拟节点 | build_resolver 虚拟节点描述不精确 |

## 修复方案

### 修复 #1 + #3：genome_layer 扩展

在 dispatch-dag.yaml 的 genome_layer 中新增两个子节点：

- `grammar_rules`: 覆盖 `.claude/rules/` 下的 6 个语法规则文件（090号：严格性是语法规则）
- `knowledge_templates`: 覆盖 `.claude/skills/` 下的 6 个知识结晶 skill

两者均标注 Claude Code 平台加载机制（物理约束）+ 蜂群内化后自主遵守。

### 修复 #2：manifest.yaml 完全重写

manifest.yaml v1.0 → v2.0，与实际文件系统严格对齐：

| 类别 | v1.0 | v2.0 |
|------|------|------|
| agents | 含幽灵（database-reviewer 等 4 个不存在的 agent） | 19 个（9 event_skill_map + 10 platform_layer），标注 dag_position |
| hooks | 仅 10 个 | 全部 21 个，按 event 类型分组 |
| commands | 未列出 | 13 个 |
| knowledge_skills | 未列出 | 6 个 |
| theorems | 未列出 | 12 个（含验证状态） |

### 修复 #4：build-error-resolver 归位

- 将 `build-error-resolver` 加入 platform_layer（现 10 个 agent）
- 修正 `build_resolver` 虚拟节点描述："构建错误修复触发点（激活 platform_layer 的 build-error-resolver agent），不是独立 agent"

### 附带修复：hooks shebang 损坏

- `definition-write-guard.sh` 第一行 `yange#!/usr/bin/env bash` → `#!/usr/bin/env bash`
- `genealogy-write-guard.sh` 第一行 `e#!/usr/bin/env bash` → `#!/usr/bin/env bash`

## 推导链

1. 089号完成后，编排者要求 Gemini 全系统孤岛扫描
2. Gemini challenge 四维度审计：孤岛检测 + 谱系 RTAS 检查 + 090号一致性 + genome_layer 完整性
3. 发现 #1（FATAL）：skills 6 个文件完全不在 dispatch-dag 中 → 拓扑孤岛
4. 发现 #2（FATAL）：manifest.yaml 含不存在的 agent、缺失实际 agent → 声明-能力严重不一致（084号同构）
5. 发现 #3（IMPORTANT）：genome_layer 遗漏 rules/ → 语法规则不在基因组中（090号矛盾）
6. 发现 #4（IMPORTANT）：build-error-resolver 有 agent 文件但只是虚拟节点 → 本体论身份模糊
7. 交叉验证确认四个发现均成立 → 制定修复方案
8. 四个修复全部执行：dispatch-dag 扩展 + manifest 重写 + build-error-resolver 归位
9. ∴ 系统拓扑/声明一致性恢复

## 边界条件

1. 新增 agent/hook/skill 时，必须同时更新 dispatch-dag 和 manifest.yaml
2. genome_layer 的子节点列表必须覆盖 `.claude/` 下所有被蜂群依赖的目录
3. manifest.yaml 的验证应自动化（目前人工维护，应考虑脚本化扫描生成）

## 影响

1. `.chanlun/dispatch-dag.yaml`: genome_layer 新增 grammar_rules + knowledge_templates，platform_layer 新增 build-error-resolver
2. `.chanlun/manifest.yaml`: 完全重写为 v2.0
3. `.claude/hooks/definition-write-guard.sh`: shebang 修复
4. `.claude/hooks/genealogy-write-guard.sh`: shebang 修复

## 谱系链接

- **089号**（严格扬弃）→ 本次审计的触发点，genome_layer 内化后的完整性检查
- **090号**（严格性语法规则）→ 发现 #3 直接违反 090号——语法规则文件不在基因组中
- **084号**（声明-能力缺口）→ 发现 #2 是 084号模式的再现——manifest 声明与实际能力不一致
- **075号**（结构工位转 skill）→ skills 的拓扑位置正是 075号决策的下游影响
- **073号**（蜂群能修改一切）→ 直接修复，无需仪式门控
