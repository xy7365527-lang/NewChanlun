# 持续同调（PH）理论细节总结

> 本文是 A 系统持续同调引擎（`src/newchan/a_persistence_barcode.py` + `a_divergence_topo.py`）
> 的完整理论说明。涵盖算法、级别判断、背驰判据，以及 PH 与 MACD / 缠论的边界。
>
> **存在论位置（一句话）**：缠论对象（笔→线段→中枢→走势）提供**形态学递归**；
> 持续同调在递归每一层提供**动力学度量**。PH 不是缠论的替代，是跑在缠论**旁边**的
> 独立结构监视层（pending-012 定理类结算）。
>
> **认识论等级总览**（formalization-validity-domain 规则）：
> - 所有算法本身（barcode / Wasserstein / 压制比 / ATR）：**L0**（纯算法，确定性推导，零信息增量）。
> - "persistence = 力度""dominant = 主导级别"等经验断言：真实数据 L2/L3 验证前停留 **L1**。
> - 压制比 / aufheben 决定性实验：**L2**（腾讯 700 单标的真实数据，可否证，已产生否定性结果）。

---

## 1. H0 sublevel persistence —— 精确算法

**位置**：`a_persistence_barcode.sublevel_h0_bars()`

**直觉**：1D 价格序列在路径图（相邻样本相连）上做**下水平集滤波**（sublevel set
filtration）。阈值 `t` 从低到高扫描，局部极小处诞生连通分量，分量在相遇的极大值处合并。
按 **elder rule**：诞生更早（值更小）的分量存活，年轻分量死亡。每个死亡事件的
`persistence = death − birth = 该摆动的 prominence`。这是 1D 持续同调的标准
merge-tree 算法。

### 输入
- `values: Sequence[float]` —— 价格序列（通常是 close）。
- `finite_cap: float | None` —— 全局（最久）分量的死亡值封顶。本应为 +∞；默认
  `max(values)`，使全局 bar 的 persistence = 价格全幅。

### 步骤
1. `n = len(values)`；`n==0` 返回空，`n==1` 返回单个 persistence=0 的 bar。
2. `cap = max(vals)`（或 `finite_cap`）。
3. `order = sorted(range(n), key=values[i])` —— 索引按价格升序（滤波扫描序）。
4. 并查集 `_UnionFind`，记录每个根的诞生值，路径压缩。
5. 按 `order` 逐点 `i` 加入并置 `active[i]=True`：
   - 检查相邻 `j ∈ {i−1, i+1}` 中已 active 的点，取其根。
   - **无 active 邻居** → 局部极小 → 新分量诞生（`continue`）。
   - **有邻居** → 合并自己 + 邻居根；`elder = argmin(birth)` 存活；对每个非 elder 根
     `r`：若 `birth[r] < vi`（真实特征），追加 `Bar(birth[r], vi, vi−birth[r], 0)`；
     诞生于 `vi` 的奇点 persistence=0，跳过。
6. 扫描结束后，路径图最终全连通 → 恰好剩一个 alive 根，死亡值封顶：
   `Bar(birth, cap, cap−birth, 0)`。
7. 按 persistence 降序排序。

### 输出
- `tuple[Bar, ...]`，每个 `Bar(birth, death, persistence, dimension=0)`，persistence 降序。
- `bars[0]` = 全幅 prominence（推动浪本身）；`bars[1]` = 段内最大反向摆动 prominence。

### 复杂度与性质
- **O(n log n)**，精确，无外部依赖。
- **方向不变性**（L1 已验证）：sublevel 与 superlevel 的**有限** persistence 多重集相同
  （同一组 局部极小↔局部极大 配对）；全局 bar 两个方向都 = 段全幅。
  → **上涨笔 / 下跌笔可直接在 close 上算，无需翻转符号**。
- **维度语义**：H0 ≈ 笔/趋势级力度。单调趋势 → 单一大 bar（persistence=趋势全幅）；
  震荡 → 多个小 bar。

---

## 2. H1 Takens 嵌入 + Vietoris-Rips —— 精确算法

**位置**：`a_persistence_barcode.rips_h1_bars()` + `_takens_embedding()`

**直觉**：1D 函数的 sublevel filtration 只产生 H0（区间上函数的下水平集无 1 维环）。
要得到 H1（loop）必须先 **time-delay 嵌入**把时间序列升维成相空间点云。
中枢 = 价格在区间内往返振荡 = 相空间轨迹的 **1-cycle**。loop 的 persistence 度量
振荡在相空间中的几何持续性（半径 × 规整度），≈ 中枢级力度。

