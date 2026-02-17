# MaiMai Rules v1 — 买卖点识别规范（草案）

> 本规范基于 maimai.md v0.7（已结算）。原 5 个 TBD 问题已在定义中全部结算。

---

## 1. 缠论定义

> 概念溯源标签说明：
> - `[旧缠论]`：缠师原始博文定义
> - `[新缠论]`：本项目扩展/精化
> - `[旧缠论:选择]`：原文有多种理解空间，本项目做出的选择

| 概念 | 定义 | 溯源 |
|------|------|------|
| **第一类买点 (1B)** | 某级别**下跌趋势**中，一个次级别走势类型向下跌破**最后一个中枢**后形成的**趋势背驰点** | `[旧缠论]` 第17课、第21课、编纂版§4 |
| **第一类卖点 (1S)** | 某级别**上涨趋势**中，一个次级别走势类型向上突破**最后一个中枢**后形成的**趋势背驰点** | `[旧缠论]` 第17课 |
| **第二类买点 (2B)** | 第一类买点出现后，次级别上涨结束后**再次下跌**的那个次级别走势的**结束点** | `[旧缠论]` 第17课、第21课 |
| **第二类卖点 (2S)** | 第一类卖点出现后，次级别下跌结束后**再次上涨**的那个次级别走势的**结束点** | `[旧缠论]` 第17课 |
| **第三类买点 (3B)** | 次级别走势向上**离开**中枢 → 次级别走势**回试** → 低点**不跌破 ZG** | `[旧缠论]` 第20课 |
| **第三类卖点 (3S)** | 次级别走势向下**离开**中枢 → 次级别走势**回抽** → 高点**不升破 ZD** | `[旧缠论]` 第20课 |
| **前提：下跌趋势** | `[TBD-1]` Move(kind="trend", direction="down") — 至少 2 个依次向下中枢 | `[旧缠论:选择]` 严格口径 |
| **前提：上涨趋势** | Move(kind="trend", direction="up") — 至少 2 个依次向上中枢 | `[旧缠论]` |
| **背驰** | Divergence(kind="trend") — 趋势最后一段力度 < 倒数第二段力度 | `[旧缠论]` 第24课 |
| **盘整背驰** | Divergence(kind="consolidation") — 中枢同向离开段力度衰竭 | `[旧缠论]` 第24课 |

### 关键定理引用

| 定理 | 内容 | 出处 |
|------|------|------|
| **买卖点定律一** | 任何级别的第二类买卖点都由次级别相应走势的第一类买点构成 | `[旧缠论]` 第17课 |
| **趋势转折定律** | 任何级别的上涨转折都由某级别的第一类卖点构成；下跌转折由某级别的第一类买点构成 | `[旧缠论]` 第17课 |
| **买卖点完备性定理** | 市场必然产生赢利的买卖点，只有第一、二、三类 | `[旧缠论]` 第21课 |
| **背驰-买卖点定理** | 任一背驰都必然制造某级别买卖点；任一级别的买卖点都必然源自某级别走势的背驰 | `[旧缠论]` 第24课 |

### 买卖点互斥/重合约束

| 买点对 | 可否重合 | 说明 | 溯源 |
|--------|---------|------|------|
| 1B 与 2B | **不可能** | 时间上前后出现 | `[旧缠论]` 第21课 |
| 1B 与 3B | **不可能** | 1B 在中枢下方，3B 在中枢上方 | `[旧缠论]` 第21课 |
| 2B 与 3B | **可以重合** | V 型反转：1B 后凌厉上破最后中枢 + 回抽不触及该中枢 | `[旧缠论]` 第21课 |

卖点的互斥/重合关系完全对称。

### 走势状态 → 可能的买卖点类型

| 走势状态 | 可能的买点 | 可能的卖点 | 溯源 |
|----------|-----------|-----------|------|
| 下跌趋势进行中 | 无 | — | `[旧缠论]` |
| 下跌趋势背驰 | 1B | — | `[旧缠论]` 第17课 |
| 1B 后第一次回调 | 2B | — | `[旧缠论]` 第21课 |
| 上涨趋势确立后 | 仅 3B | — | `[旧缠论]` 第21课 |
| 上涨趋势进行中 | — | 无 | 对称 |
| 上涨趋势背驰 | — | 1S | 对称 |
| 1S 后第一次反弹 | — | 2S | 对称 |
| 下跌趋势确立后 | — | 仅 3S | 对称 |

