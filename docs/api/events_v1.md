# 域事件 Schema v1（冻结）

> **schema_version = 1**
>
> 本文档为 v1 冻结 schema。v1 生命周期内仅允许 **additive** 变更（新增可选字段、新增事件类型）。
> 任何破坏性变更必须提升至 v2。版本化策略详见 [versioning.md](versioning.md)。

---

## 1. DomainEvent 基类

所有域事件共享以下基类字段：

| 字段 | 类型 | 必填 | 说明 |
|------|------|:----:|------|
| `event_type` | `string` | Y | 事件类型标识符，取值见下文各事件定义 |
| `bar_idx` | `int` | Y | 触发此事件的 bar 在原始序列中的位置索引（0-based） |
| `bar_ts` | `float` | Y | 触发此事件的 bar 的时间戳（epoch 秒） |
| `seq` | `int` | Y | 全局单调递增事件序号，同一回放会话内永不回退 |
| `event_id` | `string` | Y | 事件唯一标识符，计算规则见第 4 节 |
| `schema_version` | `int` | Y | Schema 版本号，当前固定为 `1` |

**后端实现**：`src/newchan/events.py` — `DomainEvent` dataclass
**前端实现**：`frontend/src/types/events.ts` — `DomainEventBase` interface

---

## 2. 笔事件（Stroke Events）

### 2.1 `stroke_candidate` — 新笔候选

新的未确认笔出现。对应引擎内部 `Stroke.confirmed=False`。

| 字段 | 类型 | 说明 |
|------|------|------|
| `event_type` | `"stroke_candidate"` | 固定值 |
| `bar_idx` | `int` | 基类字段 |
| `bar_ts` | `float` | 基类字段 |
| `seq` | `int` | 基类字段 |
| `event_id` | `string` | 基类字段 |
| `schema_version` | `int` | 基类字段 |
| `stroke_id` | `int` | 笔在当前快照中的索引位置 |
| `direction` | `"up" \| "down"` | `"up"` = 底分型→顶分型，`"down"` = 顶分型→底分型 |
| `i0` | `int` | 起点在 merged bar 序列中的位置索引 |
| `i1` | `int` | 终点在 merged bar 序列中的位置索引 |
| `p0` | `float` | 起点分型极值价 |
| `p1` | `float` | 终点分型极值价 |

```json
{
  "event_type": "stroke_candidate",
  "bar_idx": 42,
  "bar_ts": 1707800000.0,
  "seq": 7,
  "event_id": "a3f1b2c4d5e6f708",
  "schema_version": 1,
  "stroke_id": 3,
  "direction": "up",
  "i0": 10,
  "i1": 15,
  "p0": 78.2,
  "p1": 79.5
}
```

### 2.2 `stroke_settled` — 笔结算

之前的候选笔变为已确认。通常在新候选笔产生时，前一笔自动结算。

| 字段 | 类型 | 说明 |
|------|------|------|
| `event_type` | `"stroke_settled"` | 固定值 |
| `bar_idx` | `int` | 基类字段 |
| `bar_ts` | `float` | 基类字段 |
| `seq` | `int` | 基类字段 |
| `event_id` | `string` | 基类字段 |
| `schema_version` | `int` | 基类字段 |
| `stroke_id` | `int` | 笔在当前快照中的索引位置 |
| `direction` | `"up" \| "down"` | 笔方向 |
| `i0` | `int` | 起点在 merged bar 序列中的位置索引 |
| `i1` | `int` | 终点在 merged bar 序列中的位置索引 |
| `p0` | `float` | 起点分型极值价 |
| `p1` | `float` | 终点分型极值价 |

```json
{
  "event_type": "stroke_settled",
  "bar_idx": 48,
  "bar_ts": 1707801800.0,
  "seq": 9,
  "event_id": "b7e2c9d4f1a0386e",
  "schema_version": 1,
  "stroke_id": 3,
  "direction": "up",
  "i0": 10,
  "i1": 15,
  "p0": 78.2,
  "p1": 79.5
}
```

### 2.3 `stroke_extended` — 笔延伸

末笔（candidate）的终点分型移动了。笔的 stroke_id 和 direction 不变，但 i1/p1 发生变化。

| 字段 | 类型 | 说明 |
|------|------|------|
| `event_type` | `"stroke_extended"` | 固定值 |
| `bar_idx` | `int` | 基类字段 |
| `bar_ts` | `float` | 基类字段 |
| `seq` | `int` | 基类字段 |
| `event_id` | `string` | 基类字段 |
| `schema_version` | `int` | 基类字段 |
| `stroke_id` | `int` | 笔在当前快照中的索引位置 |
| `direction` | `"up" \| "down"` | 笔方向 |
| `old_i1` | `int` | 变更前的终点位置索引 |
| `new_i1` | `int` | 变更后的终点位置索引 |
| `old_p1` | `float` | 变更前的终点极值价 |
| `new_p1` | `float` | 变更后的终点极值价 |

```json
{
  "event_type": "stroke_extended",
  "bar_idx": 44,
  "bar_ts": 1707800600.0,
  "seq": 8,
  "event_id": "c4d5e6f708a1b2c3",
  "schema_version": 1,
  "stroke_id": 3,
  "direction": "up",
  "old_i1": 15,
  "new_i1": 17,
  "old_p1": 79.5,
  "new_p1": 80.1
}
```

### 2.4 `stroke_invalidated` — 笔否定

之前存在的笔在新快照中消失。可能因为分型条件不再成立，或笔被合并/替换。

