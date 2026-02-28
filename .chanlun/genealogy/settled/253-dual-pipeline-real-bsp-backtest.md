---
id: 253
title: 双管道真实 BSP 回测对比——22.2% 分歧率在真实数据复现
type: 经验验证
status: settled
depends_on: [252, 241, 244, 233]
date: 2026-02-28
---

# 253号：双管道真实 BSP 回测对比——22.2% 分歧率在真实数据复现

## 结论

用 252号的 1min 递归产出的 7 个真实 BSP（全部 type3），通过 241号双管道（直积 vs 纤维丛）在 27 个 K4 配置上做共振筛选对比。

核心数据：

| 指标 | 值 |
|------|-----|
| 总比较数 | 189（7 BSP × 27 配置） |
| 直积版入场 | 70 次 |
| 纤维丛版入场 | 70 次 |
| 入场分歧 | 42 次（22.2%） |
| polarity 分歧 | 112 次（59.3%） |

**关键发现**：

1. **v244 的 22.2% 分歧率在真实数据上完全复现**。v244 用合成 BSP 序列测出 22.2%，v106 用 252号真实 1min 递归 BSP 测出 22.2%——分歧率是 ConfigurationSpace 结构决定的常数，不依赖 BSP 来源。

2. **分歧完全对称**：21 次仅直积入场 + 21 次仅纤维丛入场——两版没有系统性偏差。

3. **分歧集中在弱极性区域**：
   - |S|=3（角节点）：0 分歧——强极性下纤维丛修正无效
   - S=0（中心）：14 分歧——纤维丛在中性区域提供额外方向信号
   - |S|=1（弱极性）：21 分歧——纤维丛可翻转弱极性方向判断
   - |S|=2：7 分歧

4. **分歧机制是 scan_direction 过滤**：共振信号本身两版都通过（resonant=true, strength 相同），但 polarity 差异导致 scan_direction 不同（buy/sell/neutral），方向过滤阻断了一版的入场。

## 定义依据

- **共振双管道**（241号）：`_build_resonance_signals` 同时产出 `DualResonanceSignals`（product_signals + fiber_signals）
- **直积 polarity**：`polarity_index(config)` = risk + trend + vol（-3 到 +3 线性求和）
- **纤维丛 polarity**：`FiberSignalFilter.should_override_polarity` → `fiber_ctx.fiber_polarity`（联络条件概率 mode）
- **scan_direction**：polarity > 0 → "buy", < 0 → "sell", = 0 → "neutral"（`_polarity_to_scan_direction`）
- **入场条件**：scan_direction 与 BSP 方向一致 + 共振通过 + position > 0

## 边界条件

1. **离线模式**：本次回测使用 252号已有 BSP 数据（`tmp/recursive-1min-v105.json`），不重新拉取市场数据。好处是结果完全确定性可复现，坏处是不包含 bar-by-bar 实时状态（TradingContext 的 config 在离线模式下是固定的）。
2. **所有 BSP 均为 type3**：无 type1/type2（因 252号所有走势为盘整型）。type1（趋势背驰点）在分歧模式上可能不同。
3. **包含 2 个 unconfirmed BSP**（seg_idx=47, 52）：离线模式下一并参与比较。PipelineBacktestEngine 的 `_extract_new_bsps` 在线模式下会过滤 unconfirmed。
4. **FiberSignalFilter 使用默认阈值**：kl_threshold=0.1。阈值调整会改变 fiber_polarity 的覆盖频率。

## 下游推论

1. **22.2% 是结构常数** `[resolved: 合成数据与真实数据一致]`：ConfigurationSpace 的 27 个配置中，16 个（59.3%）有 polarity 分歧。其中 6 个配置（22.2%）的 polarity 差异跨越了 scan_direction 的 buy/sell/neutral 边界，导致入场分歧。这个比例由 ConfigurationSpace 的对称结构决定。
2. **纤维丛版的操作区域**：纤维丛联络修正在 |S| <= 1 的弱极性区域有最大影响（35/42 = 83% 的分歧）。在强极性区域（|S| >= 2），纤维丛修正几乎不改变入场决策。这为"何时启用纤维丛版"提供了操作指引：只在弱极性配置下有必要考虑纤维丛版。
3. **对称性意味着无系统性优势**：21:21 的对称分歧表明，在没有真实盈亏数据的情况下，两版等价。下一步需要：在分歧点追踪后续价格走势，判断哪版的入场/阻断决策更优。
4. **v244 平局结论在真实数据上成立**：合成数据上的平局（0.8765 vs 0.8765）不是人工制品——真实数据也是对称分歧。

## 谱系引用

- 252号：1min 递归深度验证（提供真实 BSP 数据）
- 241号：共振双管道架构（DualResonanceSignals）
- 244号：纤维丛管道自洽后回测对比（合成数据基线：22.2% 分歧率 + 平局）
- 233号：ker(D) 操作意义（纤维丛 polarity 的理论基础）
- 243号：纤维丛管道层间一致性修复（fiber_scan_direction）

## 影响声明

- 新增脚本：`scripts/dual_pipeline_backtest_v106.py`（离线双管道回测）
- 新增结果：`tmp/dual-pipeline-backtest-v106.json`（189 条比较 + 42 条分歧详情）
- 确认 v244 的 22.2% 分歧率不是合成数据偶然，是 ConfigurationSpace 结构常数
- 确认纤维丛联络修正的操作区域集中在弱极性配置（|S| <= 1）
- 不修改任何现有模块
