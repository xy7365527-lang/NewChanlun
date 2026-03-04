# Gemini 异质审计：195号拓扑化方案数学准确性验证

## 审计任务

对 A 系统 7 个模块中的拓扑语义标注（docstring 中的"## 拓扑语义（195号）"章节）
进行数学准确性审计。对每个标注评级：PASS / WARN / FAIL。

审计维度：
1. "结构映射"是否有数学错误？
2. "映射的边界"是否诚实？是否遗漏了已知的限度？
3. `a_topology.py` 的 DecompositionFingerprint / StructuralDelta / TransitionResult 设计是否合理？
4. 强不变量候选分类是否经得起推敲？

---

## 模块 1：a_inclusion.py — 包含处理（商空间标注）

### 拓扑语义（195号）原文（第1-22行）

结构映射：
- raw K线序列 X 上的偏序关系 ≤ 由"包含"定义：a ≤ b ⇔ [a.low, a.high] ⊆ [b.low, b.high]
- 等价关系 ~：x ~ y ⇔ x 与 y 有包含关系且相邻（传递闭包后形成等价类）
- merge_inclusion() 是商映射 π: X → X/~，将每个等价类映射到一个 merged bar
- merged_to_raw 是商映射的纤维结构：记录每个 merged bar 对应的 raw 范围
- 方向状态 dir_state 决定"向上取并/向下取交"——等价类代表元的极值选择规则

映射的边界：
- 商映射 π 是精确的——merge_inclusion 确实计算了等价类的代表元
- 但不应称 Alexandrov 拓扑——Alexandrov 拓扑在有限离散序列上是平凡的（每个点既开又闭）
- 真正的拓扑内容在于商映射的纤维结构（merged_to_raw = π 的逐纤维分解），不在开集

### 关键实现细节

```python
# _merge_loop (第44-70行)
has_inclusion = (last_h >= curr_h and last_l <= curr_l) or (
    curr_h >= last_h and curr_l <= last_l
)
```

等价关系声明为：x ~ y ⇔ x 与 y 有包含关系且相邻（传递闭包）。

实现中等价关系定义为"相邻且有包含"，合并是贪心增量：
- 只在尾部（最后一个 merged bar）检查包含关系
- 传递闭包是通过顺序扫描隐式实现的

---

## 模块 2：a_fractal.py — 分型识别（Morse 临界点标注）

### 拓扑语义（195号）原文（第7-25行）

结构映射：
- 顶分型 = 局部极大值点（index-1 临界点的类比）
- 底分型 = 局部极小值点（index-0 临界点的类比）
- 双条件（h_curr > h_prev AND h_curr > h_next AND l_curr > l_prev AND l_curr > l_next）
  = 严格局部极值条件（Morse 非退化条件的离散一维对应：严格不等式 = 非退化）

映射的边界：
- 这里的"Morse"是连续 Morse 理论的一维类比，不是 Forman 离散 Morse 理论的精确实例
- Forman 理论需要 CW 复形上的梯度向量场配对结构，分型检测不具备这个结构

### 关键实现细节

```python
# _classify_fractal（第61-71行）
if (
    h_curr > h_prev and h_curr > h_next
    and l_curr > l_prev and l_curr > l_next
):
    return Fractal(idx=idx, kind="top", price=float(h_curr))
```

关键问题：顶分型双条件要求 h 和 l 同时严格大于相邻 bar。
这意味着中心 bar 的价格区间 [l_curr, h_curr] 完全"高于"相邻 bar，
而不仅仅是 high 是局部极大。

标注称 index-1 临界点的类比，但实际检测的是二维意义上的极值（high 和 low 同时极值），
不仅是函数高度的局部极大值。

---

## 模块 3：a_stroke.py — 笔构造（CW 1-cell 标注）

### 拓扑语义（195号）原文（第7-25行）

结构映射：
- 分型 = 0-cell（CW 复形的顶点集）
- 笔 = 1-cell（连接两个 0-cell 的边）
- 胶合映射 φ: ∂D¹ → X⁰ 由 (i0, i1) 定义
- 连续性保证 strokes[i].i1 == strokes[i+1].i0 = CW 胶合条件（相邻 1-cell 共享 0-cell）
- 顶底交替 = CW 复形的定向性
- gap 检查 = 1-cell 的最小长度约束

映射的边界：
- CW 复形的 1-cell 类比在此处是精确的
- 限度：CW 复形通常允许 1-cell 自环（端点相同），笔不允许（顶底交替）
- 这是 CW 复形的一个受限特例——定向 1 维 CW 复形

### 关键实现细节

```python
# strokes_from_fractals（第230-267行）
# 连续性验证：最后一笔 confirmed=False，连续性保证 strokes[i].i1 == strokes[i+1].i0
```

