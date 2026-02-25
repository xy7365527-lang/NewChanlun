# 真实市场数据验证报告

生成时间：2026-02-25 19:29
谱系引用：195号-5（拓扑不变量真实数据验证）+ 202号-3（gauge equivalence 经验验证）

## 数据来源

- 真实数据：akshare `stock_zh_a_daily`（前复权）
- 时间范围：20230601 ~ 20260225
- 合成数据：深递归结构（1200根30min）+ 多中枢背驰结构（1500根15min）
- 数据集：
  - sz000001 平安银行（大盘股）— 661 根K线
  - sh600036 招商银行（大盘股）— 661 根K线
  - sz000858 五粮液（中盘股）— 661 根K线
  - sh601318 中国平安（大盘股）— 661 根K线
  - sz002475 立讯精密（中小盘股）— 661 根K线
  - SYN-A 深递归合成（合成）— 1200 根K线
  - SYN-B 多中枢背驰合成（合成）— 1500 根K线

## 结果汇总

| 数据集 | K线数 | T1(barcode) | T3(centers) | T4(beta1) | T5 | T7 | T8 | 强不变量通过率 |
|--------|-------|-------------|-------------|-----------|-----|-----|-----|---------------|
| sz000001 平安银行 | 661 | 1/3 | 3/3 | 3/3 | N/A | N/A | 0/0(inc:3) | 73.3% |
| sh600036 招商银行 | 661 | 1/3 | 1/3 | 1/3 | N/A | N/A | N/A | 46.7% |
| sz000858 五粮液 | 661 | 1/3 | 3/3 | 3/3 | N/A | N/A | 0/0(inc:3) | 73.3% |
| sh601318 中国平安 | 661 | 1/3 | 1/3 | 1/3 | N/A | N/A | N/A | 46.7% |
| sz002475 立讯精密 | 661 | 1/3 | 3/3 | 3/3 | N/A | N/A | 0/0(inc:3) | 73.3% |
| SYN-A 深递归合成 | 1200 | 0/3 | 1/3 | 1/3 | N/A | N/A | 0/0(inc:2) | 20.0% |
| SYN-B 多中枢背驰合成 | 1500 | 1/3 | 3/3 | 3/3 | N/A | N/A | 0/0(inc:2) | 60.0% |

## T8 背驰拓扑详细结果

### sz000001 平安银行

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | wide | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | strict | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | new | 1 |

非inconclusive：0项

### sz000858 五粮液

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | bottom | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | wide | 1 |
| 0 | consolidation | bottom | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | strict | 1 |
| 0 | consolidation | bottom | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | new | 1 |

非inconclusive：0项

### sz002475 立讯精密

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive | centers_a | centers_c | mode | level |
|-----|------|-----------|------|------|---------|--------|--------------|-----------|-----------|------|-------|
| 0 | consolidation | bottom | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | wide | 1 |
| 0 | consolidation | bottom | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | strict | 1 |
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True | 0 | 0 | new | 1 |

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
- stroke_diff=-8, segment_diff=-2, center_diff=0, level_diff=0
- bottleneck=0.6800, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=6, segment_diff=-2, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict -> new**
- stroke_diff=14, segment_diff=0, center_diff=0, level_diff=0
- bottleneck=0.6800, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

### sh600036 招商银行

**wide -> strict**
- stroke_diff=-17, segment_diff=-5, center_diff=-1, level_diff=0
- bottleneck=1.9000, beta1_tau_diff=-1
- trend_mutations=[]
- strong: {'n_centers': False, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': False, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=4, segment_diff=0, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict -> new**
- stroke_diff=21, segment_diff=5, center_diff=1, level_diff=0
- bottleneck=1.9000, beta1_tau_diff=1
- trend_mutations=[]
- strong: {'n_centers': False, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': False, 'barcode_bottleneck': False}

### sz000858 五粮液

**wide -> strict**
- stroke_diff=-8, segment_diff=-2, center_diff=0, level_diff=0
- bottleneck=23.0500, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=3, segment_diff=0, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict -> new**
- stroke_diff=11, segment_diff=2, center_diff=0, level_diff=0
- bottleneck=23.0500, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

### sh601318 中国平安

**wide -> strict**
- stroke_diff=-16, segment_diff=-3, center_diff=-1, level_diff=0
- bottleneck=3.0750, beta1_tau_diff=-1
- trend_mutations=[]
- strong: {'n_centers': False, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': False, 'barcode_bottleneck': False}

**wide -> new**
- stroke_diff=6, segment_diff=1, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict -> new**
- stroke_diff=22, segment_diff=4, center_diff=1, level_diff=0
- bottleneck=3.0750, beta1_tau_diff=1
- trend_mutations=[]
- strong: {'n_centers': False, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': False, 'barcode_bottleneck': False}

### sz002475 立讯精密

**wide -> strict**
- stroke_diff=-8, segment_diff=0, center_diff=0, level_diff=0
- bottleneck=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**wide -> new**
- stroke_diff=6, segment_diff=4, center_diff=0, level_diff=0
- bottleneck=3.3150, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

**strict -> new**
- stroke_diff=14, segment_diff=4, center_diff=0, level_diff=0
- bottleneck=3.3150, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

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
- 强不变量通过率：47/75 = 62.7%
- T5：0/0
- T7：0/0
- T8 非inconclusive：0/0，inconclusive：9

### 合成数据
- 强不变量通过率：12/30 = 40.0%
- T5：0/0
- T7：0/0
- T8 非inconclusive：0/0，inconclusive：4

### 总体
- 强不变量通过率：59/105 = 56.2%
- T5：0/0
- T7：0/0
- T8 非inconclusive：0/0，inconclusive：13
