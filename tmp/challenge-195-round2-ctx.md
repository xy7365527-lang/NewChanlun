# Gemini Round 2 审核上下文

## 任务

验证以下修复是否正确落地：
1. T7 贪心→最大二分匹配（增广路径法）
2. Codex N1/N2/N3 修复

---

## 核心实现：_check_barcode_inclusion（a_topology.py:468-532）

```python
def _check_barcode_inclusion(
    bars_high: tuple[tuple[float, float], ...],
    bars_trimmed_low: tuple[tuple[float, float], ...],
    epsilon: float,
) -> tuple[bool, tuple[tuple[float, float], ...]]:
    """检查 bars_high 是否 ε-嵌入 bars_trimmed_low（真单射）。

    使用最大二分匹配（增广路径法）消除贪心假阴性。
    复杂度 O(n²·m)，对 n_centers < 20 的场景足够。
    """
    if not bars_high:
        return (True, ())
    if not bars_trimmed_low:
        return (False, bars_high)

    n = len(bars_high)
    m = len(bars_trimmed_low)

    # 构建相容矩阵：compat[i][j] = bars_high[i] 与 bars_trimmed_low[j] 在 ε 内
    compat: list[list[bool]] = [
        [
            abs(bars_high[i][0] - bars_trimmed_low[j][0]) <= epsilon
            and abs(bars_high[i][1] - bars_trimmed_low[j][1]) <= epsilon
            for j in range(m)
        ]
        for i in range(n)
    ]

    # 最大二分匹配——增广路径法（Hungarian-style augmenting paths）
    # match_low[j] = 匹配到 bars_trimmed_low[j] 的 bars_high 索引，-1 表示未匹配
    match_low: list[int] = [-1] * m

    def _augment(i: int, visited: list[bool]) -> bool:
        for j in range(m):
            if compat[i][j] and not visited[j]:
                visited[j] = True
                if match_low[j] == -1 or _augment(match_low[j], visited):
                    match_low[j] = i
                    return True
        return False

    matched_count = 0
    for i in range(n):
        if _augment(i, [False] * m):
            matched_count += 1

    if matched_count == n:
        return (True, ())

    # 找出未匹配的 bars_high
    matched_high: set[int] = set(match_low[j] for j in range(m) if match_low[j] != -1)
    unmatched = tuple(bars_high[i] for i in range(n) if i not in matched_high)
    return (False, unmatched)
```

---

## Round 1 反例测试（test_a_topology.py:424-432）

```python
def test_bipartite_matching_avoids_greedy_false_negative(self):
    """ε-邻域交叠场景：贪心会假阴性，二分匹配应通过。"""
    # Gemini 反例：H1 邻域覆盖 L1 和 L2，H2 邻域只覆盖 L1
    # 贪心 H1→L1 锁定后 H2→L2 失败；最优 H1→L2, H2→L1 通过
    high = ((1.0, 3.0), (1.08, 3.08))
    low = ((1.05, 3.05), (0.95, 2.95))
    passed, unmatched = _check_barcode_inclusion(high, low, 0.1)
    assert passed is True
    assert unmatched == ()
```

---

## N1 修复：getattr→直接属性访问（a_topology.py:615）

修复前：
```python
confirmed_only = all(
    getattr(m, "confirmed", True) for m in moves
)
```

修复后：
```python
confirmed_only = all(m.confirmed for m in moves)
```

---

## N2 修复：< → <=（a_topology.py:501-502）

修复前：
```python
abs(bars_high[i][0] - bars_trimmed_low[j][0]) < epsilon
and abs(bars_high[i][1] - bars_trimmed_low[j][1]) < epsilon
```

修复后（相容矩阵构建，a_topology.py:499-505）：
```python
abs(bars_high[i][0] - bars_trimmed_low[j][0]) <= epsilon
and abs(bars_high[i][1] - bars_trimmed_low[j][1]) <= epsilon
```

N2 验证测试（test_a_topology.py:417-422）：
```python
def test_exact_match_epsilon_zero(self):
    """epsilon=0.0 时精确相同的 bar 应匹配。"""
    bars = ((1.0, 3.0),)
    passed, unmatched = _check_barcode_inclusion(bars, bars, 0.0)
    assert passed is True
    assert unmatched == ()
```

---

## N3 修复：T5 失败路径测试（test_a_topology.py:518-543）

```python
class TestT5FailurePaths:
    def test_t5_fails_with_unconfirmed_move(self):
        """moves 中含 unconfirmed trend 时 confirmed_only 应为 False。"""
        from types import SimpleNamespace as NS
        trend_confirmed = NS(confirmed=True, kind="up_trend")
        trend_unconfirmed = NS(confirmed=False, kind="down_trend")
        level_0 = NS(level=0, trends=[trend_confirmed], centers=[], moves=[])
        level_1 = NS(level=1, trends=[], centers=[], moves=[trend_unconfirmed])
        results = check_trend_move_equivalence([level_0, level_1])
        assert len(results) == 1
        assert results[0].confirmed_only is False
        assert results[0].passed is False

    def test_t5_fails_with_identity_mismatch(self):
        """moves 与 confirmed_trends 引用不同时 identity 应为 False。"""
        from types import SimpleNamespace as NS
        trend_a = NS(confirmed=True, kind="up_trend")
        trend_b = NS(confirmed=True, kind="up_trend")  # 不同对象
        level_0 = NS(level=0, trends=[trend_a], centers=[], moves=[])
        level_1 = NS(level=1, trends=[], centers=[], moves=[trend_b])
        results = check_trend_move_equivalence([level_0, level_1])
        assert len(results) == 1
        assert results[0].confirmed_only is True
        assert results[0].identity is False  # trend_b is not trend_a
        assert results[0].passed is False
```

---

## check_trend_move_equivalence 完整实现（a_topology.py:597-630）

```python
def check_trend_move_equivalence(levels) -> list[T5Result]:
    results: list[T5Result] = []
    for i in range(len(levels) - 1):
        low_level = levels[i]
        high_level = levels[i + 1]
        confirmed_trends = [t for t in low_level.trends if t.confirmed]
        moves = high_level.moves
        confirmed_only = all(m.confirmed for m in moves)
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

---

## 审核要求

1. **验证 Round 1 反例是否正确落地**：
   - 增广路径法对反例 (high=((1.0,3.0),(1.08,3.08)), low=((1.05,3.05),(0.95,2.95)), ε=0.1) 的执行轨迹
   - 是否等价于最大二分匹配（数学正确性）

2. **对 Codex N1/N2/N3 的回应**：
   - N1（getattr→直接属性）：是否同意？
   - N2（< → <=）：是否同意？
   - N3（T5 失败路径测试）：是否充分？

3. **是否有残留问题**？

4. **显式共识声明**：APPROVED 或具体分歧。
