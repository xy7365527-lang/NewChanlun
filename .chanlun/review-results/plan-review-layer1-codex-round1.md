# Codex Review — Layer 1 T7/T5 技术方案评审 Round 1

**模式**: review
**日期**: 2026-02-25
**评审工位**: codex-reviewer (Claude Sonnet 4.6)
**被评审方案**: a_topology.py Layer 1 新增 T7（递归条形码偏序）+ T5（走势类型≡上级笔）
**代码上下文**: a_topology.py, a_recursive_engine.py, a_trendtype_v0.py

---

## 评审结论汇总

| 问题 | 严重性 | 判定 |
|------|--------|------|
| T7-A: ε-单射匹配定义的自洽性与边界 | HIGH | agree — 有两处遗漏 |
| T7-B: 贪心 vs 最优匹配 | MEDIUM | agree — 贪心在小规模下足够，但需要排序保证 |
| T5-A: 三层检查中哪些是 trivially true | CRITICAL | 重新定性 — 数量检查和方向检查均 trivially true；只有区间兼容有实质内容 |
| T5-B: 区间兼容容差策略 | MEDIUM | agree — 推荐相对容差 |
| 过度工程检查 | LOW | agree — T7 的最优匹配在此场景过度工程 |

---

## T7-A: ε-单射匹配定义自洽性评审

### 原始定义

```python
def _check_barcode_inclusion(bars_high, bars_trimmed_low, epsilon):
    """对 bars_high 中的每个 bar (b,d)，要求存在 bars_trimmed_low 中的
    某个 bar (b',d') 使得 |b-b'| < ε 且 |d-d'| < ε。
    这是单射匹配，不是双射。"""
```

### 自洽性分析

**定义本身的数学含义**：

每个 bars_high 中的 bar 必须在 bars_trimmed_low 中找到 ε-邻域内的 partner。
这是"bars_high ε-嵌入 bars_trimmed_low"的定义。

**发现的两处边界遗漏**：

**遗漏 1：没有要求 bars_trimmed_low 非空时的退化情况**

若 bars_high 为空，条件 trivially true（全称量词对空集成立）。
若 bars_trimmed_low 为空但 bars_high 非空，条件 trivially false。
当前 docstring 没有说明这两种退化情况的返回值规范。

建议：
```python
if not bars_high:
    return True   # 空集总被包含
if not bars_trimmed_low:
    return False  # 非空集不被空集包含
```

**遗漏 2："单射"语义在定义中实际上是弱于真正的单射**

docstring 说"这是单射匹配"，但定义只要求"存在某个 bar'"——允许 bars_trimmed_low 中同一个 bar' 被多个 bars_high 中的 bar 匹配。
这是**多对一关系**，不是真正的单射（单射要求不同的 bars_high bar 对应不同的 bars_trimmed_low bar'）。

对 T7 的语义影响：若两层递归的条形码有相同的 bar（中枢重叠），这个"伪单射"会通过多对一匹配给出 True，但实际上 bars_high 的多个 bar 都锁定了 bars_trimmed_low 的同一个 bar。

如果 T7 的语义是"bars_high 的所有信息都被 bars_trimmed_low 覆盖"，多对一匹配是过宽的——多个高层 bar 匹配同一个低层 bar 意味着高层结构有冗余或低层分辨率不足。

**推荐**：明确说明是"多对一允许"还是"真正单射"。若要严格单射：
```python
matched_low = set()
for b, d in bars_high:
    found = False
    for j, (b2, d2) in enumerate(bars_trimmed_low):
        if j not in matched_low and abs(b - b2) < epsilon and abs(d - d2) < epsilon:
            matched_low.add(j)
            found = True
            break
    if not found:
        return False
return True
```

### 判定

遗漏 1（退化情况）：**HIGH** — 空集边界缺失会导致 NoneType 或逻辑错误。
遗漏 2（伪单射 vs 真单射）：**HIGH** — 语义声明与实现不匹配。"单射"是强声明，若实现是多对一，则声明是错的。

---

## T7-B: 贪心 vs 最优匹配评审

### 分析

场景参数：n_centers 通常 < 20，实际多数情况 < 10。

