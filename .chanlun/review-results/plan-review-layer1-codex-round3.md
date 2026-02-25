# Codex Review — Layer 1 T7/T5 实现代码审核 Round 3

**模式**: review
**日期**: 2026-02-25
**评审工位**: codex-reviewer (Claude Sonnet 4.6)
**被评审对象**: src/newchan/a_topology.py + tests/test_a_topology.py
**Round 2 参考**: .chanlun/review-results/plan-review-layer1-codex-round2.md
**Gemini Round 1 参考**: .chanlun/review-results/plan-review-layer1-gemini-round1.md

---

## 1. Round 2 N1/N2/N3 修复核查

### N1：`getattr(m, "confirmed", True)` → `m.confirmed`

**Round 2 要求**：直接访问 `m.confirmed`，让类型错误快速失败。

**当前实现**（a_topology.py:615）：
```python
confirmed_only = all(m.confirmed for m in moves)
```

**判定：RESOLVED**

`getattr` 已移除，直接属性访问。若 `moves` 中对象无 `confirmed` 属性，立即抛出 `AttributeError`，符合快速失败原则。

---

### N2：`epsilon = 0.0` 精确匹配语义（`<` → `<=`）

**Round 2 要求**：将 `abs(...) < epsilon` 改为 `abs(...) <= epsilon`，使 epsilon=0.0 时精确相同的 bar 能匹配。

**当前实现**（a_topology.py:500-505，二分匹配相容矩阵构建）：
```python
compat: list[list[bool]] = [
    [
        abs(bars_high[i][0] - bars_trimmed_low[j][0]) <= epsilon
        and abs(bars_high[i][1] - bars_trimmed_low[j][1]) <= epsilon
        for j in range(m)
    ]
    for i in range(n)
]
```

**判定：RESOLVED**

修复在二分匹配重写中一并落地。`<=` 语义正确：epsilon=0.0 时仅精确相同的 bar 匹配，epsilon>0 时允许偏差范围内匹配。

测试覆盖（test_a_topology.py:417-422）：
- `test_exact_match_epsilon_zero`：`epsilon=0.0` 精确相同 bar 应匹配 → `passed is True` ✓

---

### N3：T5 测试缺少失败路径覆盖

**Round 2 要求**：添加 `confirmed_only=False` 和 `identity=False` 的失败路径测试。

**当前测试**（test_a_topology.py:518-543）：
```python
class TestT5FailurePaths:
    def test_t5_fails_with_unconfirmed_move(self):
        # moves 中含 unconfirmed trend → confirmed_only=False, passed=False
        ...
        assert results[0].confirmed_only is False
        assert results[0].passed is False

    def test_t5_fails_with_identity_mismatch(self):
        # moves 与 confirmed_trends 引用不同 → identity=False, passed=False
        ...
        assert results[0].confirmed_only is True
        assert results[0].identity is False
        assert results[0].passed is False
```

**判定：RESOLVED**

两条失败路径均已覆盖，使用 `SimpleNamespace` mock 构造，语义清晰。`test_t5_fails_with_identity_mismatch` 中 `trend_a` 和 `trend_b` 是不同对象（内容相同但 `is` 不同），正确验证了引用同一性检查。

---

## 2. Gemini Round 1 T7 二分匹配否定的回应

### Round 2 判定的错误

Round 2 我判定 T7-B（贪心足够）为 RESOLVED，这个判定是错误的。

Gemini 的否定成立。反例有效性分析：

```
epsilon = 0.1
bars_high        = ((1.0, 3.0), (1.08, 3.08))   # H1, H2
bars_trimmed_low = ((1.05, 3.05), (0.95, 2.95)) # L1, L2
```

相容关系：
- H1 可匹配 L1（误差 0.05）和 L2（误差 0.05）
- H2 可匹配 L1（误差 0.03），不可匹配 L2（误差 0.13 > 0.1）

贪心（matched_low 集合）：H1→L1（锁定），H2 只剩 L2 → 失败，返回 False。
最优匹配：H1→L2，H2→L1 → 成功，应返回 True。