---

## 2. 数据类

```python
@dataclass(frozen=True, slots=True)
class BuySellPoint:
    """买卖点实例。

    Attributes
    ----------
    kind : Literal["type1", "type2", "type3"]
        买卖点类型：第一、二、三类。
    side : Literal["buy", "sell"]
        方向：买点 / 卖点。
    level_id : int
        所属递归级别（级别 = 递归层级，非时间周期）。

    # --- 身份键 ---
    seg_idx : int
        触发买卖点的线段索引（在 confirmed segments 中）。
        对于 type1：背驰段的终段索引。
        对于 type2：回调/反弹段的终段索引。
        对于 type3：回试/回抽段的终段索引。

    # --- 关联字段 ---
    move_seg_start : int
        关联走势类型实例的 seg_start（Move 身份键）。
    divergence_key : tuple[int, int, int] | None
        关联背驰的键 = (center_idx, seg_c_start, seg_c_end)。
        type1 必有；type2 继承自 type1；type3 为 None。
    center_zd : float
        关联中枢的 ZD（下沿）。type3 用于判定回试/回抽是否突破。
    center_zg : float
        关联中枢的 ZG（上沿）。type3 用于判定。
    center_seg_start : int
        关联中枢的 seg_start（Zhongshu 身份键）。

    # --- 状态字段 ---
    price : float
        买卖点对应的价格（分型极值价）。
    bar_idx : int
        买卖点对应的 bar 索引（前端定位用）。
    confirmed : bool
        是否已确认。[TBD-3] 确认时机定义待结算。
    settled : bool
        是否已结算（后续走势已验证买卖点有效性）。

    # --- 可选：2B+3B 重合标记 ---
    overlaps_with : Literal["type2", "type3"] | None
        如果此买卖点与另一类型重合，记录重合类型。
        仅 2B+3B 重合时有值。
    """

    kind: Literal["type1", "type2", "type3"]
    side: Literal["buy", "sell"]
    level_id: int

    # 身份键
    seg_idx: int

    # 关联字段
    move_seg_start: int
    divergence_key: tuple[int, int, int] | None
    center_zd: float
    center_zg: float
    center_seg_start: int

    # 状态字段
    price: float
    bar_idx: int
    confirmed: bool
    settled: bool

    # 可选
    overlaps_with: Literal["type2", "type3"] | None = None
```

---

## 3. 身份/状态分离

| 字段 | 归属 | 说明 |
|------|------|------|
| `seg_idx` | **身份** | 触发买卖点的段索引，生命周期内不变 |
| `kind` | **身份** | 买卖点类型，一旦确定不变 |
| `side` | **身份** | 买/卖方向，一旦确定不变 |
| `level_id` | **身份** | 递归级别，一旦确定不变 |
| `price` | 状态 | 可能因段延伸而微调（分型价更新） |
| `confirmed` | 状态 | False → True（确认后不可回退） |
| `settled` | 状态 | False → True（后续走势验证后） |
| `overlaps_with` | 状态 | 可能在后续识别中发现重合 |

**身份键**：`(seg_idx, kind, side, level_id)` — 四元组唯一确定一个买卖点。

**设计理由** `[新缠论]`：与 Move 的单字段身份键不同，买卖点需要多字段身份键，因为同一个 seg_idx 在不同级别或不同类型下可能对应不同的买卖点（如 2B+3B 重合场景下，同一个段可同时是两类买卖点）。

---

## 4. 纯函数算法

### 4.1 主函数签名

```python
def buysellpoints_from_level(
    segments: list[Segment],       # 已确认线段
    zhongshus: list[Zhongshu],     # 中枢列表
    moves: list[Move],             # 走势类型实例
    divergences: list[Divergence], # 背驰列表
    level_id: int,                 # 递归级别
) -> list[BuySellPoint]:
    """从某一递归层级的走势结构中识别所有买卖点。

    纯函数：无副作用，每次全量计算。增量通过 diff 层实现。
    """
```

