# K4/ω L2 验证 — 金油比 regime 与 SPY 涨跌对比

生成时间：2026-06-06 00:49

## 1. 数据摘要

### Daily 数据（用于 ω 分析）

| 品种 | 天数 | 起始 | 结束 | 首价 | 末价 |
|------|------|------|------|------|------|
| GLD | 2,515 | 2016-06-06 | 2026-06-05 | 118.92 | 396.24 |
| USO | 2,515 | 2016-06-06 | 2026-06-05 | 96.32 | 133.02 |
| UUP | 2,515 | 2016-06-06 | 2026-06-05 | 20.33 | 28.02 |
| SPY | 2,515 | 2016-06-06 | 2026-06-05 | 179.56 | 737.55 |

### 1min 数据（缓存供后续使用）

| 品种 | Bars | 来源 |
|------|------|------|
| GLD | 790,129 | databento/yfinance |
| USO | 725,581 | databento/yfinance |
| UUP | 158,242 | databento/yfinance |
| SPY | 1,225,071 | databento/yfinance |

## 2. ω 金油比序列

- 有效天数：**2,515**
- 日期范围：2016-06-06 → 2026-06-05
- ω 范围：0.88 ~ 9.44
- ω 均值：2.77
- ω 当前：2.98

### ω 分位数

| 分位 | 值 |
|------|-----|
| 10% | 1.23 |
| 25% | 1.45 |
| 50% | 2.55 |
| 75% | 3.49 |
| 90% | 5.34 |

## 3. Regime 分布

| Regime | 天数 | 占比 |
|--------|------|------|
| bull_commodity | 909 | 37.0% |
| bull_equity | 978 | 39.8% |
| neutral | 568 | 23.1% |

## 4. Regime 切换点

共 **82** 次切换

