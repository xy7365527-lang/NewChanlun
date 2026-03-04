# T6 Layer 3 Leray 条件可计算近似——Round 2 Verify 上下文

## 任务

verify 模式：确认 Round 1 识别的五个问题是否已全部正确修复。

## Round 1 问题清单

| 议题 | 类型 | 修正方案 |
|------|------|---------|
| A: W₁ 单调性方向 | 概念层矛盾（成立） | 改为 W₁ 比率有界：w1_ratio ≤ λ（默认 3.0） |
| B: 小样本 KL Laplace 平滑主导 | 工程缺陷（成立） | min_bars_kl=5，低于此值标记 inconclusive，KL 不参与 passed |
| C: rl_first 作用域泄漏 | 工程质量（成立） | 显式赋值 rl_for_t6 = rl_first if t7_results else _run_pipeline(...)[4] |
| D: δ=5.0 量纲 | 工程问题（成立） | docstring 明确说明价格单位相关性 |
| E: Leray 声明 | 否定不成立 | 无需修正 |

## 修正后代码（src/newchan/a_topology.py）

### T6Result dataclass（第 935-967 行）

```python
@dataclass(frozen=True, slots=True)
class T6Result:
    """T6 跨层 Leray 条件可计算近似结果。

    T6 原始定义：R¹f_* = 0（Leray 谱序列高阶直像层消失）。
    不可直接计算（需要 sheaf 库 + 范畴结构定义）。

    可计算近似（204号，Gemini Round 1 修正）：三个跨层诊断指标——
    1. W₁ 比率有界：W₁(B_{k+1}) / W₁(B_k) ≤ λ（信息量增长受控）
       注意：缠论高级别中枢价格跨度更大，W₁ 可以增加（不要求单调递减），
       但增长比率应有界——无界增长暗示递归构造创造了虚假信息。
    2. Bottleneck 距离：层级间持续图距离 ≤ δ（信息损失有界）
    3. KL 散度：条带长度分布漂移 ≤ κ（分布形状保持）
       当 min(n_bars_low, n_bars_high) < min_bars_kl 时标记 inconclusive，
       KL 不参与 passed 判定（小样本下 Laplace 平滑主导分布）。
    """
    level_low: int
    level_high: int
    passed: bool
    w1_low: float
    w1_high: float
    w1_ratio: float
    w1_ratio_bounded: bool
    w1_lambda: float
    bottleneck_dist: float
    bottleneck_bounded: bool
    kl_divergence: float
    kl_bounded: bool
    inconclusive: bool
    delta: float
    kappa: float
    n_bars_low: int
    n_bars_high: int
```

### check_cross_level_leray 函数（第 993-1085 行）

```python
def check_cross_level_leray(
    levels,
    *,
    tau: float = 0.0,
    delta: float = 5.0,
    kappa: float = 1.0,
    w1_lambda: float = 3.0,
    min_bars_kl: int = 5,
) -> list[T6Result]:
    """T6 Leray 条件的可计算近似——跨层诊断。

    对每对相邻层级 (k, k+1) 检查三个条件：
    1. W₁ 比率有界：W₁(B_{k+1}) / W₁(B_k) ≤ λ
       缠论高级别中枢跨度更大，W₁ 可以增加但增长比率应有界。
    2. Bottleneck 有界：d_B(Dgm_k, Dgm_{k+1}) ≤ δ
    3. KL 散度有界：KL(len_dist_{k+1} || len_dist_k) ≤ κ
       当 min(n_bars) < min_bars_kl 时标记 inconclusive，KL 不参与判定。

    注意：bottleneck 距离是价格单位相关的绝对值。delta 的默认值 5.0
    适用于单一股票的内部比较。跨股票比较时需根据价格尺度调整 delta。
    ...
    """
    results: list[T6Result] = []
    for i in range(len(levels) - 1):
        low_level = levels[i]
        high_level = levels[i + 1]
        bc_low = centers_to_barcode(low_level.centers)
        bc_high = centers_to_barcode(high_level.centers)

        # W₁ 范数 + 比率有界（Gemini Round 1 修正：不要求单调递减）
        w1_low = _w1_norm(bc_low, tau=tau)
        w1_high = _w1_norm(bc_high, tau=tau)
        if w1_low > 0:
            w1_ratio = w1_high / w1_low
        else:
            w1_ratio = 0.0 if w1_high == 0 else float("inf")
        w1_ratio_bounded = w1_ratio <= w1_lambda

        # Bottleneck 距离
        dgm_low = barcode_to_diagram(bc_low)
        dgm_high = barcode_to_diagram(bc_high)
        bn_dist = _bottleneck_distance(dgm_low, dgm_high)
        bn_bounded = bn_dist <= delta

        # KL 散度（小样本 inconclusive）
        n_low = len(bc_low)
        n_high = len(bc_high)
        kl_inconclusive = min(n_low, n_high) < min_bars_kl
        dist_low = _bar_length_distribution(bc_low)
        dist_high = _bar_length_distribution(bc_high)
        kl = _kl_divergence(dist_high, dist_low)
        kl_bounded = kl <= kappa

        # 合取判定：inconclusive 的 KL 不参与
        if kl_inconclusive:
            passed = w1_ratio_bounded and bn_bounded
        else:
            passed = w1_ratio_bounded and bn_bounded and kl_bounded

        results.append(T6Result(
            level_low=low_level.level,
            level_high=high_level.level,
            passed=passed,
            w1_low=w1_low,
            w1_high=w1_high,
            w1_ratio=w1_ratio,
            w1_ratio_bounded=w1_ratio_bounded,
            w1_lambda=w1_lambda,
            bottleneck_dist=bn_dist,
            bottleneck_bounded=bn_bounded,
            kl_divergence=kl,
            kl_bounded=kl_bounded,
            inconclusive=kl_inconclusive,
            delta=delta,
            kappa=kappa,
            n_bars_low=n_low,
            n_bars_high=n_high,
        ))
    return results
```

