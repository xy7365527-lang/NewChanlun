---
id: '225'
number: 225
title: ceremony rescan→evaluate 原子性断裂——rescan 完成后被用户消息中断
type: meta-rule
status: 已结算
date: 2026-02-27
depends_on:
  - '224'   # push→rescan 原子性断裂（同构问题，扩展修复）
  - '137'   # 否定性禁令对行为执行层无效
tensions_with: []
topo_effect: "post-commit-flow.md 新增 rescan→evaluate 原子性规则 + flow-continuity-guard.sh 覆盖 ceremony_scan.py"
---

# 225号：ceremony rescan→evaluate 原子性断裂

## 现象

224号修复了 push→rescan 的原子性断裂，但 Lead 在 rescan 完成后再次"断掉"：
- ceremony_scan.py --phase rescan 执行完成，输出了 JSON
- Lead 没有评估 workstations 并执行步骤9/10
- 而是被编排者消息吸引走了

编排者观察："你又断了"。

## 根因

224号修复不完整——只覆盖了原子链的前半段（push→rescan），后半段（rescan→evaluate）有同样的漏洞：

1. **flow-continuity-guard.sh 不覆盖 ceremony_scan.py**：hook 只检测 `git commit` 和 `git push`，ceremony_scan.py 作为 Bash 调用执行后没有守卫注入继续执行指令
2. **post-commit-flow.md 缺口**：只规定了 push→rescan 原子性，没有规定 rescan→evaluate 原子性
3. **同构问题**：224号和225号是同一个结构性问题的两个实例——ceremony 步骤之间缺少原子性保证

## 修复（与224号同构的三层联动）

1. **hook 层**：flow-continuity-guard.sh 扩展覆盖 ceremony_scan.py，rescan 完成后注入正面指令"立即处理扫描结果"
2. **规则层**：post-commit-flow.md 新增 rescan→evaluate 原子性规则
3. **完整原子链声明**：步骤 7→8→9→10 是一条不可中断的原子链

## 边界条件

- 与224号相同：INTERRUPT 优先级高于原子性，普通消息不构成 INTERRUPT
- 原子链覆盖范围：push(7)→rescan(8)→evaluate(9)→terminate/spawn(10)

## 下游推论

1. 224号和225号共同确立：ceremony 步骤 7-10 是完整的原子块，不可被用户消息中断
2. 如果未来发现其他 ceremony 步骤之间有同样的断裂，修复模式相同：hook 守卫 + 规则声明 + 正面指令
