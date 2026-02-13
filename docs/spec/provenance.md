# Provenance 规范 — MVP-B0 脚手架

## 目的

为后续递归结构（线段/中枢/背驰）铺设事件溯源基础设施。
MVP-B0 阶段只做脚手架，不实现推理。

## parents 语义

### 同流父子（intra-stream）
同一条流内的事件因果链。例如：
- `stroke_settled:3` → parents 可含 `stroke_candidate:3`（候选→确认）
- `stroke_invalidated:2` → parents 可含 `stroke_settled:2`（确认→否定）

### 跨流挂接（cross-stream）
不同流的事件关联。例如：
- 高级别线段事件 → parents 含低级别构成笔的 event_id
- 递归构造时，level_id=1 的事件引用 level_id=0 的事件

### MVP-B0 状态
- parents 始终为空 tuple `()`
- 脚手架已就绪：EventEnvelopeV1.parents / provenance 字段可填充
- 待 MVP-C 线段推理实现后填充实际 parents

## envelope_id 规范

```
envelope_id = sha256(json.dumps({
    "event_id": <原始 event_id>,
    "stream_id": <流标识>,
    "parents": sorted(<父事件 event_id 列表>),
}, sort_keys=True))[:16]
```

- **不替代 event_id**：event_id 是事件的语义标识，envelope_id 是事件在流中的位置标识
- **parents 排序后参与哈希**：消除顺序依赖
- **确定性**：同 event_id + 同 stream_id + 同 parents → 同 envelope_id

## subject_id 格式

| 事件类型 | subject_id 格式 |
|---------|----------------|
| stroke_candidate | `stroke:{stroke_id}` |
| stroke_settled | `stroke:{stroke_id}` |
| stroke_extended | `stroke:{stroke_id}` |
| stroke_invalidated | `stroke:{stroke_id}` |
| invariant_violation | `event:invariant_violation` |
| （未来）segment_settled | `segment:{segment_id}` |
| （未来）center_formed | `center:{center_id}` |

## 递归扩展路线（MVP-C+）

1. **线段事件**引入时，每个 `segment_settled` 的 parents 包含构成它的所有 `stroke_settled` 的 event_id
2. **中枢事件**引入时，parents 包含构成中枢的线段事件 event_id
3. **背驰事件**引入时，parents 包含比较的两段走势的最终事件 event_id
4. 跨 level_id 引用通过 parents 实现：level_id=1 事件 → level_id=0 事件的 event_id

## wrap_event 工具函数

```python
from newchan.core.provenance import wrap_event, make_subject_id

envelope = wrap_event(
    event=domain_event,
    stream_id=stream_id.value,
    parents=(),           # MVP-B0: 空
    provenance="bi_differ:v1",
    subject_id=make_subject_id(domain_event),
)
```
