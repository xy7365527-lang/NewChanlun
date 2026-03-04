# Gemini 异质质询上下文 — Layer 1 T7/T5 实现审核

## 审核目标

`src/newchan/a_topology.py` 的 Layer 1 新增实现：
- **T7**: `_check_barcode_inclusion` + `check_recursive_barcode_order`（ε-真单射匹配，B_{k+1} ⊆ Trim(B_k, τ)）
- **T5**: `check_trend_move_equivalence`（confirmed_only + identity 构造不变式验证）

以及配套测试文件 `tests/test_a_topology.py` 中对应测试（TestBarcodeInclusion + TestT7RecursiveOrder + TestT5TrendMoveEquivalence）。

## 背景

195号谱系：缠论代码拓扑化。Layer 1 在原有 DecompositionFingerprint/StructuralDelta 基础上新增：
1. T7 递归条形码偏序（B_{k+1} ⊆ Trim(B_k, τ_{k+1})）
2. T5 走势类型≡上级笔（Trend_k ≅ E(X_{k+1})）

Codex Round 1 已评审，提出 CRITICAL 修正：T5 三层检查（数量/方向/区间）均为 trivially true，
应改为构造不变式验证（confirmed_only + identity）。代码已按 Codex 修正实现。

## T7 当前实现（完整代码）

```python
def _check_barcode_inclusion(
    bars_high: tuple[tuple[float, float], ...],
    bars_trimmed_low: tuple[tuple[float, float], ...],
    epsilon: float,
) -> tuple[bool, tuple[tuple[float, float], ...]]:
    """检查 bars_high 是否 ε-嵌入 bars_trimmed_low（真单射）。

    对 bars_high 中的每个 bar (b,d)，要求存在 bars_trimmed_low 中的
    某个 **尚未被匹配** 的 bar (b',d') 使得 |b-b'| < ε 且 |d-d'| < ε。
    真单射：bars_trimmed_low 中每个 bar 至多被匹配一次。

    边界情况：
    - bars_high 为空 → True（空集总被包含）
    - bars_trimmed_low 为空但 bars_high 非空 → False
    """
    if not bars_high:
        return (True, ())
    if not bars_trimmed_low:
        return (False, bars_high)
    matched_low: set[int] = set()
    unmatched: list[tuple[float, float]] = []
    for b, d in bars_high:
        found = False
        for j, (b2, d2) in enumerate(bars_trimmed_low):
            if j not in matched_low and abs(b - b2) < epsilon and abs(d - d2) < epsilon:
                matched_low.add(j)
                found = True
                break
        if not found:
            unmatched.append((b, d))
    return (len(unmatched) == 0, tuple(unmatched))


def check_recursive_barcode_order(
    levels,
    tau: float = 0.0,
    epsilon: float = 1.0,
) -> list[T7Result]:
    """检查递归层级间的条形码偏序（T7）。

    对每对相邻层级 (k, k+1)：
    - B_k = 第 k 层中枢的条形码
    - Trim(B_k, τ) = {bar in B_k | death - birth > τ}
    - 验证 B_{k+1} ε-嵌入 Trim(B_k)（真单射匹配）
    """
    results: list[T7Result] = []
    for i in range(len(levels) - 1):
        low_level = levels[i]
        high_level = levels[i + 1]
        bc_low = centers_to_barcode(low_level.centers)
        bc_high = centers_to_barcode(high_level.centers)
        trimmed_low = tuple(bar for bar in bc_low if bar[1] - bar[0] > tau)
        passed, unmatched = _check_barcode_inclusion(bc_high, trimmed_low, epsilon)
        results.append(T7Result(
            level_low=low_level.level,
            level_high=high_level.level,
            passed=passed,
            trimmed_low_count=len(trimmed_low),
            high_count=len(bc_high),
            unmatched_bars=unmatched,
        ))
    return results
```

## T5 当前实现（完整代码）

```python
def check_trend_move_equivalence(levels) -> list[T5Result]:
    """验证递归层级间的走势≡上级笔构造不变式（T5）。

    对每对相邻层级 (k, k+1) 检查：
    1. confirmed_only：levels[k+1].moves 中所有元素的 confirmed 为 True
    2. identity：levels[k+1].moves 与 levels[k].confirmed_trends 内容一致
    """
    results: list[T5Result] = []
    for i in range(len(levels) - 1):
        low_level = levels[i]
        high_level = levels[i + 1]
        confirmed_trends = [t for t in low_level.trends if t.confirmed]
        moves = high_level.moves
        confirmed_only = all(
            getattr(m, "confirmed", True) for m in moves
        )
        identity = (
            len(moves) == len(confirmed_trends)
            and all(m is ct for m, ct in zip(moves, confirmed_trends))
        )
        passed = confirmed_only and identity
        results.append(T5Result(
            level_low=low_level.level,
            level_high=high_level.level,
            passed=passed,
            confirmed_only=confirmed_only,
            identity=identity,
            n_moves=len(moves),
            n_confirmed_trends=len(confirmed_trends),
        ))
    return results
```