CW 胶合条件声明：`strokes[i].i1 == strokes[i+1].i0`

但实现中分型索引（idx）在特定情况下可能不同——
当 _extend_prev_stroke 延伸一笔时，i1 被更新为更极端的分型 idx，
但 i0 保持不变。这意味着实际实现中相邻笔共享的是"同一个分型"这一概念，
但 idx 在延伸后可能不精确连续。

---

## 模块 4：a_segment_v1.py — 线段（1-chain / 子复形标注）

### 拓扑语义（195号）原文（第17-33行）

结构映射：
- 线段 = X¹ 的一个子复形 σ = {连续笔的链 c₁+c₂+...+cₙ}
- 特征序列（反向笔序列 + 包含处理 + 分型检测）= 子复形边界的标定算法
- 断段操作 = 子复形的分割（subdivision）
- break_evidence = 分割点的证书

映射的边界：
- 线段不是 2-cell——整个构造在 1 维骨架上操作
- 特征序列的"分型"和 K 线分型不是同一层——它是子复形分割的标记，不是 CW 附着映射

### 关键实现细节

断段条件（第393-397行）：
```python
if k + 2 >= n or not _three_stroke_overlap(
    strokes[k], strokes[k + 1], strokes[k + 2]
):
    feat.skip_trigger(k)
    return None
```

"结算锚验证"：新段前三笔必须有重叠。这不是线性代数的 1-chain 边界标定，
而是一个组合条件。声明"subdivision"是否准确？

---

## 模块 5：a_center_v0.py — 中枢（闭区间有限交标注）

### 拓扑语义（195号）原文（第17-35行）

结构映射：
- 中枢 [ZD, ZG] = 三个线段价格区间 [seg.low, seg.high] 的交集
- ZG = min(g₁, g₂), ZD = max(d₁, d₂) = 交集的精确计算（区间交集公式）
- 延伸 = 交集的扩展：新段 [l, h] 仍与 [ZD, ZG] 有非空交集
- 破坏 = 交集条件断裂

映射的边界：
- 中枢是 3 段闭区间的交集——这是有限交的一个特例，不是有限交性质（FIP）的一般实例
- 称紧致性是过度命名——这只是因为实数闭区间本身就是紧致的

### 关键实现细节（第129-139行）：

```python
def _three_seg_overlap_all(s1, s2, s3) -> tuple[float, float]:
    """三段全部重叠的 (ZD_all, ZG_all)，用于中枢成立判定。"""
    return max(s1.low, s2.low, s3.low), min(s1.high, s2.high, s3.high)

def _zseg_interval(s1, s3) -> tuple[float, float]:
    """Z走势段 (s1 与 s3，方向一致) 的 ZD, ZG。"""
    return max(s1.low, s3.low), min(s1.high, s3.high)
```

注意：_try_init_center 计算了两个不同的区间：
1. `_three_seg_overlap_all(s1, s2, s3)` — 三段全部交集（用于成立判定）
2. `_zseg_interval(s1, s3)` — 仅 Z走势段（s1, s3）的交集（用作实际 ZD/ZG）

标注声明 "ZG = min(g₁, g₂), ZD = max(d₁, d₂) = 交集的精确计算"，
但 ZG/ZD 计算只用了 s1 和 s3（Z走势段），不是三段的交集。

---

## 模块 6：a_trendtype_v0.py — 走势类型（路径空间组合分类标注）

### 拓扑语义（195号）原文（第15-33行）

结构映射：
- 走势 = 路径空间 P(X, a, b) 中从起点 a 到终点 b 的有向路径
- 走势类型 = 路径的同伦类型，由中枢的组合数据决定
- 趋势（≥2 同向中枢）= 穿越多个紧致集的路径类
- 盘整（1 中枢）= 被单个紧致集约束的路径类

映射的边界：
- 走势不是环路——它有起点和终点，不闭合。因此 π₁（基本群）不直接适用
- 分类由中枢的有限组合数据确定，是有限组合分类，不需要基本群机制
- 辫群（braid group）是一个可能更精确的类比，但当前实现是直接组合判定

### 关键实现细节（第99-114行）：

```python
def _centers_relation_by_gg_dd(c_prev, c_next) -> str:
    prev_gg, prev_dd = c_prev.gg, c_prev.dd
    next_gg, next_dd = c_next.gg, c_next.dd
    if next_gg < prev_dd:
        return "down"
    if next_dd > prev_gg:
        return "up"
    if c_next.high < c_prev.low and next_gg >= prev_dd:
        return "higher_center"
    if c_next.low > c_prev.high and next_dd <= prev_gg:
        return "higher_center"
    return "none"
```

"路径同伦类型"的声明：这是一个有限组合判断，是否能称为"同伦类型"需要核实。

---

## 模块 7：a_recursive_engine.py — 递归引擎（分层构造标注）

