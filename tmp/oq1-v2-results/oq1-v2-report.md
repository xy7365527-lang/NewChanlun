# OQ1 v2 层间结构事件时序关系实验报告

生成时间: 2026-03-01T09:15:36.796079
认识论等级: L2（真实数据验证）
谱系引用: 284号（OQ v2）、283号、279号

## 1. 实验设计

### 284号变更

- 背驰判断（走势延续 + MACD 面积缩小）是缠论内部的统一判断
- 形态学和动力学不是两层独立的东西
- D 算子 + 背驰判断一起跑在每条边上
- RecursiveOrchestrator 内部已整合形态+背驰+买卖点

### 管线

- RecursiveOrchestrator 从日线递归
- 层1：K4 完全图六条边（顶点边 E/$, Au/$, R/$ + 比价边 E/Au, Au/R, E/R）
- 层2：SPY, GLD, TLT, XLK, XLF, XLE
- 公共交易日: 5352 (2004-11-18 ~ 2026-02-27)
- 总 bar: 5352

## 2. 结构事件统计

| 流 | 层 | 总 bar | 总事件 | 事件明细 |
|---|---|---|---|---|
| Au/$ | L1 | 5352 | 112 | bsp_candidate=14, bsp_confirm=4, move_candidate=8, move_settle=6, segment_settle=46, zhongshu_candidate=26, zhongshu_settle=8 |
| Au/R | L1 | 5352 | 124 | bsp_candidate=10, bsp_confirm=2, move_candidate=7, move_settle=6, segment_settle=50, zhongshu_candidate=42, zhongshu_settle=7 |
| E/$ | L1 | 5352 | 114 | bsp_candidate=24, bsp_confirm=5, move_candidate=9, move_settle=7, segment_settle=37, zhongshu_candidate=23, zhongshu_settle=9 |
| E/Au | L1 | 5352 | 147 | bsp_candidate=31, bsp_confirm=5, move_candidate=10, move_settle=8, segment_settle=48, zhongshu_candidate=35, zhongshu_settle=10 |
| E/R | L1 | 5352 | 135 | bsp_candidate=28, bsp_confirm=4, move_candidate=7, move_settle=6, segment_settle=47, zhongshu_candidate=36, zhongshu_settle=7 |
| R/$ | L1 | 5352 | 144 | bsp_candidate=25, bsp_confirm=4, move_candidate=9, move_settle=6, segment_settle=55, zhongshu_candidate=36, zhongshu_settle=9 |
| GLD | L2 | 5352 | 112 | bsp_candidate=14, bsp_confirm=4, move_candidate=8, move_settle=6, segment_settle=46, zhongshu_candidate=26, zhongshu_settle=8 |
| SPY | L2 | 5352 | 114 | bsp_candidate=24, bsp_confirm=5, move_candidate=9, move_settle=7, segment_settle=37, zhongshu_candidate=23, zhongshu_settle=9 |
| TLT | L2 | 5352 | 144 | bsp_candidate=25, bsp_confirm=4, move_candidate=9, move_settle=6, segment_settle=55, zhongshu_candidate=36, zhongshu_settle=9 |
| XLE | L2 | 5352 | 162 | bsp_candidate=37, bsp_confirm=4, move_candidate=10, move_settle=8, segment_settle=53, zhongshu_candidate=40, zhongshu_settle=10 |
| XLF | L2 | 5352 | 117 | bsp_candidate=20, bsp_confirm=4, move_candidate=7, move_settle=5, segment_settle=42, zhongshu_candidate=32, zhongshu_settle=7 |
| XLK | L2 | 5352 | 117 | bsp_candidate=18, bsp_confirm=5, move_candidate=10, move_settle=8, segment_settle=37, zhongshu_candidate=29, zhongshu_settle=10 |

## 3. 层间时序配对分析

正 mean_lag = L1 领先 L2（比价/顶点边事件先于个股事件）
负 mean_lag = L2 领先 L1（个股事件先于比价事件）

