# 真实市场数据验证报告

生成时间：2026-02-25 22:42
谱系引用：195号-5（拓扑不变量真实数据验证）+ 202号-3（gauge equivalence 经验验证）

## 数据来源

- 真实数据：akshare `stock_zh_a_daily`（前复权）
- 时间范围：20180101 ~ 20260225
- 合成数据：深递归结构（1200根30min）+ 多中枢背驰结构（1500根15min）
- 数据集：
  - sz000001 平安银行（大盘股）— 1974 根K线
  - sh600036 招商银行（大盘股）— 1974 根K线
  - sz000858 五粮液（中盘股）— 1974 根K线
  - sh601318 中国平安（大盘股）— 1974 根K线
  - sz002475 立讯精密（中小盘股）— 1974 根K线
  - SYN-A 深递归合成（合成）— 1200 根K线
  - SYN-B 多中枢背驰合成（合成）— 1500 根K线

## 结果汇总

| 数据集 | K线数 | T1(barcode) | T3(centers) | T4(beta1) | T5 | T6 | T7 | T8 | 强不变量通过率 |
|--------|-------|-------------|-------------|-----------|-----|-----|-----|-----|---------------|
| sz000001 平安银行 | 1974 | 0/0 | 1/3 | 1/3 | N/A | N/A | N/A | 2/3 | 55.6% |
| sh600036 招商银行 | 1974 | 0/0 | 3/3 | 3/3 | N/A | N/A | N/A | 1/3 | 100.0% |
| sz000858 五粮液 | 1974 | 0/0 | 1/3 | 1/3 | N/A | N/A | N/A | 2/2 | 33.3% |
| sh601318 中国平安 | 1974 | 0/0 | 3/3 | 3/3 | N/A | N/A | N/A | 3/5 | 77.8% |
| sz002475 立讯精密 | 1974 | 0/0 | 3/3 | 3/3 | N/A | N/A | N/A | 1/1 | 77.8% |
| SYN-A 深递归合成 | 1200 | 0/0 | 1/3 | 1/3 | N/A | N/A | N/A | 2/2 | 33.3% |
| SYN-B 多中枢背驰合成 | 1500 | 0/0 | 3/3 | 3/3 | N/A | N/A | N/A | 0/2 | 77.8% |

## T6 跨层 Leray 可计算近似详细结果

所有数据集均无 T6 结果（递归级别 < 2）。

## T6 参数校准（207号-3）

无 T6 数据可供校准。

## T8 背驰拓扑详细结果

### sz000001 平安银行

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 2.5227 | 0.5932 | 1.9294 | True | False | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 1.1286 | 1.6880 | -0.5594 | False | False | 0 | 0 | strict | 1 |
| 0 | consolidation | top | 2.6667 | 0.5932 | 2.0734 | True | False | 0 | 0 | new | 1 |

非inconclusive：3项，通过2项

### sh600036 招商银行

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 1.3048 | 1.7635 | -0.4587 | False | False | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 1.7504 | 1.5618 | 0.1886 | True | False | 0 | 0 | strict | 1 |
| 0 | consolidation | top | 1.4514 | 1.7635 | -0.3121 | False | False | 0 | 0 | new | 1 |

非inconclusive：3项，通过1项

### sz000858 五粮液

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 2.0342 | 0.8652 | 1.1691 | True | False | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 3.1681 | 2.3023 | 0.8658 | True | False | 0 | 0 | strict | 1 |

非inconclusive：2项，通过2项

### sh601318 中国平安

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 2.5846 | 1.7023 | 0.8823 | True | False | 0 | 0 | wide | 1 |
| 1 | consolidation | top | 1.1580 | 1.6468 | -0.4887 | False | False | 0 | 0 | wide | 1 |
| 0 | consolidation | bottom | 1.0912 | 1.4702 | -0.3790 | False | False | 0 | 0 | strict | 1 |
| 0 | consolidation | top | 2.6180 | 1.7023 | 0.9157 | True | False | 0 | 0 | new | 1 |
| 1 | consolidation | bottom | 1.3965 | 0.9699 | 0.4266 | True | False | 0 | 0 | new | 1 |

非inconclusive：5项，通过3项

