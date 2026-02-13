# Chan Spec v0.2 —— 缠中说禅技术理论可实现规格（Step7）

> 目标：将《缠中说禅-股市技术理论》《缠中说禅定理》（含思维导图）的核心定义整理为
> **可被代码实现 + 可被断言校验**的规格。
>
> 本规格刻意区分三类对象：
> - 生成对象（Generate）：分型、笔（Stroke）
> - 结构对象（Structure）：线段（Segment）、次级走势段（Move）
> - 判定对象（Judge）：中枢（Center）、走势类型实例（TrendType Instance）、级别锁定（L*）
>
> 关键公理（必须保持）：**中枢必须由 ≥3 个连续"次级别走势类型（Move）"的重叠构成。**
> 笔层重叠只能做候选，不得确认中枢。

---

## 0. 术语与规范用语

- **必须 / MUST**：实现与断言必须满足。
- **应该 / SHOULD**：推荐实现，允许在参数化范围内调整。
- **可以 / MAY**：可选模块或加速器，不影响正确性。

### 0.1 时间周期（TF）与结构级别（Level）分离

- **TF（1m/5m/30m/4h/1d…）**：数据采样与显示窗口（B系统职责）。
- **Level（结构级别）**：递归构造层级（A系统职责）。
  Level 不等于 TF；TF 只是你观察 Level 的"显微镜倍数"。

---

## 1. 数据对象与记号

### 1.1 Bar（原始K线）
- 必须字段：`ts, open, high, low, close`
- 约束：`high >= max(open, close)`, `low <= min(open, close)`, `high >= low`

### 1.2 MergedBar（包含处理后的K线）
- 从 Bar 序列生成
- 必须保存映射：`merged_to_raw[i] = (raw_start, raw_end)` 用于定位与回画

---

## 2. 包含关系（Inclusion）——必须先做（MUST）

### 2.1 定义（只看 high/low）
相邻两根K线 `A, B` 若满足任一：
- `A.high >= B.high AND A.low <= B.low`（A 包含 B）
- `B.high >= A.high AND B.low <= A.low`（B 包含 A）
则称二者存在包含关系。

### 2.2 合并方向（MUST）
合并方向由**当前趋势方向 dir ∈ {UP, DOWN}**决定：
- **向上合并（UP）**：
  - `high = max(A.high, B.high)`
  - `low  = max(A.low,  B.low)`
- **向下合并（DOWN）**：
  - `high = min(A.high, B.high)`
  - `low  = min(A.low,  B.low)`

合并后 OHLC 的推荐工程处理（SHOULD）：
- `open = A.open`
- `close = B.close`
- `ts = B.ts`
- `volume = A.volume + B.volume`（若有）

### 2.3 方向判定（dir）规则（MUST：无歧义）
dir 的更新只在"相邻两根**无包含**"时发生。
给定相邻两根无包含K线 `K[i-1], K[i]`：

- 若 `K[i].high > K[i-1].high AND K[i].low > K[i-1].low` → `dir = UP`
- 若 `K[i].high < K[i-1].high AND K[i].low < K[i-1].low` → `dir = DOWN`
- 否则（高低一升一降）→ `dir` 保持不变（若仍为 None，则继续等待下一对确定）

初始化（SHOULD）：
- `dir = None`，直到遇到第一对满足上述双条件的无包含相邻K线后确定。

### 2.4 处理顺序（先左后右，MUST）
- 按时间顺序扫描，遇到包含则合并成新K线；
- 合并后的新K线必须继续与其右侧相邻K线比较（递推）；
- 完成后，输出序列中任意相邻两根不得再存在包含关系。

---

## 3. 分型（Fractal）——只在包含处理后识别（MUST）

在 MergedBar 序列上，对连续三根 `K[i-1], K[i], K[i+1]`：

### 3.1 顶分型 Top（MUST：双条件）
- `K[i].high > K[i-1].high AND K[i].high > K[i+1].high`
- **且**
- `K[i].low  > K[i-1].low  AND K[i].low  > K[i+1].low`

顶分型极值：`price = K[i].high`

### 3.2 底分型 Bottom（MUST：双条件）
- `K[i].low  < K[i-1].low  AND K[i].low  < K[i+1].low`
- **且**
- `K[i].high < K[i-1].high AND K[i].high < K[i+1].high`

