# gauge_equivalence_report 经验验证报告

## 实验参数

- **数据长度**: [100, 200, 300, 500]
- **随机种子**: [42, 99, 123, 456, 789]
- **tau 值**: [0.0, 1.0, 3.0]
- **tolerance 值**: [0.0, 1.0]
- **模式**: ['wide', 'strict', 'new']
- **成功/失败/总计**: 120/0/120

## 1. 总体强不变量保持率

| 强不变量候选 | 保持次数/总比较次数 | 保持率 |
|---|---|---|
| n_centers | 288/360 (80.0%) | 80.0% |
| center_zd_zg_pairs | 210/360 (58.3%) | 58.3% |
| trend_kinds | 312/360 (86.7%) | 86.7% |
| beta1_tau | 288/360 (80.0%) | 80.0% |
| barcode_bottleneck | 210/360 (58.3%) | 58.3% |

## 2. 按 tau 分组的保持率

| 强不变量候选 | tau=0.0 | tau=1.0 | tau=3.0 |
|---|---|---|---|
| n_centers | 96/120 (80.0%) | 96/120 (80.0%) | 96/120 (80.0%) |
| center_zd_zg_pairs | 70/120 (58.3%) | 70/120 (58.3%) | 70/120 (58.3%) |
| trend_kinds | 104/120 (86.7%) | 104/120 (86.7%) | 104/120 (86.7%) |
| beta1_tau | 96/120 (80.0%) | 96/120 (80.0%) | 96/120 (80.0%) |
| barcode_bottleneck | 70/120 (58.3%) | 70/120 (58.3%) | 70/120 (58.3%) |

## 3. 按 tolerance 分组的保持率

| 强不变量候选 | tol=0.0 | tol=1.0 |
|---|---|---|
| n_centers | 144/180 (80.0%) | 144/180 (80.0%) |
| center_zd_zg_pairs | 102/180 (56.7%) | 108/180 (60.0%) |
| trend_kinds | 156/180 (86.7%) | 156/180 (86.7%) |
| beta1_tau | 144/180 (80.0%) | 144/180 (80.0%) |
| barcode_bottleneck | 102/180 (56.7%) | 108/180 (60.0%) |

## 4. 按数据长度分组的保持率

| 强不变量候选 | n=100 | n=200 | n=300 | n=500 |
|---|---|---|---|---|
| n_centers | 100.0% | 86.7% | 60.0% | 73.3% |
| center_zd_zg_pairs | 100.0% | 73.3% | 33.3% | 26.7% |
| trend_kinds | 100.0% | 86.7% | 86.7% | 73.3% |
| beta1_tau | 100.0% | 86.7% | 60.0% | 73.3% |
| barcode_bottleneck | 100.0% | 73.3% | 33.3% | 26.7% |

## 5. 模式对之间的结构差异统计

### strict -> new

- 样本数: 120
- **bottleneck_distance**: mean=1.274, std=1.503, min=0.000, max=4.663, median=0.462
- **center_count_diff**: mean=0.250, std=0.433, min=0.000, max=1.000, median=0.000
- **stroke_count_diff**: mean=7.150, std=4.767, min=0.000, max=18.000, median=6.000
- **segment_count_diff**: mean=0.950, std=1.499, min=-3.000, max=4.000, median=1.000
- **beta1_tau_diff**: mean=0.250, std=0.433, min=0.000, max=1.000, median=0.000

### wide -> new

- 样本数: 120
- **bottleneck_distance**: mean=1.182, std=1.583, min=0.000, max=4.663, median=0.000
- **center_count_diff**: mean=0.150, std=0.357, min=0.000, max=1.000, median=0.000
- **stroke_count_diff**: mean=3.250, std=2.773, min=0.000, max=10.000, median=2.000
- **segment_count_diff**: mean=0.600, std=1.200, min=-2.000, max=3.000, median=0.000
- **beta1_tau_diff**: mean=0.150, std=0.357, min=0.000, max=1.000, median=0.000

### wide -> strict

- 样本数: 120
- **bottleneck_distance**: mean=1.088, std=1.591, min=0.000, max=4.283, median=0.000
- **center_count_diff**: mean=-0.100, std=0.436, min=-1.000, max=1.000, median=0.000
- **stroke_count_diff**: mean=-3.900, std=2.528, min=-8.000, max=0.000, median=-3.000
- **segment_count_diff**: mean=-0.350, std=0.910, min=-2.000, max=1.000, median=0.000
- **beta1_tau_diff**: mean=-0.100, std=0.436, min=-1.000, max=1.000, median=0.000

## 6. 经验分类

基于保持率的经验分类（阈值：保持率 >= 90% 为强候选，50%-90% 为中等，< 50% 为弱候选）：

| 不变量候选 | 总体保持率 | 经验分类 |
|---|---|---|
| n_centers | 80.0% | 中等候选 |
| center_zd_zg_pairs | 58.3% | 中等候选 |
| trend_kinds | 86.7% | 中等候选 |
| beta1_tau | 80.0% | 中等候选 |
| barcode_bottleneck | 58.3% | 中等候选 |

## 7. tau 敏感性分析

beta1_tau 和 barcode_bottleneck 在不同 tau 下的保持率变化：

| tau | beta1_tau 保持率 | barcode_bottleneck 保持率 |
|---|---|---|
| 0.0 | 80.0% | 58.3% |
| 1.0 | 80.0% | 58.3% |
| 3.0 | 80.0% | 58.3% |
