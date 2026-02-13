# 新缠论形式化宪法（Object Graph + Recursion + Invariants）

> 本文档是 NewChanlun 系统的**形式化公理基础**。
> 所有代码改动必须不违反本文档中的不变量。
> 审计时以本文档为准，而非以代码现状为准。

---

## 0. 语言约定

- **实体对象（Entity）**：可以被实例化、持有区间、写入账本的对象
- **完全分类（Classification）**：对实体对象的属性标记，不产生新实体
- **生成态（虚线 / Hypothesis）**：允许撤销/重画/多解
- **结算态（实线 / Settlement）**：追加不可改（append-only）

---

## 1. 实体对象图（Object Graph）

### 1.1 基底数据实体

- **Bar**：原始 OHLCV（raw）
- **MergedBar**：包含处理后的 K（merged，带 raw 映射）

### 1.2 生成层实体（Generate Layer）

- **Fractal**：顶/底分型（在 merged 上）
- **Stroke**：笔（分型序列连接）
- **FeatureBar**：特征序列元素（从笔抽取）

> 这些属于"生成态优先"，不能直接构成中枢裁决。

### 1.3 结构层实体（Structure Layer）

- **Segment**：线段（v0/v1；由笔构造，Move[0] 基元）
- **Center[k]**：第 k 层中枢（由 Move[k-1] 构造，核 [ZD,ZG] 固定）
- **Move[k]**：第 k 层"可构造对象"（由 TrendTypeInstance[k] 承担）

### 1.4 裁决层实体（Decision Layer）

- **TrendTypeInstance[k]**：第 k 层走势类型实例（盘整/趋势）
- **LStar**：当前裁决级别输出（唯一）
- **AnchorSet**：结算锚/运行锚/事件锚 + 否定条件（死亡）

---

## 2. 完全分类体系（Classification，不产生新实体）

### 2.1 分型分类

```
Fractal.kind ∈ {top, bottom}
```

### 2.2 笔分类

```
Stroke.direction ∈ {up, down}
Stroke.confirmed ∈ {true, false}
```

> 最后一笔未确认是**时间性**，不是结构第三态。

### 2.3 线段分类

```
Segment.direction ∈ {up, down}
Segment.confirmed ∈ {true, false}
```

### 2.4 中枢分类

```
Center.kind ∈ {candidate, settled}
```

- **candidate**：结构已满足"3 Move 重叠"，但尚未进入结算确认策略
- **settled**：已通过结算确认策略（sustain_m 或其他规则）

> 注意：candidate/settled 是"账本状态"，不是第三种中枢定义。

### 2.5 走势类型分类

```
TrendTypeInstance.kind ∈ {consolidation, trend}
TrendTypeInstance.direction ∈ {up, down}
```

### 2.6 买卖点分类（未实现，可预留）

```
BSP ∈ {1, 2, 3} × {buy, sell}
```

---

## 3. 递归定义（Recursion, Non-circular）

> 这是整个"级别如何确定"的数学核心。

### 3.1 Move 的归纳定义（避免循环）

```
Move[0] := Segment（confirmed）
```

对任意 k >= 1：

```
Center[k]            := overlap(Move[k-1], Move[k-1], Move[k-1])
TrendTypeInstance[k] := maximal continuous sequence of Move[k-1]
                        containing at least one Center[k]
Move[k]              := TrendTypeInstance[k]（confirmed）
```

**关键点**：

- Segment 的存在**不依赖**中枢
- 中枢是对 Move 序列施加的**判定算子**
- 走势类型实例是对包含中枢的 Move 序列的**分类实体**
- 因此**无循环论证**

### 3.2 中枢（Center）的定义（硬公理）

对连续三段 m1, m2, m3 ∈ Move[k-1]：

```
ZD = max(m1.low, m2.low, m3.low)
ZG = min(m1.high, m2.high, m3.high)
Center 成立当且仅当 ZG > ZD
```

Z走势段统计量（思维导图 #36-44）：

```
Z走势段 = 与中枢方向一致的 Move（即特征序列段）
ZG = min(g1, g2)    // 前两个 Z走势段的 high 取 min
ZD = max(d1, d2)    // 前两个 Z走势段的 low 取 max
GG = max(gn)        // 所有 Z走势段 high 的最大值
DD = min(dn)        // 所有 Z走势段 low 的最小值
G  = min(gn)        // 所有 Z走势段 high 的最小值
D  = max(dn)        // 所有 Z走势段 low 的最大值
```

