# Agent A 分析报告：匿名比率时间序列结构性断点检测与聚类

数据范围：2000-08-30 to 2026-03-02
观测数量：6397
检测到的结构性断点：24 个
聚类数量：4 个
聚类置信度：low
平均轮廓系数：0.200

## 检测方法

- 主方法：PELT（rbf 核，惩罚参数=20，最小段长=60）
- 交叉验证：BinSeg（l2 模型）+ 120日滚动方差突变检测
- 共识机制：至少两种方法在40个交易日内确认的断点被保留

## 断点列表

| 序号 | 日期 | 比率值 | 聚类 | 断点置信度 | 聚类置信度 |
|------|------|--------|------|-----------|-----------|
| 01 | 2001-06-12 | 0.002672 | 1 | low | medium |
| 02 | 2002-07-17 | 0.002303 | 0 | low | low |
| 03 | 2004-01-16 | 0.002683 | 0 | high | medium |
| 04 | 2005-05-23 | 0.003573 | 0 | low | low |
| 05 | 2006-04-10 | 0.004583 | 0 | medium | medium |
| 06 | 2006-12-20 | 0.004737 | 1 | low | medium |
| 07 | 2007-03-20 | 0.004607 | 1 | low | medium |
| 08 | 2007-10-29 | 0.004452 | 0 | low | medium |
| 09 | 2008-10-03 | 0.003263 | 2 | high | low |
| 10 | 2009-04-03 | 0.002233 | 1 | medium | medium |
| 11 | 2009-07-22 | 0.002642 | 1 | low | medium |
| 12 | 2011-08-05 | 0.002495 | 3 | medium | low |
| 13 | 2013-05-08 | 0.002287 | 1 | low | medium |
| 14 | 2015-01-07 | 0.002310 | 0 | low | low |
| 15 | 2016-01-05 | 0.001937 | 1 | low | medium |
| 16 | 2016-11-10 | 0.002015 | 1 | low | medium |
| 17 | 2019-06-04 | 0.002020 | 1 | low | medium |
| 18 | 2020-01-29 | 0.001627 | 1 | medium | medium |
| 19 | 2020-11-18 | 0.001707 | 0 | low | low |
| 20 | 2021-02-17 | 0.002166 | 1 | low | medium |
| 21 | 2022-06-15 | 0.002296 | 0 | low | medium |
| 22 | 2023-04-25 | 0.001929 | 1 | low | medium |
| 23 | 2024-07-26 | 0.001725 | 1 | medium | high |
| 24 | 2025-07-31 | 0.001315 | 1 | low | medium |

## 各断点后动态描述

### BP01 — 2001-06-12

比率值：0.002672
聚类分配：Cluster 1
动态描述：large downward level shift (-0.1725 in log-ratio); moderate initial departure speed; volatility surged (2.2x pre-breakpoint); continued downward drift (slope=-0.001198/day); diverging behavior (distance from prior level increasing over time); low oscillation (trending or stable); rapid stabilization (~19 trading days); established a clearly distinct new regime

- 水平位移（对数）：-0.1725
- 波动率变化倍数：2.18x
- 趋势斜率：-0.001198/日
- 离场速度：0.024685
- 均值回归相关性：0.616
- 振荡率：0.033
- 稳定化天数：19
- 新制度距离：6.77

### BP02 — 2002-07-17

比率值：0.002303
聚类分配：Cluster 0
动态描述：moderate downward level shift (-0.0794 in log-ratio); rapid initial departure; volatility increased (2.0x); continued downward drift (slope=-0.000084/day); neither clearly mean-reverting nor diverging; low oscillation (trending or stable); gradual stabilization (~28 trading days); established a clearly distinct new regime

- 水平位移（对数）：-0.0794
- 波动率变化倍数：1.97x
- 趋势斜率：-0.000084/日
- 离场速度：0.039564
- 均值回归相关性：0.078
- 振荡率：0.092
- 稳定化天数：28
- 新制度距离：3.55

### BP03 — 2004-01-16

比率值：0.002683
聚类分配：Cluster 0
动态描述：large upward level shift (+0.2522 in log-ratio); rapid initial departure; volatility increased (1.5x); continued upward drift (slope=0.000324/day); neither clearly mean-reverting nor diverging; low oscillation (trending or stable); gradual stabilization (~38 trading days); established a clearly distinct new regime