| L1 vs L2 | 配对数 | mean lag | median lag | L1先% | L2先% | 同时% | p-value | 显著性 |
|---|---|---|---|---|---|---|---|---|
| Au/$ vs GLD | 112 | 0.0 | 0.0 | 0.0 | 0.0 | 100.0 | 1.0 | 不显著 (p>=0.10) |
| Au/$ vs SPY | 66 | 17.91 | 35.0 | 60.6 | 39.4 | 0.0 | 0.0002 | 高度显著 (p<0.01) |
| Au/$ vs TLT | 79 | 4.46 | 0.0 | 49.4 | 45.6 | 5.1 | 0.1889 | 不显著 (p>=0.10) |
| Au/$ vs XLE | 77 | -2.82 | -5.0 | 49.4 | 50.6 | 0.0 | 0.4068 | 不显著 (p>=0.10) |
| Au/$ vs XLF | 83 | -10.29 | -7.0 | 45.8 | 54.2 | 0.0 | 0.0024 | 高度显著 (p<0.01) |
| Au/$ vs XLK | 66 | 12.58 | 23.0 | 71.2 | 28.8 | 0.0 | 0.0037 | 高度显著 (p<0.01) |
| Au/R vs GLD | 86 | 2.65 | 1.0 | 55.8 | 36.0 | 8.1 | 0.3815 | 不显著 (p>=0.10) |
| Au/R vs SPY | 71 | 4.72 | 16.0 | 66.2 | 32.4 | 1.4 | 0.1855 | 不显著 (p>=0.10) |
| Au/R vs TLT | 86 | -3.1 | -1.0 | 44.2 | 52.3 | 3.5 | 0.3233 | 不显著 (p>=0.10) |
| Au/R vs XLE | 105 | -0.51 | -4.0 | 40.0 | 55.2 | 4.8 | 0.8123 | 不显著 (p>=0.10) |
| Au/R vs XLF | 93 | -2.56 | 2.0 | 54.8 | 41.9 | 3.2 | 0.344 | 不显著 (p>=0.10) |
| Au/R vs XLK | 82 | -3.11 | 0.0 | 47.6 | 47.6 | 4.9 | 0.321 | 不显著 (p>=0.10) |
| E/$ vs GLD | 90 | 8.88 | 13.0 | 67.8 | 32.2 | 0.0 | 0.0259 | 显著 (p<0.05) |
| E/$ vs SPY | 114 | 0.0 | 0.0 | 0.0 | 0.0 | 100.0 | 1.0 | 不显著 (p>=0.10) |
| E/$ vs TLT | 72 | 3.14 | 0.0 | 48.6 | 45.8 | 5.6 | 0.4411 | 不显著 (p>=0.10) |
| E/$ vs XLE | 86 | -4.83 | -1.0 | 34.9 | 52.3 | 12.8 | 0.0573 | 边缘显著 (p<0.10) |
| E/$ vs XLF | 87 | -11.91 | -14.0 | 32.2 | 55.2 | 12.6 | 0.002 | 高度显著 (p<0.01) |
| E/$ vs XLK | 78 | -3.91 | -3.0 | 38.5 | 52.6 | 9.0 | 0.2536 | 不显著 (p>=0.10) |
| E/Au vs GLD | 96 | -6.12 | -3.5 | 36.5 | 53.1 | 10.4 | 0.019 | 显著 (p<0.05) |
| E/Au vs SPY | 93 | 3.05 | 0.0 | 48.4 | 46.2 | 5.4 | 0.389 | 不显著 (p>=0.10) |
| E/Au vs TLT | 110 | -0.22 | -4.0 | 49.1 | 50.9 | 0.0 | 0.946 | 不显著 (p>=0.10) |
| E/Au vs XLE | 116 | 0.37 | 7.0 | 54.3 | 44.0 | 1.7 | 0.8853 | 不显著 (p>=0.10) |
| E/Au vs XLF | 110 | -2.15 | -3.0 | 33.6 | 60.9 | 5.5 | 0.4611 | 不显著 (p>=0.10) |
| E/Au vs XLK | 79 | -1.48 | -5.0 | 43.0 | 57.0 | 0.0 | 0.7105 | 不显著 (p>=0.10) |
| E/R vs GLD | 87 | -3.48 | -8.0 | 40.2 | 56.3 | 3.4 | 0.268 | 不显著 (p>=0.10) |
| E/R vs SPY | 74 | 6.16 | 10.5 | 55.4 | 35.1 | 9.5 | 0.0949 | 边缘显著 (p<0.10) |
| E/R vs TLT | 97 | 7.8 | 9.0 | 59.8 | 38.1 | 2.1 | 0.0022 | 高度显著 (p<0.01) |
| E/R vs XLE | 113 | -2.84 | 0.0 | 38.1 | 48.7 | 13.3 | 0.2619 | 不显著 (p>=0.10) |
| E/R vs XLF | 99 | 3.35 | 6.0 | 60.6 | 35.4 | 4.0 | 0.2935 | 不显著 (p>=0.10) |
| E/R vs XLK | 73 | -9.66 | -10.0 | 28.8 | 64.4 | 6.8 | 0.0026 | 高度显著 (p<0.01) |
| R/$ vs GLD | 90 | 10.2 | 8.0 | 56.7 | 35.6 | 7.8 | 0.0051 | 高度显著 (p<0.01) |
| R/$ vs SPY | 64 | 14.58 | 22.0 | 71.9 | 26.6 | 1.6 | 0.0007 | 高度显著 (p<0.01) |
| R/$ vs TLT | 144 | 0.0 | 0.0 | 0.0 | 0.0 | 100.0 | 1.0 | 不显著 (p>=0.10) |
| R/$ vs XLE | 113 | 9.9 | 22.0 | 61.9 | 33.6 | 4.4 | 0.0021 | 高度显著 (p<0.01) |
| R/$ vs XLF | 104 | 5.12 | 6.0 | 63.5 | 34.6 | 1.9 | 0.0736 | 边缘显著 (p<0.10) |
| R/$ vs XLK | 79 | 7.33 | 7.0 | 57.0 | 31.6 | 11.4 | 0.0185 | 显著 (p<0.05) |

