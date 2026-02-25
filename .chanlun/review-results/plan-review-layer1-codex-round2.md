# Codex Review — Layer 1 T7/T5 实现代码审核 Round 2

**模式**: review
**日期**: 2026-02-25
**评审工位**: codex-reviewer (Claude Sonnet 4.6)
**被评审对象**: src/newchan/a_topology.py + tests/test_a_topology.py
**Round 1 参考**: .chanlun/review-results/plan-review-layer1-codex-round1.md

---

## Round 1 问题落地核查

### T7-A HIGH-1：空集边界处理

**Round 1 要求**：
```python
if not bars_high:
    return True   # 空集总被包含
if not bars_trimmed_low:
    return False  # 非空集不被空集包含
```

**实现**（a_topology.py:487-490）：
```python
if not bars_high:
    return (True, ())
if not bars_trimmed_low:
    return (False, bars_high)
```

**判定：RESOLVED**

两个边界情况均正确处理。返回值改为 `(bool, unmatched_tuple)` 元组，与函数签名一致。
`bars_trimmed_low` 为空时返回 `(False, bars_high)` — 所有 bars_high 均未匹配，语义正确。

测试覆盖（test_a_topology.py:368-378）：
- `test_empty_high_always_passes` ✓
- `test_empty_low_nonempty_high_fails` ✓

---

### T7-A HIGH-2：真单射语义（matched_low 集合）

**Round 1 要求**：使用 `matched_low` 集合追踪已匹配索引，防止 bars_trimmed_low 中同一 bar 被多次匹配。

**实现**（a_topology.py:491-502）：
```python
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
```

**判定：RESOLVED**

`matched_low` 集合正确实现真单射语义。docstring 明确声明"真单射：bars_trimmed_low 中每个 bar 至多被匹配一次"，与实现一致。

测试覆盖（test_a_topology.py:402-408）：
- `test_true_injective_no_double_match`：两个相同 bars_high bar 对应一个 bars_low bar → False ✓

---

### T5 CRITICAL：confirmed_only + identity 构造不变式验证

**Round 1 要求**：放弃三层兼容性检查（均为 trivially true），改为：
1. `confirmed_only`：moves 中所有元素 confirmed = True
2. `identity`：moves 与 confirmed_trends 是同一引用

**T5Result 实现**（a_topology.py:549-564）：
```python
@dataclass(frozen=True, slots=True)
class T5Result:
    level_low: int
    level_high: int
    passed: bool
    confirmed_only: bool
    identity: bool
    n_moves: int
    n_confirmed_trends: int
```

**check_trend_move_equivalence 实现**（a_topology.py:567-602）：
```python
confirmed_only = all(
    getattr(m, "confirmed", True) for m in moves
)
identity = (
    len(moves) == len(confirmed_trends)
    and all(m is ct for m, ct in zip(moves, confirmed_trends))
)
passed = confirmed_only and identity
```

**判定：RESOLVED**

三层 trivially true 检查已完全替换为 confirmed_only + identity 两项构造不变式验证。
`identity` 使用 `m is ct`（对象引用相同），比 Round 1 建议的列表引用检查更严格——逐元素验证引用，而非仅验证列表容器。

测试覆盖（test_a_topology.py:479-498）：
- `test_t5_from_pipeline`：验证 `r.confirmed_only is True`、`r.identity is True`、`r.passed is True` ✓

---

### T7-B：贪心 O(n·m) 足够

**判定：RESOLVED**

实现使用带 `matched_low` 集合的贪心 O(n·m)，未引入匈牙利算法。

---

## 新发现问题

### N1：`getattr(m, "confirmed", True)` 默认值掩盖类型错误

**位置**：a_topology.py:585-587

```python
confirmed_only = all(
    getattr(m, "confirmed", True) for m in moves
)
```

**问题**：若 `moves` 中的对象没有 `confirmed` 属性（类型错误），`getattr` 默认返回 `True`，`confirmed_only` 静默通过。这违反快速失败原则——类型错误应该抛出 `AttributeError`，而不是被掩盖。

**严重性**：MEDIUM

**修复**：
```python
confirmed_only = all(m.confirmed for m in moves)
```

若 `m` 没有 `confirmed` 属性，立即抛出 `AttributeError`，暴露引擎传递了错误类型的对象。

---

### N2：`epsilon = 0.0` 时精确匹配不通过

**位置**：a_topology.py:496