| # | 日期 | 方向 | ω | 强度 |
|---|------|------|---|------|
| 1 | 2016-09-02 | bull_commodity→neutral | 1.55 | 0.0161 |
| 2 | 2016-10-07 | neutral→bull_equity | 1.32 | -0.0227 |
| 3 | 2016-11-11 | bull_equity→neutral | 1.50 | -0.0193 |
| 4 | 2016-12-06 | neutral→bull_equity | 1.23 | -0.0237 |
| 5 | 2017-02-02 | bull_equity→neutral | 1.26 | -0.0174 |
| 6 | 2017-02-16 | neutral→bull_commodity | 1.29 | 0.0216 |
| 7 | 2017-06-02 | bull_commodity→neutral | 1.54 | 0.0191 |
| 8 | 2017-06-08 | neutral→bull_commodity | 1.61 | 0.0209 |
| 9 | 2017-07-24 | bull_commodity→neutral | 1.57 | 0.0168 |
| 10 | 2017-08-14 | neutral→bull_equity | 1.57 | -0.0207 |
| 11 | 2017-08-28 | bull_equity→neutral | 1.64 | -0.0157 |
| 12 | 2017-09-18 | neutral→bull_commodity | 1.53 | 0.0200 |
| 13 | 2017-09-21 | bull_commodity→neutral | 1.50 | 0.0199 |
| 14 | 2017-10-09 | neutral→bull_equity | 1.53 | -0.0210 |
| 15 | 2018-02-15 | bull_equity→neutral | 1.30 | -0.0190 |
| 16 | 2018-04-13 | neutral→bull_equity | 1.18 | -0.0219 |
| 17 | 2018-06-15 | bull_equity→neutral | 1.16 | -0.0191 |
| 18 | 2018-07-05 | neutral→bull_equity | 1.01 | -0.0201 |
| 19 | 2018-10-29 | bull_equity→neutral | 1.03 | -0.0132 |
| 20 | 2018-11-05 | neutral→bull_commodity | 1.09 | 0.0265 |
| 21 | 2019-02-04 | bull_commodity→neutral | 1.35 | 0.0161 |
| 22 | 2019-02-20 | neutral→bull_equity | 1.33 | -0.0214 |
| 23 | 2019-05-24 | bull_equity→neutral | 1.24 | -0.0186 |
| 24 | 2019-06-04 | neutral→bull_commodity | 1.40 | 0.0227 |
| 25 | 2019-09-24 | bull_commodity→neutral | 1.52 | 0.0181 |
| 26 | 2019-11-15 | neutral→bull_equity | 1.43 | -0.0212 |
| 27 | 2020-01-16 | bull_equity→neutral | 1.49 | -0.0169 |
| 28 | 2020-01-28 | neutral→bull_commodity | 1.64 | 0.0271 |
| 29 | 2020-06-16 | bull_commodity→neutral | 5.87 | 0.0084 |
| 30 | 2020-06-19 | neutral→bull_equity | 5.81 | -0.0228 |
| 31 | 2020-08-05 | bull_equity→neutral | 6.33 | -0.0132 |
| 32 | 2020-08-13 | neutral→bull_commodity | 6.03 | 0.0203 |
| 33 | 2020-08-28 | bull_commodity→neutral | 6.00 | 0.0189 |
| 34 | 2020-09-14 | neutral→bull_commodity | 6.83 | 0.0221 |
| 35 | 2020-10-12 | bull_commodity→neutral | 6.43 | 0.0160 |
| 36 | 2020-11-05 | neutral→bull_commodity | 6.74 | 0.0217 |
| 37 | 2020-11-11 | bull_commodity→neutral | 6.02 | 0.0199 |
| 38 | 2020-11-27 | neutral→bull_equity | 5.37 | -0.0218 |
| 39 | 2021-04-27 | bull_equity→neutral | 3.86 | -0.0192 |
| 40 | 2021-06-18 | neutral→bull_equity | 3.41 | -0.0263 |
| 41 | 2021-08-17 | bull_equity→neutral | 3.58 | -0.0190 |
| 42 | 2021-09-02 | neutral→bull_commodity | 3.46 | 0.0203 |
| 43 | 2021-09-16 | bull_commodity→neutral | 3.24 | 0.0146 |
| 44 | 2021-09-27 | neutral→bull_equity | 3.09 | -0.0220 |
| 45 | 2021-11-26 | bull_equity→neutral | 3.36 | -0.0147 |
| 46 | 2021-12-03 | neutral→bull_commodity | 3.47 | 0.0256 |
| 47 | 2022-01-06 | bull_commodity→neutral | 2.94 | 0.0191 |
| 48 | 2022-01-19 | neutral→bull_equity | 2.82 | -0.0274 |
| 49 | 2022-07-13 | bull_equity→neutral | 2.21 | -0.0183 |
| 50 | 2022-07-29 | neutral→bull_commodity | 2.10 | 0.0208 |
| 51 | 2022-09-13 | bull_commodity→neutral | 2.20 | 0.0179 |
| 52 | 2022-09-26 | neutral→bull_commodity | 2.39 | 0.0205 |
| 53 | 2022-10-11 | bull_commodity→neutral | 2.16 | 0.0189 |
| 54 | 2022-10-31 | neutral→bull_equity | 2.12 | -0.0218 |
| 55 | 2022-11-17 | bull_equity→neutral | 2.34 | -0.0182 |
| 56 | 2022-12-02 | neutral→bull_commodity | 2.39 | 0.0262 |
| 57 | 2023-02-14 | bull_commodity→neutral | 2.49 | 0.0189 |
| 58 | 2023-03-20 | neutral→bull_commodity | 3.07 | 0.0263 |
| 59 | 2023-04-21 | bull_commodity→neutral | 2.70 | 0.0170 |
| 60 | 2023-05-08 | neutral→bull_commodity | 2.92 | 0.0222 |
| 61 | 2023-06-06 | bull_commodity→neutral | 2.86 | 0.0175 |
| 62 | 2023-07-14 | neutral→bull_equity | 2.69 | -0.0222 |
| 63 | 2023-10-27 | bull_equity→neutral | 2.38 | -0.0140 |
| 64 | 2023-11-08 | neutral→bull_commodity | 2.58 | 0.0243 |
| 65 | 2024-01-29 | bull_commodity→neutral | 2.61 | 0.0181 |
| 66 | 2024-02-13 | neutral→bull_equity | 2.54 | -0.0209 |
| 67 | 2024-03-20 | bull_equity→neutral | 2.62 | -0.0189 |
| 68 | 2024-05-01 | neutral→bull_commodity | 2.82 | 0.0208 |
| 69 | 2024-06-25 | bull_commodity→neutral | 2.72 | 0.0181 |
| 70 | 2024-07-09 | neutral→bull_equity | 2.74 | -0.0230 |
| 71 | 2024-07-24 | bull_equity→neutral | 2.89 | -0.0199 |
| 72 | 2024-08-08 | neutral→bull_commodity | 2.97 | 0.0209 |
| 73 | 2024-11-20 | bull_commodity→neutral | 3.40 | 0.0199 |
| 74 | 2025-01-07 | neutral→bull_equity | 3.13 | -0.0240 |
| 75 | 2025-02-10 | bull_equity→neutral | 3.47 | -0.0175 |
| 76 | 2025-02-19 | neutral→bull_commodity | 3.50 | 0.0217 |
| 77 | 2025-06-12 | bull_commodity→neutral | 4.16 | 0.0177 |
| 78 | 2025-06-23 | neutral→bull_equity | 4.07 | -0.0226 |
| 79 | 2025-08-13 | bull_equity→neutral | 4.27 | -0.0184 |
| 80 | 2025-08-29 | neutral→bull_commodity | 4.25 | 0.0223 |
| 81 | 2026-02-23 | bull_commodity→neutral | 5.95 | 0.0185 |
| 82 | 2026-03-06 | neutral→bull_equity | 4.35 | -0.0252 |

