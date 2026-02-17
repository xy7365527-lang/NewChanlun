# Level Recursion Interface v1 — Move 统一接口设计规范

> 状态：设计稿 v1.0
> 日期：2026-02-16
> 前置：level_recursion.md v0.1, move_rules_v1.md, zhongshu_rules_v1.md
> 溯源：[新缠论]（统一接口为编排者扩展；递归构造链为 [旧缠论]）

---

## 0. 问题陈述

当前引擎链为**单级别四层同构管线**：

```
BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine → BuySellPointEngine
```

- `ZhongshuEngine` 硬编码消费 `Segment`（= Move[0]）
- `MoveEngine` 硬编码消费 `Zhongshu`（其组件为 Segment）
- **无法**将 Move[1] 输出作为 Center[2] 的输入——因为 `Move` 和 `Segment` 是不同的数据类型

递归构造链要求：

```
Move[0] = Segment
Center[1] = zhongshu_from_segments(Move[0])
Move[1] = moves_from_zhongshus(Center[1])
Center[2] = zhongshu_from_moves(Move[1])     ← 缺失
Move[2] = moves_from_zhongshus(Center[2])     ← 缺失
...
```

**核心矛盾**：`zhongshu_from_segments()` 的输入类型是 `list[Segment]`，而 Move[1] 的类型是 `Move`。二者字段集不同，无法直接互换。

---

## 1. 接口差异分析

### 1.1 Segment（Move[0]）的字段

```python
@dataclass(frozen=True, slots=True)
class Segment:
    s0: int                    # 起点 stroke index
    s1: int                    # 终点 stroke index
    i0: int                    # merged idx 起点
    i1: int                    # merged idx 终点
    direction: "up" | "down"
    high: float                # 段内最高价
    low: float                 # 段内最低价
    confirmed: bool            # 最后一段 False
    kind: "candidate" | "settled"
    ep0_i, ep0_price, ep0_type # 起点端点
    ep1_i, ep1_price, ep1_type # 终点端点
    break_evidence: BreakEvidence | None
```

### 1.2 Move（Move[k], k>=1）的字段

```python
@dataclass(frozen=True, slots=True)
class Move:
    kind: "consolidation" | "trend"
    direction: "up" | "down"
    seg_start: int             # 第一中枢首段索引（身份键）
    seg_end: int               # 最后中枢尾段索引
    zs_start: int              # 中枢列表索引起
    zs_end: int                # 中枢列表索引止
    zs_count: int              # 中枢数量
    settled: bool              # 最后一个 False
    high: float                # max(center.zg)
    low: float                 # min(center.zd)
    first_seg_s0: int          # 前端定位
    last_seg_s1: int           # 前端定位
```

### 1.3 差异矩阵

| 语义 | Segment 字段 | Move 字段 | 差异说明 |
|------|-------------|-----------|----------|
| **方向** | `direction` | `direction` | 同名同类型，兼容 |
| **高/低** | `high` / `low` | `high` / `low` | 同名同类型，兼容 |
| **确认状态** | `confirmed` | `settled` | **命名不同，语义近似** |
| **身份起点** | `s0`（stroke idx） | `seg_start`（segment idx） | **坐标空间不同** |
| **身份终点** | `s1`（stroke idx） | `seg_end`（segment idx） | **坐标空间不同** |
| **端点价格** | `ep0_price`/`ep1_price` | 无 | Move 无端点价格 |
| **内部结构** | `break_evidence` | `zs_start`/`zs_end`/`zs_count` | 完全不同 |
| **类型分类** | `kind`="candidate"/"settled" | `kind`="consolidation"/"trend" | **语义完全不同** |

### 1.4 中枢构造消费端分析

`zhongshu_from_segments()` 实际使用的 Segment 字段：

| 用途 | 使用的字段 | 是否可从 Move 获取 |
|------|-----------|-------------------|
| 三段重叠：ZD 计算 | `seg.low` | Move.low |
| 三段重叠：ZG 计算 | `seg.high` | Move.high |
| 延伸重叠判定 | `seg.high`, `seg.low` | Move.high, Move.low |
| 确认过滤 | `seg.confirmed` | Move.settled |
| 身份定位 | `seg.s0`, `seg.s1` | 需要适配 |
| 前端时间 | `seg.s0`→stroke→i0 | Move.first_seg_s0 |
| 波动区间 | `seg.high`, `seg.low` | Move.high, Move.low |