**贪心 O(n·m)**：按 bars_high 顺序逐一在 bars_trimmed_low 中找第一个 ε-邻域内的匹配。
**最优（匈牙利算法）O(n³)**：构建二部图，找最大匹配或最小代价匹配。

**贪心的误判风险**：

考虑如下情况（n=2, m=2, ε=1.0）：
```
bars_high    = [(0.0, 2.0), (0.5, 2.5)]
bars_low     = [(0.4, 2.4), (0.0, 2.1)]
```
贪心：(0.0, 2.0) 先匹配 (0.4, 2.4)（距离 0.4 < 1），锁定；
     (0.5, 2.5) 找剩下的 (0.0, 2.1)，|0.5-0.0|=0.5 < 1, |2.5-2.1|=0.4 < 1，匹配成功。
结论：True。

若顺序不同：(0.0, 2.0) 先匹配 (0.0, 2.1)，(0.5, 2.5) 再找 (0.4, 2.4)，也 True。

贪心误判的触发条件：bars_high 的第一个 bar 抢占了 bars_low 中"唯一能覆盖后续 bar"的 bar，导致后续 bar 找不到匹配。这在小规模（n < 20）且 ε 合理（不极小）的情况下极少发生。

**推荐**：

贪心已足够，但需要一个 **排序前处理**：按 `|b - b'| + |d - d'|` 贪心地排序 bars_high（或先排 bars_high 再遍历 bars_low）不能从根本上防止误判。

正确的防误判方法：若需严格单射语义，用带 `matched_low` 集合的贪心（如 T7-A 遗漏 2 的代码），这已足够防止多对一；真正的最优匹配（匈牙利）仅在需要"全局最小代价"时才有价值，此处不需要。

**结论**：使用带 `matched_low` 集合的贪心 O(n·m)。不需要匈牙利算法。

---

## T5-A: 三层检查的 trivially true 分析（CRITICAL）

### 递归引擎关键代码

```python
confirmed_trends = [t for t in level.trends if t.confirmed]
moves = confirmed_trends  # 下一层的 moves 就是上一层的 confirmed_trends
```

即：`levels[k+1].moves` is `levels[k].confirmed_trends`（Python 对象引用）。

### 三层检查的实质性分析

**层 1：数量相等 `len(confirmed_trends_k) == len(moves_{k+1})`**

**trivially true**。

`moves_{k+1}` 的定义就是 `confirmed_trends_k`（同一个 Python 列表）。
`levels[k+1].moves` 在 `_build_single_level` 中接收的就是 `confirmed_trends`。

检查这个等式等价于检查 `len(x) == len(x)`，恒真。

**层 2：方向一致 `trend.direction == move.direction`**

**trivially true**。

同一原因：`levels[k+1].moves[i]` is `levels[k].confirmed_trends[i]`——是同一个 TrendTypeInstance 对象。
`direction` 字段在 `_stamp_trends` 中也不被修改（只更新 `level_id`）。

检查方向等价于 `obj.direction == obj.direction`，恒真。

**层 3：区间兼容 `trend.price_range ≈ move.price_range`**

**有实质内容**，但需要重新定义语义。

同一对象的 `high/low` 字段显然相等（`trend.high == move.high`），这也是 trivially true。

但方案中提到"每个 trend 的价格范围应近似等于对应 move 的价格范围"——如果指的是 `confirmed_trends_k[i]` 的 `high/low` 与 `levels[k+1].moves[i]` 的 `high/low` 之间的比较，这同样是 trivially true（同一对象）。

**区间兼容检查有实质内容的唯一形式**：检查 `levels[k].confirmed_trends[i]` 与 `levels[k+1].centers[j].price_range`（或 `levels[k+1].trends[j].price_range`）的关系，而不是与自身比较。

### T5 的正确语义

T5 的原始缠论命题是：**走势类型实例≡上级笔**（即同级别的已确认走势类型实例是下一级别的"笔"）。

在当前代码结构下，这个等价关系是**由构造保证的（构造恒真）**，不是需要运行时验证的约束。