- 水平位移（对数）：+0.2522
- 波动率变化倍数：1.55x
- 趋势斜率：0.000324/日
- 离场速度：0.031365
- 均值回归相关性：0.201
- 振荡率：0.125
- 稳定化天数：38
- 新制度距离：6.86

### BP04 — 2005-05-23

比率值：0.003573
聚类分配：Cluster 0
动态描述：moderate upward level shift (+0.1206 in log-ratio); rapid initial departure; volatility surged (2.4x pre-breakpoint); continued upward drift (slope=0.000863/day); diverging behavior (distance from prior level increasing over time); low oscillation (trending or stable); gradual stabilization (~24 trading days); established a clearly distinct new regime

- 水平位移（对数）：+0.1206
- 波动率变化倍数：2.36x
- 趋势斜率：0.000863/日
- 离场速度：0.086925
- 均值回归相关性：0.773
- 振荡率：0.075
- 稳定化天数：24
- 新制度距离：9.35

### BP05 — 2006-04-10

比率值：0.004583
聚类分配：Cluster 0
动态描述：large upward level shift (+0.3095 in log-ratio); rapid initial departure; volatility increased (1.3x); continued upward drift (slope=0.000915/day); diverging behavior (distance from prior level increasing over time); low oscillation (trending or stable); gradual stabilization (~26 trading days); established a clearly distinct new regime

- 水平位移（对数）：+0.3095
- 波动率变化倍数：1.31x
- 趋势斜率：0.000915/日
- 离场速度：0.057520
- 均值回归相关性：0.578
- 振荡率：0.225
- 稳定化天数：26
- 新制度距离：7.85

### BP06 — 2006-12-20

比率值：0.004737
聚类分配：Cluster 1
动态描述：large downward level shift (-0.2570 in log-ratio); rapid initial departure; volatility roughly unchanged; continued downward drift (slope=-0.000470/day); neither clearly mean-reverting nor diverging; low oscillation (trending or stable); rapid stabilization (~19 trading days); partial regime shift (new level distinguishable but not far)

- 水平位移（对数）：-0.2570
- 波动率变化倍数：0.83x
- 趋势斜率：-0.000470/日
- 离场速度：0.067954
- 均值回归相关性：0.125
- 振荡率：0.067
- 稳定化天数：19
- 新制度距离：2.94

### BP07 — 2007-03-20

比率值：0.004607
聚类分配：Cluster 1
动态描述：large upward level shift (+0.2176 in log-ratio); rapid initial departure; volatility roughly unchanged; continued upward drift (slope=0.000214/day); neither clearly mean-reverting nor diverging; low oscillation (trending or stable); rapid stabilization (~19 trading days); established a clearly distinct new regime

- 水平位移（对数）：+0.2176
- 波动率变化倍数：0.71x
- 趋势斜率：0.000214/日
- 离场速度：0.044361
- 均值回归相关性：0.159
- 振荡率：0.083
- 稳定化天数：19
- 新制度距离：3.32

### BP08 — 2007-10-29

比率值：0.004452
聚类分配：Cluster 0
动态描述：large downward level shift (-0.2317 in log-ratio); rapid initial departure; volatility increased (1.7x); continued upward drift (slope=0.000653/day); mean-reverting behavior (distance from prior level decreasing over time); low oscillation (trending or stable); gradual stabilization (~26 trading days); established a clearly distinct new regime

- 水平位移（对数）：-0.2317
- 波动率变化倍数：1.72x
- 趋势斜率：0.000653/日
- 离场速度：0.056053
- 均值回归相关性：-0.362
- 振荡率：0.100
- 稳定化天数：26
- 新制度距离：4.96

### BP09 — 2008-10-03

比率值：0.003263
聚类分配：Cluster 2
动态描述：large downward level shift (-0.7113 in log-ratio); rapid initial departure; volatility surged (2.7x pre-breakpoint); continued downward drift (slope=-0.004233/day); diverging behavior (distance from prior level increasing over time); low oscillation (trending or stable); gradual stabilization (~20 trading days); established a clearly distinct new regime