## 5. Regime 切换后 SPY 表现

### 逐次切换明细

| 日期 | 切换方向 | ω | SPY 5d% | SPY 10d% | SPY 20d% | SPY 60d% |
|------|---------|---|---------|----------|----------|----------|
| 2016-09-02 | bull_commodity→neutral | 1.55 | -0.93 | -1.78 | -0.69 | +1.67 |
| 2016-10-07 | neutral→bull_equity | 1.32 | -0.89 | -0.49 | -3.02 | +5.99 |
| 2016-11-11 | bull_equity→neutral | 1.50 | +0.96 | +1.88 | +4.54 | +7.18 |
| 2016-12-06 | neutral→bull_equity | 1.23 | +2.73 | +2.72 | +2.72 | +7.85 |
| 2017-02-02 | bull_equity→neutral | 1.26 | +1.24 | +3.05 | +4.68 | +5.25 |
| 2017-02-16 | neutral→bull_commodity | 1.29 | +0.86 | +1.58 | +1.42 | +2.82 |
| 2017-06-02 | bull_commodity→neutral | 1.54 | -0.31 | -0.14 | -0.49 | +0.65 |
| 2017-06-08 | neutral→bull_commodity | 1.61 | -0.00 | +0.10 | -0.20 | +2.16 |
| 2017-07-24 | bull_commodity→neutral | 1.57 | -0.02 | +0.43 | -1.59 | +4.02 |
| 2017-08-14 | neutral→bull_equity | 1.57 | -1.48 | -0.80 | +1.42 | +5.44 |
| 2017-08-28 | bull_equity→neutral | 1.64 | +0.61 | +2.24 | +2.35 | +6.83 |
| 2017-09-18 | neutral→bull_commodity | 1.53 | -0.32 | +1.04 | +2.23 | +6.83 |
| 2017-09-21 | bull_commodity→neutral | 1.50 | +0.38 | +2.11 | +2.57 | +7.41 |
| 2017-10-09 | neutral→bull_equity | 1.53 | +0.53 | +0.85 | +1.93 | +7.50 |
| 2018-02-15 | bull_equity→neutral | 1.30 | +0.62 | -1.45 | +0.83 | +0.38 |
| 2018-04-13 | neutral→bull_equity | 1.18 | +0.55 | +0.53 | +2.90 | +5.66 |
| 2018-06-15 | bull_equity→neutral | 1.16 | -0.86 | -2.11 | +0.80 | +4.30 |
| 2018-07-05 | neutral→bull_equity | 1.01 | +2.29 | +2.52 | +3.40 | +6.93 |
| 2018-10-29 | bull_equity→neutral | 1.03 | +3.61 | +3.30 | +1.72 | +0.55 |
| 2018-11-05 | neutral→bull_commodity | 1.09 | -0.30 | -1.57 | -1.15 | +0.06 |
| 2019-02-04 | bull_commodity→neutral | 1.35 | -0.49 | +2.17 | +2.60 | +7.77 |
| 2019-02-20 | neutral→bull_equity | 1.33 | +0.28 | -0.39 | +1.57 | +3.79 |
| 2019-05-24 | bull_equity→neutral | 1.24 | -2.90 | +2.19 | +4.35 | +3.08 |
| 2019-06-04 | neutral→bull_commodity | 1.40 | +2.98 | +4.23 | +6.18 | +3.48 |
| 2019-09-24 | bull_commodity→neutral | 1.52 | -0.89 | -2.48 | +1.06 | +8.02 |
| 2019-11-15 | neutral→bull_equity | 1.43 | -0.27 | -0.05 | +2.47 | +8.64 |
| 2020-01-16 | bull_equity→neutral | 1.49 | -0.65 | -2.78 | +2.02 | -13.74 |
| 2020-01-28 | neutral→bull_commodity | 1.64 | +0.66 | +2.56 | -4.71 | -14.12 |
| 2020-06-16 | bull_commodity→neutral | 5.87 | +0.15 | -1.04 | +3.29 | +7.16 |
| 2020-06-19 | neutral→bull_equity | 5.81 | -2.78 | +2.72 | +5.08 | +10.22 |
| 2020-08-05 | bull_equity→neutral | 6.33 | +1.60 | +1.54 | +7.71 | -0.24 |
| 2020-08-13 | neutral→bull_commodity | 6.03 | +0.43 | +3.41 | -0.82 | +4.37 |
| 2020-08-28 | bull_commodity→neutral | 6.00 | -2.28 | -3.46 | -4.29 | +2.37 |
| 2020-09-14 | neutral→bull_commodity | 6.83 | -3.01 | -0.87 | +4.54 | +9.81 |
| 2020-10-12 | bull_commodity→neutral | 6.43 | -2.96 | -3.70 | +0.60 | +8.03 |
| 2020-11-05 | neutral→bull_commodity | 6.74 | +0.85 | +2.15 | +5.60 | +9.49 |
| 2020-11-11 | bull_commodity→neutral | 6.02 | -0.11 | +1.68 | +2.82 | +9.88 |
| 2020-11-27 | neutral→bull_equity | 5.37 | +1.70 | +0.72 | +2.77 | +5.58 |
| 2021-04-27 | bull_equity→neutral | 3.86 | -0.46 | -0.79 | +0.17 | +4.64 |
| 2021-06-18 | neutral→bull_equity | 3.41 | +2.82 | +4.53 | +2.42 | +7.05 |
| 2021-08-17 | bull_equity→neutral | 3.58 | +0.89 | +1.69 | +0.86 | +4.74 |
| 2021-09-02 | neutral→bull_commodity | 3.46 | -1.71 | -2.29 | -3.87 | +2.85 |
| 2021-09-16 | bull_commodity→neutral | 3.24 | -0.57 | -3.72 | -0.73 | +5.61 |
| 2021-09-27 | neutral→bull_equity | 3.09 | -3.16 | -1.80 | +2.92 | +4.98 |
| 2021-11-26 | bull_equity→neutral | 3.36 | -1.21 | +2.56 | +4.35 | -7.74 |
| 2021-12-03 | neutral→bull_commodity | 3.47 | +3.82 | +1.78 | +5.73 | -3.09 |
| 2022-01-06 | bull_commodity→neutral | 2.94 | -0.73 | -6.40 | -4.11 | -2.08 |
| 2022-01-19 | neutral→bull_equity | 2.82 | -4.07 | +1.24 | -1.14 | -2.79 |
| 2022-07-13 | bull_equity→neutral | 2.21 | +4.21 | +5.86 | +10.87 | -1.08 |
| 2022-07-29 | neutral→bull_commodity | 2.10 | +0.36 | +3.67 | -1.62 | -7.66 |
| 2022-09-13 | bull_commodity→neutral | 2.20 | -1.89 | -7.18 | -8.62 | +0.43 |
| 2022-09-26 | neutral→bull_commodity | 2.39 | +0.63 | -1.18 | +4.00 | +4.93 |
| 2022-10-11 | bull_commodity→neutral | 2.16 | +3.74 | +7.60 | +6.78 | +8.98 |
| 2022-10-31 | neutral→bull_equity | 2.12 | -1.62 | +2.31 | +2.34 | +5.52 |
| 2022-11-17 | bull_equity→neutral | 2.34 | +2.05 | +3.21 | -2.34 | +5.49 |
| 2022-12-02 | neutral→bull_commodity | 2.39 | -3.35 | -5.38 | -5.98 | -1.79 |
| 2023-02-14 | bull_commodity→neutral | 2.49 | -3.42 | -4.34 | -5.66 | +0.26 |
| 2023-03-20 | neutral→bull_commodity | 3.07 | +0.70 | +4.37 | +5.20 | +11.03 |
| 2023-04-21 | bull_commodity→neutral | 2.70 | +0.90 | +0.10 | +1.56 | +10.84 |
| 2023-05-08 | neutral→bull_commodity | 2.92 | +0.07 | +1.47 | +3.70 | +9.15 |
| 2023-06-06 | bull_commodity→neutral | 2.86 | +2.02 | +1.99 | +3.10 | +5.61 |
| 2023-07-14 | neutral→bull_equity | 2.69 | +0.65 | +1.70 | -0.81 | -3.44 |
| 2023-10-27 | bull_equity→neutral | 2.38 | +5.85 | +7.29 | +10.67 | +19.32 |
| 2023-11-08 | neutral→bull_commodity | 2.58 | +2.84 | +4.06 | +4.80 | +13.43 |
| 2024-01-29 | bull_commodity→neutral | 2.61 | +0.26 | +1.98 | +3.19 | +3.20 |
| 2024-02-13 | neutral→bull_equity | 2.54 | +0.63 | +2.47 | +4.43 | +5.61 |
| 2024-03-20 | bull_equity→neutral | 2.62 | +0.52 | -1.42 | -4.03 | +4.28 |
| 2024-05-01 | neutral→bull_commodity | 2.82 | +3.37 | +5.88 | +4.45 | +9.23 |
| 2024-06-25 | bull_commodity→neutral | 2.72 | +0.77 | +3.03 | -0.66 | +4.80 |
| 2024-07-09 | neutral→bull_equity | 2.74 | +1.63 | -0.37 | -6.06 | +2.66 |
| 2024-07-24 | bull_equity→neutral | 2.89 | +1.77 | -4.17 | +3.58 | +7.93 |
| 2024-08-08 | neutral→bull_commodity | 2.97 | +4.22 | +4.82 | +1.83 | +7.94 |
| 2024-11-20 | bull_commodity→neutral | 3.40 | +1.41 | +2.74 | -0.75 | +3.71 |
| 2025-01-07 | neutral→bull_equity | 3.13 | +0.71 | +3.59 | +3.01 | -13.90 |
| 2025-02-10 | bull_equity→neutral | 3.47 | +1.10 | -1.75 | -8.09 | -6.95 |
| 2025-02-19 | neutral→bull_commodity | 3.50 | -3.00 | -4.87 | -7.47 | -3.38 |
| 2025-06-12 | bull_commodity→neutral | 4.16 | -1.28 | +2.15 | +3.79 | +8.03 |
| 2025-06-23 | neutral→bull_equity | 4.07 | +2.95 | +3.36 | +4.78 | +9.84 |
| 2025-08-13 | bull_equity→neutral | 4.27 | -1.05 | +0.27 | +1.98 | +4.23 |
| 2025-08-29 | neutral→bull_commodity | 4.25 | +0.59 | +2.46 | +3.17 | +3.96 |
| 2026-02-23 | bull_commodity→neutral | 5.95 | +0.58 | -0.60 | -3.70 | +7.82 |
| 2026-03-06 | neutral→bull_equity | 4.35 | -1.50 | -3.28 | -1.73 | +13.28 |