## T7 测试（相关测试类）

```python
class TestBarcodeInclusion:
    def test_empty_high_always_passes(self):
        passed, unmatched = _check_barcode_inclusion((), ((1.0, 3.0),), 1.0)
        assert passed is True
        assert unmatched == ()

    def test_empty_low_nonempty_high_fails(self):
        passed, unmatched = _check_barcode_inclusion(((1.0, 3.0),), (), 1.0)
        assert passed is False
        assert unmatched == ((1.0, 3.0),)

    def test_exact_match(self):
        bars = ((1.0, 3.0), (2.0, 5.0))
        passed, unmatched = _check_barcode_inclusion(bars, bars, 0.01)
        assert passed is True
        assert unmatched == ()

    def test_epsilon_tolerance(self):
        high = ((1.0, 3.0),)
        low = ((1.05, 3.05),)
        passed, _ = _check_barcode_inclusion(high, low, 0.1)
        assert passed is True

    def test_outside_epsilon_fails(self):
        high = ((1.0, 3.0),)
        low = ((2.0, 4.0),)
        passed, unmatched = _check_barcode_inclusion(high, low, 0.5)
        assert passed is False
        assert len(unmatched) == 1

    def test_true_injective_no_double_match(self):
        high = ((1.0, 3.0), (1.0, 3.0))  # 两个相同的 bar
        low = ((1.0, 3.0),)  # 只有一个
        passed, unmatched = _check_barcode_inclusion(high, low, 0.1)
        assert passed is False
        assert len(unmatched) == 1

    def test_surplus_low_bars_ok(self):
        high = ((1.0, 3.0),)
        low = ((1.0, 3.0), (5.0, 8.0), (10.0, 15.0))
        passed, _ = _check_barcode_inclusion(high, low, 0.1)
        assert passed is True
```

## T5 测试（相关测试类）

```python
class TestT5TrendMoveEquivalence:
    def test_single_level_no_check(self):
        """只有一层时 T5 无需检查，返回空列表。"""
        # 使用短数据（50个bar），若layers<=1则验证空列表

    def test_t5_from_pipeline(self, df_raw):
        """用真实数据运行管线，验证 T5 构造不变式。"""
        # 若 levels >= 2，则 verified:
        # - results 长度 == len(levels) - 1
        # - 每个 r.confirmed_only is True
        # - 每个 r.identity is True
        # - 每个 r.passed is True
```

## Codex Round 1 评审结论摘要

Codex 对 T7 的判定：
- T7-A CRITICAL: 真单射 vs 伪单射 → 推荐 matched_low 集合实现（已按此实现）
- T7-B HIGH: 贪心足够，不需要匈牙利算法

Codex 对 T5 的判定：
- T5-A CRITICAL: 三层检查（数量/方向/区间）均为 trivially true → 改为 confirmed_only + identity 检查

## 审核要求（verify 模式）

从以下角度严格审核：

**数学正确性**：
1. T7 的 ε-真单射匹配是否正确实现了 B_{k+1} ⊆ Trim(B_k, τ)？
   - matched_low 集合实现的真单射语义是否与集合包含语义一致？
   - 贪心顺序是否会导致误判（bars_high 先到先得，可能抢占 bars_trimmed_low 中"更好的"匹配）？
2. β₁^τ 的计算（compute_beta1_tau）是否正确？
3. bottleneck 距离的使用是否数学上合理？

**概念忠实度**：
4. T5 作为"构造不变式验证"而非"运行时约束"——Codex 的 CRITICAL 判定是否正确？
5. identity 检查（`m is ct`，Python 对象引用同一性）是否比内容相等（`m == ct`）更严格且语义更准确？
6. T4（tau = gauge choice）的语义化是否准确？

**测试充分性**：
7. T7 边界条件测试是否充分（空集、真单射、多余 bars）？
8. T5 从管线运行的测试是否覆盖了构造不变式的关键性质？
9. 是否存在未覆盖的边界条件？

**对 Codex 评审的评估**：
10. Codex 判定 T5 三层检查均为 trivially true——你是否同意？
11. 是否有 Codex 未发现的问题？

产出格式：每个问题 agree/disagree + 理由。新问题标注 CRITICAL/HIGH/MEDIUM/LOW。
最终判定：APPROVED / NEEDS_REVISION / CRITICAL_ISSUES。
