# 元规则观察报告：refactor-batch1 蜂群

**观察者**: meta-observer 结构工位
**日期**: 2026-02-20
**蜂群**: refactor-batch1
**type**: meta-rule

## 蜂群组成

| 类型 | 工位 | Task ID | 状态 |
|------|------|---------|------|
| 结构（mandatory） | genealogist | #17 | in_progress |
| 结构（mandatory） | quality-guard | #18 | in_progress |
| 结构（mandatory） | meta-observer | #19 | in_progress |
| 任务 | recursion-refactor | #4 | in_progress（子任务 #10-15） |
| 任务 | algo-refactor | #5 | in_progress（子任务 #7-9） |
| 任务 | tf-annotator | #6 | in_progress（子任务 #3） |

## 元规则合规检查

### 033号（dispatch-spec）：通过

3 个 mandatory 结构工位全部 spawn。验证条件 `spawn 数量 == structural_stations.length` 满足。

### 032号（Lead 只做路由）：通过

Task list 中所有实际工作均由任务工位和结构工位执行。Lead 持有的 #1、#2 是父级分组任务（pending），未自行执行任何重构。

### 037号（递归蜂群）：部分适用

- recursion-refactor 将 6 个 diff 函数分解为 6 个子任务（#10-15）
- algo-refactor 将 3 个大函数分解为 3 个子任务（#7-9）
- 子任务在共享 task list 中可见（符合 visibility 规则）

**待确认**：子任务是否由独立子 teammates 并行执行，还是由父工位顺序执行？037 规定"可 spawn"而非"必须 spawn"，但如果 6 个独立子任务被顺序执行，则未充分利用递归蜂群能力。这不是违规，但是效率观察。

## 发现的张力

### 张力 1：meta-observer agent 定义与 dispatch-spec 矛盾

| 来源 | 内容 |
|------|------|
| `.claude/agents/meta-observer.md:97` | "不发 SendMessage（所有产出走文件系统）" |
| `dispatch-spec.yaml:41` | tools_required 包含 SendMessage |
| team-lead dispatch 指令 | "SendMessage 给 team-lead 汇报元规则观察结果" |

**分析**：agent 定义写于 dispatch-spec v1.0 之前。dispatch-spec v1.2 为所有结构工位统一添加了 SendMessage（037号谱系要求）。agent 定义中的"不发 SendMessage"约束未同步更新。

**四分法分类**：定理（已结算原则的逻辑推论）。dispatch-spec 是 033 号谱系的产物，优先级高于 agent 定义中的旧约束。agent 定义需要更新以消除矛盾。

**建议修复**：删除 meta-observer.md 第 97 行的"不发 SendMessage"约束，或改为"优先通过文件系统产出，必要时使用 SendMessage 与 Lead 通信"。

### 无其他张力

本次蜂群为纯技术重构（大函数拆分），不涉及域概念定义变更。所有任务属于 018 四分法中的"行动"（l'acte）类型，无概念层信号需要捕获。

## 未显式化规则扫描（语法记录候选）

本次观察未发现新的隐性规则在运作。蜂群行为与 dispatch-spec 声明一致。

## 边界条件

- 如果重构过程中某个 diff 函数的拆分暴露了域概念定义问题（如"中枢"的边界条件在代码中被隐式假设），则需要从"行动"升级为"选择"或"语法记录"，走上浮流程
- 如果 meta-observer agent 定义不更新，后续蜂群中 meta-observer 的通信方式将持续处于矛盾状态