### sz002475 立讯精密

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | bottom | 1.8157 | 0.5974 | 1.2184 | True | False | 0 | 0 | strict | 1 |

非inconclusive：1项，通过1项

### SYN-A 深递归合成

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 1.4033 | 0.6422 | 0.7611 | True | False | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 0.6494 | 0.6422 | 0.0072 | True | False | 0 | 0 | new | 1 |

非inconclusive：2项，通过2项

### SYN-B 多中枢背驰合成

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 2.9610 | 3.2211 | -0.2602 | False | False | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 2.9610 | 4.3143 | -1.3533 | False | False | 0 | 0 | new | 1 |

非inconclusive：2项，通过0项

## T7 递归条形码偏序详细结果

所有数据集均无 T7 结果（递归级别 < 2）。

## T5 走势等价详细结果

所有数据集均无 T5 结果（递归级别 < 2）。

## 模式对 Transition 详情

### sz000001 平安银行

**wide -> strict**
- stroke_diff=-30, segment_diff=-2, center_diff=-1, level_diff=0
- bottleneck=2.3050, beta1_tau_diff=-1
- trend_mutations=[]
- strong: {'n_centers': False, 'trend_kinds': True, 'beta1_tau': False}

**wide -> new**
- stroke_diff=14, segment_diff=0, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': True, 'beta1_tau': True}

**strict -> new**
- stroke_diff=44, segment_diff=2, center_diff=1, level_diff=0
- bottleneck=2.3050, beta1_tau_diff=1
- trend_mutations=[]
- strong: {'n_centers': False, 'trend_kinds': True, 'beta1_tau': False}

### sh600036 招商银行

**wide -> strict**
- stroke_diff=-29, segment_diff=-3, center_diff=0, level_diff=0
- bottleneck=4.4300, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': True, 'beta1_tau': True}

**wide -> new**
- stroke_diff=22, segment_diff=2, center_diff=0, level_diff=0
- bottleneck=3.1150, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': True, 'beta1_tau': True}

**strict -> new**
- stroke_diff=51, segment_diff=5, center_diff=0, level_diff=0
- bottleneck=1.4300, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': True, 'beta1_tau': True}

### sz000858 五粮液

**wide -> strict**
- stroke_diff=-14, segment_diff=0, center_diff=2, level_diff=0
- bottleneck=18.8300, beta1_tau_diff=2
- trend_mutations=[]
- strong: {'n_centers': False, 'trend_kinds': True, 'beta1_tau': False}

**wide -> new**
- stroke_diff=17, segment_diff=-8, center_diff=0, level_diff=0
- bottleneck=9.3050, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': False, 'beta1_tau': True}

**strict -> new**
- stroke_diff=31, segment_diff=-8, center_diff=-2, level_diff=0
- bottleneck=18.8300, beta1_tau_diff=-2
- trend_mutations=[]
- strong: {'n_centers': False, 'trend_kinds': False, 'beta1_tau': False}

### sh601318 中国平安

**wide -> strict**
- stroke_diff=-48, segment_diff=-7, center_diff=0, level_diff=0
- bottleneck=4.2950, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': False, 'beta1_tau': True}

**wide -> new**
- stroke_diff=10, segment_diff=1, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': True, 'beta1_tau': True}

**strict -> new**
- stroke_diff=58, segment_diff=8, center_diff=0, level_diff=0
- bottleneck=4.2950, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': False, 'beta1_tau': True}

### sz002475 立讯精密

**wide -> strict**
- stroke_diff=-26, segment_diff=11, center_diff=0, level_diff=0
- bottleneck=3.7650, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': False, 'beta1_tau': True}

**wide -> new**
- stroke_diff=18, segment_diff=0, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': True, 'beta1_tau': True}

**strict -> new**
- stroke_diff=44, segment_diff=-11, center_diff=0, level_diff=0
- bottleneck=3.7650, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': False, 'beta1_tau': True}

### SYN-A 深递归合成

**wide -> strict**
- stroke_diff=-42, segment_diff=-13, center_diff=-1, level_diff=0
- bottleneck=10.5300, beta1_tau_diff=-1
- trend_mutations=[]
- strong: {'n_centers': False, 'trend_kinds': False, 'beta1_tau': False}