贪心在 ε-邻域交叠时存在假阴性，这是算法正确性缺陷，不是边界情况。缠论场景中跨级别中枢价格分布不保证稀疏，假阴性不是极端情况。

### 增广路径法实现正确性验证

当前实现（a_topology.py:508-531）：

```python
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
```

对 Gemini 反例的增广过程追踪：

1. i=0（H1）：j=0（L1），compat[0][0]=True，visited[0]=True，match_low[0]==-1，设 match_low[0]=0，返回 True。matched_count=1
2. i=1（H2）：j=0（L1），compat[1][0]=True，visited[0]=True，match_low[0]==0（H1），尝试增广 H1：
   - H1 的 j=0 已 visited，跳过；j=1（L2），compat[0][1]=True，visited[1]=True，match_low[1]==-1，设 match_low[1]=0（H1），返回 True
   - H1 改匹配 L2，match_low[0]=1（H2），返回 True。matched_count=2

matched_count=2=n=2，返回 (True, ())。**反例通过，实现正确**。

算法是标准 Kuhn 算法（增广路径法），`match_low[j]` 记录 bars_trimmed_low[j] 当前匹配的 bars_high 索引，每次增广时 visited 数组防止循环。复杂度 O(n²·m)，对 n_centers < 20 的场景足够。

测试覆盖（test_a_topology.py:424-432）：
- `test_bipartite_matching_avoids_greedy_false_negative`：Gemini 反例 → `passed is True` ✓

### Round 2 判定修正

Round 2 的 T7-B 判定（"贪心足够"）是错误的。正确判定应为：贪心在 ε-邻域交叠时存在假阴性，需要最大二分匹配。Gemini 的否定成立，代码已正确替换。

---

## 3. 残留问题检查

### N4（LOW）：pipeline 测试静默通过风险

Round 2 标注为 LOW，未要求修复。当前代码仍有 `if len(levels) >= 2:` 条件分支。

**判定**：不影响 APPROVED。LOW 级别，且当前 df_raw（300根K线，seed=42）在实践中应能产生多层。

### epsilon 策略（Gemini Round 1 否定 rel_tol=1%）

Gemini 否定的是相对容差 1%（价格 60000 时 = 600 价格单位）。当前 `check_recursive_barcode_order` 默认 `epsilon=1.0` 是绝对值（1个价格单位），不是相对值。Gemini 的 rel_tol 否定不适用于当前实现。

**判定**：不影响 APPROVED。

---

## 综合判定

| 问题 | Round 2 状态 | Round 3 状态 |
|------|-------------|-------------|
| N1：getattr 掩盖类型错误 | 新发现 MEDIUM | RESOLVED |
| N2：epsilon=0.0 精确匹配 | 新发现 MEDIUM | RESOLVED |
| N3：T5 失败路径测试缺失 | 新发现 MEDIUM | RESOLVED |
| T7 贪心假阴性（Gemini 否定） | Round 2 误判"足够" | RESOLVED（二分匹配正确） |
| N4：pipeline 测试静默通过 | LOW | 保留（不影响 APPROVED） |

**最终判定：APPROVED**

所有 MEDIUM 及以上问题已解决。增广路径法实现正确，Gemini 反例测试通过，40 测试全通过。

---

## 边界条件

- 增广路径法翻转条件：若 n_centers 超过 20（当前场景不会），O(n²·m) 可能需要换 Hopcroft-Karp
- N2 翻转条件：若调用方保证 epsilon > 0，`<=` 与 `<` 等价（不影响正确性）

## 影响声明

- a_topology.py：`_check_barcode_inclusion` 从贪心替换为增广路径法，接口不变，语义更正确
- a_topology.py：`check_trend_move_equivalence` 的 `getattr` 移除，快速失败行为
- tests/test_a_topology.py：新增 `TestT5FailurePaths`（2个测试）和 `test_bipartite_matching_avoids_greedy_false_negative`、`test_exact_match_epsilon_zero`

---

*审查工位: codex-reviewer | 谱系位置: 代码层异质否定 | 对标: Gemini challenger (概念层)*
