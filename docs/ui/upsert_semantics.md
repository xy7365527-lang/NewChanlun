# Upsert Semantics — 前端事件消费指引

> PR-C0.5 引入，指导前端如何正确消费 Segment/Zhongshu 事件流。

## 1. 背景

后端管线（BiEngine → SegmentEngine → ZhongshuEngine）通过 diff 算法产生域事件流。
PR-C0.5 引入了"身份保持升级"机制：**同身份实体的状态更新不再产生 invalidate + 重新 emit，
而是直接产生升级/更新事件**。

前端需要理解这些事件的语义才能正确渲染 overlay。

## 2. 事件类型与消费策略

### 2.1 Segment 事件

| 事件类型 | 语义 | 前端操作 |
|----------|------|---------|
| `segment_break_pending` | 段出现破坏待确认 | Upsert overlay（key = `segment_id`） |
| `segment_settle` | 段结算确认 | Upsert overlay（key = `segment_id`），更新样式为已确认 |
| `segment_invalidate` | 段被否定 | 删除 overlay（key = `segment_id`） |

### 2.2 Zhongshu 事件

| 事件类型 | 语义 | 前端操作 |
|----------|------|---------|
| `zhongshu_candidate` | 中枢候选出现/延伸 | Upsert overlay（key = `zhongshu_id`） |
| `zhongshu_settle` | 中枢闭合确认 | Upsert overlay（key = `zhongshu_id`），更新样式为已确认 |
| `zhongshu_invalidate` | 中枢被否定 | 删除 overlay（key = `zhongshu_id`） |

## 3. 身份键与 Subject ID

前端应使用以下键做 overlay map-by-identity：

```typescript
// Segment overlay
const segmentSubjectId = `${streamId}:seg:${event.segment_id}`;

// Zhongshu overlay
const zhongshuSubjectId = `${streamId}:zs:${event.zhongshu_id}`;
```

**Candidate 和 BreakPending 是 upsert 操作**：
- 第一次收到 `zhongshu_candidate(id=0)` → 创建 overlay
- 再次收到 `zhongshu_candidate(id=0)` → 更新同一 overlay（段延伸导致区间扩大）

**Invalidate 是删除操作**：
- 收到 `zhongshu_invalidate(id=0)` → 删除 overlay
- **保证**：invalidate 后不会再收到同身份的 candidate/settle（I17 不变量保护）

## 4. 当前实现状态

当前前端实现（`EventMarkerPrimitive.ts`）为**追加流**模式——每个事件作为独立 marker 追加渲染，
不做 upsert 去重。这在数据量小时可行，但在长时间运行时可能产生重复 marker。

### 建议迁移路径

后续版本应迁移到 **map-by-identity** 模式：

```typescript
// 维护一个 Map<SubjectId, OverlayState>
const overlays = new Map<string, OverlayState>();

function processEvent(event: ChanEvent) {
    const subjectId = deriveSubjectId(event);

    if (event.event_type.includes('invalidate')) {
        overlays.delete(subjectId);
    } else {
        overlays.set(subjectId, deriveOverlayState(event));
    }
}
```

## 5. 事件流保证

后端 diff 算法保证以下事件流特性：

1. **I12 保证**：`zhongshu_settle` 之前必有对应 `zhongshu_candidate`
2. **I6 保证**：`segment_settle` 之前必有对应 `segment_break_pending`
3. **I16 保证**：同身份更新不产生 invalidate（前端不会看到"先删后建"的抖动）
4. **I17 保证**：invalidate 后同身份不再出现事件（前端删除后不会"复活"）
5. **确定性**：同输入 → 同事件流（支持回放和调试）

## 6. 注意事项

- `zhongshu_id` 和 `segment_id` 是 **positional index**（列表中的位置），不是全局唯一 ID。
  因此前端 subject key 必须包含 `stream_id` 前缀以避免跨流冲突。
- 中枢的 `zd`/`zg` 是固定区间（D2 规则），不会随延伸改变——适合作为渲染位置的基准。
- 中枢 candidate 的 marker 可显示为半透明/虚线，settle 后切换为实线。