**关键发现**：中枢构造算法的核心逻辑只需要 `high`, `low`, `confirmed/settled`，以及定位索引。差异主要在**坐标空间**和**命名**。

---

## 2. 设计方案：MoveProtocol + LevelAdapter

### 2.1 设计哲学

- **不修改 Segment 和 Move 的现有定义**（冻结规格不可破坏）
- **不引入继承层次**（避免 OOP 膨胀）
- 使用 **Protocol（结构化子类型）** 定义中枢构造的最小输入接口
- 使用 **适配器函数** 将 Move → Protocol 实例
- 每层引擎通过 **level_id 参数化**，而非硬编码具体类型

### 2.2 MoveProtocol：中枢构造的最小输入协议

[新缠论] 定义一个结构化协议，描述"可作为中枢组件的对象"的最小字段集：

```python
from typing import Protocol, Literal, runtime_checkable

@runtime_checkable
class MoveProtocol(Protocol):
    """可作为中枢组件的对象的最小接口。

    Segment 天然满足此协议（duck typing）。
    Move 通过 MoveAsComponent 适配器满足。

    语义：
    - idx: 在当前级别的组件列表中的位序（由外部赋值）
    - high/low: 该走势段的价格区间
    - direction: 走势方向
    - completed: 是否已完成（可作为高级别中枢组件）
    """

    @property
    def component_idx(self) -> int:
        """在当前级别组件序列中的位置索引。"""
        ...

    @property
    def high(self) -> float:
        """价格区间上界。"""
        ...

    @property
    def low(self) -> float:
        """价格区间下界。"""
        ...

    @property
    def direction(self) -> Literal["up", "down"]:
        """走势方向。"""
        ...

    @property
    def completed(self) -> bool:
        """是否已完成。True = 可作为高级别中枢组件。"""
        ...

    @property
    def start_epoch(self) -> float:
        """起始时间（epoch 秒），用于前端定位和事件关联。"""
        ...

    @property
    def end_epoch(self) -> float:
        """结束时间（epoch 秒）。"""
        ...

    @property
    def level_id(self) -> int:
        """该对象所属的递归级别。Segment = 0, Move[1] = 1, ...。"""
        ...
```

### 2.3 字段语义映射表

| MoveProtocol 字段 | Segment 映射 | Move 映射 | 说明 |
|-------------------|-------------|-----------|------|
| `component_idx` | 在 Segment 列表中的 index | 在 Move 列表中的 index | 由外层 enumerate 赋值 |
| `high` | `seg.high` | `move.high` | 直接映射 |
| `low` | `seg.low` | `move.low` | 直接映射 |
| `direction` | `seg.direction` | `move.direction` | 直接映射 |
| `completed` | `seg.confirmed and seg.kind == "settled"` | `move.settled` | Segment 需组合两字段 |
| `start_epoch` | 需从 stroke → bar 反查 | 需从 first_seg_s0 反查 | 需要时间索引服务 |
| `end_epoch` | 同上 | 同上 | 需要时间索引服务 |
| `level_id` | 固定 `0` | `move` 所在递归层 | 由构造时指定 |

### 2.4 MoveAsComponent：Move → MoveProtocol 适配器

[新缠论] 不可变适配器，将 `Move` 包装为满足 `MoveProtocol` 的对象：

```python
@dataclass(frozen=True, slots=True)
class MoveAsComponent:
    """将 Move[k] 适配为 MoveProtocol，作为 Center[k+1] 的输入组件。

    溯源：[新缠论] — 适配器不改变 Move 的语义，只提供统一访问接口。
    """

    _move: Move
    _component_idx: int
    _level_id: int
    _start_epoch: float = 0.0
    _end_epoch: float = 0.0

    @property
    def component_idx(self) -> int:
        return self._component_idx

    @property
    def high(self) -> float:
        return self._move.high

    @property
    def low(self) -> float:
        return self._move.low

    @property
    def direction(self) -> Literal["up", "down"]:
        return self._move.direction

    @property
    def completed(self) -> bool:
        return self._move.settled

    @property
    def start_epoch(self) -> float:
        return self._start_epoch

    @property
    def end_epoch(self) -> float:
        return self._end_epoch

    @property
    def level_id(self) -> int:
        return self._level_id

    @property
    def source_move(self) -> Move:
        """溯源：返回原始 Move 对象（审计用）。"""
        return self._move
```

