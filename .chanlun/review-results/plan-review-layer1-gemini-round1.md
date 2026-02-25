---
trigger: team-lead 指派 — Layer 1 T7/T5 Codex Round 1 修正评审
target: plan-review-layer1-codex-round1
mode: verify
result: conditional
date: 2026-02-25
model: gemini-3.1-pro-preview
---

# Gemini 评审 — Layer 1 T7/T5 Codex Round 1 修正验证

## 评审结论汇总

| 修正项 | Gemini 判定 | 异质质询判定 |
|--------|-------------|-------------|
| T7：真单射数学直觉 | 同意 | 成立 |
| T7：贪心实现（matched_low 集合） | 否定（假阴性） | 成立 |
| T5：三层检查均 trivially true | 同意 | 成立 |
| T5：构造不变式验证方向 | 同意（含补充） | 成立 |
| epsilon rel_tol=1% | 强烈否定 | 部分成立（见详析） |

---

## 修正1：T7 真单射评审

### Gemini 判定：同意数学直觉，否定贪心实现

**数学理由**：

条形码子集关系 B_{k+1} ⊆ Trim(B_k) 在 ε-邻域定义下等价于二分图最大匹配问题，不是贪心问题。

Gemini 构造的假阴性反例：

```
bars_high = [A, B]
bars_low  = [X, Y]

A 的 ε-邻域覆盖 X 和 Y
B 的 ε-邻域只覆盖 X
```

Codex 贪心执行（matched_low 集合）：
- A 遍历到 j=0（X），匹配，locked；
- B 遍历 j=0（locked），跳过；遍历 j=1（Y），B 不在 Y 邻域 → False。

但存在合法单射：A→Y，B→X → 应返回 True。

结论：Codex 贪心代码在 ε 较大时存在假阴性。

### 第二轮 Gemini 审核（2026-02-25 本次调用）补充

本次审核进一步给出了具体数值反例：

```
epsilon = 0.1
bars_high        = ((1.0, 3.0), (1.08, 3.08))   # H1, H2
bars_trimmed_low = ((1.05, 3.05), (0.95, 2.95)) # L1, L2
```

贪心执行：
1. H1 (1.0, 3.0) 匹配 L1 (1.05, 3.05)：误差 0.05 < 0.1，锁定
2. H2 (1.08, 3.08) 只剩 L2 (0.95, 2.95)：误差 0.13 > 0.1，失败
3. 返回 False

最优匹配：H1→L2（误差0.05），H2→L1（误差0.03）→ True

**两轮 Gemini 审核结果一致**：贪心实现在 ε>0 时存在 false negative，属于算法正确性缺陷。

### 简化质询结果

**定义回溯**：二分图最大匹配（Hopcroft-Karp，O(E√V)）能消除假阴性。对于 n_centers < 20 的场景，O(n²√n) 可接受。

**反例有效性**：反例在数学上有效——只要 epsilon 不极小（即存在一个 bar 的 ε-邻域覆盖多个低层 bar），贪心就可能产生假阴性。缠论场景中，跨级别中枢在价格空间的分布不保证稀疏，因此假阴性不是极端情况。

**推论一致性**：使用最大二分匹配不改变 T7 的数学语义，只修正实现的正确性。

**判定：否定成立（Codex 贪心代码需要替换为最大二分匹配）**

### 正确实现建议

```python
def _check_barcode_inclusion_injective(bars_high, bars_trimmed_low, epsilon):
    """ε-单射包含检查：使用最大二分匹配（无假阴性）"""
    if not bars_high:
        return True
    if not bars_trimmed_low:
        return False
    n, m = len(bars_high), len(bars_trimmed_low)
    # 构建相容矩阵
    compat = [
        [abs(bars_high[i][0]-bars_trimmed_low[j][0]) < epsilon
         and abs(bars_high[i][1]-bars_trimmed_low[j][1]) < epsilon
         for j in range(m)]
        for i in range(n)
    ]
    # 增广路径法（简单实现，n<20 足够）
    match = [-1] * m

    def augment(i, visited):
        for j in range(m):
            if compat[i][j] and not visited[j]:
                visited[j] = True
                if match[j] == -1 or augment(match[j], visited):
                    match[j] = i
                    return True
        return False

    matched = sum(1 for i in range(n) if augment(i, [False]*m))
    return matched == n
```