- 水平位移（对数）：-0.7113
- 波动率变化倍数：2.68x
- 趋势斜率：-0.004233/日
- 离场速度：0.117180
- 均值回归相关性：0.743
- 振荡率：0.050
- 稳定化天数：20
- 新制度距离：11.27

### BP10 — 2009-04-03

比率值：0.002233
聚类分配：Cluster 1
动态描述：large upward level shift (+0.3068 in log-ratio); rapid initial departure; volatility compressed (0.5x); continued upward drift (slope=0.001187/day); diverging behavior (distance from prior level increasing over time); low oscillation (trending or stable); rapid stabilization (~19 trading days); established a clearly distinct new regime

- 水平位移（对数）：+0.3068
- 波动率变化倍数：0.53x
- 趋势斜率：0.001187/日
- 离场速度：0.060598
- 均值回归相关性：0.547
- 振荡率：0.147
- 稳定化天数：19
- 新制度距离：3.89

### BP11 — 2009-07-22

比率值：0.002642
聚类分配：Cluster 1
动态描述：large upward level shift (+0.1732 in log-ratio); rapid initial departure; volatility roughly unchanged; continued downward drift (slope=-0.000042/day); neither clearly mean-reverting nor diverging; low oscillation (trending or stable); gradual stabilization (~20 trading days); established a clearly distinct new regime

- 水平位移（对数）：+0.1732
- 波动率变化倍数：0.92x
- 趋势斜率：-0.000042/日
- 离场速度：0.039977
- 均值回归相关性：-0.033
- 振荡率：0.075
- 稳定化天数：20
- 新制度距离：3.59

### BP12 — 2011-08-05

比率值：0.002495
聚类分配：Cluster 3
动态描述：large downward level shift (-0.2578 in log-ratio); rapid initial departure; volatility surged (2.2x pre-breakpoint); continued downward drift (slope=-0.000265/day); neither clearly mean-reverting nor diverging; low oscillation (trending or stable); gradual stabilization (~52 trading days); established a clearly distinct new regime

- 水平位移（对数）：-0.2578
- 波动率变化倍数：2.16x
- 趋势斜率：-0.000265/日
- 离场速度：0.075103
- 均值回归相关性：0.160
- 振荡率：0.050
- 稳定化天数：52
- 新制度距离：9.05

### BP13 — 2013-05-08

比率值：0.002287
聚类分配：Cluster 1
动态描述：moderate upward level shift (+0.0931 in log-ratio); rapid initial departure; volatility roughly unchanged; continued upward drift (slope=0.000450/day); diverging behavior (distance from prior level increasing over time); low oscillation (trending or stable); rapid stabilization (~19 trading days); established a clearly distinct new regime

- 水平位移（对数）：+0.0931
- 波动率变化倍数：1.06x
- 趋势斜率：0.000450/日
- 离场速度：0.041049
- 均值回归相关性：0.506
- 振荡率：0.100
- 稳定化天数：19
- 新制度距离：3.73

### BP14 — 2015-01-07

比率值：0.002310
聚类分配：Cluster 0
动态描述：moderate downward level shift (-0.0937 in log-ratio); rapid initial departure; volatility increased (1.7x); continued upward drift (slope=0.001000/day); mean-reverting behavior (distance from prior level decreasing over time); low oscillation (trending or stable); gradual stabilization (~23 trading days); partial regime shift (new level distinguishable but not far)

- 水平位移（对数）：-0.0937
- 波动率变化倍数：1.66x
- 趋势斜率：0.001000/日
- 离场速度：0.048255
- 均值回归相关性：-0.608
- 振荡率：0.108
- 稳定化天数：23
- 新制度距离：1.70

### BP15 — 2016-01-05

比率值：0.001937
聚类分配：Cluster 1
动态描述：moderate downward level shift (-0.1333 in log-ratio); rapid initial departure; volatility increased (1.5x); continued downward drift (slope=-0.000849/day); diverging behavior (distance from prior level increasing over time); low oscillation (trending or stable); rapid stabilization (~19 trading days); established a clearly distinct new regime

- 水平位移（对数）：-0.1333
- 波动率变化倍数：1.54x
- 趋势斜率：-0.000849/日
- 离场速度：0.046107
- 均值回归相关性：0.665
- 振荡率：0.075
- 稳定化天数：19
- 新制度距离：6.15