### 2.5 SegmentAsComponent：Segment → MoveProtocol 适配器

[新缠论] Segment 天然满足 `MoveProtocol` 大部分字段，但 `completed` 需要组合判定：

```python
@dataclass(frozen=True, slots=True)
class SegmentAsComponent:
    """将 Segment 适配为 MoveProtocol（通常不需要，因为 duck typing 足够）。

    仅在需要显式类型标注或序列化场景时使用。
    """

    _segment: Segment
    _component_idx: int
    _start_epoch: float = 0.0
    _end_epoch: float = 0.0

    @property
    def component_idx(self) -> int:
        return self._component_idx

    @property
    def high(self) -> float:
        return self._segment.high

    @property
    def low(self) -> float:
        return self._segment.low

    @property
    def direction(self) -> Literal["up", "down"]:
        return self._segment.direction

    @property
    def completed(self) -> bool:
        return self._segment.confirmed and self._segment.kind == "settled"

    @property
    def start_epoch(self) -> float:
        return self._start_epoch

    @property
    def end_epoch(self) -> float:
        return self._end_epoch

    @property
    def level_id(self) -> int:
        return 0  # Segment 固定为 level 0

    @property
    def source_segment(self) -> Segment:
        return self._segment
```

---

## 3. 泛化中枢构造函数

### 3.1 zhongshu_from_components()

[新缠论] 将现有 `zhongshu_from_segments()` 泛化为接受 `MoveProtocol` 序列的版本：

```python
@dataclass(frozen=True, slots=True)
class LevelZhongshu:
    """泛化中枢 — 组件为 MoveProtocol 而非硬编码 Segment。

    与 Zhongshu v1 的主要区别：
    - 索引字段指向 component_idx 而非 segment 索引
    - 新增 level_id 字段
    - 保留 zd/zg/gg/dd 语义不变
    """

    zd: float               # 中枢下沿 = max(初始3组件的 low)
    zg: float               # 中枢上沿 = min(初始3组件的 high)
    comp_start: int          # 第一组件在 MoveProtocol 列表中的索引
    comp_end: int            # 最后包含组件的索引
    comp_count: int          # 构成组件数 (>= 3)
    settled: bool            # True = 已被突破组件闭合
    break_comp: int          # 突破组件索引（-1 = 未闭合）
    break_direction: str     # "up" / "down" / ""
    gg: float                # 波动区间上界
    dd: float                # 波动区间下界
    level_id: int            # 该中枢的递归级别
    start_epoch: float       # 第一组件的 start_epoch（前端定位）
    end_epoch: float         # 最后组件的 end_epoch（前端定位）


def zhongshu_from_components(
    components: list[MoveProtocol],
    *,
    level_id: int,
) -> list[LevelZhongshu]:
    """从 MoveProtocol 组件列表计算中枢。

    算法与 zhongshu_from_segments() 完全相同：
    1. 过滤 completed=True 的组件
    2. 滑窗三组件：ZD=max(lows), ZG=min(highs)
    3. ZG > ZD → 中枢成立
    4. 延伸 + 突破
    5. 续进

    参数
    ----
    components : list[MoveProtocol]
        当前级别的组件列表（k级：Move[k-1] 的列表）。
    level_id : int
        构造出的中枢的级别标记（k级中枢 → level_id=k）。
    """
    ...
```

### 3.2 与现有 zhongshu_from_segments() 的关系

**不删除现有函数**。采用"保留 + 桥接"策略：

```
zhongshu_from_segments(segments)                   # 原有：level=1 专用，冻结不变
zhongshu_from_components(components, level_id=k)    # 新增：泛化版，level>=2 使用
```

level=1 的引擎链保持现有行为不变（SegmentEngine → ZhongshuEngine）。level>=2 使用新的 `zhongshu_from_components()` 路径。