> 技术记录（语法记录，非补丁）：H0 与 H1 用**两种不同 filtration**，各取所长——
> 这是严格的设计，不是 workaround。

### 输入
- `values: Sequence[float]`。
- `embedding_dim: int = 3` —— 嵌入维度 d。
- `embedding_delay: int = 1` —— 嵌入延迟 τ。
- `max_dimension: int = 1` —— ripser 算到的最高同调维度。

### 步骤
1. **Takens 嵌入**：`x(t) → [x(t), x(t−τ), ..., x(t−(d−1)τ)]`，
   `n_pts = len(values) − (d−1)·τ`。点云不足（`< embedding_dim+2`）→ 返回空。
2. `ripser(cloud, maxdim=1)["dgms"]` —— Vietoris-Rips 复形的持续同调。
3. 取 `diagrams[1]`（H1 维度）；对每个 `(birth, death)`：
   - `death == inf` → 跳过（未死亡的 essential class）。
   - 否则 `Bar(birth, death, death−birth, dimension=1)`。
4. 按 persistence 降序排序。

### 输出
- `tuple[Bar, ...]`（dimension=1），persistence 降序。点云过小 → 空。

### 性质
- **维度语义**：H1 ≈ 中枢级力度。携带振荡的几何信息（中枢的循环性），**比纯幅度多**，
  是真正独立于 ker(D) 的新信号（见 §5）。
- `barcode_from_prices(prices, maxdim>=1)` 统一入口：H0 永远算，H1 当 `maxdim>=1` 时算，
  合并后按 persistence 降序。

---

## 3. 压制比（suppression ratio）—— 定义与背驰规则

**位置**：`scripts/tencent_suppression_ratio.py`（L2 实测脚本）；底层用 `sublevel_h0_bars`。

### 定义
一段推动浪在其 close 序列上做 sublevel-set H0 持续同调，得到按 persistence 降序的 bar：
- **主 bar** = `bars[0].persistence` = 该段全幅 prominence（推动浪本身）。
- **次大 bar** = `bars[1].persistence` = 段内最大反向摆动 prominence（最大反抗/反弹）。

```
suppression_ratio = 主bar / 次大bar
```

- 比值高（→ +∞ 为纯单调，`n_bars==1` 无内部反抗）= 一路碾压，中间没有像样反抗 → **推动强**。
- 比值低 = 推动还在，但中间反弹很大，对手方在抵抗 → **推动弱**。

> **与 MACD 面积的区别**：压制比是**段内结构比值**（主推动 / 最大内部反抗），度量的是
> "对手抵抗的相对强度"；MACD 面积是**动量的时间积分**，度量"动量绝对量"。两者概念不同。

### 背驰判据
对同向相邻笔对 `A = stroke[i]`、`C = stroke[i+2]`（A 在前 C 在后）：

| 判据 | 公式 | 含义 |
|------|------|------|
| 压制比背驰 `SR_div` | `suppression_ratio(C) < suppression_ratio(A)` | 对手抵抗相对增强 |
| MACD 面积背驰 `MACD_div` | `macd_area(C) < macd_area(A)` | 动量相对减弱 |

两者概念同向：后一段更弱 → 反转预警。

### 操作意义判据（编排者给定，L2）
- 若存在 **≥1 个 case：SR 背驰但 MACD 未背驰（SR 领先）**，且该 case 后续被走势验证
  （确实反转）→ 压制比有**独立操作意义**。
- 若所有 case 中 SR 信号都不早于 MACD → 压制比只是 MACD 的**冗余投影**。

### 干净测试前提（避免伪信号）
- A、C 都脱离 MACD 预热（`r1 >= 35`，慢线 EMA(26)+信号线(9)）。
- 双侧都有实质动量（`MACD > 3.0`），否则 MACD 背驰退化为对 0 的算术比较。
- C 沿趋势方向**创新极值**（上涨创新高 / 下跌创新低），否则只是中枢震荡，无"背驰"可言。
- 反转验证窗口 `LOOKAHEAD = 25` 根 K 线：C 终点之后未沿 C 方向创新极值 → 反转确认。

---

## 4. PH 定级别的精确方法（persistence → span → 级别）