| 字段 | 类型 | 说明 |
|------|------|------|
| `event_type` | `"stroke_invalidated"` | 固定值 |
| `bar_idx` | `int` | 基类字段 |
| `bar_ts` | `float` | 基类字段 |
| `seq` | `int` | 基类字段 |
| `event_id` | `string` | 基类字段 |
| `schema_version` | `int` | 基类字段 |
| `stroke_id` | `int` | 被否定的笔在前一次快照中的索引位置 |
| `direction` | `"up" \| "down"` | 被否定的笔方向 |
| `i0` | `int` | 被否定的笔起点位置索引 |
| `i1` | `int` | 被否定的笔终点位置索引 |
| `p0` | `float` | 被否定的笔起点极值价 |
| `p1` | `float` | 被否定的笔终点极值价 |

```json
{
  "event_type": "stroke_invalidated",
  "bar_idx": 45,
  "bar_ts": 1707800900.0,
  "seq": 10,
  "event_id": "d6e7f8091a2b3c4d",
  "schema_version": 1,
  "stroke_id": 2,
  "direction": "down",
  "i0": 5,
  "i1": 10,
  "p0": 80.0,
  "p1": 78.2
}
```

---

## 3. 事件类型汇总

| event_type | 语义 | payload 特有字段 |
|-----------|------|-----------------|
| `stroke_candidate` | 新笔候选出现 | stroke_id, direction, i0, i1, p0, p1 |
| `stroke_settled` | 笔从候选变为确认 | stroke_id, direction, i0, i1, p0, p1 |
| `stroke_extended` | 末笔终点移动 | stroke_id, direction, old_i1, new_i1, old_p1, new_p1 |
| `stroke_invalidated` | 笔在新快照中消失 | stroke_id, direction, i0, i1, p0, p1 |

---

## 4. event_id 计算规则

`event_id` 用于事件去重和幂等性保证。计算方式：

```
event_id = sha256(canonical_json(bar_idx, bar_ts, event_type, seq, payload))[:16]
```

具体步骤：

1. 构造一个包含以下键的字典（按键名字典序排列）：
   - `bar_idx`：int
   - `bar_ts`：float
   - `event_type`：string
   - `seq`：int
   - `payload`：dict（事件特有字段，如 stroke_id, direction 等）
2. 将字典序列化为 **canonical JSON**（键按字典序排列，无多余空白，数值不含尾随零）
3. 对 JSON 字符串（UTF-8 编码）计算 SHA-256 哈希
4. 取哈希十六进制表示的 **前 16 个字符** 作为 `event_id`

**示例**：

```python
import hashlib, json

data = {
    "bar_idx": 42,
    "bar_ts": 1707800000.0,
    "event_type": "stroke_candidate",
    "payload": {
        "direction": "up",
        "i0": 10,
        "i1": 15,
        "p0": 78.2,
        "p1": 79.5,
        "stroke_id": 3
    },
    "seq": 7
}
canonical = json.dumps(data, sort_keys=True, separators=(",", ":"))
event_id = hashlib.sha256(canonical.encode()).hexdigest()[:16]
```

---

## 5. WsEvent 消息格式

域事件通过 WebSocket 推送时，封装为 `WsEvent` 消息。这是服务端→客户端的传输格式。

| 字段 | 类型 | 说明 |
|------|------|------|
| `type` | `"event"` | WS 消息类型标识，固定为 `"event"` |
| `event_type` | `string` | 域事件类型（如 `"stroke_candidate"`） |
| `bar_idx` | `int` | 触发事件的 bar 位置索引 |
| `bar_ts` | `float` | 触发事件的 bar 时间戳（epoch 秒） |
| `seq` | `int` | 全局单调递增序号 |
| `payload` | `object` | 事件特有字段（如 stroke_id, direction, i0, i1 等） |
| `event_id` | `string` | 事件唯一标识符 |
| `schema_version` | `int` | Schema 版本号 |

**JSON 示例**：

```json
{
  "type": "event",
  "event_type": "stroke_candidate",
  "bar_idx": 42,
  "bar_ts": 1707800000.0,
  "seq": 7,
  "payload": {
    "stroke_id": 3,
    "direction": "up",
    "i0": 10,
    "i1": 15,
    "p0": 78.2,
    "p1": 79.5
  },
  "event_id": "a3f1b2c4d5e6f708",
  "schema_version": 1
}
```

**后端实现**：`src/newchan/contracts/ws_messages.py` — `WsEvent` Pydantic model
**前端实现**：`frontend/src/types/events.ts` — `WsEventMessage` interface

---

## 6. 不变量

1. **seq 单调递增**：同一回放会话中 `seq` 永远递增，不回退
2. **无未来函数**：`bar_idx=k` 的事件仅基于 `bars[:k+1]` 的数据产生
3. **确定性**：相同的 bar 序列永远产生相同的事件序列
4. **每 bar 事件上限**：每根 bar 最多产生 4 个事件（invalidate + settle + candidate + extend）
5. **event_id 唯一性**：在同一回放会话中，event_id 不重复

---

## 7. MVP-1 预留事件类型

以下事件类型将在 v1 schema 内以 additive 方式新增（无需升级至 v2）：

- `segment_candidate` / `segment_settled` / `segment_invalidated` — 线段事件
- `center_formed` / `center_extended` / `center_terminated` — 中枢事件
- `divergence_detected` — 背驰检测
- `signal_buy` / `signal_sell` — 买卖信号