**兼容性保证**：

- 对于 level=1，`zhongshu_from_segments()` 和 `zhongshu_from_components(SegmentAsComponent 列表)` 的输出在 zd/zg/settled/break 等核心字段上**必须一致**（由交叉验证测试保证）
- `LevelZhongshu` 可以向下转换为 `Zhongshu`（通过丢弃 level_id 和坐标映射）

### 3.3 泛化走势构造函数

```python
def moves_from_level_zhongshus(
    zhongshus: list[LevelZhongshu],
    *,
    level_id: int,
) -> list[Move]:
    """从 LevelZhongshu 列表构造 Move。

    算法与 moves_from_zhongshus() 完全相同：
    1. 过滤 settled=True 的中枢
    2. 贪心分组（ascending/descending）
    3. 每组 → 一个 Move

    返回的 Move 对象与现有 Move 类型相同，
    但其 seg_start/seg_end 指向 comp_start/comp_end（组件索引空间）。
    """
    ...
```

---

## 4. 事件驱动递归层引擎

### 4.1 RecursiveLevelEngine：单层递归引擎

[新缠论] 每个递归级别维护一个 `RecursiveLevelEngine` 实例：

```python
class RecursiveLevelEngine:
    """单层递归引擎 — 消费低级别 MoveSnapshot，产生本级别的中枢和走势事件。

    数据流：
        MoveSnapshot[k-1] → adapt_to_components()
                          → zhongshu_from_components() → diff → LevelZhongshuSnapshot
                          → moves_from_level_zhongshus() → diff → MoveSnapshot[k]

    参数
    ----
    level_id : int
        本层的递归级别（k）。接收 level_id=k-1 的 Move 作为组件。
    stream_id : str
        流标识。
    """

    def __init__(self, level_id: int, stream_id: str = "") -> None:
        self._level_id = level_id
        self._stream_id = stream_id
        # 内部状态
        self._components: list[MoveAsComponent] = []
        self._prev_zhongshus: list[LevelZhongshu] = []
        self._prev_moves: list[Move] = []
        self._zs_event_seq: int = 0
        self._move_event_seq: int = 0

    def process_move_snapshot(
        self, move_snap: MoveSnapshot, *, source_level: int
    ) -> RecursiveLevelSnapshot:
        """处理低级别 MoveSnapshot，产生本级别事件。

        步骤：
        1. 从 move_snap.moves 中提取 completed=True 的 Move
        2. 包装为 MoveAsComponent（适配 MoveProtocol）
        3. zhongshu_from_components() → diff → 中枢事件
        4. moves_from_level_zhongshus() → diff → 走势事件
        5. 打包为 RecursiveLevelSnapshot
        """
        ...
```

### 4.2 RecursiveLevelSnapshot

```python
@dataclass
class RecursiveLevelSnapshot:
    """一层递归计算后的完整快照。"""

    level_id: int
    bar_idx: int
    bar_ts: float
    components: list[MoveAsComponent]           # 输入组件（溯源用）
    zhongshus: list[LevelZhongshu]              # 本层中枢
    moves: list[Move]                            # 本层走势类型
    zhongshu_events: list[DomainEvent]           # 中枢事件
    move_events: list[DomainEvent]               # 走势事件
    # 组合所有事件
    @property
    def events(self) -> list[DomainEvent]:
        return self.zhongshu_events + self.move_events
```

### 4.3 RecursiveStack：递归栈调度器

[新缠论] 管理多层 `RecursiveLevelEngine` 的自动递归：