**位置**：`active_bars()` / `dominant_bar()` / `atr_noise_threshold()`。

### 级别谱：persistence 排序即级别序
PH 的核心优势：**persistence diagram 本身就是连续尺度谱**，一次性从单序列读全级别，
无需逐级切周期重算（区间套必须切图重算）。

1. **噪声阈值** `τ = multiple × ATR`（`atr_noise_threshold`）。persistence 低于 τ 的特征
   视为噪声（震荡/毛刺），不构成级别。`multiple` 越大越保守（只认大级别）。
2. **active_bars(barcode, τ)** = persistence > τ 的 bar，按 persistence 降序
   = **从主导级别到最次级别的有序级别谱**。
3. **dominant_bar(barcode, τ)** = `argmax(persistence)` = 当前主导级别。

### span → 缠论级别（经验映射，L2）
单个 bar 覆盖的 **span（原始 K 线根数）** 索引它对应的缠论级别——级别**从数据推出**，
不由预选周期决定。腾讯 700 实测（`tencent_ph_nav.py`，2026-05-22 收盘，L2）：

| 数据序列 | 主导特征 span | 对应缠论级别 |
|---------|-------------|------------|
| 日线 | span = 124 | 周线级笔 |
| 30min | span = 285 | 日线级笔 |

即：高一级别的笔，在低级别序列上表现为一个 span 很大、persistence 很高的 H0 特征。
`active_bars` 的降序列表 = 该序列上同时在运作的多级别结构（区间套的连续谱表达）。

> **认识论标注**：span→级别的具体数值映射是 **L2**（单标的单时段，可否证）；
> persistence 排序=级别序的机制是 **L1**（管线，尚无跨标的 L3 验证）。

---

## 5. PH 不能平替 MACD —— 精确原因（ker(D) + 0 轴判据）

**谱系**：pending-013（已结算，定理类，settlement=强化）；实验
`scripts/tencent_ema_reset_decisive.py`（L2，**已产生否定性结果**）。

### 命题
PH（即使加时间维度修正 `persistence × √span`）**不能在功能上扬弃（aufheben = 否定+保留+提升）
MACD**。二者互补，不可相互归约。

### 原因一：ker(D) 与 H0 的时间盲（239 号）
- **振幅力度 ∈ ker(D)**：离散化算子 D（笔→线段→中枢）系统性吸收幅度信息。
- **H0 sublevel persistence ≈ 摆动 prominence ≈ 幅度维度**——它测量的正是 D 选择性吸收的
  那个维度，**时间盲，只看端点**。
- 实证（腾讯 700 日线，`tencent_recursive_persistence.py`）：
  - 笔#33 跌 55.1 HKD，totH0=72.6，**MACD 面积 = 0.644**
  - 笔#37 跌 54.0 HKD，totH0=82.1，**MACD 面积 = 13.467**
  - 几乎相同幅度，MACD 面积差 **20 倍**——MACD 区分"缓跌 vs 急跌"（动量时间积分），
    H0 不能。**用 H0 persistence 作笔力度 = 丢掉动量-时间信息 = 降级。**

### 原因二：0 轴判据依赖全局连续 EMA（第 24 课，一级权威）
> 「这个中枢一般会把 **MACD 的黄白线（DIFF/DEA）回拉到 0 轴附近**。而 C 段对应的
> **MACD 柱子面积比 A 段对应的面积要小**，这时候就构成标准的背弛。」

缠论 MACD 背驰判据**本质依赖全局连续 EMA**：
- **0 轴 = DIFF = 0 = EMA12 = EMA26 = 价格相对于自身移动历史均衡的基准**。
- 「黄白线回拉 0 轴」是背驰**前提条件**（B 段中枢的作用）。

### 决定性实验（隔离跨笔记忆）
MACD 用 `ewm(adjust=False)` = IIR 滤波器，每笔起点 EMA 初值由笔前历史指数加权决定；
PH（H0）只看笔内序列。隔离方法：对笔内序列**独立调 `compute_macd`（EMA 从段首重置）**，
严格消除跨笔记忆，使 MACD information set 与 PH 对齐。

