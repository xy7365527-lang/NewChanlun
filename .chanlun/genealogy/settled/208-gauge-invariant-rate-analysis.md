---
id: "208"
number: 208
type: 概念发现
status: 已结算
date: 2026-02-25
trigger: gangju_analysis.py 自动检测——强不变量保持率偏低 + T8 全部 inconclusive
depends_on:
  - "195"
  - "207"
---

# 208号：gauge 不变量保持率模式分析 + T8 inconclusive 根因

## 结论

### 强不变量保持率 49.3% 的结构性解释

模式对 transition 数据揭示保持率差异的根因：

| 模式对 | 典型保持率 | 根因 |
|--------|-----------|------|
| wide→new | ~100% | bottleneck=0, center_diff=0, beta1_tau_diff=0（两种模式结构高度一致） |
| wide→strict | ~20% | center_diff≠0, bottleneck>0（strict 模式笔数少 → 线段/中枢结构变化） |
| strict→new | ~40% | 介于两者之间 |

49.3% 是三种模式对的加权平均。wide→new 接近 100% 说明 gauge 等价性在"宽松→新笔"方向成立；wide→strict 低保持率说明 strict 模式的笔过滤改变了中枢拓扑结构。

这不是 bug——是 gauge 结构的正常表现。strict 模式的 min_strict_sep 过滤器改变了笔的数量和位置，导致线段划分不同，进而中枢数量/位置变化。gauge 等价性的边界在此处显现。

### T8 全部 inconclusive 的结构性解释

18 个 T8 结果全部 inconclusive（centers_a=0, centers_c=0）。根因链：

1. Divergence 的 seg_a/seg_c 是**连接段**（a+A+b+B+c 中的 a 和 c），不是中枢
2. 连接段是单线段（seg_a=[9,9], seg_c=[11,11]）
3. 中枢至少跨 3 线段 → 单线段内不可能有中枢
4. `_centers_in_segment_range` 要求中枢完全落在范围内 → 返回空列表

这是 T8 在 level 1 的结构性约束：背驰的 A/C 段（连接段）内不含中枢是正常的。T8 的 Dgm(A)/Dgm(C) 构造需要重新定义——不是"A/C 段内的中枢条形码"，而应该是"A/C 段本身的价格序列持续图"。

## 定义依据

- gauge 等价性（195号）：不同笔模式下拓扑不变量的保持
- T8 背驰拓扑化（共识报告 Layer 2）：W₁ 带容差单调下降
- Divergence 结构（a_divergence.py:36）：seg_a/seg_c = 连接段索引

## 边界条件

- 如果 strict 模式的 min_strict_sep 参数调小，wide→strict 保持率会上升
- 如果 T8 的 Dgm 构造改为价格序列持续图（而非中枢条形码），inconclusive 率会下降
- 更高频数据（5min/15min）可能产生更多中枢，使 A/C 段内有中枢

## 下游推论

1. gauge 保持率的模式对差异是结构性的，不需要修复 → [resolved: 写入验证报告作为已知特征]
2. T8 Dgm 构造需要重新定义：从"A/C 段内中枢条形码"改为"A/C 段价格序列持续图" → 待实现
3. 更高频数据验证 → 长期工程项（与 207号-3 同类）

## 谱系引用

- 195号：缠论代码拓扑化
- 207号：W₁ 单调性矛盾修正
- 验证报告：`.chanlun/review-results/real-market-data-verification-report.md`
