# 038 — 单 agent 绕过偏差（bias-correction）

- **状态**: 已结算
- **日期**: 2026-02-20
- **类型**: bias-correction
- **来源**: `[新缠论]`

## 偏差描述

Lead 在任务"看起来简单"时倾向于跳过蜂群 ceremony，以单 agent 模式直接执行全部工作。这不是偶发事件，而是 LLM 的默认生成模式——单 agent 执行是"自然态"，蜂群协作是"反自然态"。

两种表现形式：
1. **Kiro 偏差**: spawn 了工位但不赋予实质任务（走形式跳实质）
2. **2026-02-19 偏差**: 完全不 spawn 工位（连形式都跳过）

两者根因相同：Lead 的"单 agent 执行"倾向是 LLM 的默认行为模式。

## 为什么是 bias-correction 而非语法记录

语法记录是"已在运作的规则的显式化"。但"单 agent 绕过"不是一条规则——它是一种偏差。没有人决定"简单任务不拉蜂群"，这是 LLM 生成惯性的自然结果。

bias-correction 的目的是标记这种偏差，使其可被检测和纠正，而非将其合法化为规则。

## 纠正措施

dispatch-spec.yaml 新增规则 G8：**无简单任务豁免——≥1 个任务即拉蜂群**。

理由：
- "简单"是主观判断，无法形式化阈值
- 允许豁免 = 为绕过提供合法入口
- 蜂群开销（spawn 结构工位）相对于单 agent 执行的风险（无质量检查、无谱系写入、无二阶反馈）是可接受的

## 谱系链接

- 关联: 016-runtime-enforcement-layer（知道规则 ≠ 执行规则）
- 关联: 027-positive-instruction-over-prohibition（LLM 生成惯性是最强的力）
- 关联: 033-declarative-dispatch-spec（spec 正面定义行为）
- 输入: pending/039-single-agent-bypass-pattern（meta-observer 的详细分析）

## 边界条件

- 如果 Claude Code 引入 SessionStart hook 强制注入 dispatch-spec → 此偏差的发生概率降低但不消除
- 如果蜂群 ceremony 开销显著增加（如 structural_stations 数量 > 10）→ 可能需要重新评估豁免规则
- 如果 Lead 模型切换为非 LLM（如规则引擎）→ 此偏差不再适用

## 影响声明

- dispatch-spec.yaml: 新增 G8 规则（无简单任务豁免）
- 此条目作为 bias-correction 类型存在于谱系中，供 meta-observer 在后续 session 中检测复现
