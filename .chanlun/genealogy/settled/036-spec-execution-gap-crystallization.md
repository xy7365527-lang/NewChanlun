---
id: "036"
title: "声明-能力一致性原则结晶：约束链 016→032→033→034 的模式识别"
status: "已结算"
type: "meta-rule（知识结晶）"
date: "2026-02-20"
depends_on: []
related: ["016", "020", "020a", "032", "033", "034"]
negated_by: []
negates: []
derived: ["016", "032", "033", "034"]
---

# 036 — 声明-能力一致性原则结晶：约束链 016→032→033→034 的模式识别

- **状态**: 已结算
- **日期**: 2026-02-20
- **结算日期**: 2026-02-20
- **类型**: meta-rule（知识结晶）
- **来源**: `[新缠论]`

## 矛盾/发现

约束链 016→032→033→034 在四个层级重复发现了同一个同构模式：声明（declaration）与实际能力（capability）的不一致导致系统失效。

| 谱系 | 层级 | 声明 vs 能力 | 净新区分 |
|------|------|-------------|---------|
| 016 | 规则层 | 知道规则 ≠ 执行规则 | 高（首次识别） |
| 032 | 权限层 | 外部强制（hook）→ 自我限制（capability restriction） | 高（范式转换） |
| 033 | 路由层 | 约束（prohibition）→ 规范（specification） | 高（范式转换） |
| 034 | 工具层 | 工具声明 ≠ 工具能力（016 在工具层的同构） | 低（同构复现，净新区分递减） |

034 的净新区分量明显低于 032 和 033，且 034 自身标注了"稳定信号：背驰（3 次验证，净新区分=0）+ 分型（从尝试修复到确认不可修复，范围闭合）"。

## 结晶信号

- **背驰**: 034 的净新区分量 < 033 的净新区分量（发现力度衰竭）
- **分型**: 范围序列 032(权限) < 033(路由+分派) > 034(工具层) → 顶分型成立
- **结论**: 背驰 + 分型 = 结晶条件满足

## 结晶产物

> **声明-能力一致性原则**：系统中任何层级（规则层、权限层、工具层、路由层），声明（declaration）与实际能力（capability）的不一致都会导致同构的失效模式。解法不是在声明层加更多约束（无限维打地鼠），而是正面定义行为空间（specification）+ 物理约束能力边界（capability restriction）。

结晶为 skill: `.claude/skills/spec-execution-gap/SKILL.md`

## 推导链

1. 016 发现：知道规则 ≠ 执行规则 [首次识别]
2. 032 在权限层验证：hook 强制 < capability restriction [范式转换]
3. 033 在路由层验证：prohibition < specification [范式转换]
4. 034 在工具层验证：工具声明 ≠ 工具能力 [同构复现，净新区分递减 = 背驰]
5. 范围序列 032 < 033 > 034 = 顶分型
6. 背驰 + 分型 → 结晶条件满足（020a 知识结晶流程）
7. 压缩为原则：声明-能力一致性 + 双重解法（specification + capability restriction）

## 谱系链接

- 前置: 016-runtime-enforcement-layer（约束链起点）
- 前置: 032-divine-madness-lead-self-restriction（权限层验证）
- 前置: 033-declarative-dispatch-spec（路由层验证）
- 前置: 034-tool-declaration-vs-capability（工具层验证，背驰点）
- 关联: 020a-unified-ontology-statement（结晶流程定义）
- 关联: 020-constitutive-contradiction（构成性矛盾框架）

## 结算条件（全部满足）

1. [x] skill 文件创建完成（`.claude/skills/spec-execution-gap/SKILL.md`）
2. [x] frontmatter 含 `genealogy_source: "036"`
3. [x] 约束链四个谱系的核心洞察被压缩为可复用的模式识别指令（SKILL.md 含"同构模式（四层验证）"表、"检测模式"3 条、"修复模式"2 步）

## 影响声明

- 新增谱系: 本条（036）
- 新增 skill: `.claude/skills/spec-execution-gap/SKILL.md`
- knowledge-crystallization SKILL.md 需承认 016 模式
- genealogist.md 需增加结晶检测职责