| 时段 | 全局 MACD R² | 段内 MACD R²（重置 EMA） | ΔR² | 段内时间指数 b |
|------|------------|----------------------|-----|--------------|
| 日线 (n=27) | 0.399 | **0.954** | +0.555 | 0.57（≈0.5，√span 成立） |
| 30 分 (n=22) | 0.654 | 0.771 | +0.117 | **−0.10**（负，速度敏感） |
| 合并 (n=49) | 0.586 | 0.860 | +0.274 | 0.31 |

**含义被翻转**：日线段内 R²=0.954 **不是**"PH 能平替 MACD"的证据——它恰恰证明
**PH 等价的是被抽掉记忆后的退化 MACD**。被抽掉的「0 轴跨段相对基准」正是缠论背驰判据
赖以成立的核心。**PH 等价的恰恰是缠论 MACD 超越掉的那个退化版本。**

### 结论：两个 PH 定义上不可约的维度
MACD 携带、PH 定义上无法表达：
1. **跨笔递归记忆 / 0 轴跨段相对基准**（日线主导，残差 55%）——H0 birth/death = 段内**绝对**
   价位（每段独立，`finite_cap=max(段)`，无移动基准），范畴上排除"移动均衡"。
2. **笔内路径几何 / 速度敏感**（30 分主导，残差 23%，b<0 与 √span 方向相反）。

反之 **PH 独有 H1 中枢 loop 几何**（相空间往返），MACD 完全无。**二者互补，互不归约。**

---

## 6. PH ↔ 缠论接口 —— 完整规则

**谱系**：pending-012（已结算，定理类，由 239 号修正）。三种互斥定位中，
(a)(b) 被原则排除，(c) 是唯一存活项（被原则强制，非选择）。

### 三层架构定位
| 层 | 主体 | 性质 | 产出 |
|----|------|------|------|
| **决策/操作层** | 缠论 | 因果、离散、可触发 | 买卖点（一类买点 = 动作） |
| **结构监视层** | PH | 非因果、连续、描述性 | 力度/距离标量（描述，不触发） |

- **PH 跑在缠论旁边（不是里面）**：连续读级别谱、稳定结构描述、Wasserstein 结构突变报警。
- **PH 不进入决策层**：persistence diagram **非因果**（特征 death 可落在未来），无法产出
  缠论买卖点的**因果离散触发**。
- **PH 不作力度附庸**：H0 ≈ 振幅 ∈ ker(D)，劣于 MACD（§5 原因一）→ 笔/趋势力度维度不用 PH。

### 接口规则（强制）
1. **力度维度**：用 MACD（`a_divergence` / `a_macd`），**不**用 H0 persistence。PH 不替换
   `a_divergence` 的 MACD 力度判据。
2. **结构维度**：用 PH（H1 中枢几何 + Wasserstein 结构重组 + bottleneck 稳定性）——
   缠论完全缺失的能力。`topo_force(barcode, dimension=1)` 推荐用 H1（信息最独立于 ker(D)）。
3. **交叉验证**：PH 的结构信号（如 Wasserstein 跳变 = 结构断裂）去**印证或质疑**缠论的离散
   判断——这是"形态学（缠论）vs 动力学（PH）"交叉验证的本来形态。
4. **反模式（禁止）**：把 PH 塞进缠论**内部**当 MACD 替代（劣化区）。

### `topo_divergence` 接口（与 MACD 面积同构）
```
force_a = topo_force(barcode_a, dimension)   # = total_persistence = W1 到对角线
force_c = topo_force(barcode_c, dimension)
ratio   = force_c / force_a
is_divergent = (force_a > noise_floor) and (force_c < force_a)
wasserstein_ac = persim.wasserstein(diagram_a, diagram_c)   # 结构重组程度，MACD 给不出
```
- `dimension=1`（默认）：中枢 loop，信息最独立于 ker(D)，是 PH 真正的独立贡献。
- `dimension=0`：与振幅力度高度相关，可能与方向性力度（∉ ker(D)）判定不一致——
  **此时方向性力度更可信**（239 号）。
- **结论翻转边界**：若 `dimension=0` 且走势近似单调（H0 退化为单一全幅 bar），
  拓扑力度 ≡ 振幅力度 ∈ ker(D)——此时 `is_divergent` **不应作为独立证据**。

### 挂载机制
`attach_barcodes(components, prices)`：为一层缠论对象批量挂 barcode（递归每层调一次）。
- 在 D **之前的原始价格**上计算（与 D 正交的并行测量通道）。
- Protocol-based overlay，**不修改** RecursiveLevelEngine 等现有引擎（immutable + Protocol）。