### 4.2 第一类买卖点识别 — `_detect_type1()`

```
输入：moves, divergences, zhongshus, segments, level_id
输出：list[BuySellPoint]

步骤：
1. 遍历 divergences，筛选 kind="trend" 的背驰
2. 对每个趋势背驰 div：
   a. 找到关联的 Move（通过 center_idx → Zhongshu → Move 反查）
   b. 验证 Move.kind == "trend"  [TBD-1: 是否必须是趋势？]
   c. 确定 side：
      - div.direction == "bottom" → side = "buy"（下跌力竭 → 买点）
      - div.direction == "top" → side = "sell"（上涨力竭 → 卖点）
   d. 确定 seg_idx = div.seg_c_end（背驰段的终段）
   e. 确定 price = 背驰段终点的分型极值价
   f. 确定 confirmed = div.confirmed  [TBD-3: 确认时机]
   g. 构造 BuySellPoint(kind="type1", ...)
3. 返回所有第一类买卖点
```

**`[TBD-1]` 下跌确立条件** [v0.7 RESOLVED]：当前设计采用严格口径（Move.kind == "trend"），即必须有 ≥2 个同向中枢才算趋势确立。原文线索指向"走势转化"而非仅"趋势确立"，宽松口径下盘整后的下跌背驰也可能算某种形式的 1B。

**`[TBD-2]` 走势完成映射** [v0.7 RESOLVED]：背驰点出现时走势是否已"完成"？当前设计将 `confirmed` 跟随 `Divergence.confirmed`，而 `Divergence.confirmed` 跟随 `TrendTypeInstance.confirmed`。走势完成的精确定义待 zoushi.md 结算。

**`[TBD-4]` 盘整背驰与买卖点** [v0.7 RESOLVED]：当前设计中 `_detect_type1()` 只处理 `Divergence(kind="trend")`。盘整背驰（`kind="consolidation"`）不产生标准三类买卖点中的任何一类，对应的是中枢震荡操作，不纳入本规范。如果后续需要支持，可扩展一个独立的 `_detect_consolidation_signal()` 函数，但其产出不是 `BuySellPoint`。

### 4.3 第二类买卖点识别 — `_detect_type2()`

```
输入：type1_points (已识别的第一类买卖点), segments, moves, level_id
输出：list[BuySellPoint]

步骤：
1. 遍历 type1_points
2. 对每个 type1 买点 (side="buy", seg_idx=S1)：
   a. 在 S1 之后的段序列中，找到第一个向上段（次级别上涨）
   b. 找到该向上段之后的第一个向下段（次级别回调）
   c. 该回调段的结束点 = 第二类买点
   d. seg_idx = 回调段的终段索引
   e. price = 回调段终点的分型低点价
   f. confirmed：[TBD-3] 需要回调段完成后才能确认
   g. 继承 type1 的 divergence_key 和 center 信息
3. 对称处理卖点
4. 检查 2B+3B 重合条件（见 4.5 节）
5. 返回所有第二类买卖点
```

**注意**：第二类买卖点本质上是次级别的第一类买卖点（买卖点定律一）。但在当前递归层级的识别中，我们直接通过段序列定位，不进行次级别递归。这是一个 `[旧缠论:选择]`——选择在本级别的段粒度上近似识别，而非严格下钻到次级别。严格的次级别识别需要完整的多级别递归引擎支持。

### 4.4 第三类买卖点识别 — `_detect_type3()`