### 按切换方向分组统计

| 切换方向 | 次数 | SPY 5d均值 | SPY 20d均值 | SPY 60d均值 | SPY 20d正比例 |
|---------|------|-----------|------------|------------|--------------|
| neutral → bull_equity | 20 | 0.08 | 1.57 | 4.82 | 75.0% |
| neutral → bull_commodity | 21 | 0.51 | 1.29 | 3.4 | 61.9% |
| bull_equity → neutral | 19 | 0.94 | 2.47 | 2.55 | 84.2% |
| bull_commodity → neutral | 22 | -0.26 | 0.0 | 5.19 | 50.0% |

## 6. K4 框架 L2 验证结论

### ω 日收益率与 SPY 日收益率相关系数：**-0.2164**

- 60日滚动相关范围：-0.7747 ~ 0.7841
- 60日滚动相关均值：-0.1788

### ω 与 UUP（美元）日收益率相关系数：**-0.1704**

### 分析耗时：138.9s

## 结果包六要素

**结论**：ω 金油比 regime 切换与 SPY 涨跌的 L2 验证结果（见上述统计表格）。

**定义依据**：
- ω = gold/oil ratio（卢麒元框架：L = M × ω）
- Regime 定义：omega_regime.py 双均线偏离法（MA20/MA60, threshold=2%）
- BULL_COMMODITY：ω 上升（信用收缩）
- BULL_EQUITY：ω 下降（信用扩张）

**边界条件**：
- MA 窗口 = 20/60，threshold = 0.02（不同参数可能改变结果）
- ω 用 GLD/USO 代理，非现货 gold/oil
- SPY 作为美股代理，不代表全部板块

**下游推论**：
- 若 regime 切换后 SPY 收益率方向与理论一致 → ω 有预测价值
- 若不一致 → ω regime 检测方法需要改进，或理论在当前数据窗口不成立
- 否定性结果同样有价值（缩小有效域边界）

**谱系引用**：
- 用户交易方向记忆：押注金油比下降（油涨）
- 卢麒元框架记忆：资本三流、金油两锚

**影响声明**：新建验证脚本和报告，不修改引擎代码。

**认识论等级**：L2（真实数据，多品种日线；可产生否定性结果）。