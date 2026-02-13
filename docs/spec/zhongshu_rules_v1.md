# Zhongshu（中枢）Rules v1 — 冻结规格

## 1. 核心定义

**中枢** = 至少 3 段连续已确认线段的价格区间重叠。

- **ZD**（中枢下沿）= max(seg_i.low for i in 初始3段)
- **ZG**（中枢上沿）= min(seg_i.high for i in 初始3段)
- **成立条件**：ZG > ZD（严格不等，ZG == ZD 不算重叠）
- **价格区间来源**：Segment.high / Segment.low（段内所有笔极值）

## 2. 生命周期

```
三段重叠 → Candidate → [延伸] → Settle（突破闭合）
                                  ↓
                            续进扫描 → 下一个 Candidate → ...
```

| 阶段 | 事件 | 说明 |
|------|------|------|
| 成立 | ZhongshuCandidateV1 | 三段重叠确认 |
| 延伸 | ZhongshuCandidateV1（更新） | 后续段仍与 [ZD, ZG] 重叠 |
| 闭合 | ZhongshuSettleV1 | 突破段到来，中枢终结 |
| 否定 | ZhongshuInvalidateV1 | 构成段被否定，中枢消失 |

## 3. 固定区间策略（D2）

初始 3 段确定 [ZD, ZG] 后，延伸段只判重叠，**不改变**中枢区间。

延伸判定：`seg.high > ZD AND seg.low < ZG`（与中枢区间有交集）。

## 4. 续进策略（D4）

突破后，下一个中枢扫描从 `max(break_seg_idx - 2, seg_end_idx)` 开始。
允许突破段及前面的段参与新中枢的形成。

## 5. 突破方向

| 条件 | 方向 |
|------|------|
| breaker.low >= ZG | "up" |
| breaker.high <= ZD | "down" |
| 其他（防御） | breaker.high > ZG ? "up" : "down" |

## 6. 否定传播

当段被否定（从 confirmed 列表中消失），`diff_zhongshu()` 自动产生中枢否定事件。

同身份中枢（zd/zg/seg_start 相同）的 seg_end 变化或 settled 升级不产生 invalidate，只产生更新事件。

## 7. 事件类型表

| 事件类型 | 字段 | 说明 |
|---------|------|------|
| zhongshu_candidate | zhongshu_id, zd, zg, seg_start, seg_end, seg_count | 中枢成立/更新 |
| zhongshu_settle | zhongshu_id, zd, zg, seg_start, seg_end, seg_count, break_seg_id, break_direction | 中枢闭合 |
| zhongshu_invalidate | zhongshu_id, zd, zg, seg_start, seg_end | 中枢否定 |

所有事件继承 DomainEvent 基类：`event_type, bar_idx, bar_ts, seq, event_id, schema_version`。

## 8. 不变量 I11-I15

| 编号 | 名称 | 描述 |
|------|------|------|
| I11 | ZHONGSHU_OVERLAP | ZhongshuCandidate 只在 ZG > ZD 时产生 |
| I12 | CANDIDATE_BEFORE_SETTLE | ZhongshuSettle 之前必有同 zhongshu_id 的 Candidate |
| I13 | PARENTS_TRACEABLE | 中枢事件的 seg_start/seg_end 必须合法（seg_end >= seg_start, seg_count >= 3） |
| I14 | ZHONGSHU_INVALIDATE_IDEMPOTENT | 同 (zd, zg, seg_start, seg_end) 不重复 invalidate |
| I15 | ZHONGSHU_REPLAY_DETERMINISM | 同输入 → 同 event_id / order（由测试覆盖） |

## 9. 数据流

```
BiEngine → BiEngineSnapshot
  → SegmentEngine → SegmentSnapshot
    → ZhongshuEngine → ZhongshuSnapshot
      → snap.events += zs_events
        → EventBus → gateway → WS
```
