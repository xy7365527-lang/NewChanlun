# Diff Identity Rules — 三层同构管线身份/状态分离规范

> PR-C0.5 引入，适用于 Segment 层和 Zhongshu 层的 diff 算法。

## 1. 核心概念

每个 domain entity（笔/线段/中枢）都有两组字段：

- **身份（Identity）**：标识"它是谁"——同一身份的实体在整个生命周期中保持不变。
- **状态（State）**：标识"它现在怎样"——状态可以随新数据到达而更新。

Diff 算法基于这个分离做出事件决策：
- **身份相同 + 状态变化** → 禁止 invalidate，改为 emit 升级/更新事件
- **身份消失** → 允许 emit invalidate

## 2. 身份键定义

| 层级 | 身份键 (Identity Key) | 状态键 (State Key) |
|------|----------------------|-------------------|
| Stroke | `(i0, direction)` | `(i1, p0, p1, confirmed)` |
| **Segment** | **`(s0, direction)`** | `(s1, confirmed, kind, break_evidence)` |
| **Zhongshu** | **`(zd, zg, seg_start)`** | `(seg_end, settled)` |

**共享 helper**：`src/newchan/core/diff/identity.py`

```python
segment_identity_key(seg) -> (int, str)       # (s0, direction)
same_segment_identity(a, b) -> bool
zhongshu_identity_key(zs) -> (float, float, int)  # (zd, zg, seg_start)
same_zhongshu_identity(a, b) -> bool
```

## 3. Diff 算法伪代码

```
function diff(prev, curr):
    common_len = 找严格相等的公共前缀长度

    # prev 后缀 → invalidations（带身份跳过）
    for i in [common_len, len(prev)):
        if curr[i] 存在 AND same_identity(prev[i], curr[i]):
            continue   # 同身份升级 → 跳过 invalidate
        emit Invalidate(prev[i])

    # curr 后缀 → new/upgrade events
    for i in [common_len, len(curr)):
        if prev[i] 存在 AND same_identity(prev[i], curr[i]):
            emit 升级/更新事件（具体取决于状态变化类型）
        else:
            emit 全新实体事件
```

## 4. 允许的事件序列（per identity key）

```
┌────────────────┐
│   (不存在)      │
└───────┬────────┘
        │ Candidate / BreakPending
        v
┌────────────────┐    Settle / 升级
│   Candidate     │──────────────────┐
│   (活跃)        │                  │
└───────┬────────┘                  v
        │ Invalidate         ┌──────────────┐
        v                    │   Settled     │
┌────────────────┐           │   (已结算)    │
│   终态          │           └──────┬───────┘
│   (不可复活)    │                  │ Invalidate
│                │                  v
│                │           ┌──────────────┐
│                │           │   终态        │
└────────────────┘           └──────────────┘
```

**关键规则**：
- Invalidate 是 **终态**（I17）——同身份实体一旦被 invalidate，不得再出现后续事件。
- 身份保持更新 **不得** 产生 invalidate（I16）——只允许产生升级/更新事件。

## 5. Segment 层事件矩阵

| prev[i] 状态 | curr[i] 状态 | 同身份？ | invalidate? | 产生事件 |
|---|---|---|---|---|
| unconfirmed, be=None | unconfirmed, s1 变, be=None | 是 | 跳过 | 无（纯延伸） |
| unconfirmed, be=None | unconfirmed, s1 变, be=有 | 是 | 跳过 | BreakPending |
| unconfirmed, be=有(k=3) | unconfirmed, be=有(k=5) | 是 | 跳过 | BreakPending（更新） |
| unconfirmed, be=有 | confirmed, settled, be=有 | 是 | 跳过 | BreakPending + Settle |
| s0=0, dir=up | s0=3, dir=down | 否 | invalidate | 按全新段逻辑 |
| 存在 | 不存在 | N/A | invalidate | N/A |

## 6. Zhongshu 层事件矩阵

| prev[i] 状态 | curr[i] 状态 | 同身份？ | invalidate? | 产生事件 |
|---|---|---|---|---|
| settled=False, seg_end=2 | settled=False, seg_end=3 | 是 | 跳过 | CandidateV1（延伸） |
| settled=False | settled=True | 是 | 跳过 | SettleV1（升级） |
| zd/zg/seg_start 变 | — | 否 | invalidate | 按全新中枢逻辑 |
| 存在 | 不存在 | N/A | invalidate | N/A |

## 7. 不变量

| Code | 名称 | 说明 |
|------|------|------|
| I16 | IDENTITY_PRESERVING_NO_INVALIDATE | 身份保持更新不得产生 invalidate（由 diff 算法保证） |
| I17 | INVALIDATE_IS_TERMINAL | invalidate 后同身份不得再出现事件（由 checker 运行时验证） |

## 8. 测试覆盖

- `test_segment_identity_skip.py` — 9 个 Segment 层身份跳过测试
- `test_zhongshu_identity_skip.py` — 5 个 Zhongshu 层身份跳过测试
- `test_event_sequence_monotonic.py` — 5 个 I17 终态验证测试（含 checker 正/反例）

## 9. 注意事项

- `_segments_equal()` **不检查** `break_evidence`——break_evidence 变化单独不触发 diff 后缀逻辑。
  实际场景中，新笔产生时 `s1` 总会延伸，因此 break_evidence 变化总伴随 s1 变化。
- Stroke 层（bi_differ）保持红线保护，不做身份跳过修改。
- 身份键选择遵循"最小唯一标识"原则：segment 用 `(s0, direction)` 而非 `(s0, s1, direction)`，
  因为同一段的 s1 会随数据到达而延伸。
