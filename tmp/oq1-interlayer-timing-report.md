# OQ1 层间时序关系实验报告

生成时间: 2026-03-01T07:18:29.029712
认识论等级: L2（真实数据验证）
谱系引用: 279号（OQ1 层间关系假说）

## 1. 实验设计

- **管线**: 口径 A（RecursiveOrchestrator 从日线递归）
- **层1比价边**: E/Au (SPY/GLD), E/R (SPY/TLT), Au/R (GLD/TLT)
- **层2个股/ETF**: SPY, GLD, TLT, XLK, XLF, XLE
- **公共交易日**: 5352 (2004-11-18 ~ 2026-02-27)
- **总 bar**: 5352

## 2. 结构事件统计

| 流 | 层 | 总 bar | 总事件 | 事件明细 |
|---|---|---|---|---|
| Au/R | L1 | 5352 | 100 | bsp_confirm=2, move_candidate=6, move_settle=4, segment_settle=48, zhongshu_candidate=34, zhongshu_settle=6 |
| E/Au | L1 | 5352 | 135 | bsp_confirm=7, move_candidate=14, move_settle=12, segment_settle=49, zhongshu_candidate=39, zhongshu_settle=14 |
| E/R | L1 | 5352 | 119 | bsp_confirm=5, move_candidate=9, move_settle=8, segment_settle=47, zhongshu_candidate=41, zhongshu_settle=9 |
| GLD | L2 | 5352 | 98 | bsp_confirm=4, move_candidate=8, move_settle=6, segment_settle=46, zhongshu_candidate=26, zhongshu_settle=8 |
| SPY | L2 | 5352 | 90 | bsp_confirm=5, move_candidate=9, move_settle=7, segment_settle=37, zhongshu_candidate=23, zhongshu_settle=9 |
| TLT | L2 | 5352 | 119 | bsp_confirm=4, move_candidate=9, move_settle=6, segment_settle=55, zhongshu_candidate=36, zhongshu_settle=9 |
| XLE | L2 | 5352 | 125 | bsp_confirm=4, move_candidate=10, move_settle=8, segment_settle=53, zhongshu_candidate=40, zhongshu_settle=10 |
| XLF | L2 | 5352 | 96 | bsp_confirm=4, move_candidate=7, move_settle=5, segment_settle=42, zhongshu_candidate=31, zhongshu_settle=7 |
| XLK | L2 | 5352 | 95 | bsp_confirm=5, move_candidate=10, move_settle=8, segment_settle=35, zhongshu_candidate=27, zhongshu_settle=10 |

## 3. 层间时序配对分析

正 mean_lag = L1 领先 L2（比价事件先于个股事件）
负 mean_lag = L2 领先 L1（个股事件先于比价事件）

| L1 vs L2 | 配对数 | mean lag | median lag | L1先% | L2先% | 同时% | p-value | 显著性 |
|---|---|---|---|---|---|---|---|---|
| Au/R vs GLD | 60 | -5.28 | -6.0 | 35.0 | 60.0 | 5.0 | 0.1685 | 不显著 (p>=0.10) |
| Au/R vs SPY | 52 | -2.81 | 4.0 | 51.9 | 48.1 | 0.0 | 0.5751 | 不显著 (p>=0.10) |
| Au/R vs TLT | 74 | -5.62 | 0.0 | 50.0 | 50.0 | 0.0 | 0.1242 | 不显著 (p>=0.10) |
| Au/R vs XLE | 81 | 0.59 | 0.0 | 49.4 | 40.7 | 9.9 | 0.8595 | 不显著 (p>=0.10) |
| Au/R vs XLF | 72 | -4.42 | 0.0 | 48.6 | 47.2 | 4.2 | 0.1283 | 不显著 (p>=0.10) |
| Au/R vs XLK | 64 | -11.88 | -7.0 | 21.9 | 73.4 | 4.7 | 0.0011 | 高度显著 (p<0.01) |
| E/Au vs GLD | 94 | 0.83 | 0.0 | 47.9 | 38.3 | 13.8 | 0.788 | 不显著 (p>=0.10) |
| E/Au vs SPY | 77 | 11.45 | 8.0 | 61.0 | 31.2 | 7.8 | 0.0052 | 高度显著 (p<0.01) |
| E/Au vs TLT | 105 | -0.02 | -1.0 | 48.6 | 51.4 | 0.0 | 0.9977 | 不显著 (p>=0.10) |
| E/Au vs XLE | 117 | 3.71 | 7.0 | 53.0 | 46.2 | 0.9 | 0.1339 | 不显著 (p>=0.10) |
| E/Au vs XLF | 105 | 6.09 | 10.0 | 60.0 | 36.2 | 3.8 | 0.0639 | 边缘显著 (p<0.10) |
| E/Au vs XLK | 74 | -2.72 | -13.0 | 43.2 | 54.1 | 2.7 | 0.5288 | 不显著 (p>=0.10) |
| E/R vs GLD | 81 | -10.91 | -16.0 | 43.2 | 56.8 | 0.0 | 0.0021 | 高度显著 (p<0.01) |
| E/R vs SPY | 82 | -7.17 | -5.0 | 40.2 | 58.5 | 1.2 | 0.0517 | 边缘显著 (p<0.10) |
| E/R vs TLT | 82 | -4.18 | 0.0 | 45.1 | 46.3 | 8.5 | 0.3088 | 不显著 (p>=0.10) |
| E/R vs XLE | 96 | 4.22 | 2.0 | 55.2 | 41.7 | 3.1 | 0.1283 | 不显著 (p>=0.10) |
| E/R vs XLF | 74 | 0.91 | 1.0 | 55.4 | 41.9 | 2.7 | 0.7851 | 不显著 (p>=0.10) |
| E/R vs XLK | 70 | -0.53 | 1.0 | 51.4 | 48.6 | 0.0 | 0.8762 | 不显著 (p>=0.10) |