### BP16 — 2016-11-10

比率值：0.002015
聚类分配：Cluster 1
动态描述：large upward level shift (+0.2630 in log-ratio); rapid initial departure; volatility roughly unchanged; continued downward drift (slope=-0.000566/day); mean-reverting behavior (distance from prior level decreasing over time); low oscillation (trending or stable); gradual stabilization (~22 trading days); established a clearly distinct new regime

- 水平位移（对数）：+0.2630
- 波动率变化倍数：0.85x
- 趋势斜率：-0.000566/日
- 离场速度：0.042918
- 均值回归相关性：-0.523
- 振荡率：0.167
- 稳定化天数：22
- 新制度距离：5.27

### BP17 — 2019-06-04

比率值：0.002020
聚类分配：Cluster 1
动态描述：large downward level shift (-0.2001 in log-ratio); moderate initial departure speed; volatility increased (1.7x); continued downward drift (slope=-0.001033/day); diverging behavior (distance from prior level increasing over time); low oscillation (trending or stable); rapid stabilization (~19 trading days); established a clearly distinct new regime

- 水平位移（对数）：-0.2001
- 波动率变化倍数：1.66x
- 趋势斜率：-0.001033/日
- 离场速度：0.025034
- 均值回归相关性：0.642
- 振荡率：0.042
- 稳定化天数：19
- 新制度距离：6.61

### BP18 — 2020-01-29

比率值：0.001627
聚类分配：Cluster 1
动态描述：large downward level shift (-0.2077 in log-ratio); rapid initial departure; volatility surged (2.2x pre-breakpoint); continued downward drift (slope=-0.000406/day); neither clearly mean-reverting nor diverging; low oscillation (trending or stable); rapid stabilization (~19 trading days); established a clearly distinct new regime

- 水平位移（对数）：-0.2077
- 波动率变化倍数：2.15x
- 趋势斜率：-0.000406/日
- 离场速度：0.030773
- 均值回归相关性：0.200
- 振荡率：0.033
- 稳定化天数：19
- 新制度距离：5.74

### BP19 — 2020-11-18

比率值：0.001707
聚类分配：Cluster 0
动态描述：large upward level shift (+0.1833 in log-ratio); rapid initial departure; volatility increased (1.4x); continued upward drift (slope=0.001834/day); diverging behavior (distance from prior level increasing over time); low oscillation (trending or stable); gradual stabilization (~21 trading days); established a clearly distinct new regime

- 水平位移（对数）：+0.1833
- 波动率变化倍数：1.35x
- 趋势斜率：0.001834/日
- 离场速度：0.039344
- 均值回归相关性：0.784
- 振荡率：0.183
- 稳定化天数：21
- 新制度距离：7.28

### BP20 — 2021-02-17

比率值：0.002166
聚类分配：Cluster 1
动态描述：large upward level shift (+0.2267 in log-ratio); rapid initial departure; volatility roughly unchanged; continued upward drift (slope=0.000248/day); neither clearly mean-reverting nor diverging; low oscillation (trending or stable); rapid stabilization (~19 trading days); established a clearly distinct new regime

- 水平位移（对数）：+0.2267
- 波动率变化倍数：0.81x
- 趋势斜率：0.000248/日
- 离场速度：0.043605
- 均值回归相关性：0.263
- 振荡率：0.158
- 稳定化天数：19
- 新制度距离：5.56

### BP21 — 2022-06-15

比率值：0.002296
聚类分配：Cluster 0
动态描述：moderate downward level shift (-0.1338 in log-ratio); rapid initial departure; volatility increased (1.3x); continued upward drift (slope=0.000465/day); mean-reverting behavior (distance from prior level decreasing over time); low oscillation (trending or stable); gradual stabilization (~29 trading days); established a clearly distinct new regime

- 水平位移（对数）：-0.1338
- 波动率变化倍数：1.35x
- 趋势斜率：0.000465/日
- 离场速度：0.045376
- 均值回归相关性：-0.442
- 振荡率：0.167
- 稳定化天数：29
- 新制度距离：4.07

### BP22 — 2023-04-25