**wide -> new**
- stroke_diff=18, segment_diff=2, center_diff=0, level_diff=0
- bottleneck=12.2058, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': True, 'beta1_tau': True}

**strict -> new**
- stroke_diff=60, segment_diff=15, center_diff=1, level_diff=0
- bottleneck=13.2733, beta1_tau_diff=1
- trend_mutations=[]
- strong: {'n_centers': False, 'trend_kinds': False, 'beta1_tau': False}

### SYN-B 多中枢背驰合成

**wide -> strict**
- stroke_diff=-42, segment_diff=-9, center_diff=0, level_diff=0
- bottleneck=8.5594, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': False, 'beta1_tau': True}

**wide -> new**
- stroke_diff=6, segment_diff=0, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': True, 'beta1_tau': True}

**strict -> new**
- stroke_diff=48, segment_diff=9, center_diff=0, level_diff=0
- bottleneck=8.5594, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'trend_kinds': False, 'beta1_tau': True}

## 第二轮验证：期货 + A 股指数（208号-3）

生成时间：2026-02-26
数据来源：akshare `futures_zh_daily_sina` + `stock_zh_index_daily`（新浪数据源）

### 数据集

| 品种 | 代码 | K线数 | 时间范围 | T8 passed/total | gauge 保持率 |
|------|------|-------|---------|-----------------|-------------|
| 棉花 | CF0 | 5142 | 2005~2026 | 2/3 | 0.25 |
| 豆粕 | M0 | 5144 | 2005~2026 | 3/3 | 0.33 |
| 玉米 | C0 | 5138 | 2005~2026 | 2/3 | 0.08 |
| 螺纹钢 | RB0 | 4105 | 2009~2026 | 1/1 | 0.33 |
| 铜 | CU0 | 5141 | 2005~2026 | 0/0 | 0.33 |
| 黄金 | AU0 | 4414 | 2008~2026 | 1/1 | 0.83 |
| 白银 | AG0 | 3355 | 2012~2026 | 0/1 | 0.50 |
| 焦炭 | J0 | 3606 | 2011~2026 | 0/0 | 0.25 |
| 玻璃 | FG0 | 3201 | 2013~2026 | 1/1 | 0.33 |
| 铝 | AL0 | 5141 | 2005~2026 | 0/1 | 0.08 |
| 棕榈油 | P0 | 4455 | 2008~2026 | 0/0 | 0.08 |
| 豆油 | Y0 | 4887 | 2006~2026 | 0/1 | 1.00 |
| 铁矿石 | I0 | 3002 | 2013~2026 | 0/1 | 1.00 |
| 镍 | NI0 | 2653 | 2015~2026 | 1/1 | 0.50 |
| 白糖 | SR0 | 4885 | 2006~2026 | 1/2 | 0.33 |
| 甲醇 | MA0 | 2705 | 2014~2026 | 0/1 | 1.00 |
| 沪深300 | sh000300 | 5854 | 2002~2026 | 2/2 | 0.00 |
| 上证50 | sh000016 | 5376 | 2004~2026 | 0/0 | 1.00 |
| 创业板指 | sz399006 | 3819 | 2010~2026 | 0/0 | 0.83 |
| 中小板指 | sz399005 | 4289 | 2006~2026 | 0/1 | 0.50 |

### 第二轮汇总

- T8：14/23 passed（60.9%），0 inconclusive，14 MACD-W₁ consensus
- gauge 保持率：mean=0.479，min=0.000，max=1.000
- T6：0/0（无品种产生 Level 2 confirmed centers）
- T7：0/0（同上）

### 递归深度分析

最深递归品种：
- 棉花 CF0：51 段 → 5 centers（4 settled）→ 4 confirmed trends → Level 2 形成 1 个 unconfirmed center → 0 trends → 递归停止
- 豆粕 M0：37 段 → 5 centers（4 settled）→ 4 confirmed trends → 同上
- 玉米 C0：32 段 → 5 centers（4 settled）→ 3 confirmed trends → Level 2 尝试但 moves 不足

