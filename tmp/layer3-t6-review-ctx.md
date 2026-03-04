# Layer 3 T6 审核上下文——Leray 条件可计算近似

## 审核对象

`src/newchan/a_topology.py` 第 933-1062 行：T6 Leray 条件的可计算近似实现。

## 背景

### 原始 T6 定义（共识报告 Layer 3）
- Leray R¹f_* = 0：递归层级构造 f: Level_k → Level_{k+1} 上，高阶直像层消失
- 原搁置理由：无 sheaf 工程基础，不可计算
- 204号谱系否定搁置：157号已否定"长期搁置的合法性"，应在约束条件下寻找可计算近似

### 可计算近似设计（204号）
T6 原始定义不可直接计算，但其语义可分解为三个可计算的必要条件：

1. **W₁ 单调性**：W₁(B_{k+1}) ≤ W₁(B_k)
   - 直觉：粗粒化后条形码总持续量不应增加
   - 如果 R¹=0（无信息丢失），高级别的"拓扑复杂度"不应超过低级别

2. **Bottleneck 距离有界**：d_B(Dgm_k, Dgm_{k+1}) ≤ δ
   - 直觉：层级间持续图不应剧烈变化
   - 如果 R¹=0，相邻层级的持续图应"接近"

3. **KL 散度有界**：KL(len_dist_{k+1} || len_dist_k) ≤ κ
   - 直觉：条带长度分布不应漂移
   - 如果 R¹=0，粗粒化不应改变条带长度的统计分布

三个条件全部满足 → T6 passed（Leray 近似成立）。

### 与 T7 的关系
- T7 检查 B_{k+1} ⊆ Trim(B_k, τ)（集合包含）
- T6 检查层级间的信息损失量（W₁/bottleneck/KL）
- T7 是离散的 pass/fail，T6 提供连续的诊断指标
- 两者互补：T7 检测"是否有虚假结构"，T6 检测"信息损失有多大"

## 实现代码

### T6Result dataclass（第 937-967 行）
14 个字段：level_low/high, passed, w1_low/high/monotone, bottleneck_dist/bounded, kl_divergence/bounded, delta, kappa, n_bars_low/high

### _bar_length_distribution（第 970-985 行）
条带长度 → 归一化直方图（10 bins）。Laplace 平滑避免 log(0)。空条形码返回均匀分布。

### _kl_divergence（第 988-990 行）
KL(P || Q) = Σ p_i * log(p_i / q_i)。假设输入已归一化且无零元素（Laplace 平滑保证）。

### check_cross_level_leray（第 993-1062 行）
主函数。遍历相邻层级对，计算三个指标，合取判定 passed。

### gauge_equivalence_report 集成（第 690-718 行）
T6 结果加入 gauge report 返回 dict 的 `t6_cross_level_leray` 字段。

## 测试（8 个）

1. test_t6_bar_length_distribution_basic — 直方图归一化
2. test_t6_bar_length_distribution_empty — 空条形码均匀分布
3. test_t6_kl_divergence_same — 相同分布 KL=0
4. test_t6_kl_divergence_different — 不同分布 KL>0
5. test_t6_synthetic_two_levels — 合成两层递归端到端
6. test_t6_w1_monotone_pass — W₁ 单调性验证
7. test_t6_single_level_no_result — 单层无结果
8. test_t6_delta_kappa_params — 参数化验证

全部通过。1947 全量回归无破坏。

## 审核要点

1. **数学正确性**：三个指标是否是 R¹f_*=0 的合理必要条件？
2. **实现正确性**：W₁ 范数、bottleneck 距离、KL 散度的计算是否正确？
3. **边界条件**：空条形码、单层递归、相同条形码等边界情况处理是否正确？
4. **参数选择**：默认 δ=5.0, κ=1.0 是否合理？
5. **与 T7 的一致性**：T6 和 T7 是否可能产生矛盾结果？
6. **声明与能力一致性**：docstring 中的"可计算近似"声明是否准确？

## 谱系引用

- 195号：缠论代码拓扑化（六层对应）
- 204号：搁置模式否定——T6 可计算近似的来源
- 共识报告：`tmp/consensus-report-topo-scheme.md` Layer 3 节
