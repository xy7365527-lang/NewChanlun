# Segment v1 规格文档

> 线段事件驱动引擎 — 从"批量函数"升级为"事件流"

## 1. 核心概念

### 线段（Segment）

线段是笔的高级结构，由**至少连续三笔**且**起始三笔有重叠**构成。

缠论原文三条核心规则：
1. 线段至少由连续的三笔构成，且起始三笔必须有重叠部分
2. 线段被破坏，当且仅当至少被有重叠部分的连续三笔的其中一笔破坏
3. 线段被破坏的充要条件就是另一个线段生成

### 特征序列

向上段的特征序列取**反向（down）笔**的 (high, low)；向下段取 (up) 笔。

特征序列经标准包含处理后，检测分型：
- 向上段 → 检测**顶分型**（b.high > a.high, b.high > c.high, b.low > a.low, b.low > c.low）
- 向下段 → 检测**底分型**

### 两种分型情况（gap 分类）

| 分类 | 条件 | 验真要求 |
|------|------|---------|
| **none**（无缺口） | a 与 b 有重叠（b_low < a_high） | 直接触发 → 检查新段三笔重叠 → 结算 |
| **gap**（有缺口） | a 与 b 无重叠（b_low ≥ a_high） | 需要后一特征序列出现任意分型才确认 |

> **关键**：第二种（gap）情况下，后一特征序列出现任意类型分型即可（不分顶底，不分第一二种），只要有分型就触发。

---

## 2. 事件类型

### SegmentBreakPendingV1（线段断裂挂起）

特征序列分型已触发，标记断裂可能发生的位置。

| 字段 | 类型 | 说明 |
|------|------|------|
| segment_id | int | 被断裂的旧段 ID |
| direction | "up"/"down" | 旧段方向 |
| break_at_stroke | int | 分型中心 b 对应的 stroke index (k) |
| gap_class | "none"/"gap" | 缺口分类 |
| fractal_type | "top"/"bottom" | 触发分型类型 |
| s0 | int | 旧段起点 stroke index |
| s1 | int | 旧段暂定终点 (k-1) |

### SegmentSettleV1（线段结算）

旧段终结确认，新段已满足三笔重叠。

| 字段 | 类型 | 说明 |
|------|------|------|
| segment_id | int | 被结算的旧段 ID |
| direction | "up"/"down" | 旧段方向 |
| s0, s1 | int | 旧段起点/终点 |
| ep0_price, ep1_price | float | 起点/终点分型价 |
| gap_class | "none"/"gap" | 缺口分类 |
| new_segment_s0 | int | 新段起点 (= break_at_stroke) |
| new_segment_direction | "up"/"down" | 新段方向 |

### SegmentInvalidateV1（线段否定）

之前的线段在新快照中消失（因笔否定导致重算）。

| 字段 | 类型 | 说明 |
|------|------|------|
| segment_id | int | 被否定的段 ID |
| direction | "up"/"down" | 段方向 |
| s0, s1 | int | 段起点/终点 |

---

## 3. 两阶段结算流程

```
BiEngineSnapshot(strokes)
  → SegmentEngine.process_snapshot()
     → segments_from_strokes_v1(strokes)  // 全量计算
     → diff_segments(prev, curr)          // 差分产生事件
        ┌─ invalidate: prev 后缀中消失的段
        ├─ pending:    新出现的分型触发
        └─ settle:     confirmed + settled + 三笔重叠锚
```

### 结算锚规则

- **旧段终点** `s1 = break_at_stroke - 1`（I7）
- **新段起点** `new_segment_s0 = break_at_stroke`（I7）
- **新段方向** = 旧段方向取反
- **pending 必须先于 settle**（I6）

---

## 4. 不变量（I6-I10）

