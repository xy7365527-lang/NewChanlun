# 完整版四操作分离 + 多级别 PH 多重赋格回测 — 1分钟数据

## 架构

6状态 FSM + 三级别 PH 分层 + 四操作独立判断：

```
SCANNING ──BSP+L0settle──→ LONG_OPEN ←→ LONG_COST_REDUCING
    │                         │
    │                    平多(卖点/L2翻转/invalidate)
    │                         │
    │                         ↓
    │                     OBSERVING ←── 平空
    │                         │            │
    │                    新L2 settle       │
    │                         │            │
    │                         ↓            │
    └──BSP+L0settle──→ SHORT_OPEN ←→ SHORT_COST_REDUCING
```

### 四操作独立判断

| 操作 | 条件 |
|------|------|
| 开多 | L2方向=多 + 买点candidate + L0 rank-1 settle |
| 平多 | 卖点candidate / L2翻空 / BSP invalidate → OBSERVING |
| 开空 | L2方向=空 + 卖点candidate + L0 rank-1 settle |
| 平空 | 买点candidate / L2翻多 / BSP invalidate → OBSERVING |

**关键区别**：平仓后进入 OBSERVING，等待新 L2 PH settle 确认后才可重新进场。不立刻反手。

| PH级别 | 输入 | 更新频率 | 信号用途 |
|--------|------|---------|---------|
| L0 | 1min close | 每bar | 进场门控（rank-1 settle） |
| L1 | L1线段端点价格 | ~数千次 | 降成本触发/加仓比例（non-rank-1 settle） |
| L2 | L1走势端点价格 | ~数百次 | 方向裁决/止损（rank-1 settle） |

## QQQ

- 数据：**728,030** bars (1min)
- 价格：268.82 → 738.28
- Buy-and-hold: **+174.64%**
- 回测耗时：369.5s

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 728,030 (每bar) | 350,072 | 3962 |
| L1 | 6,119 | 5,804 | 286 |
| L2 | 389 | 190 | 25 |
| **L2 方向翻转** | — | — | **14** |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 12 (多6/空6) |
| 胜率 | 50.0% |
| 平均收益 | -0.123% |
| 复利累计 | **-1.52%** |
| 最大回撤 | -3.11% |
| 平均持仓 | 390 bars |
| 有加仓的交易 | 0/12 |
| 有降成本的交易 | 2/12 |
| 达到本金回收 | 0/12 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |
|---|------|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 多 | 313.67@45926 | 312.05@45958 | 32 | -0.52 | 0 | 0 | 否 | sell_candidate | LONG_OPEN |
| 2 | 空 | 366.49@160970 | 366.95@160977 | 7 | -0.13 | 0 | 0 | 否 | bsp_invalidate | SHORT_OPEN |
| 3 | 多 | 389.75@186026 | 387.46@186738 | 712 | -0.59 | 0 | 0 | 否 | sell_candidate | LONG_OPEN |
| 4 | 空 | 420.71@268939 | 420.78@269115 | 176 | -0.02 | 0 | 0 | 否 | buy_candidate | SHORT_OPEN |
| 5 | 多 | 456.21@288172 | 457.94@289224 | 1,052 | +0.38 | 0 | 1 | 否 | sell_candidate | LONG_COST_REDUCING→LONG_OPEN |
| 6 | 空 | 451.11@333303 | 450.29@333363 | 60 | +0.18 | 0 | 0 | 否 | buy_candidate | SHORT_OPEN |
| 7 | 空 | 506.78@386210 | 507.95@386411 | 201 | -0.23 | 0 | 0 | 否 | buy_candidate | SHORT_OPEN |
| 8 | 多 | 519.99@400611 | 522.34@401101 | 490 | +0.45 | 0 | 0 | 否 | sell_candidate | LONG_OPEN |
| 9 | 空 | 402.75@470586 | 413.45@470792 | 206 | -2.66 | 0 | 0 | 否 | buy_candidate | SHORT_OPEN |
| 10 | 多 | 546.49@521167 | 547.80@521509 | 342 | +0.24 | 0 | 0 | 否 | sell_candidate | LONG_OPEN |
| 11 | 空 | 621.54@617240 | 620.52@617435 | 195 | +0.16 | 0 | 0 | 否 | bsp_invalidate | SHORT_OPEN |
| 12 | 多 | 649.98@699681 | 652.86@700894 | 1,213 | +1.24 | 0 | 1 | 否 | sell_candidate | LONG_COST_REDUCING→LONG_OPEN |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 162,179 | 22.3% |
| LONG_OPEN | 2,802 | 0.4% |
| LONG_COST_REDUCING | 1,039 | 0.1% |
| SHORT_OPEN | 845 | 0.1% |
| SHORT_COST_REDUCING | 0 | 0.0% |
| OBSERVING | 561,165 | 77.1% |

