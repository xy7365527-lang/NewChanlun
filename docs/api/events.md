# 域事件 Schema — MVP-0

## 事件基类

所有事件共享以下字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `event_type` | string | 事件类型标识符 |
| `bar_idx` | int | 触发事件的 bar 位置索引（0-based） |
| `bar_ts` | float | 触发事件的 bar 时间戳（epoch 秒） |
| `seq` | int | 全局单调递增序号 |

## 笔事件（MVP-0）

### `stroke_candidate` — 新笔候选

新的未确认笔出现。对应引擎内部 `Stroke.confirmed=False`。

```json
{
  "event_type": "stroke_candidate",
  "bar_idx": 42,
  "bar_ts": 1707800000.0,
  "seq": 7,
  "stroke_id": 3,
  "direction": "up",
  "i0": 10,
  "i1": 15,
  "p0": 78.2,
  "p1": 79.5
}
```

| 字段 | 说明 |
|------|------|
| `stroke_id` | 笔在当前快照中的索引位置 |
| `direction` | `"up"` (底→顶) 或 `"down"` (顶→底) |
| `i0`, `i1` | 起/终点在 merged bar 序列中的位置索引 |
| `p0`, `p1` | 起/终点分型极值价 |

### `stroke_settled` — 笔结算

之前的候选笔变为已确认。通常在新候选笔产生时，前一笔自动结算。

```json
{
  "event_type": "stroke_settled",
  "bar_idx": 48,
  "bar_ts": 1707801800.0,
  "seq": 9,
  "stroke_id": 3,
  "direction": "up",
  "i0": 10,
  "i1": 15,
  "p0": 78.2,
  "p1": 79.5
}
```

### `stroke_extended` — 笔延伸

末笔（candidate）的终点分型移动了。笔的 i0/direction 不变，但 i1/p1 变化。

```json
{
  "event_type": "stroke_extended",
  "bar_idx": 44,
  "bar_ts": 1707800600.0,
  "seq": 8,
  "stroke_id": 3,
  "direction": "up",
  "old_i1": 15,
  "new_i1": 17,
  "old_p1": 79.5,
  "new_p1": 80.1
}
```

### `stroke_invalidated` — 笔否定

之前存在的笔在新快照中消失。可能因为分型条件不再成立，或笔被合并。

```json
{
  "event_type": "stroke_invalidated",
  "bar_idx": 45,
  "bar_ts": 1707800900.0,
  "seq": 10,
  "stroke_id": 2,
  "direction": "down",
  "i0": 5,
  "i1": 10,
  "p0": 80.0,
  "p1": 78.2
}
```

## 事件生成机制

事件通过**差分快照**产生：

1. 每来一根新 bar，重跑完整管线（包含→分型→笔）
2. 对比前后两次 `list[Stroke]` 快照
3. 差异映射为事件：
   - 公共前缀中的笔：无事件
   - prev 后缀中的笔 → `stroke_invalidated`
   - curr 后缀中 confirmed=True 的笔 → `stroke_settled`
   - curr 后缀中 confirmed=False 的笔 → `stroke_candidate`
   - 同位置笔 i1/p1 变化 → `stroke_extended`

## 不变量

1. **seq 单调递增**：同一回放中 seq 永远递增
2. **无未来函数**：bar_idx=k 的事件仅基于 bars[:k+1] 的数据
3. **确定性**：相同的 bar 序列永远产生相同的事件序列
4. **每 bar 最多 4 个事件**：invalidate(旧) + settle(旧) + candidate(新) + extend(末)

## MVP-1 扩展（预留）

未来将新增：
- `segment_candidate` / `segment_settled` / `segment_invalidated`
- `center_formed` / `center_extended` / `center_terminated`
- `divergence_detected`
- `signal_buy` / `signal_sell`