---

## 7. 嵌套结构与内在递归（merge tree = 缠论级别递归？）

**问题**（2026-05-26 编排者）：PH 能否从 barcode 嵌套结构**内在地**给出"次级别"关系？
若能，递归变成内在的，不需人为构造（"日线的次级别是 30 分钟"）。

**实测**：`scripts/tencent_ph_nesting_tree.py`（腾讯 700 日线 + 30 分钟各 300 根，L2，弱 L3）。

### 7.1 结构性确认：H0 barcode 是真正的区间套树

H0 sublevel persistence 的 bar 按 [birth, death] 价格区间嵌套形成 **merge tree**
（数学必然：年轻分量 merge 进年长分量 → `b_elder ≤ b_younger` 且
`d_younger = saddle < d_elder` → containment 成立）。实测进一步证明这个**价格嵌套
同时是时间嵌套**：

| 周期 | 时间嵌套违例 child[lo,hi]⊄parent[lo,hi] |
|------|------------------------------------------|
| 日线 | **0 / 78** |
| 30 分 | **0 / 67** |

→ **每个父特征的时间区间严格包含其所有子特征的时间区间**。barcode 一次计算就给出
完整的嵌套树（root=全幅趋势，逐层向下到单 bar 摆动），**无需人为预选 L0、无需逐级
切周期**。用户的核心结构判断成立：**递归在 H0 barcode 中是内在的。**

### 7.2 价格轴与时间轴在腾讯 700 上高度一致（超出 §5 的悲观预期）

实测前的理论担心（§5、239 号）：merge tree 按**价格 prominence** 嵌套，缠论按
**时间 span** 嵌套，两轴可能分裂。决定性测量：

| 周期 | corr(persistence, span) | corr(depth, span) | corr(depth, persistence) |
|------|------------------------|-------------------|--------------------------|
| 日线 | **+0.924** | −0.318 | −0.329 |
| 30 分 | **+0.990** | −0.365 | −0.384 |

- **corr(persistence, span) ≈ +0.92~+0.99**：prominence 层级 ≈ span 层级，**两轴在此
  数据上高度一致**。ker(D) 时间盲在腾讯 700 上没有撕裂层级结构——这比 §5 预期乐观。

### 7.3 但"树深度 = 级别"被否证

| 周期 | depth | n | span 均值 | span 范围 |
|------|-------|---|----------|----------|
| 日线 | 0/1/2/3/4 | 1/22/24/25/7 | 300 / 11.5 / 7.0 / 2.6 / 1.0 | 各层 [1,152]/[1,38]/[1,10]/[1,1] |

- `corr(depth, span)` 仅 **−0.32~−0.37**（弱），且各深度层的 span 范围**大幅重叠**
  （depth-1 节点 span 可以是 1 也可以是 152）。
- 根因：树**不平衡**——一个微小摆动若直接贴着趋势主干，会挂在 depth-1，尽管它 span=1。
- **结论：不能"往下走一层 = 次级别"。** 级别不是**深度**（垂直），而是
  **persistence 阈值的水平横切**——正是 §4 的 `active_bars(barcode, τ)`。
  从树中取缠论式级别，按 persistence/span 阈值**横切**，不按深度分层。

### 7.4 PH 叶节点 ≠ 缠论笔（更丰富，但不更严格）

| 周期 | 缠论新笔 span（均值/中位/范围） | PH 叶节点 span（均值/中位/范围） |
|------|------------------------------|-------------------------------|
| 日线 | 12.1 / 11 / [5,34]（n=27） | 1.7 / 1 / [1,5]（n=59） |
| 30 分 | 15.6 / 13 / [7,42]（n=22） | 1.8 / 1 / [1,7]（n=50） |

- PH 叶节点 span≈1（单 bar 摆动），远细于缠论笔（span≈12~16）。
- **PH 树是缠论笔的超集**：分辨率更细（连续谱，59 叶 vs 27 笔），但**不更严格**——
  叶层含 span-1 噪声；缠论新笔有最小跨度 + 缺口/重叠规则**过滤噪声**。
  PH 要对齐笔级，必须 τ 横切过滤（回到 §4），不能直接用叶节点。

### 7.5 定位判定：精炼 pending-012，不推翻