### 拓扑语义（195号）原文（第12-32行）

结构映射：
- RecursiveLevel(k) = 第 k 层 stratum S_k
- 递归门槛：confirmed trends >= 3 才触发下一层构造
- build_recursive_levels() = 自下而上的分层构造算法

映射的边界：
- 这里的"分层"是组合学意义上的层级分解——空间 = ∪S_k，各层不相交且由低层构造
- 不是 Whitney 分层——Whitney 条件 (a)(b) 需要光滑流形的切空间/割线收敛条件

### 关键实现细节（第165-181行）：

```python
for k in range(1, max_levels + 1):
    if len(moves) < 3:
        break
    level = _build_single_level(moves, k, sustain_m, df_macd, merged_to_raw)
    if level is None:
        break
    levels.append(level)
    confirmed_trends = [t for t in level.trends if t.confirmed]
    if len(confirmed_trends) < 3:
        break
    moves = confirmed_trends
```

声明"空间 = ∪S_k，各层不相交"——但 RecursiveLevel.moves[k] 是从 level[k-1].trends 取出的，
各层之间有引用关系而非精确的不相交分解。

---

## 模块 8：a_topology.py — 转换函数框架

### 拓扑语义（195号）原文（第10-21行）

结构映射：
- 分解空间 D(X) = {所有合法分解} 是集合
- 转换函数 T: D(X) → D(X) 是 D(X) 上的自映射
- 等价关系 A ~ B ⇔ ∃T: T(A) = B = D(X) 上的轨道等价

映射的边界：
- 转换函数不是显式构造的同构——它通过运行两次管线（不同参数）隐式定义
- 不变量是候选（待验证），不是已证明的定理

### 强不变量候选（第35-42行）：

```python
@dataclass(frozen=True, slots=True)
class DecompositionFingerprint:
    # 强不变量候选（待验证——不同笔模式可能影响中枢数量和区间）
    n_centers: int
    center_zd_zg_pairs: tuple[tuple[float, float], ...]
    trend_kinds: tuple[str, ...]
```

关键问题：n_centers 是否应该是"强不变量候选"？
001号谱系指出"同一 0-cell 集可生成不同的 1-cell 集"（gauge choice），
意味着不同笔模式可能导致不同的笔数量 → 不同的线段 → 不同的中枢数量。

### 结构差异对象（第49-62行）：

```python
@dataclass(frozen=True, slots=True)
class StructuralDelta:
    stroke_count_diff: int
    segment_count_diff: int
    center_count_diff: int
    center_interval_diffs: tuple[tuple[float, float], ...]  # 每个中枢的 (ΔZD, ΔZG)
    trend_kind_mutations: tuple[tuple[str, str], ...]  # (source_kind, target_kind) 对
    level_diff: int
```

center_interval_diffs 是通过索引对齐两组中枢（取较短长度），
但当 center_count_diff ≠ 0 时，索引对齐可能没有数学意义。

---

## 请审计的具体问题

请对以下每个问题给出判定：

**Q1 (a_inclusion.py)**：等价关系 ~ 的定义"有包含关系且相邻（传递闭包后形成等价类）"——
实际实现是贪心增量合并，不显式计算传递闭包。这个标注是否精确？

**Q2 (a_fractal.py)**：双条件检测的是二维意义上的极值（high 和 low 同时严格极值），
标注称 index-1 临界点类比（局部极大值），是否存在维度错配？

**Q3 (a_stroke.py)**：CW 胶合条件 strokes[i].i1 == strokes[i+1].i0 是否被代码精确保证？
在 _extend_prev_stroke 延伸的情况下是否成立？

**Q4 (a_segment_v1.py)**："断段 = 子复形 subdivision"——subdivision 在代数拓扑中是
在边上插入新顶点的操作，而这里的"断段"是在一个笔链上确定终止点。这个类比是否精确？

**Q5 (a_center_v0.py)**：ZG/ZD 计算使用 Z走势段（s1, s3）而非三段全部交集，
但标注声明 "ZG = min(g₁, g₂), ZD = max(d₁, d₂) = 交集的精确计算"。
这是否是标注与实现的偏差？

**Q6 (a_trendtype_v0.py)**："路径同伦类型"是否是准确的拓扑概念？
实际实现是有限组合分类，是否使用"同伦"是过度类比？

**Q7 (a_recursive_engine.py)**："空间 = ∪S_k，各层不相交"——
实际实现中各层之间有引用关系（不是严格不相交）。这个描述是否准确？

**Q8 (a_topology.py)**：n_centers 作为"强不变量候选"是否合理？
从 gauge 理论角度，不同笔参数下中枢数量应该是可变的弱不变量，
而不是应当保持的强不变量。

请逐模块给出 PASS / WARN / FAIL 评级和具体数学推理。
