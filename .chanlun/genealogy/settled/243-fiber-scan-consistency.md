---
id: '243'
number: 243
title: 纤维丛管道 scan direction 层间一致性修复
type: 工程修复
status: 已结算
date: 2026-02-28
source: v98-swarm fiber-scan-fix 工位
depends_on:
  - '241'  # 共振双管道——_build_resonance_signals 双模型并行共振构建
  - '242'  # 共振回测对比——直积 vs 纤维丛筛选器干净度裁决
epistemology:
  - level: L2
    scope: 纤维丛管道内部一致性——scan direction 与共振信号使用同一 polarity 来源
    description: |
      242号回测发现纤维丛版矛盾操作率 17.65%。根因：
      - scan_configuration 使用 polarity_index(ctx.config)（直积 polarity）
      - 纤维丛版共振信号使用 fiber_polarity（来自 FiberSignalFilter）
      - 两层使用不同的 polarity 来源 → scan 说 sell 但 fiber 共振通过 buy → 矛盾操作

      修复：在 DualResonanceSignals 中增加 fiber_scan_direction 字段，
      由 fiber_polarity 推导，使纤维丛管道的 scan 层与共振层使用同一 polarity 来源。
      回测脚本中纤维丛版的矛盾检查改用 fiber_scan_direction。
---

# 243号：纤维丛管道 scan direction 层间一致性修复

## 1. 结论

修复纤维丛管道的层间不一致：scan direction 从直积 polarity 改为 fiber_polarity。

具体修改：
1. `DualResonanceSignals` 新增 `fiber_scan_direction` 字段（由 fiber_polarity 推导）
2. `_polarity_to_scan_direction` 辅助函数（polarity → "buy"/"sell"/"neutral"）
3. `_build_resonance_signals` 在构建双模型信号时同步计算 `fiber_scan_direction`
4. `dual_resonance_backtest.py` 的 `evaluate_config` 中纤维丛版矛盾检查改用 `fiber_scan_direction`

修复后，纤维丛管道完全自洽：fiber_polarity 同时决定共振信号方向和 scan direction。
直积版行为不变：scan_configuration + product_signals 都使用直积 polarity。

## 2. 定义依据

### scan direction 的定义（pipeline.py:125-149）

scan_configuration 根据 polarity 决定方向：
- S > 0 → "buy"
- S < 0 → "sell"
- S = 0 → "neutral"

### 纤维丛 polarity 的定义（fiber_pipeline_adapter.py:95-128）

fiber_polarity 由纤维丛联络条件概率的 mode 计算：
base.sigma_e + base.sigma_c + mode_r

### 一致性要求

同一管道版本内，scan direction 和共振信号的 CONFIG 层方向必须由同一 polarity 来源决定。
直积版：polarity_index(config)。纤维丛版：fiber_polarity。

242号揭示的矛盾：纤维丛版的 CONFIG 层信号用 fiber_polarity，但 scan direction 用 polarity_index → 层间不一致。

## 3. 边界条件

### 3.1 fiber_scan_direction 与 scan_configuration 的关系

fiber_scan_direction 不替代 pipeline.py 的 scan_configuration。
scan_configuration 是核心管道的一部分，使用直积 polarity，不修改。
fiber_scan_direction 是 DualResonanceSignals 的附加字段，仅供纤维丛版评估使用。

### 3.2 adapter 模式

核心管道（pipeline.py）不被修改。所有纤维丛相关逻辑在 pipeline_backtest.py 和 backtest 脚本中。

### 3.3 翻转条件

如果 fiber_polarity 等于 product_polarity（无分歧），则 fiber_scan_direction 等于直积版 scan direction，修复无行为变化。
仅当 polarity_divergence=True 时，fiber_scan_direction 与直积 scan direction 不同。

## 4. 下游推论

### 4.1 纤维丛版矛盾操作率应下降

242号中纤维丛版 17.65% 的矛盾操作率源于层间不一致。修复后重新跑回测，纤维丛版的矛盾操作率应归零或显著下降（因为 scan direction 和共振信号方向现在来自同一来源）。

### 4.2 对比裁决可能翻转

直积版在 242号中胜出主要因为纤维丛版矛盾操作率高。修复后两者矛盾操作率对齐，裁决可能改变。需要重新跑回测验证。

### 4.3 PipelineBacktestEngine 入场逻辑

当前 PipelineBacktestEngine._pipeline_enter 使用直积版信号入场。如果未来需要纤维丛版入场，fiber_scan_direction 已就绪——只需在 _try_entry 中检查 fiber_scan_direction 而非 scan_configuration 的输出。

## 5. 谱系引用

- **241号**（共振双管道）：DualResonanceSignals 结构——本修复在此结构上增加字段。
- **242号**（共振回测对比）：发现纤维丛版矛盾操作率 17.65% 的根因——层间不一致。
- **240号**（纤维丛上下文集成）：FiberTradingContext.fiber_polarity——fiber_scan_direction 的数据来源。

## 6. 影响声明

### 修改的文件

1. `src/newchan/pipeline_backtest.py`
   - `DualResonanceSignals`：新增 `fiber_scan_direction: str` 字段
   - 新增 `_polarity_to_scan_direction(polarity: int) -> str` 辅助函数
   - `_build_resonance_signals`：构建时计算并填充 `fiber_scan_direction`

2. `scripts/dual_resonance_backtest.py`
   - `SignalDecision`：新增 `fiber_scan_direction: str` 字段
   - `ConfigResult`：新增 `fiber_scan_direction: str` 字段
   - `evaluate_config`：捕获 `dual.fiber_scan_direction`
   - `compute_cleanliness`：纤维丛版矛盾检查改用 `d.fiber_scan_direction`
   - 输出 JSON 中增加 `fiber_scan_direction` 字段

3. `tests/test_dual_resonance.py`
   - 更新手动构造 DualResonanceSignals 的测试以包含新字段
   - 新增 `TestPolarityToScanDirection`（3 个测试）
   - 新增 `TestFiberScanDirectionConsistency`（4 个测试）

### 不修改的文件

- `src/newchan/pipeline.py`：核心管道不变（adapter 模式）
- `src/newchan/topology/fiber_pipeline_adapter.py`：纤维丛适配器不变
- `src/newchan/topology/config_space.py`：配置空间不变