中枢破坏定理：

```
中枢被破坏 ⟺ 一个次级别走势离开中枢 ∧ 其后的次级别回抽走势不重新回到 [ZD,ZG] 内
```

前后同级别中枢关系（思维导图 #32-35）：

```
后GG < 前DD        → 下跌及其延续
后DD > 前GG        → 上涨及其延续
后ZG < 前ZD ∧ 后GG >= 前DD  → 形成高级别走势中枢
后ZD > 前ZG ∧ 后DD <= 前GG  → 形成高级别走势中枢
```

### 3.3 级别 L* 的定义（裁决级别）

在所有 k 层中，找最高层的 Center[k] 满足：

```
kind == settled
Alive == true（三锚状态机判定存活）
```

该 k 即为 L*。

对应实现：自下而上构造所有层 → 自上而下扫描最高存活锚。

---

## 4. 三锚状态机（NewChan 核心）

给任意 settled Center 的核 [ZD, ZG]：

- **结算锚**：价格在核内，或当前段仍与核重叠且在中心延伸区间内
- **运行锚**：离开核后的离开段（exit）阶段
- **事件锚**：第一次回抽出现但未结算

**否定条件（死亡）**：

- 回抽后再确认创新高/新低
- 或超时否定

---

## 5. 不变量列表（Invariants, 可断言）

### I0：账本隔离

- Hypothesis ledger 可撤销
- Settlement ledger append-only

### I1：不可跳级（Non-skip）

- Center[k] 的组件必须来自 Move[k-1]
- 禁止 Move[k-2] 直接参与构造 Center[k]

### I2：笔不裁决

- Stroke / FeatureBar 不得作为 Center 组件
- 笔层重叠只能做候选热区，不得进入 Center 结算对象

### I3：对象连续覆盖（Coverage）

- Segment 序列必须覆盖 Stroke 序列（不允许丢笔）
- TrendTypeInstance[k] 必须覆盖其所引用的 Move[k-1] 序列（不允许空洞）

### I4：对象边界一致

- Segment.i0/i1 与其端点 strokes 一致
- Center.t0/t1 必须落在其 seg0/seg1 覆盖的时间区间内
- L* 指向的 center_id 必须存在且唯一

### I5：中枢核固定

- Center 的核 [ZD, ZG] 由初始三段确定
- 延伸只改变 seg1/时间窗，不改写核

### I6：单裁决级别

- 任一时刻只有一个 L*

### I7：渲染语义约束（给 B 系统）

- Center 必须以矩形（BOX）表达，不得跨中心连线
- 段/笔若用线表达，必须对象间断开（禁止叙事连线）

---

## 6. 审计输出（Audit Artifacts）

系统每次运行应输出：

- **实体对象数量**：n_fractals / n_strokes / n_segments / n_centers / n_trend_instances
- **不变量检查结果**（assertion report）
- **L***：level, center_id, regime, death_reason
- **三锚 AnchorSet**：核/离开/回抽/死亡

---

## 7. 当前审计状态（映射到代码）

| 不变量 | 代码文件 | 状态 |
|--------|----------|------|
| I0 账本隔离 | `a_assertions.py` | 占位（未强制） |
| I1 不可跳级 | `a_assertions.assert_non_skip` | 已实现 |
| I2 笔不裁决 | `a_assertions.assert_no_pen_center` | 占位 |
| **I3 连续覆盖** | `a_segment_v1.py` | **违反**（段间可能丢笔） |
| I4 边界一致 | `a_assertions.assert_segment_min_three_strokes_overlap` | 部分 |
| I5 核固定 | `a_center_v0.py` | OK（延伸不改 ZG/ZD） |
| I6 单裁决 | `a_assertions.assert_single_lstar` | 已实现 |
| I7 渲染语义 | `ChanTheoryPrimitive.ts` | 部分违反（叙事连线） |

---

## 变更记录

- v1.0 (2026-02-11)：初始版本，基于缠中说禅定理思维导图 + 股市技术理论