```
输入：zhongshus (已 settled), segments, level_id
输出：list[BuySellPoint]

步骤：
1. 遍历已 settled 的中枢 zs（zs.settled == True, zs.break_direction != ""）
2. 对每个中枢 zs：
   a. 确定离开段：break_seg（zs.break_seg 索引对应的线段）
   b. 确定离开方向：zs.break_direction
   c. 在离开段之后，找到第一个反方向段（回试/回抽段）
      - 如果 break_direction == "up"：找第一个向下段
      - 如果 break_direction == "down"：找第一个向上段
   d. 如果找不到反方向段 → 跳过（回试尚未发生）
   e. 第三类买点判定（break_direction == "up"）：
      - 回试段的 low > zs.zg  →  3B 成立
      - 回试段的 low ≤ zs.zg  →  3B 不成立（跌破 ZG）
   f. 第三类卖点判定（break_direction == "down"）：
      - 回抽段的 high < zs.zd  →  3S 成立
      - 回抽段的 high ≥ zs.zd  →  3S 不成立（升破 ZD）
   g. "第一次"约束：只处理每个中枢的**第一次**离开后的回试
      [TBD-5: 中枢范围] 这里 ZG/ZD 使用中枢初始三段确定的固定区间
   h. seg_idx = 回试/回抽段的索引
   i. price = 回试段终点的分型价（买点取低点，卖点取高点）
   j. confirmed：[TBD-3] 回试段完成后确认
3. 返回所有第三类买卖点
```

**`[TBD-5]` 中枢范围** [v0.7 RESOLVED]：当前设计使用 Zhongshu 数据类中的 `zg`/`zd`（初始三段确定，延伸不改变）。这与 zhongshu_rules_v1.md 的"固定区间策略 (D2)"一致：`[旧缠论:选择]`。

**`[TBD-3]` 确认时机** [v0.7 RESOLVED]：第三类买卖点的确认需要回试段"完成"。当前段引擎中，段的"完成"通过 `SegmentSettleV1` 事件确认。但回试段本身可能尚未结算。设计选择：`confirmed = True` 当且仅当回试段已被下一个段的出现所结算。

### 4.5 2B+3B 重合检测 — `_detect_overlap()`

```
输入：type2_points, type3_points
输出：更新后的 type2_points, type3_points（设置 overlaps_with 字段）

条件（第21课）：
  1B 出现后，次级别走势凌厉地直接上破前面下跌的最后一个中枢
  → 回抽不触及该中枢
  → 2B 与 3B 在同一个 seg_idx 上重合

判定：
  对每个 type2 买点 t2 和每个 type3 买点 t3：
    如果 t2.seg_idx == t3.seg_idx 且 t2.level_id == t3.level_id：
      → t2.overlaps_with = "type3"
      → t3.overlaps_with = "type2"
```

### 4.6 总装函数

```python
def buysellpoints_from_level(...) -> list[BuySellPoint]:
    type1 = _detect_type1(moves, divergences, zhongshus, segments, level_id)
    type2 = _detect_type2(type1, segments, moves, level_id)
    type3 = _detect_type3(zhongshus, segments, level_id)
    _detect_overlap(type2, type3)  # 标记 2B+3B 重合
    return sorted(type1 + type2 + type3, key=lambda bp: bp.seg_idx)
```

---

## 5. Diff 算法 — `diff_buysellpoints()`

与 `diff_moves`、`diff_zhongshu` 同构：

```
输入：prev: list[BuySellPoint], curr: list[BuySellPoint]
输出：list[DomainEvent]

步骤：
1. 找公共前缀（_bsp_equal 严格比较身份键 + 状态字段）
2. prev 后缀：
   - 同身份键在 curr 中存在 → 跳过（不 invalidate，走更新逻辑）
   - 同身份键在 curr 中不存在 → BuySellPointInvalidateV1
3. curr 后缀：
   - 全新（身份键在 prev 中不存在）：
     - confirmed=True → Candidate + Confirm（保证 I24）
     - confirmed=False → Candidate
   - 同身份键 + confirmed 升级 (F→T) → Confirm
   - 同身份键 + settled 升级 (F→T) → Settle
   - 同身份键 + price/overlaps_with 变化 → Candidate（更新）
```

---

## 6. 事件类型

| 事件 | event_type | 触发条件 |
|------|-----------|---------|
| BuySellPointCandidateV1 | `bsp_candidate` | 新买卖点出现 / 状态更新 |
| BuySellPointConfirmV1 | `bsp_confirm` | confirmed: False → True |
| BuySellPointSettleV1 | `bsp_settle` | settled: False → True（后续走势验证） |
| BuySellPointInvalidateV1 | `bsp_invalidate` | 买卖点消失（前提条件被否定） |

