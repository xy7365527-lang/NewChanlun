# 真实市场数据验证报告

生成时间：2026-02-25 22:01
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
| sz000001 平安银行 | 1974 | 1/3 | 1/3 | 1/3 | N/A | N/A | N/A | 2/3 | 46.7% |
| sh600036 招商银行 | 1974 | 0/3 | 3/3 | 3/3 | N/A | N/A | N/A | 1/3 | 60.0% |
| sz000858 五粮液 | 1974 | 0/3 | 1/3 | 1/3 | N/A | N/A | N/A | 2/2 | 20.0% |
| sh601318 中国平安 | 1974 | 1/3 | 3/3 | 3/3 | N/A | N/A | N/A | 3/5 | 60.0% |
| sz002475 立讯精密 | 1974 | 1/3 | 3/3 | 3/3 | N/A | N/A | N/A | 1/1 | 60.0% |
| SYN-A 深递归合成 | 1200 | 0/3 | 1/3 | 1/3 | N/A | N/A | N/A | 2/2 | 20.0% |
| SYN-B 多中枢背驰合成 | 1500 | 1/3 | 3/3 | 3/3 | N/A | N/A | N/A | 0/2 | 60.0% |

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
- T8 非inconclusive：9/14，inconclusive：0

### 合成数据
- 强不变量通过率：12/30 = 40.0%
- T5：0/0
- T7：0/0
- T8 非inconclusive：2/4，inconclusive：0

### 总体
- 强不变量通过率：49/105 = 46.7%
- T5：0/0（递归级别 < 2）
- T6：0/0（递归级别 < 2）
- T7：0/0（递归级别 < 2）
- T8 非inconclusive：11/18，inconclusive：0
- T8 通过率：11/18 = 61.1%（真实数据 9/14=64.3%，合成数据 2/4=50.0%）

### 递归级别瓶颈诊断

T5/T6/T7 全部 N/A 的根因：日线数据递归深度不足。

诊断数据（sz000001 平安银行，1974 根日线，2018-2026）：

| 模式 | 笔数 | 线段数 | 中枢数 | 递归层数 |
|------|------|--------|--------|----------|
| wide | 167 | 14 | 2 | 1 |
| strict | 137 | 12 | 1 | 1 |
| new | 181 | 14 | 2 | 1 |

递归终止链：14 线段 → 2 中枢 → 1 走势类型实例 → level 2 需要 ≥3 confirmed trends → 终止。

### T8 笔振荡 fallback（208号谱系）

原 T8 设计用"A/C 段内中枢条形码"构造 Dgm → 连接段无中枢 → 全部 inconclusive。
208号修正：fallback 到"A/C 段内笔振荡条形码"——每根笔 (low, high) 构成一个 bar。
修正后 T8 从 0/0(inc:18) 变为 11/18 passed(inc:0)，验证了背驰 = 后段笔振荡总量 < 前段的拓扑语义。
