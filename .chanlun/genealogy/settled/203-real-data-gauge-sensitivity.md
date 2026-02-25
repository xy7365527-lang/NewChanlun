---
id: "203"
number: 203
type: 概念发现
status: 已结算
date: 2026-02-25
trigger: 真实市场数据验证（195号-5）
depends_on:
  - "195"
  - "199"
---

# 203号：真实市场数据验证——gauge 不变量的笔定义敏感性

## 结论

真实 A 股数据（3 只股票，661 根日 K 线）上的 gauge_equivalence_report 验证揭示：

1. T3（中枢数量）、T4（β₁^τ）在三种笔定义间完全一致（9/9 = 100%）
2. T1（barcode bottleneck）在 wide→strict 和 strict→new 转换时不一致（4/9 失败）
3. 失败模式集中在 center_zd_zg_pairs 和 barcode_bottleneck 两个子检查
4. T8 在级别 0 全部 inconclusive（2/2），与 199号一致

## 定义依据

- gauge equivalence 的定义：不同笔定义产生的拓扑不变量应一致（195号）
- T1 barcode bottleneck 检查：d_B(Dgm_wide, Dgm_strict) ≤ ε
- center_zd_zg_pairs：中枢的 (ZD, ZG) 对在不同笔定义下应相同

## 边界条件

- 招商银行（600036）三种转换全部通过（9/9）——说明不一致不是普遍现象
- 平安银行和五粮液的 wide→strict 转换 bottleneck 非零（0.79 和 23.63）
- wide→new 转换全部通过——说明宽笔和新笔的结构差异小于宽笔和严格笔

## 下游推论

1. T1 barcode 不一致的根因是笔定义差异导致线段数量不同（stroke_diff 非零），进而影响中枢边界 → [resolved: 203号结论本身]
2. T3/T4 的完全一致说明中枢数量和 β₁ 是真正的强不变量——对笔定义不敏感 → [resolved: 203号结论本身]
3. T1 barcode bottleneck 的阈值 ε 可能需要根据真实数据校准 → 后续工程项
4. 需要更多股票样本验证 T1 不一致的频率和分布

## 经验证据

- 000001 平安银行：wide→strict bottleneck=0.79, wide→new bottleneck=0.00
- 600036 招商银行：全部 bottleneck=0.00（完全一致）
- 000858 五粮液：wide→strict bottleneck=23.63, wide→new bottleneck=0.00

## 谱系引用

- 195号：缠论代码拓扑化
- 199号：T8 在级别 0 天然 inconclusive
