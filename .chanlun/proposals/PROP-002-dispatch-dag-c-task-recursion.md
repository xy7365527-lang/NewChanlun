---
proposal_id: PROP-002
title: dispatch-dag.yaml recursion_rules/fractal_template 改 (c) 任务-DAG 递归
scope:
  - .chanlun/dispatch-dag.yaml  # genome 文件——recursion_rules + fractal_template + genealogy_ref
genealogy_ref:
  - settled/069-recursive-topology-async-self-referential-swarm.md  # 层面3：结构修改走提案模式；矛盾2/5
  - settled/095-agent-team-recursive-swarm.md  # 被 (c) 扬弃机制的子蜂群 TeamCreate 模型
  - settled/097-strict-rtas-architecture.md  # 五特征最小 DAG 模板
  - settled/153-rtas-swarm-persistence-gap.md  # 循环持久化缺口
motivation: |
  编排者裁决 (c)（2026-06-23）：harness 硬约束 teammate 不能 spawn teammate（flat roster），
  因此 worker 递归不能走 TeamCreate 子蜂群，必须改为 worker 自己 TaskCreate 子任务（unowned），
  Lead 扫无主任务 spawn 工位。dispatch-dag.yaml 的 recursion_rules / fractal_template 当前仍声明
  "teammate 必须 spawn 子蜂群（TeamCreate）"——这是 flat-roster harness 禁止的 stale 模型
  （已上浮的 spec-execution gap）。需更新为 (c) 任务-DAG 递归模型。

  本提案是 069号层面3（结构修改走提案模式）+ 矛盾2（genome 修改走提案/批准）的合规路径：
  dispatch-dag.yaml 是 genome 拓扑定义，编排者要求"genome 走 020-gated 提案，不擅改"。
  rtas-infra-fix 曾直接编辑此文件，已 revert，改为本提案。
changes: |
  对 .chanlun/dispatch-dag.yaml 的三处变更（diff 见下"详细变更"节）：

  1. fractal_template.recursion_rules（约 651-657 行）：
     - 删 "任何 teammate 面对 ≥2 个可并行子任务时，必须 spawn 子蜂群（095号）"
       → "必须 TaskCreate 子任务（unowned）向下递归（(c)裁决；095号真递归精神保留，机制从子 team 改为子任务 DAG）"
     - 删 "子蜂群通过 TeamCreate 创建独立 team"
       → "子任务 DAG 在共享 TaskList 中布设：四类节点 + blockedBy 边 + metadata.agent_type（097号五特征）"
     - 加 "teammate 不 spawn teammate（flat roster 硬约束）；Lead 是唯一 spawn 源——扫无主任务 spawn 工位"
     - "扁平 Lead→Worker 退化特例" → "扁平直接执行是原子性退化特例"
     - 不动点定义加 "TaskList 无无主/未阻塞/in_progress 任务"
     - "父 teammate 对子 teammates 负路由责任" → "对其 TaskCreate 的子任务负 SendMessage 汇报责任（parent_callback）"
     - 结构工位例外 "spawn 同类型子工位" → "TaskCreate 同类型子任务"

  2. fractal_template.description + genealogy_ref（约 630-631 行）：
     - description 改为 "子蜂群 = 子任务 DAG（(c)裁决：TaskCreate 子任务，非 TeamCreate 子 team）"
     - genealogy_ref 加 "(c)-task-dag-recursion-20260623"

  3. 顶层 genealogy_ref（第 20 行）：加 "(c)-task-dag-recursion-20260623"
approval_condition: |
  - meta-observer 注入审查任务（不自行 spawn，069号矛盾6/8）
  - gemini-challenger 高阶元原则审查（069号矛盾5：结构修改由 gemini 高阶审查，避免 quality-guard t/t+1 保守死锁）
  - 编排者 INTERRUPT 覆盖权保留
  批准后由具写权限工位实施，status 改 implemented。
status: pending_review
review_date: ""
reviewer: ""
review_verdict: ""
---

## 详细变更（reverted 自 rtas-infra-fix 的直接编辑，改为提案）

### 变更1：recursion_rules（fractal_template 下）