"从并行监视层升级为递归内在引擎"——**部分成立**。递归确实内在（7.1），价格/时间轴
一致（7.2），分辨率更丰富（7.4）。但三个理由使 PH 内在递归**仍停留结构监视层**，
不能升入缠论的因果决策层：

1. **级别 = τ 横切，非 depth**（7.3）：不是自下而上 depth 递归，仍需外部阈值定级别。
2. **H0-only / ker(D)**：H1（中枢 loop）的 birth/death 单位是相空间距离，与 H0 的价格
   单位**不可比**，无法并入同一棵树。完整嵌套树只能来自 H0，因此继承 H0 的时间盲
   （+0.92 相关性软化但未消除）。
3. **非因果（决定性）**：整棵树用**含未来的全序列**算出——每个特征的 death = **未来**
   的鞍点。树是**事后分解（hindsight decomposition）**，不是实时递归结构。缠论递归
   是**因果**的（左→右逐步确认笔/线段）。→ pending-012 排除 PH 进决策层的"非因果"
   理由**依然成立**。

**结论**：本发现**强化并精炼 pending-012**（PH=独立结构监视层），不推翻它。PH 的
内在递归是一个**因果性受限的、按 prominence 组织的事后区间套**——它给缠论的离散级别
判断提供连续、稳定、一次成形的**结构参照系**，但级别边界仍需 τ 横切给定，且不能产出
因果可触发的买卖点。**升级方向（若要进决策层）必须解决非因果性**——例如只用截至当前
的序列做在线 merge tree，但那样 death 会随未来不断改写，稳定性优势（bottleneck 定理）
随之削弱。这是一个真实的张力，留待 L3 跨标的验证。

> **认识论等级**：7.1 区间套结构 = L0（merge tree 数学必然）+ L2 验证（0 违例）；
> 7.2~7.4 相关性/分布 = L2（腾讯 700 单标的，日线+30分跨周期 = 弱 L3，可否证）；
> 7.5 定位判定 = 定理类（非因果性是 persistence diagram 的数学属性，非价值参数）。

---

## 附：结果包六要素

1. **结论**：PH 是缠论旁的独立结构监视层（H0 算笔/趋势 prominence、H1 算中枢 loop、
   Wasserstein 算结构重组、压制比算段内抵抗）；PH 不作力度附庸（H0∈ker(D)，劣于 MACD），
   不进决策层（非因果），不能扬弃 MACD（缺 0 轴跨段基准 + 笔内路径几何两个不可约维度）。
   **PH 的 H0 barcode 内在地是一棵真区间套树（§7，0 时间嵌套违例），递归无需人为构造**——
   但级别 = persistence 阈值横切（非树深度），H0-only，且事后非因果 → 强化而非推翻
   "PH=结构监视层"定位。
2. **定义依据**：第 24 课 MACD 背驰判据（0 轴 + 红绿柱面积，依赖全局连续 EMA，一级权威）；
   239 号（H0≈振幅∈ker(D)）；`a_persistence_barcode` H0 birth/death = 段内绝对价位；
   缠论买卖点 = 因果离散触发（知识库）。
3. **边界条件**：若缠论背驰**放弃 0 轴判据、改用段内重置 MACD**，日线上 PH(pers,√span) R²=0.954
   可平替（但 30 分仍差 23%），代价是放弃缠论原文判据；若未来证明 H1/diagram-距离能直接生成
   因果可触发买卖点，则 PH 可升入决策层。
4. **下游推论**：架构上 PH 模块挂缠论引擎**外**作监视器，不替换 `a_divergence` 的 MACD 力度；
   笔力度的拓扑增强走 H1/邻域几何而非 H0；PH 独立贡献走 H1 中枢几何 + 区间套监视通道；
   **§7 嵌套树可直接作"连续级别参照系"喂给缠论的离散级别判断（用 τ 横切对齐笔级），
   但不可作因果决策树**。
5. **谱系引用**：pending-012（PH=结构监视层，§7 强化）、pending-013（PH 不能扬弃 MACD）、
   239 号（ker(D)）、231 号（有效域 L0-L3）、378 号（PH 研究线）、pending-011（对齐缺口）。
6. **影响声明**：本文档为既有代码（`a_persistence_barcode.py`、`a_divergence_topo.py`）与
   谱系（pending-012/013）的理论汇总；§7 新增实测脚本 `scripts/tencent_ph_nesting_tree.py`
   （L2，merge tree 嵌套结构验证），不改 src/，不引入新概念定义。