结构性约束：Level 2 需要 ≥3 confirmed trends 作为 moves，但最后一个 trend 总是 unconfirmed（需要后续数据确认）。即使 20 年日线数据（5000+ bars），Level 1 最多产生 4 confirmed trends → Level 2 只能形成 1 个 unconfirmed center → 无法产生 trend → 递归停止。

T6 跨层 Leray 验证需要 Level 2 confirmed centers，在日线级别的真实数据中结构性不可达。需要更高频数据（分钟级）或更长时间跨度。

## 第三轮验证：A 股扩展品种（5只，yfinance 数据源）

生成时间：2026-02-26
数据来源：yfinance（akshare 连接超时，fallback 到 yfinance）
时间范围：2021-01-01 ~ 2026-02-26（约 1244 根日线）

### 数据集

| 品种 | 代码 | 类别 | K线数 | T8 passed/total | Tier加权通过率 | gauge 保持率 |
|------|------|------|-------|-----------------|---------------|-------------|
| 贵州茅台 | sh600519 | 大盘股-白酒 | 1244 | 1/1 | 33.3% | 0.39 |
| 美的集团 | sz000333 | 大盘股-家电 | 1244 | 0/0 | 33.3% | 0.33 |
| 隆基绿能 | sh601012 | 中盘股-新能源 | 1244 | 1/2 | 83.3% | 0.39 |
| 比亚迪 | sz002594 | 大盘股-新能源车 | 1244 | 1/1 | 100.0% | 0.50 |
| 中芯国际 | sh688981 | 大盘股-半导体 | 1244 | 0/0 | 100.0% | 0.39 |

### 第三轮汇总

- Tier 加权通过率：mean=70.0%，min=33.3%，max=100.0%
- T8：3/4 passed（75.0%），0 inconclusive，3 MACD-W₁ consensus
- gauge 保持率：mean=0.400，min=0.33，max=0.50
- T6：0/0（无品种产生 Level 2 confirmed centers）
- T7：0/0（同上）

### 强不变量逐项（第三轮）

| 不变量 | 保持率 |
|--------|--------|
| n_centers | 11/15 = 73.3% |
| trend_kinds | 11/15 = 73.3% |
| beta1_tau | 11/15 = 73.3% |

### 低保持率不变量失败模式分析

center_zd_zg_pairs 失败 11/15 例：
- 中枢数相同但区间不同：7 例（bottleneck mean=8.73, max=15.24）
- 中枢数不同：4 例（差异 mean=1.0, max=1）
- 根因：不同 mode 下笔划分差异导致线段端点偏移，中枢 ZD/ZG 区间随之漂移。即使中枢数保持，区间精确值不保持——符合 209号降级预期（弱不变量）

barcode_bottleneck 失败 11/15 例：
- bottleneck 距离：mean=53.62, max=256.09, min=1.19
- wide->strict 对：mean=9.70（最小漂移）
- strict->new 对：mean=58.74
- wide->new 对：mean=128.64（最大漂移）
- 根因：wide 和 new 模式的笔划分差异最大（wide 最宽松，new 最严格），导致中枢区间漂移最大。bottleneck 距离与 mode 间笔数差异正相关——这是 gauge choice 的直接体现，不是异常

### 失败模式结论

center_zd_zg_pairs 和 barcode_bottleneck 的低保持率是结构性的，不是数据依赖的：
1. 不同 mode 产生不同笔数 → 不同线段端点 → 不同中枢区间精确值
2. 中枢数（n_centers）和贝蒂数（beta1_tau）在多数情况下保持 → 拓扑结构保持
3. 区间精确值不保持 → 几何量不是拓扑不变量
4. 209号降级决策正确：这两项应为弱不变量（Tier 3）

## 第四轮验证：美股 1min（Alpha Vantage 付费 API，20只）

生成时间：2026-02-26
数据来源：Alpha Vantage TIME_SERIES_INTRADAY 1min（600/min 付费配额）
时间范围：~50 交易日（outputsize=full，约 16000-20000 bars/标的）

### 数据集