### 事件字段

```python
# 共有字段（继承 DomainEvent）
event_type, bar_idx, bar_ts, seq, event_id, schema_version

# BuySellPointCandidateV1 / ConfirmV1 / SettleV1 专有字段
bsp_id: int           # 买卖点 ID（基于身份键的 hash 或序号）
kind: str             # "type1" / "type2" / "type3"
side: str             # "buy" / "sell"
level_id: int         # 递归级别
seg_idx: int          # 触发段索引
price: float          # 买卖点价格
move_seg_start: int   # 关联 Move 身份键
center_seg_start: int # 关联中枢身份键
overlaps_with: str | None  # 重合标记

# BuySellPointInvalidateV1 专有字段
bsp_id: int
kind: str
side: str
level_id: int
seg_idx: int
```

---

## 7. 同身份状态变化事件矩阵

| prev 状态 | curr 状态 | 事件 |
|-----------|-----------|------|
| unconfirmed, unsettled | unconfirmed, unsettled | 无（公共前缀） |
| unconfirmed, unsettled | **confirmed**, unsettled | BuySellPointConfirm |
| confirmed, unsettled | confirmed, unsettled | 无（公共前缀） |
| confirmed, unsettled | confirmed, **settled** | BuySellPointSettle |
| unconfirmed, price=P1 | unconfirmed, price=**P2** | BuySellPointCandidate（价格更新） |
| 任意 | （身份消失） | BuySellPointInvalidate |

### 否定传播

买卖点的否定可由以下上游事件触发：

| 上游事件 | 否定效果 | 说明 |
|---------|---------|------|
| MoveInvalidateV1 | type1/type2 消失 | 走势类型被否定 → 背驰判定失效 → 1B/1S 消失 → 2B/2S 连带消失 |
| ZhongshuInvalidateV1 | type3 消失 | 中枢被否定 → ZG/ZD 失效 → 3B/3S 消失 |
| SegmentInvalidateV1 | 所有类型可能消失 | 触发段被否定 → 买卖点消失 |
| Divergence 消失 | type1/type2 消失 | 背驰判定翻转 → 1B/1S 消失 |

否定通过 `diff_buysellpoints()` 自动传播：当上游否定导致 `buysellpoints_from_level()` 的输出改变时，diff 自动产生 `BuySellPointInvalidateV1`。

---

## 8. 不变量

| Code | 名称 | 检查方式 |
|------|------|---------|
| I23 | BSP_TYPE_CONSTRAINT | type1 必须关联 Divergence(kind="trend")；type3 必须关联 settled Zhongshu（checker 实时） |
| I24 | BSP_CANDIDATE_BEFORE_CONFIRM | Confirm 前必有同 bsp_id 的 Candidate（checker 实时） |
| I25 | BSP_CONFIRM_BEFORE_SETTLE | Settle 前必有同 bsp_id 的 Confirm（checker 实时） |
| I26 | BSP_MUTUAL_EXCLUSION | 同级别同身份键不可同时存在 1B+2B 或 1B+3B（checker 实时） |
| I27 | BSP_INVALIDATE_TERMINAL | invalidate 后同身份不复活（checker 实时） |
| I28 | BSP_REPLAY_DETERMINISM | 同输入 → 同 event_id（测试覆盖） |
| I29 | BSP_PRICE_MONOTONE | `[TBD-3]` type1 买点 price ≤ 关联中枢 ZD；type3 买点 price > 关联中枢 ZG（checker 实时，待确认时机结算后启用） |

### I23 详细说明

```
BSP_TYPE_CONSTRAINT:
  - type1 的 divergence_key 不可为 None
  - type1 的关联 Divergence.kind == "trend"
  - type2 的 divergence_key 不可为 None（继承自 type1）
  - type3 的 divergence_key 必须为 None
  - type3 的 center_seg_start 必须对应一个 settled Zhongshu

违反时：InvariantViolation(code=I23_BSP_TYPE_CONSTRAINT)
```

### I26 详细说明