<details><summary>事件流（39条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 45,926 | 313.67 | LONG_OPEN | ENTRY_LONG@313.67 L2dir=1 |
| 45,958 | 312.05 | OBSERVING | CLOSE_LONG:sell_candidate@312.05 pnl=-0.52% |
| 133,081 | 365.51 | SCANNING | OBSERVING→SCANNING L2dir=-1 (was 1) |
| 160,970 | 366.49 | SHORT_OPEN | ENTRY_SHORT@366.49 L2dir=-1 |
| 160,977 | 366.95 | OBSERVING | CLOSE_SHORT:bsp_invalidate@366.95 pnl=-0.13% |
| 185,025 | 385.76 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
| 186,026 | 389.75 | LONG_OPEN | ENTRY_LONG@389.75 L2dir=1 |
| 186,738 | 387.46 | OBSERVING | CLOSE_LONG:sell_candidate@387.46 pnl=-0.59% |
| 267,818 | 426.80 | SCANNING | OBSERVING→SCANNING L2dir=-1 (was 1) |
| 268,939 | 420.71 | SHORT_OPEN | ENTRY_SHORT@420.71 L2dir=-1 |
| 269,115 | 420.78 | OBSERVING | CLOSE_SHORT:buy_candidate@420.78 pnl=-0.02% |
| 286,839 | 455.09 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
| 288,172 | 456.21 | LONG_OPEN | ENTRY_LONG@456.21 L2dir=1 |
| 288,365 | 456.35 | LONG_COST_REDUCING | LONG_TRIM@456.35 shares=218.4 L1ratio=0.997 |
| 289,078 | 457.78 | LONG_OPEN | LONG_CLOSE_DIFF@457.78 profit=-312.37 |
| 289,224 | 457.94 | OBSERVING | CLOSE_LONG:sell_candidate@457.94 pnl=+0.38% |
| 330,925 | 443.17 | SCANNING | OBSERVING→SCANNING L2dir=-1 (was 1) |
| 333,303 | 451.11 | SHORT_OPEN | ENTRY_SHORT@451.11 L2dir=-1 |
| 333,363 | 450.29 | OBSERVING | CLOSE_SHORT:buy_candidate@450.29 pnl=+0.18% |
| 340,879 | 483.98 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
| 386,210 | 506.78 | SHORT_OPEN | ENTRY_SHORT@506.78 L2dir=-1 |
| 386,411 | 507.95 | OBSERVING | CLOSE_SHORT:buy_candidate@507.95 pnl=-0.23% |
| 389,769 | 511.36 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
| 400,611 | 519.99 | LONG_OPEN | ENTRY_LONG@519.99 L2dir=1 |
| 401,101 | 522.34 | OBSERVING | CLOSE_LONG:sell_candidate@522.34 pnl=+0.45% |
| 448,805 | 498.50 | SCANNING | OBSERVING→SCANNING L2dir=-1 (was 1) |
| 470,586 | 402.75 | SHORT_OPEN | ENTRY_SHORT@402.75 L2dir=-1 |
| 470,792 | 413.45 | OBSERVING | CLOSE_SHORT:buy_candidate@413.45 pnl=-2.66% |
| 520,731 | 543.03 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
| 521,167 | 546.49 | LONG_OPEN | ENTRY_LONG@546.49 L2dir=1 |
| 521,509 | 547.80 | OBSERVING | CLOSE_LONG:sell_candidate@547.80 pnl=+0.24% |
| 613,327 | 605.49 | SCANNING | OBSERVING→SCANNING L2dir=-1 (was 1) |
| 617,240 | 621.54 | SHORT_OPEN | ENTRY_SHORT@621.54 L2dir=-1 |
| 617,435 | 620.52 | OBSERVING | CLOSE_SHORT:bsp_invalidate@620.52 pnl=+0.16% |
| 699,454 | 647.87 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
| 699,681 | 649.98 | LONG_OPEN | ENTRY_LONG@649.98 L2dir=1 |
| 699,898 | 649.05 | LONG_COST_REDUCING | LONG_TRIM@649.05 shares=153.6 L1ratio=0.998 |
| 700,224 | 643.89 | LONG_OPEN | LONG_CLOSE_DIFF@643.89 profit=792.67 |
| 700,894 | 652.86 | OBSERVING | CLOSE_LONG:sell_candidate@652.86 pnl=+1.24% |
</details>