```python
class RecursiveStack:
    """递归栈调度器 — 自底向上驱动多层递归。

    数据流：
        MoveSnapshot[1] ──→ RecursiveLevelEngine(level=2) ──→ RecursiveLevelSnapshot[2]
                                                                  │ move_snap
                                                                  ↓
                         RecursiveLevelEngine(level=3) ──→ RecursiveLevelSnapshot[3]
                                                                  │
                                                                  ↓
                                                                 ...
                                                                  │
                                                                  ↓ len(moves) < 3 → 终止

    参数
    ----
    max_levels : int
        最大递归深度（安全阀）。默认 6。
    stream_id : str
        流标识。
    """

    def __init__(self, max_levels: int = 6, stream_id: str = "") -> None:
        self._max_levels = max_levels
        self._stream_id = stream_id
        self._engines: dict[int, RecursiveLevelEngine] = {}

    def process_level1_move_snapshot(
        self, move_snap: MoveSnapshot
    ) -> list[RecursiveLevelSnapshot]:
        """从 level=1 的 MoveSnapshot 开始，递归向上处理所有可处理的层级。

        返回
        ----
        list[RecursiveLevelSnapshot]
            按 level_id 递增排序的各层快照。可能为空（level=1 不足以构造更高级别）。
        """
        snapshots: list[RecursiveLevelSnapshot] = []
        current_move_snap = move_snap
        current_level = 1

        while current_level < self._max_levels:
            next_level = current_level + 1

            # 懒创建引擎
            if next_level not in self._engines:
                self._engines[next_level] = RecursiveLevelEngine(
                    level_id=next_level, stream_id=self._stream_id,
                )

            engine = self._engines[next_level]
            snap = engine.process_move_snapshot(
                current_move_snap, source_level=current_level,
            )
            snapshots.append(snap)

            # 递归终止条件：本层 Move 不足 3 个
            if len(snap.moves) < 3:
                break

            # 向上递归
            current_move_snap = MoveSnapshot(
                bar_idx=snap.bar_idx,
                bar_ts=snap.bar_ts,
                moves=snap.moves,
                events=snap.move_events,
            )
            current_level = next_level

        return snapshots
```

---

## 5. 事件传递机制

### 5.1 事件带 level_id 标签

所有递归层产生的事件需要携带 `level_id`，以便下游区分来源级别：

| 事件类型 | 现有字段 | 新增字段 | 说明 |
|---------|---------|---------|------|
| ZhongshuCandidateV1 | zhongshu_id, zd, zg, ... | `level_id: int` | 标识中枢级别 |
| ZhongshuSettleV1 | ... | `level_id: int` | 同上 |
| ZhongshuInvalidateV1 | ... | `level_id: int` | 同上 |
| MoveCandidateV1 | move_id, kind, direction, ... | `level_id: int` | 标识走势级别 |
| MoveSettleV1 | ... | `level_id: int` | 同上 |
| MoveInvalidateV1 | ... | `level_id: int` | 同上 |

**兼容策略**：

- 新增 `level_id: int = 1` 到现有事件类（默认值保证向后兼容）
- 现有 level=1 管线无需修改
- `BuySellPointEngine` 已有 `level_id` 字段，无需新增

### 5.2 EventBus 扩展

现有 EventBus 按 `tf` 分区。递归层事件需要按 `level_id` 分区（或同时按 tf + level_id）：

```python
# 方案 A：复用 tf 字段，level 编码为 "L{k}"
bus.push(f"L{level_id}", snap.events, stream_id=sid)

# 方案 B（推荐）：新增 push_level 方法
class EventBus:
    def push_level(
        self, level_id: int, events: list[DomainEvent], stream_id: str = "",
    ) -> None:
        """按递归级别推送事件。"""
        for ev in events:
            self._events.append(TaggedEvent(
                tf=f"L{level_id}", event=ev, stream_id=stream_id,
            ))

    def drain_by_level(self, level_id: int) -> list[DomainEvent]:
        """取出指定递归级别的事件。"""
        return self.drain_by_tf(f"L{level_id}")
```

### 5.3 递归层间事件传播规则

[旧缠论] + [新缠论:选择]

```
低级别 Move settled      →  触发高级别重算
低级别 Move invalidated  →  触发高级别重算（可能导致高级别中枢否定）
低级别 Move extended     →  触发高级别重算（可能导致高级别中枢延伸）
```

**关键设计决策**：

1. **完全重算 vs 增量**：与现有四层同构管线一致，采用**全量重算 + diff** 策略。每次低级别 MoveSnapshot 变化，高级别 `zhongshu_from_components()` 全量重算，再 diff 产生事件。理由：
   - 与现有架构同构，降低认知负荷
   - 递归层的组件数量随级别指数衰减（level=2 约几十个 Move，level=3 约几个），全量重算开销可忽略
   - diff 保证事件正确性（增量算法的正确性极难验证）