```
BSP_MUTUAL_EXCLUSION（源自第21课）:
  - 同 level_id 下，不存在同时活跃的 (1B, seg_idx=S) 和 (2B, seg_idx=S)
  - 同 level_id 下，不存在同时活跃的 (1B, seg_idx=S) 和 (3B, seg_idx=S)
  - 允许同时活跃的 (2B, seg_idx=S) 和 (3B, seg_idx=S)（2B+3B 重合）

违反时：InvariantViolation(code=I26_BSP_MUTUAL_EXCLUSION)
```

---

## 9. TBD 汇总（maimai.md 未结算问题映射）

| TBD | maimai.md 问题 | 影响范围 | 当前临时决策 | 翻转条件 |
|-----|---------------|---------|------------|---------|
| `[TBD-1]` | #1 下跌确立条件 | `_detect_type1()` 前提判定 | 严格口径：Move.kind == "trend" | 若采纳宽松口径（盘整后下跌也算），需扩展 type1 识别逻辑，允许 Move.kind == "consolidation" 的背驰也产生 1B  [v0.7 RESOLVED] |
| `[TBD-2]` | #2 走势完成映射 | `confirmed` 语义 | 跟随 Divergence.confirmed | 若走势完成与背驰不等价，需引入独立的 `move_completed` 状态  [v0.7 RESOLVED] |
| `[TBD-3]` | #3 确认时机 | 所有类型的 `confirmed` 字段 | type1: 跟随 Divergence.confirmed；type2/type3: 跟随回调/回试段结算 | 若原文要求更严格的确认（如次级别走势类型完成），需修改确认逻辑  [v0.7 RESOLVED] |
| `[TBD-4]` | #4 盘整背驰与买卖点 | 是否扩展 BuySellPoint 覆盖 | 不纳入：盘整背驰 ≠ 三类买卖点中的任何一类 | 若决定纳入中枢震荡操作信号，需新增信号类型（非 BuySellPoint）  [v0.7 RESOLVED] |
| `[TBD-5]` | #5 第三类买卖点的中枢范围 | `_detect_type3()` 中 ZG/ZD 取值 | 使用 Zhongshu.zg/zd（初始三段固定区间） | 若改用波动区间 GG/DD 或延伸后的区间，type3 判定标准改变  [v0.7 RESOLVED] |

---

## 10. 数据流

```
BiEngine → BiEngineSnapshot
  → SegmentEngine → SegmentSnapshot
    → ZhongshuEngine → ZhongshuSnapshot
      → MoveEngine → MoveSnapshot
        → DivergenceDetector → list[Divergence]
          → BuySellPointDetector → BuySellPointSnapshot
            → snap.events += bsp_events
              → EventBus → gateway → WS
```

### 引擎集成设计

```python
class BuySellPointDetector:
    """事件驱动买卖点检测器 — 消费 MoveSnapshot + DivergenceList。

    Usage:
        detector = BuySellPointDetector(level_id=1)
        for bar in bars:
            move_snap = move_engine.process(zs_snap)
            divs = divergences_from_level(...)
            bsp_snap = detector.process(move_snap, divs, zs_snap)
            for event in bsp_snap.events:
                handle(event)
    """

    def __init__(self, level_id: int) -> None:
        self._prev_bsps: list[BuySellPoint] = []
        self._level_id = level_id

    def process(
        self,
        move_snap: MoveSnapshot,
        divergences: list[Divergence],
        zs_snap: ZhongshuSnapshot,
    ) -> BuySellPointSnapshot:
        curr_bsps = buysellpoints_from_level(
            segments=...,        # 从 zs_snap 上游获取
            zhongshus=zs_snap.zhongshus,
            moves=move_snap.moves,
            divergences=divergences,
            level_id=self._level_id,
        )
        events = diff_buysellpoints(self._prev_bsps, curr_bsps, ...)
        self._prev_bsps = curr_bsps
        return BuySellPointSnapshot(
            bar_idx=move_snap.bar_idx,
            bar_ts=move_snap.bar_ts,
            buysellpoints=curr_bsps,
            events=events,
        )
```

---

## 11. 测试覆盖

### 测试文件规划