| 品种 | K线数 | 笔数 | 线段数 | 递归深度 | T8 passed/total | Tier加权 |
|------|-------|------|--------|---------|-----------------|---------|
| AAPL | 19948 | 1657 | 125 | 2 | 1/1 | 0.083 |
| MSFT | 20152 | 1604 | 140 | 2 | 6/10 | 0.083 |
| GOOGL | 20152 | 1609 | 57 | 1 | 1/2 | 0.0 |
| AMZN | 20123 | 1588 | 92 | 1 | 3/3 | 0.333 |
| META | 19900 | 1648 | 124 | 2 | 4/7 | 0.0 |
| NVDA | 20160 | 1537 | 127 | 2 | 4/8 | 0.0 |
| AMD | 20081 | 1604 | 120 | 2 | 1/1 | 0.0 |
| TSLA | 20157 | 1611 | 150 | 2 | 1/1 | 0.0 |
| MARA | 18372 | 1363 | 7 | 1 | 1/1 | 1.0 |
| COIN | 19595 | — | — | 0 | 0/0 | 1.0 |
| GME | 16404 | 1397 | 96 | 2 | 1/2 | 0.083 |
| AMC | 16058 | — | — | 1 | 0/0 | 0.25 |
| PLTR | 20159 | 1560 | 56 | 1 | 3/4 | 0.0 |
| SOFI | 19514 | 1328 | 1 | 0 | 2/4 | 0.0 |
| RIVN | 16931 | 1324 | 7 | 1 | 3/3 | 0.083 |
| QQQ | 20156 | 1707 | 68 | 1 | 4/9 | 0.0 |
| IWM | 19212 | 1624 | 116 | 2 | 4/6 | 0.333 |
| XLF | 14694 | 1188 | 62 | 2 | 3/5 | 0.0 |
| GLD | 20067 | 1747 | 139 | 2 | 4/6 | 0.333 |
| SPY | 20167 | 1695 | 152 | 2 | 7/8 | 0.0 |

### 第四轮汇总

- T8：53/81 passed（65.4%），0 inconclusive
- 递归深度分布：depth=2（11只，55%）、depth=1（7只，35%）、depth=0（2只，10%）
- depth=2 标的特征：线段数 ≥ 62（XLF），笔数 ≥ 1188
- depth=0 标的：COIN（极低线段数）、SOFI（1 线段）
- 214号"结构性不可达"被推翻——1min 付费数据下 55% 标的可达 Level 1

### 递归深度与结构密度

| 数据级别 | 典型 bar 数 | 典型线段数 | 典型递归深度 |
|---------|------------|-----------|------------|
| 日线 max（98年） | 24654 | 35 | 1 |
| 5min 60d | 4604 | 38 | 1 |
| 1min ~50d | 20000 | 56-152 | 1-2 |

关键发现：1min 数据的结构密度远高于日线——相同 bar 数下产生 4x 线段。Level 1 递归的门槛约为 Level 0 线段数 ≥ 56。

## 结论

### 第一轮（A 股个股 + 合成数据，7 数据集）
- 强不变量通过率：41/63 = 65.1%
- T8 非inconclusive：11/18，inconclusive：0

### 第二轮（期货 + A 股指数，20 数据集）
- T8：14/23 passed（60.9%），0 inconclusive
- gauge 保持率：mean=0.479

### 第三轮（A 股扩展品种，5 数据集）
- Tier 加权通过率：mean=70.0%
- T8：3/4 passed（75.0%），0 inconclusive
- gauge 保持率：mean=0.400
- 低保持率不变量（center_zd_zg_pairs, barcode_bottleneck）失败模式确认为结构性——209号降级正确

### 第四轮（美股 1min，20 数据集）
- T8：53/81 passed（65.4%），0 inconclusive
- 递归深度：55% 标的达到 Level 1（depth=2）
- 214号"结构性不可达"被推翻

### 总体（52 数据集）
- T8：81/126 passed（64.3%），0 inconclusive
- gauge 保持率：~40%（四轮加权）
- T6/T7：日线级别结构性不可达，1min 级别 depth=2 标的待验证
- 208号修复（笔振荡 fallback）完全消除了 inconclusive（从 18/18 → 0）
- 低保持率弱不变量的失败模式已确认：几何量（区间精确值）不是拓扑不变量，符合理论预期
- 递归引擎在 1min 真实数据上正确运行，Level 1 可达
