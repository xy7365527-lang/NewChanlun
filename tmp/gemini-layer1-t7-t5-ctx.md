# Gemini 评审上下文：Layer 1 T7/T5 — Codex Round 1 修正验证

## 任务

你是 Gemini 评审角色（verify 模式）。
评审对象：Codex Round 1 对 T7（递归条形码偏序）和 T5（走势类型≡上级笔）的两项修正。
要求：从数学/概念层给出同意/否定/部分同意，附数学理由。

---

## 代码上下文：递归引擎关键结构

### build_recursive_levels（a_recursive_engine.py）

```python
def build_recursive_levels(segments, *, sustain_m=2, max_levels=10, ...):
    levels = []
    moves = list(segments)
    for k in range(1, max_levels + 1):
        if len(moves) < 3:
            break
        level = _build_single_level(moves, k, sustain_m, ...)
        if level is None:
            break
        levels.append(level)
        confirmed_trends = [t for t in level.trends if t.confirmed]
        if len(confirmed_trends) < 3:
            break
        moves = confirmed_trends   # ← 关键：直接赋值，不是复制
    return levels
```

关键点：`moves = confirmed_trends` 是直接引用赋值，不是 `list(confirmed_trends)`。
因此：`levels[k+1].moves is levels[k].confirmed_trends` 为 **True**（同一Python对象）。

### RecursiveLevel 数据类

```python
@dataclass(frozen=True, slots=True)
class RecursiveLevel:
    level: int
    moves: list          # level=1 时为 Segment，level>=2 时为 TrendTypeInstance
    centers: list[Center]
    trends: list[TrendTypeInstance]
    divergences: list[Divergence]
```

### _stamp_trends（重要：创建新对象）

```python
def _stamp_trends(trends, level_id):
    return [
        TrendTypeInstance(
            kind=t.kind, direction=t.direction,
            seg0=t.seg0, seg1=t.seg1,
            i0=t.i0, i1=t.i1,
            high=t.high, low=t.low,
            center_indices=t.center_indices,
            confirmed=t.confirmed,
            level_id=level_id,
        )
        for t in trends
    ]
```

注意：`_stamp_trends` **创建新的 TrendTypeInstance 对象**（不是同一对象引用）。
但是：`confirmed_trends = [t for t in level.trends if t.confirmed]` 选的是已 stamp 的 trends，
然后 `moves = confirmed_trends`——下一层的 moves 是当前层 stamped trends 的子集引用。

---

## Codex 修正1：T7 真单射语义

### 原始方案（简化）

```python
def _check_barcode_inclusion(bars_high, bars_trimmed_low, epsilon):
    """单射匹配：bars_high ε-嵌入 bars_trimmed_low"""
    for b, d in bars_high:
        found = False
        for b2, d2 in bars_trimmed_low:
            if abs(b-b2) < epsilon and abs(d-d2) < epsilon:
                found = True
                break
        if not found:
            return False
    return True
```

问题：允许 bars_trimmed_low 中同一个 bar 被多次匹配（多对一）。

### Codex 修正：真单射

```python
matched_low: set[int] = set()
for b, d in bars_high:
    found = False
    for j, (b2, d2) in enumerate(bars_trimmed_low):
        if j not in matched_low and abs(b-b2) < epsilon and abs(d-d2) < epsilon:
            matched_low.add(j)
            found = True
            break
    if not found:
        return False
return True
```

### 数学问题

T7 的理论背景：递归构造产生的条形码序列应满足偏序关系 B_{k+1} ⊆ Trim(B_k)。

其中：
- B_k = levels[k] 的条形码（中枢列表的 ZD/ZG 对）
- Trim(B_k) = 去掉 B_k 中最短 bar 后的子集（持续同调的 trimming 操作）
- B_{k+1} ⊆ Trim(B_k) 的语义：高级别条形码是低级别 trim 后的子集

你需要判断：
1. "子集"（⊆）关系在条形码偏序的数学语境下，要求真单射（injective map）还是多对一？
2. 持续同调中，条形码之间的"稳定性"关系（bottleneck 距离意义下）如何定义映射？
3. 贪心匹配是否可能产生假阴性（即存在合法的单射匹配但贪心找不到）？

---

## Codex 修正2：T5 trivially true（CRITICAL）

### Codex 分析

`levels[k+1].moves` 就是 `levels[k].confirmed_trends` 的同一 Python 引用（见代码）。

三层检查均 trivially true：
- 数量：`len(x) == len(x)` → 恒真
- 方向：`obj.direction == obj.direction` → 恒真
- 区间：`obj.high == obj.high` → 恒真

Codex 建议改为"构造不变式验证"：
```python
assert all(t.confirmed for t in levels[k+1].moves)
assert levels[k+1].moves is confirmed_trends_k
```

### 等待你验证的关键问题

**问题A**：Codex 说"同一 Python 引用"——但注意 `_stamp_trends` 创建了新的 TrendTypeInstance 对象。
具体执行链：
1. `level.trends` = `_stamp_trends(trends, k)` → 新对象列表
2. `confirmed_trends = [t for t in level.trends if t.confirmed]` → 过滤，同一对象
3. `moves = confirmed_trends` → 赋值
4. 下一层：`_build_single_level(moves, k+1, ...)` 接收 `moves`
5. 下一层 level 的 `moves` 字段 = 传入的 `moves`（即 `confirmed_trends`）

所以 `levels[k+1].moves is levels[k].confirmed_trends`？
- `levels[k].confirmed_trends` 是 list comprehension 产生的新列表
- `levels[k+1].moves` 是 `RecursiveLevel(moves=moves, ...)` 中的 moves
- 两者是否同一引用取决于 Python 的赋值语义

**问题B**：T5 的数学命题（Trend_k ≡ E(X_{k+1})，走势类型实例等同于上级的基础移动单元）在代码层面是"构造保证"还是"运行时约束"？

**问题C**：如果 _stamp_trends 不改变 confirmed 属性，`confirmed_only_check` 是否有意义？

---

## Codex 建议的 epsilon 策略

Codex 推荐相对容差 `rel_tol=0.01`（1%）用于条形码 birth/death 的 ε-邻域匹配。

条形码的 birth = 中枢 ZD（三段最高最低价的交集下界），death = 中枢 ZG（交集上界）。

你需要判断：
- ZD/ZG 是价格值（浮点数），1% 相对容差的数学合理性
- 1% 是否能覆盖跨模式（wide/strict/new）分型选择差异导致的价格偏移
- 是否应该使用 math.isclose 的默认 rel_tol=1e-9 还是 1e-2

---

## 输出要求

对 T7 修正（真单射）：同意/否定/部分同意 + 数学理由
对 T5 修正（trivially true）：同意/否定/部分同意 + 数学理由（含对 _stamp_trends 的分析）
对 epsilon 策略：同意/否定 + 数学理由

约束：回复 ≤ 8KB。
