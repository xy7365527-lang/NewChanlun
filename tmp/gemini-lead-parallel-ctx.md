# Gemini Discuss Context: Lead 并行化分析

## 任务

模式：discuss（讨论）

分析 team lead（CC 编排者代理）在 RTAS 循环中的串行残余，提出并行化方案。

## 当前 ceremony_sequence DAG（来自 dispatch-dag.yaml）

### Cold Start 节点依赖图

```
scan-definitions  ─┐
scan-genealogy    ─┤
scan-methodology  ─┼→ derive-work → load-skills → spawn-tasks → divine-madness → recurse
scan-skills       ─┘
```

所有 scan-* 节点 depends_on: []（无依赖，可并行）

### Warm Start 节点依赖图

```
locate-session ─┬→ version-diff  ─┐
                └→ genealogy-diff ─┴→ derive-work → spawn-tasks → divine-madness → recurse
scan-definitions ─→ version-diff
scan-genealogy   ─→ genealogy-diff
```

## 已知规则约束

### post-commit-flow.md（143号谱系）
- commit 后"总结"步骤是 RLHF 停顿点
- 修复：总结嵌入格式A，不独立存在
- 格式：`[commit 摘要] → 接下来：[下一步]` 紧跟工具调用

### no-unnecessary-escalation.md（137号谱系）
- 强制输出格式 A/B/C，不存在第四种格式
- 格式A：`→ 接下来：[行动]` 紧跟工具调用
- 禁止：以总结段落结尾但无行动声明

### swarm-architecture SKILL.md（原则10）
- 蜂群是默认工作模式，不是可选优化
- 每个工作节点必须先评估可并行的独立工位数（≥2 即拉蜂群）
- 单线程顺序执行只在任务间有严格依赖时才允许

### dispatch-dag.yaml task_template rules
- "Lead 不自行执行任务，只分派和汇总"
- "蜂群递归是默认执行模式，不是复杂度触发的优化"

## 分析问题

1. ceremony_sequence 中哪些步骤之间没有数据依赖，可以并行？
   - 已声明并行：4个 scan-* 节点（cold start）
   - 已声明并行：version-diff 和 genealogy-diff（warm start，共享 locate-session 输入）
   - 问题：lead 在实际执行时是否真的并行发出这些调用？

2. lead 的哪些行为是"RLHF 串行惯性"？
   - 先总结再行动（143号已识别）
   - 先等测试再 commit（应 spawn 测试验证工位）
   - 逐个检查工位状态（应批量轮询）
   - commit/push 后等待确认再 re-scan
   - 先关闭所有工位再开始下一轮（shutdown 和 re-scan 可以重叠）

3. 具体并行化方案：
   - ceremony_scan 和 gangju_analysis 可以并行（已经在做）
   - 测试验证应该 spawn 为工位，不是 lead 自己跑
   - commit/push 后的 re-scan 可以与 meta-observer 并行
   - 工位 shutdown 可以批量并行发出（已经在做）

4. 并行化后 ceremony skill 的步骤应该如何重写？

5. 是否需要修改 .claude/rules/ 中的某些规则来支持 lead 并行？

## 关键约束

- Lead 是 DAG 解释器，不是决策者（033号）
- 原则10：≥2 个独立工位即并行，无例外
- 格式A 要求：`→ 接下来` 和工具调用之间不允许插入任何自然语言
- 143号：commit 后的"总结"步骤是 RLHF 停顿点

## 期望输出

针对以上5个问题的分析，重点：
- 识别 ceremony_sequence 中所有可并行的步骤对
- 识别 RTAS 循环中 lead 的串行惯性模式
- 提出具体的并行化改写方案（ceremony DAG 节点级别）
- 判断是否需要新增 rules 规则
