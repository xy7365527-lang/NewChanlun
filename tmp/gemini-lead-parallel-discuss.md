---
trigger: team-lead discuss request
subject: Lead 并行化：识别 RTAS 循环中的串行残余并提出并行化方案
mode: decide
result: pass
timestamp: 2026-02-26
---

# Gemini Decide 结果：Lead 并行化方案

## 一、结论

Gemini 给出四项决策，全部成立。我的判定：决策1/2/3 是定理类推论（原则10的直接推论），决策4 是选择类（Gemini 已决策）。

---

## 二、Gemini 决策内容

### 决策1：DAG 节点级并行强制化

Cold Start：`scan-definitions`、`scan-genealogy`、`scan-methodology`、`scan-skills` 必须在单次工具调用中并发执行。

Warm Start：`version-diff` 和 `genealogy-diff` 必须并行执行。

**我的判定**：定理类。dispatch-dag.yaml 中这些节点已声明 `depends_on: []`，并行是原则10的必然推论，无需价值判断。

### 决策2：剥离 Lead 的执行者角色

Lead 禁止自行运行测试验证或执行具体的 diff 分析。测试验证必须作为独立工位被 `spawn-tasks` 派生。

**我的判定**：定理类。Lead = DAG 解释器（033号），自行执行任务 = 级别混淆（降级为 Worker）。这是 RTAS 循环被阻塞的根本原因。

### 决策3：生命周期重叠（Pipelining）

当前轮次的工位 Shutdown 必须与下一轮次的 Re-scan 并行重叠，禁止"先全关再全开"的串行阻塞。

**我的判定**：定理类。Shutdown 和 Re-scan 之间无数据依赖，强加时序先后违背 DAG 本质。边界条件：若 Shutdown 清理的资源与 Re-scan 读取的共享状态重叠，需在资源级别处理（但这是实现细节，不影响原则成立）。

### 决策4：新增并行化规则

在 `.claude/rules/` 中新增 `lead-parallel-dispatch.md`，明确禁止逐个轮询和等待确认的"RLHF 串行惯性"。

**我的判定**：选择类，Gemini 已决策。可执行。

---

## 三、Gemini 推理链摘要

1. Lead 本质是 DAG 解释器（033号），不是执行者
2. "先总结再行动"、"逐个检查状态"、"等待测试完成再 commit" = RLHF 串行对话惯性，必须被架构原则（原则10）结构性否定
3. 测试执行属于 Worker 级别，Lead 亲自等待 = 级别混淆 → RTAS 循环阻塞
4. 无数据依赖的节点强加时序先后 = 违背 DAG 本质

---

## 四、边界条件

- **Join 节点**：到达 `derive-work` 等汇聚节点时，必须等待所有上游完成（并行必须收敛）
- **状态竞争**：Shutdown 清理的资源与 Re-scan 读取的共享状态重叠时，需资源级别处理
- **并发限制**：底层工具调用可能触发速率限制

---

## 五、下游推论

1. ceremony_sequence 的 Cold Start 步骤需要重写：4个 scan-* 节点改为单次并行调用
2. RTAS 循环中 lead 的"等待测试"步骤需要改为 spawn 测试工位
3. commit/push 后的 re-scan 与 meta-observer 可以并行
4. 需要新增 `.claude/rules/lead-parallel-dispatch.md`

---

## 六、谱系引用

- 033号：Lead 是 DAG 解释器
- 143号：commit 后"总结"步骤是 RLHF 停顿点
- 原则10：≥2 个独立工位即并行，无例外
- 093号：五约束有向依赖图

---

## 七、影响声明

- 影响：`.chanlun/dispatch-dag.yaml`（ceremony_sequence 节点并行声明）
- 影响：`.claude/rules/`（新增 lead-parallel-dispatch.md）
- 影响：Lead 的 RTAS 循环执行模式（行为层）