2. **否定传播是对象否定对象**：低级别 Move 被否定 → 高级别中枢的组件消失 → 高级别中枢被否定。这完全符合谱系 005 的"对象否定对象"原则：中枢不是被"规则"否定的，而是被"组件消失"这一对象事实否定的。

---

## 6. 与现有引擎链的兼容性

### 6.1 渐进式集成策略

```
阶段 1（当前）：
BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine → BuySellPointEngine
                                                        (单级别，level=1)

阶段 2（本设计）：
BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine → BuySellPointEngine
                                                 │
                                                 ↓ MoveSnapshot (level=1)
                                          RecursiveStack
                                                 │
                                          ┌──────┼──────┐
                                          ↓      ↓      ↓
                                    Level 2  Level 3  Level 4 ...
                                    events   events   events
                                          │      │      │
                                          ↓      ↓      ↓
                                        EventBus (按 level 分区)
```

### 6.2 TFOrchestrator 改造

当前 TFOrchestrator 为每个 TF 运行独立五层管线。加入递归后：

```python
# TFOrchestrator.step() 中，在 MoveEngine 处理后追加：
move_snap = self._move_engines[tf].process_zhongshu_snapshot(zs_snap)
if move_snap.events:
    snap.events = list(snap.events) + move_snap.events

# ──── 新增：递归层处理 ────
recursive_snaps = self._recursive_stacks[tf].process_level1_move_snapshot(move_snap)
for rs in recursive_snaps:
    snap.events = list(snap.events) + rs.events
# ──── 新增结束 ────

bsp_snap = self._bsp_engines[tf].process_snapshots(move_snap, zs_snap, seg_snap)
```

**注意**：TFOrchestrator 是口径 B（多时间周期独立管线），而递归是口径 A。在 TFOrchestrator 中加入递归的语义是：**每个 TF 独立地做递归**。这不等于真正的口径 A 递归（从 1 分钟 K 线出发的单一递归链）。但作为工程参考和过渡方案，这是可接受的。

### 6.3 纯递归调度器（口径 A 正式路径）

未来的正式实现应该是一个**纯递归调度器**，不依赖 TFOrchestrator：

```python
class RecursiveOrchestrator:
    """纯递归调度器 — 从 1 分钟 K 线出发，构造全部递归级别。

    口径 A 的唯一正式路径。

    引擎链：
    BiEngine → SegmentEngine → ZhongshuEngine → MoveEngine
                                                    ↓
                                              RecursiveStack
                                                    ↓
                                            (自动递归至终止)
    """

    def __init__(self, stream_id: str = "", max_levels: int = 6) -> None:
        self._bi_engine = BiEngine(...)
        self._seg_engine = SegmentEngine(stream_id=stream_id)
        self._zs_engine = ZhongshuEngine(stream_id=stream_id)
        self._move_engine = MoveEngine(stream_id=stream_id)
        self._recursive_stack = RecursiveStack(
            max_levels=max_levels, stream_id=stream_id,
        )
        self._bus = EventBus()

    def process_bar(self, bar: Bar) -> RecursiveOrchestratorSnapshot:
        """逐 bar 驱动全链。"""
        bi_snap = self._bi_engine.process_bar(bar)
        seg_snap = self._seg_engine.process_snapshot(bi_snap)
        zs_snap = self._zs_engine.process_segment_snapshot(seg_snap)
        move_snap = self._move_engine.process_zhongshu_snapshot(zs_snap)

        # 递归
        recursive_snaps = self._recursive_stack.process_level1_move_snapshot(
            move_snap
        )

        # 汇总事件
        all_events = (
            list(bi_snap.events)
            + seg_snap.events
            + zs_snap.events
            + move_snap.events
        )
        for rs in recursive_snaps:
            all_events.extend(rs.events)

        return RecursiveOrchestratorSnapshot(
            bar_idx=bi_snap.bar_idx,
            bar_ts=bi_snap.bar_ts,
            level_snapshots={1: move_snap, **{rs.level_id: rs for rs in recursive_snaps}},
            all_events=all_events,
        )
```

---

## 7. 转化规则