| 编号 | 名称 | 规则 |
|------|------|------|
| I6 | PENDING_DIRECT_SETTLE | SegmentSettleV1 之前必须有同 segment_id 的 SegmentBreakPendingV1 |
| I7 | SETTLE_ANCHOR | s1 == break_at_stroke - 1, new_segment_s0 == break_at_stroke |
| I8 | GAP_NEEDS_SEQ2 | gap_class="gap" 必须经历第二序列分型确认 |
| I9 | INVALIDATE_IDEMPOTENT | 同 (s0, s1, direction) 不能被 invalidate 两次而中间无 settle |
| I10 | SEGMENT_REPLAY_DETERMINISM | 同输入 → 同 event_id + payload + 顺序 |
| I11 | DEGENERATE_SEGMENT_PROHIBITION | 向上段 ep1_price ≥ ep0_price；向下段 ep1_price ≤ ep0_price |

### I11 古怪线段硬约束（"顶高于底"）

**原文依据**：第78课 L20

> "同一线段中，两端的一顶一底，顶肯定要高于底，如果你划出一个不符合这基本要求的线段，那肯定是划错了。"

**形式化**：
- **向上段**：`ep1_price ≥ ep0_price`（终点顶 ≥ 起点底）
- **向下段**：`ep1_price ≤ ep0_price`（终点底 ≤ 起点顶）

违反此条件的段称为"退化段"（degenerate segment），表示笔序列产出了不合理的分型组合。

**根因与解决方案**（谱系 001 已结算）：退化段的根本原因是旧笔定义过严（merged gap ≥ 4），新笔定义（《忽闻台风可休市》）从源头消除退化段。因此 I11 在 `mode="new"` 下由笔层保证，段层仅提供断言检测（`a_assertions.py::assert_segment_theorem_v1`）。

**概念溯源**：[旧缠论] 第78课

---

## 5. 反例场景（Golden Cases）

### 5.1 no-gap 直接结算 ✓

特征序列 a,b 有重叠 + 新段三笔重叠 → 直接发出 SettleV1。

### 5.2 no-gap 无 anchor ✗

分型触发但新段三笔无重叠 → 旧段延续，无 SettleV1。

> 这是缠论原文"线段被破坏的充要条件就是另一个线段生成"的体现。

### 5.3 gap + 第二序列有分型 ✓

特征序列 a,b 有缺口 + 后一特征序列已出现分型 → SettleV1。

### 5.4 gap + 第二序列无分型 ✗

有缺口但后一特征序列只有 1-2 个元素（不足形成分型） → 暂不触发。

### 5.5 gap + 任意分型类型 ✓

后一特征序列出现顶分型或底分型均可触发（不区分第一二种）。

---

## 6. 尾窗扫描优化

`_FeatureSeqState.TAIL_WINDOW = 7`

分型检测只在特征序列最近 7 个元素内搜索，避免长序列退化为 O(n²)。

扫描起始位置：
```python
start = max(1, self.last_checked, n - self.TAIL_WINDOW)
```

语义等价于全量扫描（已由测试验证）。

---

## 7. 文件清单

| 文件 | 职责 |
|------|------|
| `events.py` | 3 个 segment 事件类型 |
| `core/recursion/segment_engine.py` | SegmentEngine（消费 BiEngineSnapshot） |
| `core/recursion/segment_state.py` | SegmentSnapshot + diff_segments() |
| `audit/invariants.py` | I6-I10 常量定义 |
| `audit/segment_checker.py` | SegmentInvariantChecker |
| `a_segment_v1.py` | 增强 gap 验真 + 尾窗扫描 |
| `orchestrator/timeframes.py` | TFOrchestrator 集成 SegmentEngine |

---

## 8. 红线保护

以下文件/类**零修改**：

- `events.py::DomainEvent` 基类（R5 红线）
- `fingerprint.py` 现有函数
- `bi_engine.py` / `bi_differ.py`
- `orchestrator/bus.py` EventBus
- `gateway.py::_event_to_ws`（asdict+_exclude 模式自动适配）
