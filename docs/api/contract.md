# API 契约 v1 — MVP-0

## 概述

MVP-0 提供两个后端服务：

| 服务 | 端口 | 框架 | 职责 |
|------|------|------|------|
| **Bottle 主服务** | 8765 | Bottle | K线数据、overlay、品种搜索（现有） |
| **FastAPI 网关** | 8766 | FastAPI | WebSocket 事件流、回放 API（新增） |

## WebSocket 端点

### `WS /ws/feed`

双向通信，用于事件推送和回放控制。

**连接**：`ws://localhost:8766/ws/feed`

**服务端 → 客户端消息类型**：

| type | 触发时机 | 关键字段 |
|------|---------|---------|
| `bar` | 每根新 bar 到达 | idx, ts, o/h/l/c/v |
| `event` | 笔状态变化 | event_type, bar_idx, seq, payload |
| `snapshot` | 连接/seek 后 | bar_idx, strokes[], event_count |
| `replay_status` | 回放状态变化 | mode, current_idx, total_bars, speed |
| `error` | 错误 | message, code |

**客户端 → 服务端命令**：

| action | 参数 | 说明 |
|--------|------|------|
| `subscribe` | symbol, tf | 订阅品种事件流 |
| `unsubscribe` | — | 取消订阅 |
| `replay_start` | symbol, tf | 开始回放会话 |
| `replay_step` | step_count | 步进 N bars |
| `replay_seek` | seek_idx | 跳转到指定 bar |
| `replay_play` | speed | 自动播放 |
| `replay_pause` | — | 暂停播放 |

## REST 回放 API

所有端点前缀：`http://localhost:8766`

### `POST /api/replay/start`

创建回放会话。

```json
// 请求
{ "symbol": "BZ", "tf": "5m", "interval": "1min" }

// 响应
{ "session_id": "uuid", "total_bars": 200, "status": "ready" }
```

### `POST /api/replay/step`

步进 N bars，返回事件。

```json
// 请求
{ "session_id": "uuid", "count": 1 }

// 响应
{
  "bar_idx": 42,
  "bar": { "type": "bar", "idx": 42, "ts": 1707800000, "o": 78.5, ... },
  "events": [
    { "type": "event", "event_type": "stroke_settled", "bar_idx": 42, "seq": 7, "payload": {...} }
  ]
}
```

### `POST /api/replay/seek`

跳转到指定位置，返回快照。

```json
// 请求
{ "session_id": "uuid", "target_idx": 50 }

// 响应
{
  "bar_idx": 50,
  "snapshot": { "type": "snapshot", "bar_idx": 50, "strokes": [...], "event_count": 12 }
}
```

### `POST /api/replay/play`

开始自动播放。

```json
// 请求
{ "session_id": "uuid", "speed": 1.0 }

// 响应
{ "status": "playing" }
```

### `POST /api/replay/pause`

暂停播放。

```json
// 请求
{ "session_id": "uuid" }

// 响应
{ "status": "paused", "bar_idx": 42 }
```

### `GET /api/replay/status?session_id=uuid`

查询回放状态。

```json
{
  "session_id": "uuid",
  "mode": "playing",
  "current_idx": 42,
  "total_bars": 200,
  "speed": 1.0
}
```

## 事件类型

见 [events.md](events.md)。

## 版本策略

- 消息格式加新字段 = 向后兼容，不需要版本号变更
- 移除/重命名字段 = 破坏性变更，需要新版本（v2）
- 当前版本：v1