### 总体统计

- 总配对数: 3274
- mean lag: 1.21 bars
- median lag: 0.0 bars
- std: 28.69 bars
- L1 领先比例: 44.4%
- L2 领先比例: 39.9%
- 同时比例: 15.7%
- 置换检验 p-value: 0.0176
- 显著性: 显著 (p<0.05)
- 方向: L1 领先 L2（比价事件先于个股事件）

## 4. Cross-Correlation 峰值

| L1 vs L2 | peak lag (bars) | peak correlation |
|---|---|---|
| Au/$ vs GLD | 0 | 1.0 |
| Au/$ vs SPY | 40 | 0.060575 |
| Au/$ vs TLT | -42 | 0.051789 |
| Au/$ vs XLE | -16 | 0.044622 |
| Au/$ vs XLF | 10 | 0.05413 |
| Au/$ vs XLK | 46 | 0.061395 |
| Au/R vs GLD | 0 | 0.034991 |
| Au/R vs SPY | 24 | 0.051885 |
| Au/R vs TLT | 8 | 0.079898 |
| Au/R vs XLE | -12 | 0.053496 |
| Au/R vs XLF | -23 | 0.046113 |
| Au/R vs XLK | 16 | 0.073292 |
| E/$ vs GLD | -40 | 0.060575 |
| E/$ vs SPY | 0 | 1.0 |
| E/$ vs TLT | 28 | 0.046989 |
| E/$ vs XLE | 0 | 0.091757 |
| E/$ vs XLF | 0 | 0.088728 |
| E/$ vs XLK | -46 | 0.056437 |
| E/Au vs GLD | 0 | 0.068517 |
| E/Au vs SPY | 39 | 0.062622 |
| E/Au vs TLT | 35 | 0.069126 |
| E/Au vs XLE | -41 | 0.059813 |
| E/Au vs XLF | 0 | 0.07242 |
| E/Au vs XLK | 43 | 0.063511 |
| E/R vs GLD | 5 | 0.052287 |
| E/R vs SPY | 0 | 0.047602 |
| E/R vs TLT | -2 | 0.039743 |
| E/R vs XLE | 0 | 0.048797 |
| E/R vs XLF | 36 | 0.059415 |
| E/R vs XLK | -33 | 0.029094 |
| R/$ vs GLD | 42 | 0.051789 |
| R/$ vs SPY | -28 | 0.046989 |
| R/$ vs TLT | 0 | 1.0 |
| R/$ vs XLE | 56 | 0.048261 |
| R/$ vs XLF | 8 | 0.041356 |
| R/$ vs XLK | -23 | 0.047934 |

