# 031 — Move 价格范围语义：极值 vs 核心区间

**类型**: 选择 → 编排者决断
**状态**: 已结算
**日期**: 2026-02-19
**结算日期**: 2026-02-19
**negation_source**: heterogeneous（Gemini 异质质询首次产出实质性否定）
**negation_form**: separation（极值 vs 核心区间——同一"价格范围"概念内部暴露不兼容语义）
**前置**: 006-level-recursion, 029-move-c-segment-coverage
**关联**: 010-construction-vs-classification, 011-level-determination-direction, 030a-gemini-position

## 矛盾描述

递归构造中，"次级别走势的价格范围"在不同级别有不同语义：

- Level 1：`Segment.low/high` = 绝对极值（笔的 min/max）
- Level 2+：`Move.low/high` = min(ZD)/max(ZG)（核心区间，不含离开段）

FSM 的 `overlap()` 统一使用 `seg.low/high`，导致同一函数在不同级别执行不同语义的比较。自同构性被打破。

## 发现来源

Gemini 异质质询工位（MCP 工具模式）。Gemini 通过 Serena 语义工具自主导航代码库后发现。这是 030a（Gemini 位置问题）的第一个实践数据点——外部工具的产出进入了谱系。

## 决断

**选项 A：用 GG/DD（波动极值）。** 编排者选择，理由：自同构性是递归构造的根基。

### 实施

- `a_move_v1.py`：`Move.high = max(center.gg)`, `Move.low = min(center.dd)` — 波动极值替代核心区间
- `a_zhongshu_level.py`：`moves_from_level_zhongshus()` 同步修改
- 测试更新：`test_v1_pipeline_e2e.py`, `test_zhongshu_level.py` 断言值更新
- 无需新增字段（直接改 high/low 语义，下游 FSM 自动修复）

### 被否定的方案

- **B（维持 ZD/ZG）**：自同构性破缺不可接受。离开段是走势类型的结构性组件（定义中枢是否终结），不是噪音。
- **C（双范围）**：过度工程化。当前无场景需要同时使用两种范围。

## 影响

- `overlap()` 在所有级别现在执行相同语义的比较（波动极值）
- 高级别中枢判定更保守（更难宣告趋势）— 这是正确的行为
- `a_divergence.py:246` 的 `m.high > c.high` 判定也自动修正为波动极值

## 030a 推进

本条是 Gemini 产出进入谱系的第一个实例。030a 的待结算条件"Gemini 实际运行后，观察其产出是否真的进入谱系改变了定义"——答案是肯定的。

后续：030a 已通过元编排重构获得系统级回应——异质否定源作为结构工位接入蜂群（methodology-v3.3.md 更新、gemini-challenger agent 创建、/challenge 命令创建）。030a 可结算。