### T6 集成代码（第 690-697 行）

```python
# T6 跨层 Leray 可计算近似（204号谱系，Gemini Round 1 修正）
t6_results = []
try:
    rl_for_t6 = rl_first if t7_results else _run_pipeline(
        df_raw, modes[0], min_strict_sep, center_sustain_m,
    )[4]
    if len(rl_for_t6) >= 2:
        t6_checks = check_cross_level_leray(rl_for_t6, tau=tau)
```

注意：`rl_first` 在 T8 块（第 651 行）中被赋值，T6 块在 T8 之后执行。
T7 块（第 629 行）和 T8 块（第 651 行）都调用 `_run_pipeline(df_raw, modes[0], ...)` 使用相同参数。

## 新增测试（tests/test_a_topology.py）

### test_t6_w1_ratio_bounded（第 852-870 行）

```python
def test_t6_w1_ratio_bounded(self):
    """高级别 W₁ 在 λ 倍以内 → w1_ratio_bounded = True。"""
    # 低级别：宽条带
    centers_low = [
        SimpleNamespace(low=0.0, high=100.0, seg0=0, seg1=2),
        SimpleNamespace(low=10.0, high=90.0, seg0=3, seg1=5),
    ]
    # 高级别：窄条带（粗粒化后信息量减少）
    centers_high = [
        SimpleNamespace(low=30.0, high=50.0, seg0=0, seg1=1),
    ]
    results = check_cross_level_leray([level0, level1])
    assert results[0].w1_ratio_bounded is True
    assert results[0].w1_ratio <= 3.0
```

### test_t6_w1_ratio_unbounded（第 872-889 行）

```python
def test_t6_w1_ratio_unbounded(self):
    """高级别 W₁ 远超低级别 → w1_ratio_bounded = False（λ=1.0 时）。"""
    # 低级别：窄条带
    centers_low = [SimpleNamespace(low=10.0, high=15.0, seg0=0, seg1=2)]
    # 高级别：宽条带（缠论递归放大）
    centers_high = [SimpleNamespace(low=0.0, high=100.0, seg0=0, seg1=1)]
    results = check_cross_level_leray([level0, level1], w1_lambda=1.0)
    assert results[0].w1_ratio > 1.0
    assert results[0].w1_ratio_bounded is False
```

## 207号谱系（已结算）

`.chanlun/genealogy/settled/207-w1-monotone-contradiction.md`

修正：W₁ 单调性 → W₁ 比率有界：W₁(B_{k+1}) / W₁(B_k) ≤ λ（默认 3.0）

## 审核问题

1. W₁ 比率有界是否正确替代了 W₁ 单调性？
2. 小样本 inconclusive 机制是否正确？
3. rl_first 作用域是否已修复（显式赋值 rl_for_t6）？
4. 新增测试是否充分？