比率值：0.001929
聚类分配：Cluster 1
动态描述：moderate downward level shift (-0.0892 in log-ratio); rapid initial departure; volatility compressed (0.5x); continued upward drift (slope=0.000263/day); mean-reverting behavior (distance from prior level decreasing over time); low oscillation (trending or stable); rapid stabilization (~19 trading days); partial regime shift (new level distinguishable but not far)

- 水平位移（对数）：-0.0892
- 波动率变化倍数：0.53x
- 趋势斜率：0.000263/日
- 离场速度：0.033232
- 均值回归相关性：-0.394
- 振荡率：0.117
- 稳定化天数：19
- 新制度距离：1.91

### BP23 — 2024-07-26

比率值：0.001725
聚类分配：Cluster 1
动态描述：large downward level shift (-0.1870 in log-ratio); moderate initial departure speed; volatility compressed (0.7x); continued downward drift (slope=-0.000765/day); diverging behavior (distance from prior level increasing over time); low oscillation (trending or stable); rapid stabilization (~19 trading days); established a clearly distinct new regime

- 水平位移（对数）：-0.1870
- 波动率变化倍数：0.67x
- 趋势斜率：-0.000765/日
- 离场速度：0.026229
- 均值回归相关性：0.777
- 振荡率：0.092
- 稳定化天数：19
- 新制度距离：4.36

### BP24 — 2025-07-31

比率值：0.001315
聚类分配：Cluster 1
动态描述：large downward level shift (-0.1773 in log-ratio); moderate initial departure speed; volatility compressed (0.5x); continued downward drift (slope=-0.000283/day); neither clearly mean-reverting nor diverging; low oscillation (trending or stable); rapid stabilization (~19 trading days); partial regime shift (new level distinguishable but not far)

- 水平位移（对数）：-0.1773
- 波动率变化倍数：0.46x
- 趋势斜率：-0.000283/日
- 离场速度：0.020287
- 均值回归相关性：0.299
- 振荡率：0.192
- 稳定化天数：19
- 新制度距离：2.50

## 聚类特征

### Cluster 0（8 个成员）

特征描述：moderate upward shift, elevated volatility, fast departure, strong new regime established

- 平均水平位移：+0.0409
- 平均波动率倍数：1.66x
- 平均离场速度：0.050550
- 平均均值回归：0.125
- 平均振荡率：0.134
- 平均稳定化天数：27
- 平均新制度距离：5.70
- 成员日期：2002-07-17, 2004-01-16, 2005-05-23, 2006-04-10, 2007-10-29, 2015-01-07, 2020-11-18, 2022-06-15

### Cluster 1（14 个成员）

特征描述：mixed/neutral direction, stable volatility, fast departure, new regime partially established

- 平均水平位移：-0.0102
- 平均波动率倍数：1.06x
- 平均离场速度：0.039058
- 平均均值回归：0.275
- 平均振荡率：0.099
- 平均稳定化天数：19
- 平均新制度距离：4.45
- 成员日期：2001-06-12, 2006-12-20, 2007-03-20, 2009-04-03, 2009-07-22, 2013-05-08, 2016-01-05, 2016-11-10, 2019-06-04, 2020-01-29, 2021-02-17, 2023-04-25, 2024-07-26, 2025-07-31

### Cluster 2（1 个成员）

特征描述：extreme downward shift, volatility explosion, fast departure, persistently diverging, strong new regime established

- 平均水平位移：-0.7113
- 平均波动率倍数：2.68x
- 平均离场速度：0.117180
- 平均均值回归：0.743
- 平均振荡率：0.050
- 平均稳定化天数：20
- 平均新制度距离：11.27
- 成员日期：2008-10-03

### Cluster 3（1 个成员）

特征描述：strong downward shift, volatility explosion, fast departure, strong new regime established

- 平均水平位移：-0.2578
- 平均波动率倍数：2.16x
- 平均离场速度：0.075103
- 平均均值回归：0.160
- 平均振荡率：0.050
- 平均稳定化天数：52
- 平均新制度距离：9.05
- 成员日期：2011-08-05

## 离散类型 vs 连续谱

The clustering is weak, suggesting the post-breakpoint behaviors form more of a continuous spectrum than discrete types. Cluster assignments should be treated as approximate groupings rather than definitive categories.