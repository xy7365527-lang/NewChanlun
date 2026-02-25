# 真实市场数据验证报告

生成时间：2026-02-25 21:34
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
| sz000001 平安银行 | 1974 | 1/3 | 1/3 | 1/3 | N/A | N/A | N/A | 0/0(inc:3) | 46.7% |
| sh600036 招商银行 | 1974 | 0/3 | 3/3 | 3/3 | N/A | N/A | N/A | 0/0(inc:3) | 60.0% |
| sz000858 五粮液 | 1974 | 0/3 | 1/3 | 1/3 | N/A | N/A | N/A | 0/0(inc:2) | 20.0% |
| sh601318 中国平安 | 1974 | 1/3 | 3/3 | 3/3 | N/A | N/A | N/A | 0/0(inc:5) | 60.0% |
| sz002475 立讯精密 | 1974 | 1/3 | 3/3 | 3/3 | N/A | N/A | N/A | 0/0(inc:1) | 60.0% |
| SYN-A 深递归合成 | 1200 | 0/3 | 1/3 | 1/3 | N/A | N/A | N/A | 0/0(inc:2) | 20.0% |
| SYN-B 多中枢背驰合成 | 1500 | 1/3 | 3/3 | 3/3 | N/A | N/A | N/A | 0/0(inc:2) | 60.0% |

## T6 跨层 Leray 可计算近似详细结果

所有数据集均无 T6 结果（递归级别 < 2）。

## T6 参数校准（207号-3）

无 T6 数据可供校准。

## T8 背驰拓扑详细结果

### sz000001 平安银行

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | strict | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | new | 1 |

非inconclusive：0项

### sh600036 招商银行

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | strict | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | new | 1 |

非inconclusive：0项

### sz000858 五粮液

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | strict | 1 |

非inconclusive：0项

### sh601318 中国平安

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | wide | 1 |
| 1 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | wide | 1 |
| 0 | consolidation | bottom | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | strict | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | new | 1 |
| 1 | consolidation | bottom | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | new | 1 |

非inconclusive：0项

### sz002475 立讯精密

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | bottom | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | strict | 1 |

非inconclusive：0项

### SYN-A 深递归合成

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | new | 1 |

非inconclusive：0项

### SYN-B 多中枢背驰合成

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | new | 1 |

非inconclusive：0项

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
- strong: {'n_centers': False, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': False, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=14, segment_diff=0, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict -> new**
- stroke_diff=44, segment_diff=2, center_diff=1, level_diff=0
- bottleneck=2.3050, beta1_tau_diff=1
- trend_mutations=[]
- strong: {'n_centers': False, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': False, 'barcode_bottleneck': False}

### sh600036 招商银行

**wide -> strict**
- stroke_diff=-29, segment_diff=-3, center_diff=0, level_diff=0
- bottleneck=4.4300, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=22, segment_diff=2, center_diff=0, level_diff=0
- bottleneck=3.1150, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

**strict -> new**
- stroke_diff=51, segment_diff=5, center_diff=0, level_diff=0
- bottleneck=1.4300, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

### sz000858 五粮液

**wide -> strict**
- stroke_diff=-14, segment_diff=0, center_diff=2, level_diff=0
- bottleneck=18.8300, beta1_tau_diff=2
- trend_mutations=[]
- strong: {'n_centers': False, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': False, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=17, segment_diff=-8, center_diff=0, level_diff=0
- bottleneck=9.3050, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': False, 'beta1_tau': True, 'barcode_bottleneck': False}

**strict -> new**
- stroke_diff=31, segment_diff=-8, center_diff=-2, level_diff=0
- bottleneck=18.8300, beta1_tau_diff=-2
- trend_mutations=[]
- strong: {'n_centers': False, 'center_zd_zg_pairs': False, 'trend_kinds': False, 'beta1_tau': False, 'barcode_bottleneck': False}

### sh601318 中国平安

**wide -> strict**
- stroke_diff=-48, segment_diff=-7, center_diff=0, level_diff=0
- bottleneck=4.2950, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': False, 'beta1_tau': True, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=10, segment_diff=1, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict -> new**
- stroke_diff=58, segment_diff=8, center_diff=0, level_diff=0
- bottleneck=4.2950, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': False, 'beta1_tau': True, 'barcode_bottleneck': False}

### sz002475 立讯精密

**wide -> strict**
- stroke_diff=-26, segment_diff=11, center_diff=0, level_diff=0
- bottleneck=3.7650, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': False, 'beta1_tau': True, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=18, segment_diff=0, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict -> new**
- stroke_diff=44, segment_diff=-11, center_diff=0, level_diff=0
- bottleneck=3.7650, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': False, 'beta1_tau': True, 'barcode_bottleneck': False}

### SYN-A 深递归合成

**wide -> strict**
- stroke_diff=-42, segment_diff=-13, center_diff=-1, level_diff=0
- bottleneck=10.5300, beta1_tau_diff=-1
- trend_mutations=[]
- strong: {'n_centers': False, 'center_zd_zg_pairs': False, 'trend_kinds': False, 'beta1_tau': False, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=18, segment_diff=2, center_diff=0, level_diff=0
- bottleneck=12.2058, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

**strict -> new**
- stroke_diff=60, segment_diff=15, center_diff=1, level_diff=0
- bottleneck=13.2733, beta1_tau_diff=1
- trend_mutations=[]
- strong: {'n_centers': False, 'center_zd_zg_pairs': False, 'trend_kinds': False, 'beta1_tau': False, 'barcode_bottleneck': False}

### SYN-B 多中枢背驰合成

**wide -> strict**
- stroke_diff=-42, segment_diff=-9, center_diff=0, level_diff=0
- bottleneck=8.5594, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': False, 'beta1_tau': True, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=6, segment_diff=0, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict -> new**
- stroke_diff=48, segment_diff=9, center_diff=0, level_diff=0
- bottleneck=8.5594, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': False, 'beta1_tau': True, 'barcode_bottleneck': False}

## 结论

### 真实数据
- 强不变量通过率：37/75 = 49.3%
- T5：0/0
- T7：0/0
- T8 非inconclusive：0/0，inconclusive：14

### 合成数据
- 强不变量通过率：12/30 = 40.0%
- T5：0/0
- T7：0/0
- T8 非inconclusive：0/0，inconclusive：4

### 总体
- 强不变量通过率：49/105 = 46.7%
- T5：0/0（递归级别 < 2）
- T6：0/0（递归级别 < 2）
- T7：0/0（递归级别 < 2）
- T8 非inconclusive：0/0，inconclusive：18

### 递归级别瓶颈诊断

T5/T6/T7 全部 N/A 的根因：日线数据递归深度不足。

诊断数据（sz000001 平安银行，1974 根日线，2018-2026）：

| 模式 | 笔数 | 线段数 | 中枢数 | 递归层数 |
|------|------|--------|--------|----------|
| wide | 167 | 14 | 2 | 1 |
| strict | 137 | 12 | 1 | 1 |
| new | 181 | 14 | 2 | 1 |

递归终止链：14 线段 → 2 中枢 → 1 走势类型实例 → level 2 需要 ≥3 confirmed trends → 终止。

产生 level 2 的最低条件：≥4 中枢 → ≥15 线段（理论下限）。
日线级别 7 年数据仅产生 12-14 线段，距离阈值差距不大但未达到。

解决路径：
1. 更高频数据（5min/15min/30min K线）→ 更多笔和线段 → 更深递归
2. 更长时间跨度日线（≥15 年）→ 可能产生足够线段
3. 207号-3（λ/δ/κ 参数校准）依赖此前提——属于长期工程项