---

## 修正2：T5 trivially true 评审

### Gemini 判定：同意（含 _stamp_trends 引用链分析）

**Gemini 的推理链**（补充了 _stamp_trends 分析）：

1. `level.trends` = `_stamp_trends(trends, k)` → 新 TrendTypeInstance 对象列表（新对象）
2. `confirmed_trends = [t for t in level.trends if t.confirmed]` → 过滤，但元素是同一对象引用
3. `moves = confirmed_trends` → 直接赋值（同一列表对象）
4. 下一层 `_build_single_level(moves, ...)` 接收 `moves`
5. `RecursiveLevel(moves=moves, ...)` 将 `moves` 挂载为字段

因此：`id(levels[k+1].moves[i])` = `id(levels[k].confirmed_trends[i])`（同一 Python 对象）

**三层检查均 trivially true** 诊断成立：
- `obj.direction == obj.direction` → 恒真（同一对象同一属性）
- `obj.high == obj.high` → 恒真
- `len(confirmed_trends) == len(moves)` → 恒真（同一列表）

### Gemini 对"构造不变式"方向的补充

Gemini 指出 Codex 建议的 `confirmed_only_check` 有意义，`identity_check` 也有意义。但 Gemini 补充了更深的一个视角：

> "如果需要验证 T5，应该验证的是'下一层的中枢/走势计算是否严格只依赖于传入的 moves 的边界属性，而没有发生语义降级'，而不是验证对象属性是否等于自身。"

这是 Codex 未覆盖的角度：T5 的真正风险不在于当前代码，而在于**递归引擎将来若改为复制语义时**，这两个不变式才会真正起作用。

### T5 identity 检查（is vs ==）异质否定评估

本次审核中 Gemini 提出 `m is ct`（Python 内存地址同一性）应改为 `m == ct`（内容相等），理由是"若管线包装对象则 is 失败"。

**简化质询**：
- `levels[k+1].moves` 直接来自递归引擎内部构造，不经过序列化
- `is` 检查的目的是验证**引擎是否正确传递引用**（而非是否内容相等）
- 如果引擎有意包装对象（`moves = [copy(t) for t in confirmed_trends]`），这违反了 T5 的构造语义
- Gemini 的"包装场景"是 T5 构造不变式**失败**的场景，不是"合理的实现变体"

**判定：T5 identity 否定不成立**（基于对"包装是合理变体"的误解）。`is` 检查语义正确，应补充 docstring 说明"is 检查假设引擎直接引用传递，若失败说明引擎存在非预期复制"。

### 简化质询结果

**定义回溯**：Codex 和 Gemini 对 trivially true 诊断一致，异质质询未发现分歧。

**边界条件**：如果 `_stamp_trends` 不改变 `confirmed` 属性（当前代码不改变），则 `confirmed_only_check` 在现有代码下是 trivially true。但这不影响其作为"将来引擎修改的守卫"的价值——它将 trivially true 转换为有意义的回归测试。

**判定：T5 trivially true 否定成立。Codex 修正方向正确，Gemini 补充了语义降级守卫的维度。**

---

## epsilon 策略评审（rel_tol=1%）

### Gemini 判定：强烈否定

Gemini 的三个否定论据：

1. **尺度灾难**：价格 60,000 的标的，1% = 600 价格单位，可能覆盖几十个中枢的范围 → 大量假阳性。

2. **误差来源**：同模式内，ZD/ZG 由浮点运算精确传递，误差仅为 IEEE 754 精度（1e-9 量级），不是 1e-2。

3. **跨模式比较**：跨模式时应使用 bottleneck 距离测量整体偏移，而非用 ε 强行硬判 True/False。

### 简化质询结果

**定义回溯**：Codex 的 1% 是用于 T7（条形码包含检查），场景是同一数据集的两个递归级别（而非跨模式）。Gemini 的"跨模式"论据部分错位。

