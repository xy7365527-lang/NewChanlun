---
id: "048"
title: "通用停机阻断（Universal Stop-Guard）"
status: "已结算"
type: "语法记录"
date: "2026-02-20"
depends_on: ["016", "028", "044"]
related: ["027", "045", "046"]
negated_by: []
negates: []
---

# 048 — 通用停机阻断（Universal Stop-Guard）

**类型**: 语法记录
**状态**: 已结算
**日期**: 2026-02-20
**negation_source**: heterogeneous（Gemini 编排者代理 decide + 人类编排者观察）
**negation_form**: expansion（044号"ceremony 专用 Stop hook"泛化为全场景覆盖）
**前置**: 044-phase-transition-runtime, 016-runtime-enforcement-layer, 028-ceremony-default-action
**关联**: 045-no-structure-no-task, 046-one-domain-one-role, 027-positive-instruction-over-prohibition

## 现象

Lead 在蜂群运行中反复掉入"等待汇报"模式。044号修复了 ceremony 完成点，028号修复了 git commit 后，但"等待"从新的场景冒出来——蜂群工位运行中等待、结构工位关闭后等待、任务分派后等待。

逐场景加 hook 是 O(N) 方案，每个新场景需要新 hook。人类编排者指出："我们的体系根本不需要等待，有递归蜂群，遇到问题就可以处理的。"

## Gemini 决策

将 `stop` 的语义从"我没话说了"重定义为"全系统任务队列为空"。用通用 Stop-Guard 替代逐场景 hook，O(1) 覆盖所有场景。

## 推导链

1. 016号：规则没有代码强制就不会被执行——"不要等待"是文本规则，需要 runtime 强制
2. 044号：ceremony 专用 Stop hook 是原型，证明了 Stop hook 机制可行
3. 027号：正面指令优于禁止——不是"禁止等待"，而是"stop 时自动注入下一步扫描指令"
4. Gemini：逐场景 hook = O(N)，通用 guard = O(1)。stop 的语义应该是"任务队列为空"而非"turn 结束"

## 已结算原则

**Stop hook 检查三层条件，任一非空则阻止停止并注入扫描指令：**

1. ceremony 进行中（`.chanlun/.ceremony-in-progress` 标记）
2. 蜂群任务队列有活跃任务（`~/.claude/tasks/` 下 pending/in_progress）
3. 谱系有生成态矛盾（`.chanlun/genealogy/pending/` 非空）

### 熔断机制

连续阻止 >= 5 次且无状态变更 → 允许停止。防止 019a 号的死循环。

### 扫描优先级（注入指令时）

1. 未完成的蜂群任务
2. 生成态谱系矛盾
3. 失败的测试
4. 代码质量扫描

## 被否定的方案

- **逐场景 hook**：O(N) 复杂度，每个新场景需要新 hook，"等待"会从未覆盖的场景冒出
- **纯文本规则**：016号已证明文本层规则不足以对抗生成惯性
- **CLAUDE.md 规则强化**：同上，文本层无 runtime 强制力

## 边界条件

- 用户显式 INTERRUPT → 允许停止（用户权优先）
- Token 熔断 → 允许停止（物理限制）
- pending 目录中的文件可能是已解决但未归档的报告 → 需要定期清理 pending 目录

## 影响声明

- `ceremony-completion-guard.sh` 从 ceremony 专用升级为通用 Stop-Guard
- 044号的 ceremony 检查逻辑被包含在新 guard 的第一层检查中
- hook 网络节点数不变（仍然是同一个 Stop hook），但覆盖范围从单场景扩展到全场景
- pending 目录获得了新的语义：不仅是"未结算的矛盾"，也是"系统是否可以停止"的信号源