### 7.1 Move → MoveAsComponent 转化

[新缠论] 转化规则严格定义如下：

```
输入：Move[k] 列表（来自 moves_from_zhongshus 或 moves_from_level_zhongshus）
输出：MoveAsComponent 列表（满足 MoveProtocol）

转化条件：
- 只有 move.settled == True 的 Move 才被转化
  理由：[旧缠论] 中枢组件必须是"已完成的次级别走势类型"
  （level_recursion.md v0.1 §次级别走势类型的完成条件）
- settled == False 的最后一个 Move 被排除
  理由：它尚未被后续走势确认完成

转化映射：
  component_idx = enumerate 序号
  high          = move.high
  low           = move.low
  direction     = move.direction
  completed     = True  （已通过 settled 过滤）
  start_epoch   = 从 time_index 服务查询（见 §7.3）
  end_epoch     = 从 time_index 服务查询
  level_id      = k  （Move[k] 的 k 值）
```

### 7.2 LevelZhongshu → Zhongshu 反向映射

当需要将递归层中枢传递给需要 `Zhongshu` 类型的旧代码（如 `moves_from_zhongshus`）时：

```
LevelZhongshu.zd             → Zhongshu.zd
LevelZhongshu.zg             → Zhongshu.zg
LevelZhongshu.comp_start     → Zhongshu.seg_start   （坐标空间变为组件索引）
LevelZhongshu.comp_end       → Zhongshu.seg_end
LevelZhongshu.comp_count     → Zhongshu.seg_count
LevelZhongshu.settled        → Zhongshu.settled
LevelZhongshu.break_comp     → Zhongshu.break_seg
LevelZhongshu.break_direction → Zhongshu.break_direction
LevelZhongshu.gg             → Zhongshu.gg
LevelZhongshu.dd             → Zhongshu.dd
LevelZhongshu.start_epoch    → Zhongshu.first_seg_s0  （语义近似，不精确）
LevelZhongshu.end_epoch      → Zhongshu.last_seg_s1   （语义近似，不精确）
```

**设计选择**：推荐创建 `moves_from_level_zhongshus()` 直接消费 `LevelZhongshu`，而非做反向映射。反向映射会丢失 `level_id` 信息且坐标空间含义模糊。

### 7.3 时间索引服务

适配器需要将 stroke/segment 索引映射到 epoch 时间。设计一个轻量服务：

```python
class TimeIndex:
    """从索引到时间的查找服务。

    由最底层 BiEngine 的 bar 序列构建，供所有层级共享。
    """

    def __init__(self, bars: list[Bar]) -> None:
        self._bar_times = [bar.ts.timestamp() for bar in bars]

    def stroke_to_epoch(self, stroke_idx: int, strokes: list[Stroke]) -> float:
        """stroke 索引 → 对应 bar 的 epoch 时间。"""
        ...

    def segment_to_epoch_range(self, seg: Segment, strokes: list[Stroke]) -> tuple[float, float]:
        """Segment → (start_epoch, end_epoch)。"""
        ...

    def move_to_epoch_range(self, move: Move, segments: list[Segment], strokes: list[Stroke]) -> tuple[float, float]:
        """Move → (start_epoch, end_epoch)。"""
        ...
```

---

## 8. Diff 与身份规则扩展

### 8.1 LevelZhongshu 的身份定义

与 Zhongshu 同构：

```
identity_key(lzs) = (lzs.zd, lzs.zg, lzs.comp_start, lzs.level_id)
```

加入 `level_id` 防止不同级别的中枢产生身份碰撞。

### 8.2 递归层 Move 的身份定义

与现有 Move 同构，但加入 level_id：

```
identity_key(move, level) = (move.seg_start, level)
```

注意：递归层的 `move.seg_start` 指向组件列表的索引（不是原始 segment 索引），不同级别的 seg_start=0 不会碰撞，因为它们的 level_id 不同。

### 8.3 diff_level_zhongshu()

与 `diff_zhongshu()` 同构，但操作 `LevelZhongshu` 列表。事件类型复用 `ZhongshuCandidateV1` 等（加 `level_id` 字段）。

---

## 9. 边界条件

### 9.1 递归终止

