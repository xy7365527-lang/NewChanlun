---
id: '242'
number: 242
title: 共振回测对比——直积 vs 纤维丛筛选器干净度裁决
type: 经验验证
status: 已结算
date: 2026-02-28
source: v97-swarm backtest-compare 工位
depends_on:
  - '236'  # 纤维丛集成——直积近似 vs 纤维丛精确解的策略偏差量化
  - '240'  # 纤维丛上下文集成——FiberTradingContext 接入策略层
  - '241'  # 共振双管道——_build_resonance_signals 双模型并行共振构建
epistemology:
  - level: L3
    scope: 直积 vs 纤维丛共振筛选器的策略干净度实证对比
    description: |
      在全部 27 种 K4 配置 × 20 个交替 BSP 序列上，
      用 _build_resonance_signals 双模型输出分别通过 resonance_check，
      统计假信号率、矛盾操作率、信号一致性。

      核心发现：
      - 59.3% 的配置存在 polarity 分歧（与 240号一致）
      - 22.2% 的 BSP 事件产生不同的入场/阻断决策
      - 直积版矛盾操作率 = 0，纤维丛版矛盾操作率 = 17.65%
      - 信号一致性两者持平（58.8%）
      - 裁决：直积版更干净（得分 0.8765 vs 0.8235）

      纤维丛版矛盾操作率高的原因：
      fiber_polarity 修正后，部分配置的 scan_direction（仍用直积）
      与 fiber polarity 方向不一致，导致 scan 说 sell 但 fiber 共振通过 buy BSP。
      这不是纤维丛模型本身的缺陷，而是当前集成方式的结构性问题——
      scan_configuration 仍使用直积 polarity，而纤维丛版共振信号使用 fiber polarity，
      两层之间缺乏一致性联动。
---

# 242号：共振回测对比——直积 vs 纤维丛筛选器干净度裁决

## 1. 结论

在全部 27 种 K4 配置上，用 20 个交替 buy/sell BSP 序列对两套共振筛选器做对比回测。

### 全局统计

| 指标 | 直积版 | 纤维丛版 |
|------|--------|----------|
| 入场次数 | 340 (63.0%) | 340 (63.0%) |
| 阻断次数 | 200 (37.0%) | 200 (37.0%) |
| 假信号率 | 0.0% | 0.0% |
| 矛盾操作率 | 0.0% | 17.65% |
| 信号一致性 | 58.8% | 58.8% |

### 裁决

**直积版更干净**（得分 0.8765 vs 0.8235）。

纤维丛版在分歧配置上产生了 60 次矛盾操作——scan_configuration（直积方向）与 fiber 共振信号方向不一致。

### 矛盾操作的根因

纤维丛版矛盾操作的来源不是纤维丛模型本身，而是**集成不完整**：
- `scan_configuration` 使用 `polarity_index(ctx.config)`（直积）
- 纤维丛版共振信号使用 `fiber_polarity`
- 当两者不同时，scan 说 "sell" 但 fiber 共振让 buy BSP 通过 → 矛盾

修复方向：如果采用纤维丛版，scan_configuration 也应使用 fiber_polarity。当前对比结果反映的是"部分集成"状态下的纤维丛版表现。

## 2. 定义依据

### 干净度定义（编排者指令）

"哪套筛选器的信号在实际操作中更干净——更少假信号、更少矛盾操作"。

本对比使用四个维度量化干净度：
1. **假信号率**：polarity 方向与 BSP 方向不一致但共振仍通过
2. **矛盾操作率**：scan_direction 与 BSP 方向不一致但共振通过
3. **信号一致性**：共振通过时，信号方向与 BSP 方向吻合的比例
4. **综合得分**：假信号率 0.4 + 矛盾率 0.3 + 一致性 0.3

### 共振检查（levels-bsp-v2 §5）

共振 = 多层信号方向一致 + 时间重叠。
CONFIG 层方向由 polarity 决定，INDEPENDENT_EDGE 层方向由 BSP 本身决定。
两者一致时共振通过（241号）。

## 3. 边界条件

### 3.1 合成数据 vs 真实数据

本对比使用合成 BSP 序列（交替 buy/sell），不是真实市场数据。
真实数据上 BSP 出现频率、配置转换路径可能改变统计分布。
但干净度的**相对排序**（直积 vs 纤维丛）在合成数据上已有结论。

### 3.2 kl_threshold = 0

使用零阈值意味着任何 KL > 0 都触发纤维丛修正。
提高阈值会让纤维丛版向直积版收敛（极端情况下完全退化为直积版），
矛盾操作率会下降。

### 3.3 scan_configuration 未修改

当前对比中 scan_configuration 始终使用直积 polarity。
如果 scan 也使用 fiber_polarity，纤维丛版的矛盾操作率会下降（可能归零），
但这需要修改 pipeline 核心逻辑，超出本对比的范围。

## 4. 下游推论

### 4.1 直积版作为默认筛选器的合理性确认

在当前集成状态下，直积版更干净。241号中"入场决策使用直积版"的选择得到实证支持。

### 4.2 纤维丛完整集成的方向

如果要让纤维丛版胜出，需要：
1. scan_configuration 也使用 fiber_polarity
2. 或引入 FiberTradingContext 版的 scan_configuration
3. 这样纤维丛版的矛盾操作率才能与直积版对齐

### 4.3 分歧配置的策略价值

22.2% 的 BSP 事件产生不同决策（120/540）。这些分歧点是两种模型信息差最大的地方。
真实数据上追踪这些分歧点的后续走势，可以判断哪种方向判断更准确。

## 5. 谱系引用

- **236号**（纤维丛集成）：adapter 模式。
- **240号**（纤维丛上下文集成）：FiberTradingContext + FiberSignalFilter。
- **241号**（共振双管道）：DualResonanceSignals 双模型并行输出。

## 6. 影响声明

### 新增的文件

1. `scripts/dual_resonance_backtest.py`：双模型回测对比脚本
   - 全 27 配置 × 20 BSP 交替序列
   - 干净度四维度量化
   - 裁决逻辑

2. `tests/test_dual_resonance_backtest.py`：19 个测试
   - BSP 序列生成（4 个）
   - polarity 方向映射（3 个）
   - 单配置评估（6 个）
   - 干净度指标计算（2 个）
   - 信号一致性（1 个）
   - 裁决逻辑（3 个）

3. `tmp/dual-resonance-backtest.json`：完整对比结果

### 不修改的文件

所有现有代码不修改。回测脚本是独立运行的。