## OKLO

- 数据：**333,613** bars (1min)
- 价格：18.07 → 63.48
- Buy-and-hold: **+251.30%**
- 回测耗时：72.8s

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 333,613 (每bar) | 154,247 | 730 |
| L1 | 2,806 | 2,657 | 102 |
| L2 | 185 | 101 | 12 |
| **L2 方向翻转** | — | — | **7** |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 6 (多3/空3) |
| 胜率 | 66.7% |
| 平均收益 | +2.402% |
| 复利累计 | **+12.41%** |
| 最大回撤 | -13.17% |
| 平均持仓 | 232 bars |
| 有加仓的交易 | 0/6 |
| 有降成本的交易 | 0/6 |
| 达到本金回收 | 0/6 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |
|---|------|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 多 | 13.08@41091 | 15.98@41355 | 264 | +22.17 | 0 | 0 | 否 | sell_candidate | LONG_OPEN |
| 2 | 空 | 28.20@77486 | 28.02@77496 | 10 | +0.64 | 0 | 0 | 否 | bsp_invalidate | SHORT_OPEN |
| 3 | 多 | 31.98@82271 | 32.94@82604 | 333 | +3.00 | 0 | 0 | 否 | sell_candidate | LONG_OPEN |
| 4 | 空 | 39.00@136306 | 38.13@136346 | 40 | +2.23 | 0 | 0 | 否 | buy_candidate | SHORT_OPEN |
| 5 | 多 | 76.58@173692 | 72.00@174128 | 436 | -5.98 | 0 | 0 | 否 | sell_candidate | LONG_COST_REDUCING→LONG_OPEN |
| 6 | 空 | 96.88@235756 | 104.29@236068 | 312 | -7.65 | 0 | 0 | 否 | bsp_invalidate | SHORT_OPEN |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 123,466 | 37.0% |
| LONG_OPEN | 896 | 0.3% |
| LONG_COST_REDUCING | 137 | 0.0% |
| SHORT_OPEN | 362 | 0.1% |
| SHORT_COST_REDUCING | 0 | 0.0% |
| OBSERVING | 208,752 | 62.6% |

<details><summary>事件流（19条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 41,091 | 13.08 | LONG_OPEN | ENTRY_LONG@13.08 L2dir=1 |
| 41,355 | 15.98 | OBSERVING | CLOSE_LONG:sell_candidate@15.98 pnl=+22.17% |
| 70,448 | 18.81 | SCANNING | OBSERVING→SCANNING L2dir=-1 (was 1) |
| 77,486 | 28.20 | SHORT_OPEN | ENTRY_SHORT@28.20 L2dir=-1 |
| 77,496 | 28.02 | OBSERVING | CLOSE_SHORT:bsp_invalidate@28.02 pnl=+0.64% |
| 77,596 | 27.47 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
| 82,271 | 31.98 | LONG_OPEN | ENTRY_LONG@31.98 L2dir=1 |
| 82,604 | 32.94 | OBSERVING | CLOSE_LONG:sell_candidate@32.94 pnl=+3.00% |
| 116,925 | 21.86 | SCANNING | OBSERVING→SCANNING L2dir=-1 (was 1) |
| 136,306 | 39.00 | SHORT_OPEN | ENTRY_SHORT@39.00 L2dir=-1 |
| 136,346 | 38.13 | OBSERVING | CLOSE_SHORT:buy_candidate@38.13 pnl=+2.23% |
| 151,324 | 66.94 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
| 173,692 | 76.58 | LONG_OPEN | ENTRY_LONG@76.58 L2dir=1 |
| 173,991 | 72.22 | LONG_COST_REDUCING | LONG_TRIM@72.22 shares=1206.1 L1ratio=0.924 |
| 174,128 | 72.00 | OBSERVING | CLOSE_LONG:sell_candidate@72.00 pnl=-5.98% |
| 232,808 | 117.34 | SCANNING | OBSERVING→SCANNING L2dir=-1 (was 1) |
| 235,756 | 96.88 | SHORT_OPEN | ENTRY_SHORT@96.88 L2dir=-1 |
| 236,068 | 104.29 | OBSERVING | CLOSE_SHORT:bsp_invalidate@104.29 pnl=-7.65% |
| 307,648 | 65.10 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
</details>