T5 的真正检查意义在于：
- **检查递归引擎是否正确传递了引用**（而不是复制了错误的对象）
- **检查没有引用泄漏**（moves 列表不包含 unconfirmed trends）

具体：
```python
# T5 的实质性检查
assert all(t.confirmed for t in levels[k+1].moves)
assert levels[k+1].moves is confirmed_trends_k  # 同一对象引用
```

**方向兼容、数量相等、区间兼容**三层检查在当前实现中均为冗余，不揭示新信息。

### 判定：CRITICAL

方案将三层检查描述为"结构兼容性检查"，但实际上所有三层均为 trivially true。
这不是代码 bug，而是方案对"T5 要检查什么"的定位有误。

**修改建议**：

T5 不应实现为"三层结构兼容性检查"，而应实现为：
1. `confirmed_only_check`：`levels[k+1].moves` 中所有元素的 `confirmed` 为 True
2. `identity_check`：`levels[k+1].moves` 与 `levels[k].confirmed_trends` 是同一引用（或相同内容）

这两个检查才是 T5 命题的实质：走势≡上级笔是构造保证，不是运行时断言，但可以作为不变式验证。

---

## T5-B: 区间兼容容差策略

### 分析

若 T5 区间兼容改为"检查 confirmed_trends_k 的 high/low 与上级 centers 的 ZG/ZD 的关系"（这才是有实质内容的检查），则容差策略如下：

**绝对容差**：`abs(trend.high - center.high) <= delta`
- 优点：直观，易于调试
- 缺点：对低价格资产（几元）和高价格资产（几万元）的容差含义不同

**相对容差**：`abs(trend.high - center.high) / max(trend.high, center.high) <= rel_tol`
- 优点：尺度无关，适用于不同价格量级的资产
- 缺点：除以0的风险（high=0 时）

**推荐**：相对容差，默认 `rel_tol=0.01`（1%），加零值保护：
```python
def _approx_equal(a, b, rel_tol=0.01):
    denom = max(abs(a), abs(b))
    if denom == 0:
        return True
    return abs(a - b) / denom <= rel_tol
```

但基于 T5-A 的分析，此检查的前提（区间兼容有实质内容）需要重新确认检查对象。

---

## 过度工程检查

**匈牙利算法**：对 n < 20 的场景，O(n³) 的匈牙利算法实现成本（代码复杂度 + 测试）远超收益。使用带 `matched_low` 集合的贪心已足够。不推荐引入。

**T5 三层检查**：若按方案实现，三层检查均为 trivially true，这是**无效工程**——代码量 > 0 但信息增量 = 0。这比过度工程更糟：它会给维护者错误的信心（以为做了有意义的检查）。必须改为有实质内容的检查。

---

## 综合判定

| 判定 | 结论 |
|------|------|
| T7-A 遗漏1（空集边界） | 否定成立，需要修复 |
| T7-A 遗漏2（伪单射 vs 真单射） | 否定成立，需要澄清语义并对齐实现 |
| T7-B 贪心 vs 最优 | 贪心足够，不需要匈牙利算法 |
| T5 三层检查均为 trivially true | CRITICAL 否定成立，方案对T5语义定位有误 |
| T5 容差 | 相对容差，但前提需先澄清检查对象 |

---

## 边界条件

- T7-A 遗漏2 翻转条件：若方案明确声明允许多对一（不要求真正单射），则遗漏2不成立，但 docstring 中"单射"一词必须改为"多对一覆盖"
- T5 trivially true 翻转条件：若递归引擎改为**复制**而非引用传递（即 `moves = list(confirmed_trends)`，对象相同但列表不同），则数量检查和方向检查依然 trivially true；只有在引擎改为构造新的 TrendTypeInstance 对象时，方向和区间检查才有实质内容

---

## 影响声明

- **a_topology.py T7 实现**：需要处理空集边界 + 澄清单射语义（真单射 vs 多对一）
- **a_topology.py T5 实现**：不应实现三层兼容性检查，应实现 confirmed_only_check + identity_check
- **测试设计**：T5 测试应验证 confirmed 属性，而非价格区间兼容性

---

*审查工位: codex-reviewer | 谱系位置: 代码层异质否定 | 对标: Gemini challenger (概念层)*