## 5. 按事件类型分组

| 事件类型对 | 配对数 | mean lag | p-value | 显著性 |
|---|---|---|---|---|
| bsp_candidate -> bsp_candidate | 98 | 0.81 | 0.6449 | 不显著 (p>=0.10) |
| bsp_candidate -> segment_settle | 416 | 2.2 | 0.1394 | 不显著 (p>=0.10) |
| bsp_candidate -> zhongshu_candidate | 7 | -14.57 | 0.1057 | 不显著 (p>=0.10) |
| bsp_confirm -> bsp_candidate | 8 | -1.0 | 0.9215 | 不显著 (p>=0.10) |
| bsp_confirm -> segment_settle | 88 | 4.39 | 0.1671 | 不显著 (p>=0.10) |
| move_candidate -> bsp_candidate | 20 | -4.0 | 0.5367 | 不显著 (p>=0.10) |
| move_candidate -> segment_settle | 182 | 3.54 | 0.1207 | 不显著 (p>=0.10) |
| move_candidate -> zhongshu_candidate | 5 | -38.8 | 0.061 | 边缘显著 (p<0.10) |
| move_settle -> bsp_candidate | 18 | -1.28 | 0.8568 | 不显著 (p>=0.10) |
| move_settle -> segment_settle | 144 | 3.43 | 0.1782 | 不显著 (p>=0.10) |
| move_settle -> zhongshu_candidate | 3 | -34.0 | 0.2528 | 不显著 (p>=0.10) |
| segment_settle -> bsp_candidate | 108 | -7.87 | 0.0008 | 高度显著 (p<0.01) |
| segment_settle -> segment_settle | 1095 | 2.71 | 0.0009 | 高度显著 (p<0.01) |
| segment_settle -> zhongshu_candidate | 18 | -24.67 | 0.0002 | 高度显著 (p<0.01) |
| zhongshu_candidate -> bsp_candidate | 74 | -9.39 | 0.0016 | 高度显著 (p<0.01) |
| zhongshu_candidate -> segment_settle | 773 | 1.05 | 0.3054 | 不显著 (p>=0.10) |
| zhongshu_candidate -> zhongshu_candidate | 9 | -17.0 | 0.0463 | 显著 (p<0.05) |
| zhongshu_settle -> bsp_candidate | 20 | -4.0 | 0.5367 | 不显著 (p>=0.10) |
| zhongshu_settle -> segment_settle | 182 | 3.54 | 0.1207 | 不显著 (p>=0.10) |
| zhongshu_settle -> zhongshu_candidate | 5 | -38.8 | 0.061 | 边缘显著 (p<0.10) |

## 6. 结果包

1. **结论**: 见上表——层1 K4六条边和层2个股/ETF之间的时序先后关系
2. **定义依据**: 284号 OQ v2——背驰是形态和力度的交叉判断，不是独立层
3. **边界条件**:
   - ETF 上市日期限制公共窗口长度
   - RecursiveOrchestrator 管线参数（笔模式、递归深度）影响事件频率
   - 60 bar 匹配窗口是可调参数
4. **下游推论**: 如果 L1 显著领先 L2 且 p<0.05，K4 配置变化可作为层2操作的先行指标
5. **谱系引用**: 284号、283号、279号
6. **影响声明**: 实验结果，不修改现有代码或定义