## HK700

- 数据：**1,408,882** bars (1min)
- 价格：11.36 → 458.20
- Buy-and-hold: **+3933.45%**
- 回测耗时：546.9s

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 1,408,882 (每bar) | 463,934 | 2312 |
| L1 | 5,014 | 4,798 | 184 |
| L2 | 298 | 141 | 11 |
| **L2 方向翻转** | — | — | **5** |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 5 (多2/空3) |
| 胜率 | 40.0% |
| 平均收益 | -0.758% |
| 复利累计 | **-4.07%** |
| 最大回撤 | -7.95% |
| 平均持仓 | 1,751 bars |
| 有加仓的交易 | 2/5 |
| 有降成本的交易 | 0/5 |
| 达到本金回收 | 0/5 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |
|---|------|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 空 | 43.30@245433 | 43.50@245646 | 213 | -0.46 | 0 | 0 | 否 | bsp_invalidate | SHORT_OPEN |
| 2 | 多 | 50.50@289428 | 46.70@295687 | 6,259 | -7.52 | 0 | 0 | 否 | sell_candidate | LONG_COST_REDUCING→LONG_OPEN |
| 3 | 空 | 292.20@766519 | 284.60@767911 | 1,392 | +2.18 | 1 | 0 | 否 | buy_candidate | SHORT_COST_REDUCING→SHORT_OPEN |
| 4 | 多 | 487.60@917774 | 484.60@917910 | 136 | -0.62 | 0 | 0 | 否 | sell_candidate | LONG_OPEN |
| 5 | 空 | 380.50@1056649 | 365.90@1057402 | 753 | +2.63 | 1 | 0 | 否 | buy_candidate | SHORT_OPEN |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 339,235 | 24.1% |
| LONG_OPEN | 2,620 | 0.2% |
| LONG_COST_REDUCING | 3,775 | 0.3% |
| SHORT_OPEN | 2,209 | 0.2% |
| SHORT_COST_REDUCING | 149 | 0.0% |
| OBSERVING | 1,060,894 | 75.3% |

<details><summary>事件流（18条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 245,433 | 43.30 | SHORT_OPEN | ENTRY_SHORT@43.30 L2dir=-1 |
| 245,646 | 43.50 | OBSERVING | CLOSE_SHORT:bsp_invalidate@43.50 pnl=-0.46% |
| 253,797 | 39.60 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
| 289,428 | 50.50 | LONG_OPEN | ENTRY_LONG@50.50 L2dir=1 |
| 291,912 | 49.70 | LONG_COST_REDUCING | LONG_TRIM@49.70 shares=1958.0 L1ratio=0.989 |
| 295,687 | 46.70 | OBSERVING | CLOSE_LONG:sell_candidate@46.70 pnl=-7.52% |
| 761,142 | 312.30 | SCANNING | OBSERVING→SCANNING L2dir=-1 (was 1) |
| 766,519 | 292.20 | SHORT_OPEN | ENTRY_SHORT@292.20 L2dir=-1 |
| 767,025 | 289.10 | SHORT_OPEN | SHORT_ADD@289.10 +227.1 L1ratio=0.664 |
| 767,762 | 284.60 | SHORT_COST_REDUCING | SHORT_TRIM@284.60 shares=200.1 L1ratio=0.351 |
| 767,911 | 284.60 | OBSERVING | CLOSE_SHORT:buy_candidate@284.60 pnl=+2.18% |
| 917,567 | 479.00 | SCANNING | OBSERVING→SCANNING L2dir=1 (was -1) |
| 917,774 | 487.60 | LONG_OPEN | ENTRY_LONG@487.60 L2dir=1 |
| 917,910 | 484.60 | OBSERVING | CLOSE_LONG:sell_candidate@484.60 pnl=-0.62% |
| 1,004,063 | 507.00 | SCANNING | OBSERVING→SCANNING L2dir=-1 (was 1) |
| 1,056,649 | 380.50 | SHORT_OPEN | ENTRY_SHORT@380.50 L2dir=-1 |
| 1,057,130 | 368.50 | SHORT_OPEN | SHORT_ADD@368.50 +162.5 L1ratio=0.618 |
| 1,057,402 | 365.90 | OBSERVING | CLOSE_SHORT:buy_candidate@365.90 pnl=+2.63% |
</details>

