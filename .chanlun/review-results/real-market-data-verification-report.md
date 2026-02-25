# 真实市场数据验证报告

生成时间：2026-02-25 19:20

## 数据来源

- akshare A 股日线数据（前复权）
- 时间范围：20230601 ~ 20260225
- 股票列表：
  - 000001 平安银行（大盘股）— 661 根K线
  - 600036 招商银行（大盘股）— 661 根K线
  - 000858 五粮液（中盘股）— 661 根K线

## 结果汇总

| 股票 | K线数 | T1(barcode) | T3(centers) | T4(beta1) | T5 | T7 | T8 | 强不变量总通过率 |
|------|-------|-------------|-------------|-----------|-----|-----|-----|-----------------|
| 000001 平安银行 | 661 | 1/3 | 3/3 | 3/3 | N/A | N/A | 0/0 (inc:1) | 73.3% |
| 600036 招商银行 | 661 | 3/3 | 3/3 | 3/3 | N/A | N/A | N/A | 100.0% |
| 000858 五粮液 | 661 | 1/3 | 3/3 | 3/3 | N/A | N/A | 0/0 (inc:1) | 73.3% |

## T8 详细结果

### 000001 平安银行

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive |
|-----|------|-----------|------|------|---------|--------|--------------|
| 0 | consolidation | top | 0.0000 | 0.0000 | 0.0000 | False | True |

### 000858 五粮液

| idx | kind | direction | W1_A | W1_C | W1_drop | passed | inconclusive |
|-----|------|-----------|------|------|---------|--------|--------------|
| 0 | consolidation | bottom | 0.0000 | 0.0000 | 0.0000 | False | True |

## T7 详细结果

所有股票均无 T7 结果（递归级别 < 2）。

## T5 详细结果

所有股票均无 T5 结果（递归级别 < 2）。

## 模式对 Transition 详情

### 000001 平安银行

**wide → strict**
- stroke_diff=-8, segment_diff=-2, center_diff=0, level_diff=0
- bottleneck_distance=0.7900, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

**wide → new**
- stroke_diff=6, segment_diff=-2, center_diff=0, level_diff=0
- bottleneck_distance=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict → new**
- stroke_diff=14, segment_diff=0, center_diff=0, level_diff=0
- bottleneck_distance=0.7900, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

### 600036 招商银行

**wide → strict**
- stroke_diff=-13, segment_diff=0, center_diff=0, level_diff=0
- bottleneck_distance=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**wide → new**
- stroke_diff=4, segment_diff=0, center_diff=0, level_diff=0
- bottleneck_distance=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict → new**
- stroke_diff=17, segment_diff=0, center_diff=0, level_diff=0
- bottleneck_distance=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

### 000858 五粮液

**wide → strict**
- stroke_diff=-8, segment_diff=-2, center_diff=0, level_diff=0
- bottleneck_distance=23.6300, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

**wide → new**
- stroke_diff=3, segment_diff=0, center_diff=0, level_diff=0
- bottleneck_distance=0.0000, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': True, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': True}

**strict → new**
- stroke_diff=11, segment_diff=2, center_diff=0, level_diff=0
- bottleneck_distance=23.6300, beta1_tau_diff=0
- trend_mutations=[]
- strong: {'n_centers': True, 'center_zd_zg_pairs': False, 'trend_kinds': True, 'beta1_tau': True, 'barcode_bottleneck': False}

## 结论

- 强不变量总体通过率：37/45 = 82.2%
- T5 通过率：0/0
- T7 通过率：0/0
- T8 非 inconclusive 通过率：0/0
- T8 inconclusive 数量：2