| 文件 | 覆盖内容 | 数量（估） |
|------|---------|-----------|
| `test_bsp_type1_golden.py` | 第一类买卖点 golden 测试（纯函数） | ~8 |
| `test_bsp_type2_golden.py` | 第二类买卖点 golden 测试 | ~6 |
| `test_bsp_type3_golden.py` | 第三类买卖点 golden 测试 | ~8 |
| `test_bsp_overlap_golden.py` | 2B+3B 重合 golden 测试 | ~3 |
| `test_bsp_diff.py` | diff 算法测试（各种状态变化） | ~10 |
| `test_bsp_invariants.py` | I23-I29 不变量正/反例 | ~14 |
| `test_bsp_determinism.py` | I28 确定性测试 | ~3 |
| `test_bsp_negation.py` | 否定传播测试（上游事件 → BSP invalidate） | ~6 |

### 关键 Golden Case 场景

#### 11.1 type1 买点 — 标准下跌趋势背驰 ✓

```
构造：2 个向下中枢 → 最后一段力度 < 前一段 → Divergence(kind="trend", direction="bottom")
期望：BuySellPoint(kind="type1", side="buy", confirmed=True)
```

#### 11.2 type1 — 盘整中无趋势背驰 ✗

```
构造：1 个中枢 → Divergence(kind="consolidation")
期望：不产生 type1 买卖点（盘整背驰不是第一类买卖点）
```

#### 11.3 type2 — 标准第二类买点 ✓

```
构造：type1 买点已识别 → 后续出现向上段 → 再出现向下段
期望：BuySellPoint(kind="type2", side="buy", seg_idx=回调段索引)
```

#### 11.4 type3 — 标准向上突破后回试不跌破 ZG ✓

```
构造：settled 中枢(ZG=100) → break_direction="up" → 回试段 low=101 > 100
期望：BuySellPoint(kind="type3", side="buy")
```

#### 11.5 type3 — 回试跌破 ZG ✗

```
构造：settled 中枢(ZG=100) → break_direction="up" → 回试段 low=99 ≤ 100
期望：不产生 type3 买点
```

#### 11.6 type3 — "仅第一次"约束 ✗

```
构造：中枢已经过一次离开+回试 → 第二次离开+回试
期望：不产生 type3 买点（非第一次）
```

#### 11.7 2B+3B 重合 ✓

```
构造：type1 买点 → 凌厉上破最后中枢 → 回抽不触及中枢
期望：type2 和 type3 同时出现，overlaps_with 互相标记
```

#### 11.8 上涨中无 1B/2B ✗

```
构造：已确立上涨趋势（Move.kind="trend", direction="up"）
期望：仅可能出现 type3 买点，不出现 type1/type2
```

---

## 12. 前端 markers

| 事件 | 形状 | 颜色 | 文本 | 位置 |
|------|------|------|------|------|
| bsp_candidate (type1 buy) | triangle-up | #4caf50 (绿) | 1B? | bar_idx 下方 |
| bsp_candidate (type1 sell) | triangle-down | #f44336 (红) | 1S? | bar_idx 上方 |
| bsp_candidate (type2 buy) | triangle-up | #81c784 (浅绿) | 2B? | bar_idx 下方 |
| bsp_candidate (type2 sell) | triangle-down | #e57373 (浅红) | 2S? | bar_idx 上方 |
| bsp_candidate (type3 buy) | triangle-up | #2196f3 (蓝) | 3B? | bar_idx 下方 |
| bsp_candidate (type3 sell) | triangle-down | #ff9800 (橙) | 3S? | bar_idx 上方 |
| bsp_confirm (buy) | 对应形状 | 同上加深 | NB✓ (N=1/2/3) | 同上 |
| bsp_confirm (sell) | 对应形状 | 同上加深 | NS✓ | 同上 |
| bsp_settle | circle | 金色 #ffd700 | NB/NS✓✓ | 同上 |
| bsp_invalidate | — | — | 不显示（移除 marker） | — |

### 重合标记

当 2B+3B 重合时，显示合并 marker：
- 形状：diamond（菱形）
- 颜色：#9c27b0 (紫)
- 文本：2B+3B? / 2S+3S?

---