**但核心否定成立**：
- 在 T7 场景（高级别条形码 vs 低级别 trim 后的条形码）中，ZD/ZG 来自不同级别中枢的精确计算，不应有大的价格偏差。
- 若 ε 过大（1% 可能等于数百个价格单位），会导致本不应匹配的条形码被错误匹配，破坏 T7 的偏序语义。
- 正确策略：ε 应基于最小笔的价格振幅百分比设定，或使用 epsilon=0（精确匹配），配合 bottleneck 距离作为连续测量。

**边界条件**：如果 ε=0（精确相等），则 T7 退化为精确多重集包含检查。这在同级别递归内部是合理的（同一数据的不同级别中枢 ZD/ZG 应精确对应），但跨模式时不适用——跨模式应用 bottleneck 距离。

**判定：epsilon 1% rel_tol 否定成立（场景不当，尺度不合理）。**

---

## 综合结论

### 否定成立的项目

| 项目 | 否定形式 | 修复方向 |
|------|----------|---------|
| T7 贪心实现 | 扩张否定（贪心有假阴性，需扩张为最大二分匹配） | 替换为增广路径法或 Hopcroft-Karp |
| T5 三层检查 trivially true | 等待否定（原方案等待一个不会发生的状态） | 改为 confirmed_only_check + identity_check |
| epsilon 1% rel_tol | 分离否定（策略与场景分离——1% 适用于价格波动估计，不适用于精确条形码匹配） | 使用 epsilon≈0 或基于数据统计的自适应 ε |

### 否定不成立的项目

- T7 数学直觉（真单射优于多对一）：成立，Codex 方向正确
- T5 构造保证方向：成立，Codex 诊断正确
- T5 confirmed_only_check 作为回归守卫：成立，有前瞻价值
- T5 identity_check（is 检查）：Gemini 的 is→== 建议不成立，is 语义正确

### T7 测试盲区

两轮审核均发现：TestBarcodeInclusion 缺少 ε-邻域交叠的测试用例（如 Gemini 反例）。
应补充：

```python
def test_greedy_antidote_bipartite(self):
    """ε-邻域交叠场景需要二分图匹配（贪心会给出错误结果）。"""
    high = ((1.0, 3.0), (1.08, 3.08))
    low  = ((1.05, 3.05), (0.95, 2.95))
    passed, unmatched = _check_barcode_inclusion(high, low, 0.1)
    assert passed is True  # 存在合法单射：(1.0,3.0)→(0.95,2.95), (1.08,3.08)→(1.05,3.05)
```

---

## 定义依据

- 条形码包含关系：Zomorodian & Carlsson (2005)，持续模块的区间分解；"子集"在多重集语境下要求单射
- 最大二分匹配：König定理，二分图最大匹配 = 最小顶点覆盖
- IEEE 754 浮点精度：64位双精度，相对误差约 2^-52 ≈ 2.2e-16
- T5 缠论命题：缠论知识库.md §7 — 走势类型实例是上级笔的等价物

## 下游推论

1. **T7 实现修正**：a_topology.py 中的 `_check_barcode_inclusion` 需要替换为最大二分匹配实现
2. **T5 实现修正**：按 Codex 建议实现 confirmed_only_check + identity_check（两者均有回归守卫价值）
3. **epsilon 策略**：T7 内部（同模式）用 epsilon=1e-9；跨模式比较用 bottleneck 距离

## 谱系引用

- 195号谱系（拓扑化框架）
- plan-review-195-self-audit-round1.md（原始方案）
- plan-review-layer1-codex-round1.md（Codex 修正）

## 影响声明

- a_topology.py：T7 实现需替换贪心为最大二分匹配
- a_topology.py：T5 实现需替换三层结构兼容性检查为构造不变式验证（已完成）
- tests/test_a_topology.py：T7 测试需增加假阴性场景覆盖（ε-邻域交叠反例）

---

*评审工位: gemini-challenger (verify) | 模型: gemini-3.1-pro-preview | 日期: 2026-02-25*
*第二轮 Gemini 调用（verify 模式）于 2026-02-25 06:52 执行，与第一轮结论一致，补充了具体数值反例*