[旧缠论] 当 `len(completed_moves_at_level_k) < 3` 时，无法构造 `Center[k+1]`，递归自然终止。

### 9.2 空层处理

如果某个中间层产出 0 个中枢或 0 个 Move，递归立即终止。不存在"跳层"。

### 9.3 级别抖动

由于采用全量重算 + diff 策略，低级别的 Move 被否定可能导致高级别整体重算。这可能产生"级别抖动"（高级别中枢反复出现/消失）。

**缓解策略**：
- 只有 `settled` 的 Move 才传递到高级别（已实现）
- diff 的公共前缀检测确保已稳定的高级别结构不会被不必要地重发
- 可配置 `min_settled_count` 参数：至少需要 N 个 settled Move 才启动高级别重算

### 9.4 Move[k-1] 完成判定

[旧缠论:选择] 当前选择 `move.settled == True` 作为完成条件（选项 B），而非原文严格的"背驰或第三类买卖点终结"（选项 A）。

**理由**：选项 A 需要背驰检测完成后才能向上递归，引入跨层依赖，且当前背驰检测（`a_divergence_v1.py`）也是单级别的。选项 B 更简洁，且 `settled` 的语义（"后续 move 已确认当前 move 终结"）在操作上等价于"被新走势否定"。

**未来升级路径**：当背驰检测支持多级别后，可以加强完成判定条件为 `settled AND (has_divergence OR has_type3_bsp)`。

---

## 10. 概念溯源总结

| 概念 | 溯源标签 | 说明 |
|------|---------|------|
| 级别 = 递归层级 | [旧缠论] | 第12/17/20课 |
| Move[0] = Segment | [旧缠论] | 归纳基底 |
| Center[k] = 三个 Move[k-1] 重叠 | [旧缠论] | 第17课 |
| 次级别走势类型为组件 | [旧缠论] | 第17课 |
| MoveProtocol 接口 | [新缠论] | 编排者扩展 |
| MoveAsComponent 适配器 | [新缠论] | 编排者扩展 |
| LevelZhongshu 泛化中枢 | [新缠论] | 编排者扩展 |
| RecursiveLevelEngine | [新缠论] | 编排者扩展 |
| RecursiveStack | [新缠论] | 编排者扩展 |
| 全量重算 + diff 策略 | [新缠论:选择] | 与现有管线同构 |
| settled 作为完成条件 | [旧缠论:选择] | 原文有多种解读 |
| 对象否定对象 | [新缠论] | 谱系 005 |

---

## 11. 谱系关联

- **前置**：level_recursion.md v0.1（级别递归定义）
- **前置**：move_rules_v1.md（Move 数据类和 diff 规则）
- **前置**：zhongshu_rules_v1.md（中枢构造规则）
- **前置**：005b-object-negates-object-grammar.md（否定传播符合对象否定对象语法规则）
- **前置**：diff_identity_rules.md（身份/状态分离原则）
- **阻塞**：beichi.md #5 区间套（依赖多级别递归）
- **影响**：events.py（需新增 level_id 字段）
- **影响**：orchestrator/bus.py（需新增 level 分区方法）
- **影响**：orchestrator/timeframes.py（TFOrchestrator 可选集成）

---

## 12. 实现路径建议

| 阶段 | 交付物 | 前置 | 估计复杂度 |
|------|--------|------|-----------|
| P1 | MoveProtocol + SegmentAsComponent + MoveAsComponent | 无 | 低 |
| P2 | LevelZhongshu + zhongshu_from_components() | P1 | 中 |
| P3 | moves_from_level_zhongshus() | P2 | 低 |
| P4 | RecursiveLevelEngine（单层） | P2 + P3 | 中 |
| P5 | RecursiveStack（多层自动递归） | P4 | 中 |
| P6 | 事件 level_id 扩展 + EventBus 扩展 | P4 | 低 |
| P7 | TFOrchestrator 集成（口径 B 过渡） | P5 + P6 | 低 |
| P8 | RecursiveOrchestrator（口径 A 正式） | P5 + P6 | 中 |
| P9 | 交叉验证测试（level=1 新旧路径一致性） | P2 + P3 | 中 |

---

## 变更历史

- 2026-02-16: v1.0 初始设计稿