## 13. 红线保护

以下文件/类**零修改**：

- `events.py::DomainEvent` 基类（R5 红线）
- `fingerprint.py` 现有函数
- `bi_engine.py` / `bi_differ.py`
- `orchestrator/bus.py` EventBus
- `gateway.py::_event_to_ws`（asdict+_exclude 模式自动适配）
- `a_divergence.py` 核心逻辑（只新增适配层，不修改）
- `a_move_v1.py` / `a_zhongshu_v1.py`（只读取，不修改）

### v0/v1 管线分裂注意

> **当前状态**：`a_divergence.py` 导入 v0 的 `TrendTypeInstance`（`a_trendtype_v0.py`），
> 其 `Divergence` 对象的索引字段（`seg_a_start/end`, `seg_c_start/end`, `center_idx`）
> 引用的是 v0 `TrendTypeInstance` 和 `Center` 列表，与 v1 的 `Move`/`Zhongshu` 索引**不兼容**。
>
> **影响**：买卖点模块不能直接混用 v1 Move + v0 Divergence 索引。
>
> **临时方案**：在适配层（`bsp_detector.py`）中做 v0→v1 索引映射，或在 v1 管线中
> 新建 `divergences_from_moves()` 纯函数直接消费 `Move`/`Zhongshu`。
>
> **长期方案**：统一为 v1 管线，创建 `a_divergence_v1.py`。

### 新增文件规划

| 文件 | 职责 |
|------|------|
| `a_buysellpoint_v1.py` | BuySellPoint 数据类 + 纯函数算法 |
| `core/recursion/bsp_detector.py` | BuySellPointDetector 引擎 |
| `core/recursion/bsp_state.py` | BuySellPointSnapshot + diff_buysellpoints() |
| `events.py`（追加） | 4 个 BSP 事件类型 |
| `audit/invariants.py`（追加） | I23-I29 常量定义 |
| `audit/bsp_checker.py` | BuySellPointInvariantChecker |

---

## 14. 谱系引用

| 谱系记录 | 关联 | 影响 |
|---------|------|------|
| `.chanlun/genealogy/settled/001-degenerate-segment.md` | 退化线段问题（✅ 已结算） | type2/type3 的段识别风险已消除（真实数据退化段率=0%） |
| `.chanlun/genealogy/settled/002-source-incompleteness.md` | 编纂版与原文差异 | 买卖点定义优先参考原始博文 |
| `.chanlun/genealogy/settled/004-provenance-framework.md` | 概念溯源标签 | 本规范所有决策均已标注溯源 |
| `.chanlun/genealogy/settled/005b-object-negates-object-grammar.md` | 对象否定对象语法规则 | 走势完成（type1 前提）必须由走势内部机制否定 |
| 不确定是否存在的谱系 | 买卖点作为新增模块，尚无直接对应的谱系条目 | — |

---

## 15. 影响声明

### 本规范的影响

1. **新增规范文件**：`docs/spec/maimai_rules_v1.md` — 买卖点识别模块设计规范草案
2. **不修改任何已有代码或定义文件**

### 影响的模块（待实现时）

| 模块 | 影响方式 |
|------|---------|
| `events.py` | 追加 4 个 BSP 事件类型 |
| `audit/invariants.py` | 追加 I23-I29 常量 |
| `orchestrator/timeframes.py` | 集成 BuySellPointDetector |
| `gateway.py` | 自动适配（asdict 模式） |

### 下游推论

1. **实现可以开始**：数据类设计和纯函数签名已明确，TBD 部分有明确的临时决策可支持初步 TDD
2. **TBD 部分不阻塞骨架代码**：5 个 TBD 都有临时决策，代码可以按临时决策实现，待定义结算后通过 `/ritual` 广播更新
3. **测试策略**：因 maimai.md 处于生成态，测试覆盖率数字不作为质量指标（testing-override 生成态例外），但仍需编写探索性测试
4. **与 v0 Divergence 的关系**：当前 `a_divergence.py` 依赖 v0 的 `TrendTypeInstance`，买卖点模块直接消费 `Divergence` 数据类即可，v0→v1 适配在买卖点模块外部处理