### 总体统计

- 总配对数: 1460
- mean lag: -1.0 bars
- median lag: 0.0 bars
- std: 30.84 bars
- L1 领先比例: 48.6%
- L2 领先比例: 47.6%
- 同时比例: 3.8%
- 置换检验 p-value: 0.2095
- 显著性: 不显著 (p>=0.10)

## 4. Cross-Correlation 峰值

| L1 vs L2 | peak lag (bars) | peak correlation |
|---|---|---|
| Au/R vs GLD | -41 | 0.045434 |
| Au/R vs SPY | 34 | 0.047273 |
| Au/R vs TLT | 1 | 0.060476 |
| Au/R vs XLE | 0 | 0.038163 |
| Au/R vs XLF | 0 | 0.040102 |
| Au/R vs XLK | -51 | 0.045633 |
| E/Au vs GLD | 0 | 0.069904 |
| E/Au vs SPY | 0 | 0.072488 |
| E/Au vs TLT | 25 | 0.058713 |
| E/Au vs XLE | -6 | 0.059592 |
| E/Au vs XLF | 0 | 0.038848 |
| E/Au vs XLK | -40 | 0.04431 |
| E/R vs GLD | -40 | 0.069849 |
| E/R vs SPY | 49 | 0.072423 |
| E/R vs TLT | 0 | 0.058753 |
| E/R vs XLE | 13 | 0.059581 |
| E/R vs XLF | 1 | 0.062317 |
| E/R vs XLK | -33 | 0.044319 |

## 5. 按事件类型分组

| 事件类型对 | 配对数 | mean lag | p-value | 显著性 |
|---|---|---|---|---|
| bsp_confirm -> segment_settle | 61 | -2.0 | 0.6313 | 不显著 (p>=0.10) |
| move_candidate -> segment_settle | 121 | -2.01 | 0.4888 | 不显著 (p>=0.10) |
| move_settle -> segment_settle | 101 | -3.27 | 0.3171 | 不显著 (p>=0.10) |
| segment_settle -> segment_settle | 571 | 0.01 | 0.9975 | 不显著 (p>=0.10) |
| segment_settle -> zhongshu_candidate | 13 | -29.15 | 0.0006 | 高度显著 (p<0.01) |
| zhongshu_candidate -> segment_settle | 453 | 0.62 | 0.6696 | 不显著 (p>=0.10) |
| zhongshu_candidate -> zhongshu_candidate | 11 | -28.0 | 0.0038 | 高度显著 (p<0.01) |
| zhongshu_settle -> segment_settle | 121 | -2.01 | 0.4888 | 不显著 (p>=0.10) |

## 6. 结论

### 结果包

1. **结论**: 见上表——层1比价事件与层2个股事件之间是否存在稳定的时序先后关系
2. **定义依据**: OQ1 层间关系假说（279号谱系）— 资本流动方向态在比价层先于个股层
3. **边界条件**:
   - ETF 上市日期不同导致公共数据长度受限
   - 结构事件频率受管线参数影响（笔模式、递归深度）
   - 60 bar 窗口是匹配阈值（可调整）
4. **下游推论**: 如果 L1 显著领先 L2 且 p<0.05，K4 配置可作为层2操作的先行指标
5. **谱系引用**: 279号（OQ1 层间关系假说）
6. **影响声明**: 实验结果，不修改现有代码或定义