底分型极值：`price = K[i].low`

### 3.3 实时性说明（SHOULD）
分型确认需要看到 `i+1`，因此实时下分型天然滞后一根K线。

---

## 4. 笔（Stroke / 笔）——生成对象（Generate），不参与中枢确认（MUST）

### 4.1 基本定义
- 向上笔：底分型 → 顶分型
- 向下笔：顶分型 → 底分型

### 4.2 分型去重（择优，MUST）
相邻同类分型只保留更极端者：
- 顶分型：保留 `high` 更高者
- 底分型：保留 `low` 更低者

并保证分型序列严格顶底交替。

### 4.3 宽笔 / 严笔（参数化，MUST）
> 你提到的"分型间取一根独立K线"属于宽笔；严笔更严格。
> 工程实现必须参数化，而不是改定义。

设分型用"中心K线索引"表示为 `idx`，一个分型窗口覆盖 `[idx-1, idx, idx+1]`。

- **宽笔（wide）**：两分型窗口不重叠且中间至少存在 1 根不属于任何窗口的K线
  等价工程条件：`idx2 - idx1 >= 4`
- **严笔（strict）**：在宽笔基础上更严格（推荐）
  等价工程条件：`idx2 - idx1 >= MIN_STRICT_SEP`（例如 5 或 6）

参数：
- `stroke_mode ∈ {wide, strict}`
- `MIN_STRICT_SEP >= 5`

### 4.4 笔的有效性（MUST）
- 向上笔：终点顶分型 `high_end` 必须 > 起点底分型 `low_start`
- 向下笔：终点底分型 `low_end` 必须 < 起点顶分型 `high_start`

### 4.5 笔的确认（实时语义，SHOULD）
- 最新一笔通常处于"延伸/未确认"状态；
- **新一笔生成**（出现反向笔的端点分型确认）后，前一笔才进入可结算态。

---

## 4.6 笔重叠候选区（你称"笔中枢"）——只做候选（MAY）

> 允许计算笔层重叠，作为"可能出现更高阶结构"的候选热区；
> 但它不具备中枢裁决资格，必须只进入 Hypothesis Ledger。

定义：对连续三笔 `stroke1, stroke2, stroke3`：
- `overlap_low  = max(stroke1.low, stroke2.low, stroke3.low)`
- `overlap_high = min(stroke1.high, stroke2.high, stroke3.high)`
若 `overlap_low < overlap_high`，则形成 **PenOverlapZone（候选区）**。

约束（MUST）：
- PenOverlapZone **不得**被写入"中枢（Center）"结算对象；
- 只能用于：提示"该区域可能生成线段/中枢"，或用于执行层的观察与成交优化。

---

## 5. 线段（Segment / 线段）——结构对象（Structure）

> 线段是"可用于构造中枢"的最小结构对象之一（本规范采用 Segment 作为 Level-0 Move 的基元）。

### 5.1 最小条件（MUST）
- 线段由连续笔构成；
- 线段至少包含 **3 笔**；
- 起始三笔必须存在**三者交集重叠**（推荐用三者交集，而不是只看1与3）。

重叠判定（MUST）：
- `overlap_low  = max(s1.low, s2.low, s3.low)`
- `overlap_high = min(s1.high, s2.high, s3.high)`
- 成立当且仅当：`overlap_low < overlap_high`

### 5.2 线段方向（SHOULD）
- 以第一笔方向为线段方向（v0）；
- v1（特征序列法）会更严格地给出方向与终结点。

### 5.3 线段终结（原文语义要求，MUST）
- 线段被破坏的充要条件：**新线段生成**。
- 实时语义：最后一段线段通常处于未确认状态，直到下一线段生成才确认。

### 5.4 线段算法分层（MUST：允许分版本）
- **v0 占位实现（必须能跑）**：用"三笔交集重叠→生成线段"的最简法构造段；
- **v1 严格实现（后续必须补）**：特征序列法
  - 向上段：取下笔序列为特征序列；向下段：取上笔序列为特征序列
  - 对特征序列做包含处理得到标准特征序列
  - 在标准特征序列中找分型（向上只看顶分型；向下只看底分型）
  - 出现该分型即判定旧线段终结，新线段生成（含缺口两类情况的分支）

