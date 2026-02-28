---
id: '241'
number: 241
title: 共振双管道——_build_resonance_signals 从双层简化升级为双模型并行共振构建
type: 工程修复 + 工程集成
status: 已结算
date: 2026-02-28
source: v97-swarm resonance-fix 工位
depends_on:
  - '236'  # 纤维丛集成——直积近似 vs 纤维丛精确解的策略偏差量化
  - '240'  # 纤维丛上下文集成——FiberTradingContext 接入策略层
epistemology:
  - level: L3
    scope: 共振信号从伪造退化到真实构建
    description: |
      _build_resonance_signals 原实现是双层简化：从单个 BSP 复制出 CONFIG + INDEPENDENT_EDGE
      双层信号，使 resonance_check 形式上通过但实质不筛选。修复后：
      - 直积版：polarity_index(config) 决定 CONFIG 层方向，BSP 本身决定 EDGE 层方向。
        两者方向一致时共振成立，不一致时 resonance_check 返回 False（真实筛选）。
      - 纤维丛版：FiberSignalFilter polarity 分歧检测后决定 CONFIG 层方向。
      - 两版并行输出在 DualResonanceSignals 结构中，为后续对比回测做准备。
---

# 241号：共振双管道——_build_resonance_signals 从双层简化升级为双模型并行共振构建

## 1. 结论

`pipeline_backtest.py::_build_resonance_signals` 从双层简化升级为双模型并行共振构建。

### 原始实现（双层简化）

从单个 BSP 复制出两个 ResonanceSignal（CONFIG + INDEPENDENT_EDGE），
两者使用相同的 BSP → 方向必然一致 → resonance_check 必然通过。
共振筛选被绕过——任何 BSP 都能触发入场。

### 修复后实现（真实共振构建）

**直积版**：
- CONFIG 层：polarity_index(ctx.config) 决定方向（> 0 → buy, < 0 → sell, = 0 → BSP 自身方向）
- INDEPENDENT_EDGE 层：BSP 自身方向
- 共振条件：CONFIG 方向与 EDGE 方向一致时通过，不一致时失败

**纤维丛版**：
- 通过 FiberSignalFilter 检测 polarity 分歧
- polarity_divergence=True 且 KL > threshold 时使用 fiber_polarity 替代 product polarity
- 否则退化为直积版

**并行输出**：
- DualResonanceSignals 同时包含 product_signals 和 fiber_signals
- 入场决策使用直积版（与现有管道语义一致）
- 纤维丛版通过 last_dual_signals 暴露给调用方做对比分析

## 2. 定义依据

### 共振的精确定义（levels-bsp-v2 §5）

共振 = 多条比价线在各自支配级别上同时出现方向一致的买卖点。
- (R1) 每条线在各自支配级别上都有买卖点
- (R2) 所有买卖点方向一致
- (R3) 时间重叠

双层简化违反了 (R2) 的精神：它通过复制同一 BSP 保证方向一致，
而不是让不同来源的信号独立给出方向后再检查一致性。

修复后 CONFIG 层和 EDGE 层的方向来自不同来源
（配置极性 vs BSP 本身），(R2) 检查才有实际筛选力。

### FiberSignalFilter 覆盖逻辑（240号）

覆盖条件 = polarity_divergence AND correction_confidence > kl_threshold。
当纤维丛联络显示 R 的条件分布 P(R|E,C) 与均匀假设有显著偏差时，
纤维丛 polarity 替代直积 polarity 决定 CONFIG 层方向。

## 3. 边界条件

### 3.1 中心点退化

CENTER (0,0,0) 处 polarity=0，CONFIG 层使用 BSP 自身方向 →
退化为修复前行为（两层同方向）。这是正确的：全盘整态下配置层不提供独立方向信息。

### 3.2 polarity 与 BSP 方向一致时

FULL_RISK_ON + buy BSP → CONFIG(buy) + EDGE(buy) = 同向 → 共振通过。
行为与修复前相同，但共振通过的原因是真实的方向一致，不是伪造的。

### 3.3 polarity 与 BSP 方向相反时

FULL_RISK_OFF + buy BSP → CONFIG(sell) + EDGE(buy) = 反向 → 共振失败 → 不入场。
修复前这种情况也不入场（被 scan_configuration 的方向过滤阻止），
但修复后有两道独立筛选（配置扫描 + 共振检查），防御更深。

### 3.4 纤维丛高阈值退化

FiberSignalFilter(kl_threshold=∞) 时纤维丛版退化为直积版。
DualResonanceSignals 的 fiber_signals 与 product_signals 方向相同。

## 4. 下游推论

### 4.1 对比回测可行

DualResonanceSignals 同时输出两版信号 + polarity_divergence 标记。
后续可对比两版信号在分歧点上的策略表现差异。

### 4.2 纤维丛版入场可选启用

当前入场决策使用直积版。纤维丛版可通过修改 _pipeline_enter 中的
`list(dual.product_signals)` → `list(dual.fiber_signals)` 切换，
或通过新的配置项控制使用哪个版本。

## 5. 谱系引用

- **236号**（纤维丛集成）：FiberPipelineAdapter + FiberCorrection 是纤维丛版信号的基础。
- **240号**（纤维丛上下文集成）：FiberTradingContext + FiberSignalFilter 提供覆盖判断。
- **meta-observer-v88**：首次观测到双层简化绕过共振筛选的问题。

## 6. 影响声明

### 修改的文件

1. `src/newchan/pipeline_backtest.py`：
   - 新增 `DualResonanceSignals` dataclass
   - 新增 `_polarity_to_bsp()` 函数
   - 新增 `_build_signals_for_polarity()` 函数
   - 重写 `_build_resonance_signals(bsp)` → `_build_resonance_signals(bsp, ctx, fiber_filter)`
   - `PipelineBacktestConfig` 新增 `fiber_filter` 字段
   - `PipelineBacktestEngine` 新增 `last_dual_signals` 属性
   - `_pipeline_enter` 使用双模型共振信号

### 新增的文件

1. `tests/test_dual_resonance.py`：25 个测试
   - TestPolarityToBsp（4 个）
   - TestBuildSignalsForPolarity（6 个）
   - TestDualResonanceSignalsStructure（2 个）
   - TestProductSignals（4 个）
   - TestFiberSignals（4 个）
   - TestCenterPointConsistency（2 个）
   - TestDivergencePoints（3 个）

### 不修改的文件

1. `src/newchan/pipeline.py`：不修改
2. `src/newchan/topology/fiber_pipeline_adapter.py`：不修改
3. `src/newchan/nesting/resonance.py`：不修改

### 向后兼容性

所有 19 个现有 pipeline_backtest 测试通过（行为无变化）。
FULL_RISK_ON + buy BSP 仍然入场（CONFIG buy + EDGE buy = 共振通过）。
CENTER + buy BSP 仍然不入场（scan_configuration 返回 neutral）。
FULL_RISK_OFF + buy BSP 仍然不入场（scan_configuration 返回 sell，方向过滤阻止）。