## 汇总对比

| 标的 | Bars | BH% | 复利% | 胜率 | 交易数(多/空) | L2翻转 | 加仓率 | 降成本率 | PW率 | 耗时 |
|------|------|-----|------|------|-------------|--------|--------|---------|------|------|
| QQQ | 728,030 | +174.6 | **-1.52** | 50% | 12(6/6) | 14 | 0/12 | 2/12 | 0/12 | 369s |
| OKLO | 333,613 | +251.3 | **+12.41** | 67% | 6(3/3) | 7 | 0/6 | 0/6 | 0/6 | 73s |
| HK700 | 1,408,882 | +3933.5 | **-4.07** | 40% | 5(2/3) | 5 | 2/5 | 0/5 | 0/5 | 547s |

## 六版横向对比

| 版本 | QQQ | OKLO | HK700 |
|------|-----|------|-------|
| 1. 单层PH（简单镜像） | -19.8% | -99% | -84% |
| 2. 多级别PH双向（简单镜像） | +58.3% | +72.3% | +222.8% |
| 3. 纯做多 | +71.7% | +228% | +292.9% |
| **4. 完整版四操作分离（本版）** | **-1.5%** | **+12.4%** | **-4.1%** |

## 结果包六要素

**结论**：完整版四操作分离 FSM 在 1 分钟级别 3 标的上的表现。核心改进：平仓后进入 OBSERVING 而非立刻反手，四操作独立判断。

**定义依据**：
- L0 PH：1min close 序列的在线因果 merge tree（§7.5）
- L1 PH：L1 线段端点 ep1_price 序列的 merge tree
- L2 PH：L1 走势端点（up→high, down→low）序列的 merge tree
- 方向裁决 = L2 rank-1 settle（结构级别匹配交易级别）
- 降成本 = L1 non-rank-1 settle（中间级别的结构信号）
- 进场门控 = L0 rank-1 settle + BSP candidate（微观确认）
- **四操作分离**：开多/平多/开空/平空各有独立判断条件，平仓→OBSERVING→新L2 settle→重新进场

**边界条件**：
- PENDING_EXPIRY = 390 bars — BSP 候选需在 L0 rank-1 settle 前出现
- OBSERVING 退出条件：L2 方向发生了与进入时不同的 settle → 回到 SCANNING
- 若 L2 方向长期不翻转 → OBSERVING 时间长 → 错过同方向机会（设计选择，不是 bug）
- 无滑点/手续费建模

**下游推论**：
- 若本版 > 简单镜像版 → OBSERVING 过滤有效，机械反手损耗可观
- 若本版 < 简单镜像版 → OBSERVING 过于保守，错过的同方向机会 > 避免的反手损耗
- 若本版 ≈ 纯做多版 → 空头操作在当前标的上无显著 alpha

**谱系引用**：
- 267号：满仓满融降成本体系
- §7.5：在线因果 merge tree
- 526号：递归存在论区分（a0 级别分层的谱系依据）

**影响声明**：新建独立回测脚本 `fugue_full_fouroperation_backtest_1min.py`，不修改引擎代码或旧版回测。

**认识论等级**：L2（真实数据，3标的 1min；可产生否定性结果）。