---

## 6. 次级别走势类型 Move（归纳递归的核心，MUST）

> **缠论原文**（缠中说禅定理思维导图）：
>
> - **中枢** = 某级别的走势类型，被至少三个连续**次级别走势类型**所重叠的部分
> - **走势类型** = 包含中枢的实体对象（趋势 ≥2 中枢 / 盘整 = 1 中枢）
>
> 走势类型和中枢**互相定义**：
> - 中枢由次级别走势类型构成（构建依据）
> - 走势类型由中枢分类（分类依据）
>
> **不存在循环**——因为有归纳基底：线段（Segment）不依赖中枢，
> 是纯结构对象，充当 Level-0 的 Move。
> 对 k ≥ 1，Move[k] = 走势类型实例[k]，其存在依赖 Center[k]，
> 而 Center[k] 由 Move[k-1] 构成——这就是归纳递归。

### 6.1 Level-0 Move（基底，MUST）
- `Move[0] := Segment（已确认的线段）`
- 线段由笔构成（特征序列法），不依赖中枢——这是递归的基底。

### 6.2 Level-k Move（归纳步，MUST）
- 对 `k >= 1`：
  - `Move[k] := TrendTypeInstance[k]（已确认的走势类型实例——趋势对象或盘整对象）`
  - 它由 `Move[k-1]` 序列构造，包含至少一个 `Center[k]`
  - `Center[k]` 由至少三个连续 `Move[k-1]` 的重叠构成
  - 因此 `Move[k]` 的存在**依赖** `Center[k]`，而 `Center[k]` 依赖 `Move[k-1]`——层层递推

---

## 7. 中枢（Center / Zhongshu）——判定对象（Judge），核心公理（MUST）

### 7.1 定义（MUST）
**某级别中枢 Center[k]** 由 **至少三个连续的 Move[k-1]** 的重叠构成。

对连续三段 `m1, m2, m3`（它们必须是 Move[k-1]）：
- `ZD = max(m1.low, m2.low, m3.low)`   （中枢下沿）
- `ZG = min(m1.high, m2.high, m3.high)`（中枢上沿）
中枢成立当且仅当：`ZG > ZD`

**硬禁（MUST）**：
- 禁止用笔（Stroke）作为 Center 的组件；
- 禁止跨级别跳跃组件（例如 Move[k-2] 直接构成 Center[k]）。

### 7.2 中枢延伸（MUST）
对后续 Move `mn`：
- 若 `mn` 的区间 `[mn.low, mn.high]` 与 `[ZD, ZG]` 有重叠（交集非空），则中枢延伸（时间区间扩大）
- 否则中枢终止（并为更高层结构/趋势形成提供条件）

### 7.3 候选与结算（工程语义，MUST）
- **Candidate Center**：刚由三段 Move 重叠形成；
- **Settled Center**：至少满足以下其一（参数化）：
  - `sustain_moves >= M`：后续还有 M 个 Move 与其重叠（延伸得到确认）
  - 或者：该中枢相关的"离开-回抽"结构被确认（后续步骤实现）

参数：
- `CENTER_SUSTAIN_M >= 1`（建议 2）

---

## 8. 走势类型实例 TrendTypeInstance（趋势对象 / 盘整对象，MUST）

> **核心区分**（缠中说禅定理思维导图）：
> - **走势类型** = 分类体系：趋势（≥2 同向中枢）或 盘整（1 中枢）
> - **趋势** = 实体对象，是走势类型的一种具体形态
> - **盘整** = 实体对象，是走势类型的另一种具体形态
> - 确认的走势类型实例 = Move[k]，参与更高级别中枢的递归构造
>
> 走势分解定理：**任何级别的任何走势，都可以分解为同级别盘整、下跌与上涨
> 三种走势类型的连接**。因此走势类型实例必须首尾相连，完全覆盖走势。

### 8.1 定义（MUST）
在 level k 上，一个 **TrendTypeInstance[k]** 是一段连续的 `Move[k-1]` 序列，满足：
- 序列中至少包含一个 `Center[k]`；
- 该实例在当前分解规则下是"极大"的（不能再向左右扩展而不改变其中心结构）。

### 8.2 分类（MUST）
- **盘整对象（Consolidation）**：实例只包含 1 个 Center[k]（含该中枢的延伸部分）
  - `kind = "consolidation"`