```python
if j not in matched_low and abs(b - b2) < epsilon and abs(d - d2) < epsilon:
```

**问题**：使用严格小于 `<`。当 `epsilon = 0.0` 时，`abs(b - b2) < 0.0` 永远为 False（绝对值 ≥ 0），导致即使 `b == b2` 也不匹配。

验证：
```python
_check_barcode_inclusion(((1.0, 3.0),), ((1.0, 3.0),), 0.0)
# → (False, ((1.0, 3.0),))  # 精确相同却不匹配
```

docstring 声明"birth/death 偏差在此范围内视为匹配"，暗示 epsilon=0 应为精确匹配，但实现不支持。

**严重性**：MEDIUM

**修复**：改为 `<=`：
```python
if j not in matched_low and abs(b - b2) <= epsilon and abs(d - d2) <= epsilon:
```

或在 docstring 中明确声明 epsilon 必须 > 0。

注：当前实际使用中 `check_recursive_barcode_order` 默认 `epsilon=1.0`，不触发此问题。但接口语义不一致。

---

### N3：T5 测试缺少失败路径覆盖

**位置**：tests/test_a_topology.py:461-498

**问题**：T5 的两个测试均只覆盖 happy path（`passed=True`）。以下代码路径没有测试：
- `confirmed_only = False`（moves 中有 unconfirmed trend）
- `identity = False`（moves 与 confirmed_trends 引用不同）
- `passed = False` 的整体路径

这意味着 `check_trend_move_equivalence` 的失败检测逻辑没有被验证。

**严重性**：MEDIUM

**建议**：添加单元测试，构造 mock levels 对象，直接测试失败路径：
```python
def test_t5_fails_when_unconfirmed():
    # 构造 moves 中含 unconfirmed trend 的 mock levels
    ...
    results = check_trend_move_equivalence(mock_levels)
    assert results[0].confirmed_only is False
    assert results[0].passed is False
```

---

### N4：T7/T5 pipeline 测试在单层数据时静默通过

**位置**：test_a_topology.py:432-454, 479-498

**问题**：
```python
if len(levels) >= 2:
    # 实际断言
```

若 `df_raw` 不产生多层递归，`if` 分支不执行，测试静默通过但未验证任何内容。

**严重性**：LOW

当前 `df_raw` 使用 300 根 K 线（seed=42），实践中应能产生多层。但测试的有效性依赖于数据特性，不是显式保证。

---

## 综合判定

| 问题 | 严重性 | 状态 |
|------|--------|------|
| T7-A HIGH-1：空集边界 | HIGH | RESOLVED |
| T7-A HIGH-2：真单射语义 | HIGH | RESOLVED |
| T5 CRITICAL：trivially true → confirmed_only + identity | CRITICAL | RESOLVED |
| T7-B：贪心足够 | MEDIUM | RESOLVED |
| N1：getattr 默认值掩盖类型错误 | MEDIUM | 新发现 |
| N2：epsilon=0.0 精确匹配不通过 | MEDIUM | 新发现 |
| N3：T5 测试缺少失败路径 | MEDIUM | 新发现 |
| N4：pipeline 测试静默通过风险 | LOW | 新发现 |

**最终判定：NEEDS_REVISION**

Round 1 的所有 HIGH/CRITICAL 问题已正确落地，实现质量良好。
新发现 3 个 MEDIUM 问题（N1/N2/N3），其中 N2（epsilon=0.0 语义）是接口级别的不一致，N1（getattr 默认值）违反快速失败原则，N3（测试覆盖）是测试完整性问题。

无 CRITICAL 新问题。修复 N1/N2/N3 后可 APPROVED。

---

## 边界条件

- N2 翻转条件：若 docstring 明确声明 epsilon 必须 > 0（调用方保证），则 N2 不成立，但接口文档需更新
- N1 翻转条件：若 moves 的类型在整个调用链中有静态类型保证（TypeVar/Protocol），则 getattr 防御是冗余的，但不是错误

## 影响声明

- a_topology.py:585-587：N1 修复影响 `check_trend_move_equivalence`
- a_topology.py:496：N2 修复影响 `_check_barcode_inclusion`（接口语义变更，需同步更新 docstring）
- tests/test_a_topology.py：N3 需要新增 T5 失败路径测试

---

*审查工位: codex-reviewer | 谱系位置: 代码层异质否定 | 对标: Gemini challenger (概念层)*