**当前（stale）**：
```yaml
  recursion_rules:
    - "任何 teammate 面对 ≥2 个可并行子任务时，必须 spawn 子蜂群（095号：真递归是默认模式）"
    - "子蜂群通过 TeamCreate 创建独立 team，复制父蜂群的完整结构（097号五特征 DAG 模板）"
    - "扁平 Lead→Worker 是退化特例，使用需要理由——必须证明子任务不需要独立治理"
    - "递归终止条件仅两个（274号）：原子性（不可分解）和不动点（无新工位）。无外部计数器。context window 耗尽→compaction→恢复继续（暂停，不是终止）"
    - "父 teammate 对子 teammates 负路由责任"
    - "结构工位默认扁平——裁判不参与走势。例外：结构工位自身任务需分解时允许 spawn 同类型子工位（受限递归），不允许 spawn 任务工位"
```

**提案（(c)）**：
```yaml
  recursion_rules:
    # (c)裁决（2026-06-23，编排者）：递归在任务结构里（TaskCreate 子任务），不在 team 结构里。
    # harness 硬约束：teammate 不能 spawn teammate（flat roster）→ 扬弃旧 TeamCreate 子蜂群模型。
    - "任何 teammate 面对 ≥2 个可并行子任务时，必须 TaskCreate 子任务（unowned）向下递归（(c)裁决；095号真递归精神保留，机制从子 team 改为子任务 DAG）"
    - "子任务 DAG 在共享 TaskList 中布设：四类节点（任务/审查/异质审计/结晶）+ blockedBy 边 + metadata.agent_type（097号五特征 DAG 模板）"
    - "teammate 不 spawn teammate（flat roster 硬约束）；Lead 是唯一 spawn 源——扫无主任务 spawn 工位"
    - "扁平直接执行是原子性退化特例，使用需要理由——必须证明子任务不可分解为 ≥2 个独立子单元"
    - "递归终止条件仅两个（274号）：原子性（不可分解）和不动点（TaskList 无无主/未阻塞/in_progress 任务）。无外部计数器。context window 耗尽→compaction→恢复继续（暂停，不是终止）"
    - "父 teammate 对其 TaskCreate 的子任务负 SendMessage 汇报责任（parent_callback）"
    - "结构工位默认扁平——裁判不参与走势。例外：结构工位自身任务需分解时 TaskCreate 同类型子任务（受限递归），不创建任务工位"
```

### 变更2：fractal_template description + genealogy_ref

**当前**：
```yaml
fractal_template:
  genealogy_ref: "037-recursive-swarm-fractal, 069-recursive-topology-async-self-referential-swarm, 075-structural-to-skill"
  description: "子蜂群实例化时，skill 全局可用（事件驱动），无需继承结构 teammates"
```

**提案**：
```yaml
fractal_template:
  genealogy_ref: "037-recursive-swarm-fractal, 069-recursive-topology-async-self-referential-swarm, 075-structural-to-skill, (c)-task-dag-recursion-20260623"
  description: "子蜂群 = 子任务 DAG（(c)裁决：TaskCreate 子任务，非 TeamCreate 子 team）；skill 全局可用（事件驱动）"
```

### 变更3：顶层 genealogy_ref（第 20 行）

末尾追加 `, (c)-task-dag-recursion-20260623`。

## 与两个 Gap 的关系（069号）

- **创世 Gap**：本提案不触碰 Swarm₀/ceremony source 节点——Lead/CC 仍是游离于拓扑外的 bootstrap 节点，
  本提案只改 worker 递归机制（teammate 层），不把 bootstrap 节点纳入循环。
- **视差 Gap**：本提案不消除异步时间差——worker TaskCreate 与 Lead spawn 之间的时间差（Stop-Guard 在
  下一个 Stop 才扫到无主任务）是结构性的，保留。

## 影响声明

本提案改 genome（dispatch-dag.yaml）3 处。配套的非-genome 实装（已直接落地，不在本提案范围）：
ceremony.md、sub-swarm-ceremony.md（skill）、agent-team-bootstrap.sh、ceremony-completion-guard.sh（hooks）。
本提案批准实施后，genome 与配套实装一致；批准前 genome 保持 stale（020-gated 不擅改的代价，符合 069 层面3）。