- **趋势对象（Trend）**：实例包含 ≥2 个 Center[k] 且"同向"
  - `kind = "trend"`
  - 上涨趋势：中心区间整体抬升：`ZG2 > ZG1 AND ZD2 > ZD1`
  - 下跌趋势：中心区间整体下移：`ZG2 < ZG1 AND ZD2 < ZD1`

**硬禁**：
- 不得出现 `"trend_leg"` 等非缠论概念。两个中枢之间的连接是次级别走势类型，
  归属于包含这些中枢的趋势对象内部，不单独命名。

### 8.3 连续性（MUST）
- 走势类型实例之间首尾相连：`instances[i].seg1 == instances[i+1].seg0`
- 第一个实例从 seg 0 开始，最后一个实例到最后一段结束

### 8.4 实时性（SHOULD）
- 最右侧 TrendTypeInstance 通常处于未确认态；
- 新的 TrendTypeInstance 生成（或更高层中心确认）后，前一实例才进入结算态。

---

## 9. 级别 Level（结构级别）与 L* 锁定（MUST）

### 9.1 级别的归纳构建（MUST）
- 已有 `Move[0]`（Segments） → 可构建 `Center[1]` → 可构建 `TrendTypeInstance[1]=Move[1]`
- 递归：`Move[k-1]` → `Center[k]` → `TrendTypeInstance[k]=Move[k]`

### 9.2 当前裁决级别 L*（MUST：唯一）
定义当前裁决级别 `L*` 为：在当前窗口中，存在 **Active Settled Center** 的最高 k（或按你系统配置选择的"最有约束力级别"）。

Active 的最小工程判定（SHOULD）：
- Center 为 settled；
- 当前价格仍处于：
  - 中枢内，或
  - 离开后的第一回抽判定区
- 若中枢距离当前过远（超过 MAX_AGE），则视为 inactive。

### 9.3 硬约束（MUST）
- 任一时刻只允许一个 `L*` 输出（唯一裁决级别）
- 低级别对象不得推翻已锁定的 L*（只能做执行层细化）

---

## 10. 账本（Ledger）与对象权限（MUST）

### 10.1 双账本
- **Hypothesis Ledger**：候选/未确认对象（候选中枢、笔重叠区、未确认最后一笔/段）
- **Settlement Ledger**：结算对象（确认线段、settled 中枢、确认走势类型实例、L*）

### 10.2 权限表（MUST）
- Stroke / PenOverlapZone：只能进入 Hypothesis，不得用于中枢确认与级别锁定
- Segment（confirmed）：可作为 Move[0]，可参与 Center 构造
- Center（settled）：可用于 L* 锁定与走势类型分类
- TrendTypeInstance（confirmed）：可作为 Move[k] 参与更高层中心构造

---

## 11. 不变量与断言清单（用于 a_assertions.py）

| ID | 断言函数 | 覆盖规格 | 说明 |
|---|---|---|---|
| A1 | assert_inclusion_no_residual | §2 | 合并后无相邻包含 |
| A2 | assert_inclusion_direction_rule | §2.3 | dir 更新规则正确、无歧义 |
| A3 | assert_fractal_definition | §3 | 分型双条件严格成立 |
| A4 | assert_stroke_alternation_and_gap | §4 | 顶底交替 + gap 满足 wide/strict 参数 |
| A5 | assert_no_pen_center | §4.6/§7.1 | 笔/笔重叠区不得成为中枢组件 |
| A6 | assert_segment_min_three_strokes_overlap | §5.1 | 线段>=3笔且三笔交集重叠非空 |
| A7 | assert_center_definition | §7.1 | Center 由3个连续 Move 重叠且 ZG>ZD |
| A8 | assert_non_skip | §7.1/§6 | Center[k] 只能由 Move[k-1] 构成 |
| A9 | assert_single_lstar | §9.2 | 任一时刻仅一个 L* |
| A10 | assert_ledger_separation | §10 | 候选不得写入结算、结算不可回写 |

---

## 12. 参数默认值（建议）

- stroke_mode = wide
- MIN_STRICT_SEP = 5
- CENTER_SUSTAIN_M = 2
- MAX_AGE（按你数据频率设置，例如 500 根 merged bars）
