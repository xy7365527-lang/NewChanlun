---
id: "077"
title: "v16b session 二阶观察"
type: "meta-rule"
status: "已结算"
date: "2026-02-21"
depends_on: ["074", "075"]
negation_source: ""
negation_form: ""
negates: []
negated_by: []
---

# 077号：v16b session 二阶观察

## 观察

### A. ceremony.md TaskOutput/SendMessage 不一致

ceremony.md 步骤 6 指示用 `TaskOutput(task_id=..., block=true)` 等待工位完成。
但 team agent（通过 `Task` + `team_name` spawn）使用 `SendMessage` 通信，`TaskOutput` 的 ID 格式不匹配。

**实际行为**：4 次 TaskOutput 调用全部失败（agent ID 和数字 ID 均不匹配），最终通过 SendMessage + TaskList 轮询完成监控。

**四分法分类**：行动（文档修正，不携带信息差）。

**修正**：ceremony.md 步骤 6 应改为 `TaskList` + `SendMessage` 模式。

### B. 下游行动缺少 `superseded` 状态

42 项未解决下游行动中，≈8 项已被后续谱系取代（060→069 架构演化），但仍标记为 `unresolved`。
导致执行率虚低（报告 26%，实际有效缺口 ≈ 26%——巧合相同但构成不同）。

**隐性规则**：后续谱系结算时，其前置谱系中被取代的下游行动自动失效。

**四分法分类**：语法记录（已在运作的隐性规则，需显式化）。

**建议**：`downstream_audit.py` 增加 `superseded` 状态 + 自动检测逻辑（基于谱系链的取代关系）。

### C. ceremony Bash 禁令边界

"除步骤 1 外不运行 Bash" 在监控阶段（步骤 6）被违反：运行了 `downstream_audit.py` 和 `mkdir`。

**四分法分类**：选择（禁令范围是步骤 1-5 还是全 ceremony 生命周期？需价值判断）。

## 下游推论

- A 项可直接修正 ceremony.md（行动）
- B 项需路由 Gemini decide 显式化（语法记录）
- C 项需路由 Gemini decide 明确边界（选择）
