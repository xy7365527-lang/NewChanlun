# 版本化策略

> 本文档定义 NewChanlun 域事件 schema 的版本化规则，确保前后端在演进过程中保持兼容。

---

## 1. 版本号规则

- **`schema_version`** 为正整数，从 `1` 开始递增
- 每个域事件和 WsEvent 消息均携带 `schema_version` 字段
- 当前版本：**v1**（冻结 schema 详见 [events_v1.md](events_v1.md)）
- 版本号只在发生 **Breaking** 变更时递增（v1 → v2）
- 同一版本内所有事件类型共享同一个 `schema_version` 值

---

## 2. 兼容策略三级别

### 2.1 Additive（v1 内允许）

在当前版本内直接进行，**不需要**递增 `schema_version`。

| 操作 | 示例 | 影响 |
|------|------|------|
| 新增可选字段 | 为 `StrokeCandidate` 新增 `confidence: float = 0.0` | 旧消费者忽略未知字段，无影响 |
| 新增事件类型 | 新增 `segment_candidate` 事件 | 旧消费者在 switch/match 的 default 分支忽略 |
| 新增 WS 消息类型 | 新增 `type: "alert"` 消息 | 旧前端在 switch default 丢弃 |

**约束**：

- 新增字段必须有默认值（Python 侧使用 dataclass default，TypeScript 侧使用可选属性 `?`）
- 新增事件类型必须继承 `DomainEvent` 基类，遵循相同的 `event_id` 计算规则
- 不得改变已有字段的语义

### 2.2 Deprecate（v1 内允许）

标记字段或事件类型为废弃，但**保留向后兼容**，不需要递增 `schema_version`。

| 操作 | 处理方式 |
|------|---------|
| 废弃字段 | 在代码注释和文档中标记 `@deprecated`；字段继续发送，值保持有效 |
| 废弃事件类型 | 在文档中标记为 deprecated；后端继续发送该事件类型 |

**约束**：

- 被废弃的字段/事件类型必须至少保留到下一个大版本（v2）
- 废弃公告应在 CHANGELOG 和版本文档中记录
- 废弃期间，后端必须同时发送旧字段和替代字段（如有）

### 2.3 Breaking（需要 v2）

以下变更属于破坏性变更，**必须**递增 `schema_version`（v1 → v2）：

| 操作 | 示例 |
|------|------|
| 删除字段 | 移除 `stroke_id` 字段 |
| 改变字段类型 | `bar_ts` 从 `float`（epoch 秒）改为 `string`（ISO 8601） |
| 改变字段语义 | `i0` 从 merged bar 索引改为原始 bar 索引 |
| 重命名字段 | `stroke_id` 改名为 `bi_id` |
| 改变 event_id 计算规则 | 更换哈希算法或输入字段 |
| 改变 WsEvent 包装结构 | `payload` 展平到顶层 |

---

## 3. 前后端兼容规则

### 3.1 后端发送新字段 → 旧前端安全忽略

**机制**：

- **后端（Python）**：Pydantic 模型的新字段设置默认值，序列化时包含
- **前端（TypeScript）**：TypeScript interface 未声明的字段在 `JSON.parse()` 后自然存在于对象上，但类型系统不引用，不影响运行

**示例**：后端新增 `confidence` 字段

```python
# 后端：新增可选字段
@dataclass(frozen=True, slots=True)
class StrokeCandidate(DomainEvent):
    ...
    confidence: float = 0.0  # v1 additive 新增
```

```typescript
// 前端（未升级）：interface 中没有 confidence，不影响编译和运行
// JSON.parse 后对象上有 confidence 字段，但前端代码不读取它
```

### 3.2 后端发送新事件类型 → 旧前端安全忽略

**机制**：前端使用 `switch (event.event_type)` 分发事件，未知类型落入 `default` 分支，静默忽略。

```typescript
switch (event.event_type) {
  case "stroke_candidate": /* 处理 */ break;
  case "stroke_settled":   /* 处理 */ break;
  case "stroke_extended":  /* 处理 */ break;
  case "stroke_invalidated": /* 处理 */ break;
  default:
    // 未知事件类型（如未来的 segment_candidate），安全忽略
    console.debug(`Unknown event type: ${event.event_type}`);
    break;
}
```

### 3.3 后端发送新 WS 消息类型 → 旧前端安全忽略

**机制**：前端使用 `switch (msg.type)` 分发 WS 消息，未知 `type` 落入 `default` 分支。

```typescript
switch (msg.type) {
  case "bar":           /* 处理 */ break;
  case "event":         /* 处理 */ break;
  case "snapshot":      /* 处理 */ break;
  case "replay_status": /* 处理 */ break;
  case "error":         /* 处理 */ break;
  default:
    // 未知消息类型，安全忽略
    break;
}
```

---

## 4. 大版本升级流程

当需要从 v1 升级到 v2 时，采用**双版本并行**策略：

### 阶段 1：后端双版本发送

```
后端同时发送 v1 和 v2 格式的事件
前端仍消费 v1
```

- 后端对每个事件同时生成 v1 和 v2 两种格式
- WS 推送中，v1 事件 `schema_version=1`，v2 事件 `schema_version=2`
- 前端根据 `schema_version` 过滤，仅处理自己支持的版本
- 持续时间：至少一个发布周期

### 阶段 2：前端升级

```
后端继续双版本发送
前端升级为消费 v2
```

- 前端更新 TypeScript 类型定义，适配 v2 schema
- 前端切换过滤条件，消费 `schema_version=2` 的事件
- 验证 v2 事件处理正确

### 阶段 3：清理 v1

```
后端停止发送 v1
前端仅消费 v2
```

- 确认所有客户端已升级到 v2
- 后端移除 v1 事件生成逻辑
- 更新文档，归档 v1 schema

---

## 5. 版本协商（预留）

当前 v1 阶段不需要版本协商。未来如果需要，可在 WebSocket 连接握手时协商版本：

```json
// 客户端连接时发送
{ "action": "subscribe", "symbol": "BZ", "tf": "5m", "schema_version": 2 }

// 服务端根据请求的 schema_version 决定发送格式
```

---

## 6. 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-02-13 | v1 | 初始冻结，包含 4 种笔事件 |
