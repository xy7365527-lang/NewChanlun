---
id: '240'
number: 240
title: 纤维丛上下文集成——FiberTradingContext 接入策略层
type: 工程集成
status: 已结算
date: 2026-02-28
source: v96-swarm fiber-context 工位
depends_on:
  - '236'  # 纤维丛集成——直积近似 vs 纤维丛精确解的策略偏差量化
epistemology:
  - level: L3
    scope: 纤维丛修正从度量层落地到策略层
    description: |
      236号量化了直积 vs 纤维丛的 polarity 差异（45% 交易日）。
      240号将该修正接入 TradingContext，使策略层可以消费纤维丛信号。
      adapter/wrapper 模式：不修改 pipeline.py，所有新增在 fiber_pipeline_adapter.py 中。
---

# 240号：纤维丛上下文集成——FiberTradingContext 接入策略层

## 1. 结论

236号建立了 FiberPipelineAdapter + FiberCorrection，量化了直积 vs 纤维丛的偏差。
240号将纤维丛修正从"度量层"提升到"策略层"：

- **FiberTradingContext**：包装 TradingContext + 纤维丛修正，使策略层可以同时看到直积 polarity 和纤维丛 polarity
- **create_fiber_context()**：从 TradingContext 创建纤维丛增强版
- **FiberSignalFilter**：polarity 分歧检测 + 覆盖决策 + 修正报告

回测对比结果（全 27 种配置）：
- polarity 分歧率：59.3%（16/27 种配置的纤维丛 polarity ≠ 直积 polarity）
- 方向翻转率：37.0%（10/27 种配置的策略方向发生翻转）
- 有效自由度 D_eff = 2.9144
- 全局 KL 散度 = 0.094

## 2. 定义依据

### FiberTradingContext（包装模式）

不继承 TradingContext（避免侵入 pipeline.py），而是包装（composition）：
- `ctx: TradingContext` — 原始上下文完整保留
- `fiber_correction: FiberCorrection` — 236号的修正结果
- `fiber_polarity: int` — 纤维丛联络下的 polarity_index
- `polarity_divergence: bool` — 两种 polarity 是否不同
- `correction_confidence: float` — KL 散度（修正置信度）

### FiberSignalFilter（覆盖决策）

覆盖条件 = polarity_divergence AND correction_confidence > kl_threshold。
kl_threshold 默认为 0.0（任何非零 KL 都足以覆盖），可配置。

### polarity 分歧的策略含义

当直积和纤维丛给出不同方向时（如直积 buy 纤维丛 sell），表示 R 的条件分布
P(R|E,C) 与均匀假设 P(R) = 1/3 有显著偏差。分歧点是两种模型信息差最大的位置。

## 3. 边界条件

### 3.1 底空间中心退化

E=FLAT, C=FLAT 时联络无效应，FiberTradingContext 退化为原始 TradingContext：
polarity_divergence=False, correction_confidence≈0。

### 3.2 平坦联络退化

Connection(0, 0) 下所有配置的 polarity_divergence=False。

### 3.3 覆盖不是替代

FiberSignalFilter.should_override_polarity() 返回 True 不意味着自动替换——
策略层消费者自行决定是否采用纤维丛 polarity。adapter 模式的核心是提供信息而非强制覆盖。

## 4. 下游推论

### 4.1 37% 方向翻转的策略影响

27 种配置中 10 种的策略方向发生翻转（buy→sell, sell→buy, 或涉及 neutral）。
如果 pipeline 采纳纤维丛修正，这些配置上的入场/离场决策会完全不同。

### 4.2 高分歧区域集中在底空间边缘

KL 散度最高的分歧点集中在 E×C 对角线上（E=UP,C=DOWN 和 E=DOWN,C=UP），
这是 risk-on/off 切换的关键市场状态。纤维丛修正在这些状态下最有价值。

### 4.3 PipelineBacktestEngine 可选集成

PipelineBacktestEngine 可以在 _try_entry() 中插入 FiberSignalFilter：
当 should_override=True 时使用 fiber_polarity 替代 product_polarity 决定入场方向。
这不需要修改 pipeline.py。

## 5. 谱系引用

- **236号**（纤维丛集成）：直接前置。FiberCorrection 和 FiberPipelineAdapter 是 240号的基础。
- **232号**（纤维丛重构）：间接前置。纤维丛联络结构定义。
- **230号**（独立性检验）：间接前置。联络参数校准数据。

## 6. 影响声明

### 新增代码

1. `src/newchan/topology/fiber_pipeline_adapter.py` — 在 236号代码基础上新增：
   - `FiberTradingContext` dataclass：包装 TradingContext + 纤维丛修正
   - `create_fiber_context()`：工厂函数
   - `FiberSignalFilter` 类：覆盖决策 + 修正报告

2. `scripts/fiber_backtest_comparison.py`
   - 全 27 种配置的直积 vs 纤维丛对比
   - 输出 JSON 到 `tmp/fiber-backtest-comparison.json`

3. `tests/test_topology/test_fiber_pipeline_adapter.py` — 新增 19 个测试：
   - TestFiberTradingContext（11 个）：构造、委托、不可变性、自定义纤维丛
   - TestPolarityDivergenceDetection（3 个）：中心退化、平坦联络、分歧存在性
   - TestFiberSignalFilter（5 个）：覆盖逻辑、阈值、报告结构

### 不修改的结构

1. `src/newchan/pipeline.py`：不修改
2. `src/newchan/pipeline_backtest.py`：不修改
3. 所有 236号代码：不修改
