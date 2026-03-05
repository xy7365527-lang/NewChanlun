# OpenClaw API 接入架构设计

## 设计范围

本文档定义系统与 OpenClaw Gateway 之间的 API 接口规格，涵盖四类 Agent 的工具接口、资源管理协议、错误处理策略、渐进信任实现。

**认识论等级**：L0（纯架构设计，不涉及数据验证）

**前置依赖**：安全审计报告 `docs/architecture/openclaw-security-audit.md`（R3 产出，已通过）

---

## 一、接口总览

### 1.1 通信协议

```
[核心回路]
    |
    | HTTPS (TLS 1.3) + Bearer Token
    |
[OpenClaw Gateway :18789 (loopback)]
    |
    +--- REST API: /api/v1/*       (同步请求-响应)
    +--- WebSocket: /ws            (事件推送)
    +--- SSE: /api/v1/events/stream (单向事件流，WebSocket 的降级方案)
```

所有 API 调用遵循统一信封格式：

**请求头**：
```
Authorization: Bearer <token-via-SecretRef>
Content-Type: application/json
X-Request-ID: <uuid>
X-Trust-Phase: <0-5>
```

**响应信封**：
```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "meta": {
    "request_id": "<uuid>",
    "timestamp": "<ISO-8601>",
    "trust_phase": 3,
    "rate_limit_remaining": 42
  }
}
```

**错误响应**：
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "BUDGET_EXCEEDED",
    "message": "Monthly budget limit reached",
    "detail": { "current_spend": 85.20, "limit": 100.00, "currency": "USD" }
  },
  "meta": { ... }
}
```

### 1.2 Agent 隔离拓扑

每个 Agent 拥有独立的 credential scope 和工具白名单，Agent 之间不可互相调用工具。
隔离由 Gateway 的 `sandbox.scope: "agent"` 强制执行，不依赖 Agent 自律。

```
Gateway
├── resource-agent    scope: cloud-provider-api
│     tools: [resource-query, resource-scale, budget-query]
│     credentials: [cloud-sts-token]
│
├── trade-agent       scope: exchange-api
│     tools: [market-data, order-submit, order-cancel, position-query]
│     credentials: [exchange-api-key (read+trade, no withdraw)]
│
├── publish-agent     scope: content-platform-api
│     tools: [content-draft, content-submit, content-status]
│     credentials: [platform-oauth-token]
│
└── genealogy-agent   scope: local-filesystem (readonly except genealogy paths)
      tools: [genealogy-read, genealogy-write, genealogy-query]
      credentials: none (local only)
```

---

## 二、资源管理 Agent 工具接口

### 2.1 工具定义

#### `resource-query` — 资源状态查询

```json
{
  "tool": "resource-query",
  "requireApproval": false,
  "trust_phase_min": 0,
  "params": {
    "resource_type": "compute | storage | instance",
    "provider": "string",
    "region": "string (optional)"
  },
  "returns": {
    "resources": [
      {
        "id": "string",
        "type": "compute | storage | instance",
        "status": "running | stopped | provisioning",
        "specs": { "cpu": "string", "memory": "string", "disk": "string" },
        "cost_per_hour": "number",
        "uptime_hours": "number"
      }
    ],
    "total_monthly_cost": "number",
    "budget_remaining": "number"
  }
}
```

#### `budget-query` — 预算状态查询

```json
{
  "tool": "budget-query",
  "requireApproval": false,
  "trust_phase_min": 0,
  "params": {},
  "returns": {
    "monthly_limit": "number (hardcoded, agent cannot modify)",
    "current_spend": "number",
    "remaining": "number",
    "currency": "string",
    "spend_by_category": {
      "compute": "number",
      "storage": "number",
      "network": "number"
    },
    "projected_month_end": "number",
    "days_remaining": "number"
  }
}
```

#### `resource-scale` — 资源扩张/收缩

```json
{
  "tool": "resource-scale",
  "requireApproval": "dynamic (see §2.2)",
  "trust_phase_min": 4,
  "params": {
    "action": "scale_up | scale_down",
    "resource_type": "compute | storage | instance",
    "target_id": "string (existing resource ID, required for scale_down)",
    "specs": {
      "cpu": "string (optional)",
      "memory": "string (optional)",
      "disk": "string (optional)",
      "instance_count": "number (optional)"
    },
    "justification": "string (required — Agent must explain why)"
  },
  "pre_checks": [
    "budget_remaining >= estimated_cost",
    "action == scale_down → backup_verified == true",
    "action == scale_down → resource is NOT genealogy storage (immutable)"
  ],
  "returns": {
    "operation_id": "string",
    "status": "approved | pending_approval | rejected | executing | completed | failed",
    "estimated_cost_delta": "number (monthly)",
    "approval_required": "boolean",
    "approval_tier": "auto | orchestrator | orchestrator_with_cooldown"
  }
}
```

### 2.2 分级审批协议

审批门槛直接映射安全审计 §2.5 的分级设计：

| 条件 | 审批层级 | `requireApproval` | 行为 |
|------|---------|-------------------|------|
| 查询操作 | 无 | `false` | 直接返回 |
| scale_up 且 cost_delta < 月预算 5% | auto | `false` | 自动执行 + 事后通知 |
| scale_up 且 5% <= cost_delta < 20% | orchestrator | `true` | 暂停，发送审批请求到紧急通道 |
| scale_up 且 cost_delta >= 20% | orchestrator_with_cooldown | `true` | 暂停 + 24h 冷却期 |
| 任何 scale_down | orchestrator | `true` | 暂停，发送审批请求 + 备份验证 |
| 新服务/新区域 | forbidden | N/A | Gateway 层拒绝，不到达 Agent |

**硬限制（Gateway 层强制，Agent 无权覆盖）**：
- `monthly_budget_limit`：硬编码于 Gateway 配置文件，API 不暴露修改接口
- `single_operation_cap`：单次操作不超过月预算 X%（X 由 Gateway 配置）
- `genealogy_storage_immutable`：谱系数据存储标记为 immutable，scale_down 操作跳过

### 2.3 事件通知

资源变更产生事件，通过 WebSocket 或 SSE 推送到核心回路：

```json
{
  "event_type": "resource.scale_up.completed | resource.scale_down.completed | resource.budget.warning | resource.budget.exceeded",
  "timestamp": "<ISO-8601>",
  "payload": {
    "operation_id": "string",
    "resource_id": "string",
    "cost_delta": "number",
    "new_budget_remaining": "number"
  }
}
```

`resource.budget.warning` 在剩余预算低于 20% 时触发。
`resource.budget.exceeded` 触发后，所有 scale_up 操作被 Gateway 自动拒绝。

---

## 三、交易执行 Agent 工具接口

### 3.1 工具定义

#### `market-data` — 市场数据查询

```json
{
  "tool": "market-data",
  "requireApproval": false,
  "trust_phase_min": 0,
  "params": {
    "symbol": "string",
    "interval": "1m | 5m | 15m | 30m | 1h | 4h | 1d",
    "limit": "number (max 1000)"
  },
  "returns": {
    "candles": [
      {
        "timestamp": "<ISO-8601>",
        "open": "number",
        "high": "number",
        "low": "number",
        "close": "number",
        "volume": "number"
      }
    ]
  }
}
```

#### `position-query` — 持仓查询

```json
{
  "tool": "position-query",
  "requireApproval": false,
  "trust_phase_min": 0,
  "params": {
    "symbol": "string (optional, all if omitted)"
  },
  "returns": {
    "positions": [
      {
        "symbol": "string",
        "side": "long | short | none",
        "size": "number",
        "entry_price": "number",
        "unrealized_pnl": "number",
        "liquidation_price": "number | null"
      }
    ],
    "total_equity": "number",
    "available_margin": "number"
  }
}
```

#### `order-submit` — 下单

```json
{
  "tool": "order-submit",
  "requireApproval": "dynamic (see §3.2)",
  "trust_phase_min": 2,
  "params": {
    "symbol": "string",
    "side": "buy | sell",
    "type": "limit | market",
    "size": "number",
    "price": "number (required if type == limit)",
    "signal_source": "string (required — 来自哪个策略信号)",
    "signal_id": "string (required — 可追溯到具体的分析事件)"
  },
  "pre_checks": [
    "size * price <= single_order_limit",
    "daily_cumulative + (size * price) <= daily_limit",
    "orders_this_minute < rate_limit_per_minute",
    "signal_source is in approved_strategies[]"
  ],
  "returns": {
    "order_id": "string",
    "status": "submitted | pending_approval | rejected",
    "rejection_reason": "string | null",
    "estimated_fill_price": "number"
  }
}
```

#### `order-cancel` — 撤单

```json
{
  "tool": "order-cancel",
  "requireApproval": false,
  "trust_phase_min": 2,
  "params": {
    "order_id": "string"
  },
  "returns": {
    "order_id": "string",
    "status": "cancelled | not_found | already_filled"
  }
}
```

### 3.2 交易风控层

风控层在 Gateway 侧实施，独立于 Agent 逻辑：

| 风控维度 | 参数 | 位置 | Agent 可修改 |
|---------|------|------|-------------|
| 单笔限额 | `single_order_limit` | Gateway 配置 | 否 |
| 日累计限额 | `daily_limit` | Gateway 配置 | 否 |
| 分钟频率限制 | `rate_limit_per_minute` | Gateway 配置 | 否 |
| 小时频率限制 | `rate_limit_per_hour` | Gateway 配置 | 否 |
| 已批准策略白名单 | `approved_strategies[]` | Gateway 配置 | 否 |
| 杠杆倍数上限 | `max_leverage` | Gateway 配置 | 否 |
| 异常检测 | 偏离策略信号的下单 | Gateway 运行时 | 否 |

**异常检测规则**：
- `signal_source` 不在 `approved_strategies[]` → 拒绝
- 下单方向与 `signal_id` 指向的分析结论矛盾 → 拒绝 + 告警
- 连续 N 笔亏损 → 暂停交易 + 告警（N 由 Gateway 配置）
- 任何涉及杠杆倍数变更的请求 → 拒绝（编排者保留操作）

**熔断机制**：
```json
{
  "circuit_breaker": {
    "trigger": "daily_loss > max_daily_loss OR consecutive_losses > N OR anomaly_detected",
    "action": "halt_all_orders",
    "notification": "emergency_channel (independent of Gateway)",
    "resume": "orchestrator_manual_only"
  }
}
```

### 3.3 交易事件

```json
{
  "event_type": "trade.order.submitted | trade.order.filled | trade.order.cancelled | trade.order.rejected | trade.circuit_breaker.triggered",
  "timestamp": "<ISO-8601>",
  "payload": {
    "order_id": "string",
    "symbol": "string",
    "side": "string",
    "size": "number",
    "price": "number",
    "signal_source": "string",
    "signal_id": "string",
    "pnl": "number | null (filled only)"
  }
}
```

所有交易事件不可删除，写入独立于 Gateway 的审计日志。

---

## 四、外化发布 Agent 工具接口

### 4.1 工具定义

#### `content-draft` — 内容草稿提交

```json
{
  "tool": "content-draft",
  "requireApproval": false,
  "trust_phase_min": 1,
  "params": {
    "platform": "string (target platform identifier)",
    "content_type": "article | analysis | signal | comment",
    "title": "string",
    "body": "string (markdown)",
    "genealogy_refs": ["string (genealogy entry IDs, if relevant)"],
    "auto_publish": false
  },
  "returns": {
    "draft_id": "string",
    "status": "drafted",
    "preview_url": "string | null"
  }
}
```

#### `content-submit` — 内容发布

```json
{
  "tool": "content-submit",
  "requireApproval": true,
  "trust_phase_min": 1,
  "params": {
    "draft_id": "string"
  },
  "pre_checks": [
    "draft exists and status == 'drafted'",
    "content does NOT contain credentials/API keys/internal paths",
    "content does NOT contain unresolved genealogy references (status: 生成态)"
  ],
  "returns": {
    "publish_id": "string",
    "status": "pending_approval | published",
    "platform_url": "string | null"
  }
}
```

**关键约束**：`content-submit` 的 `requireApproval` 始终为 `true`——外化内容必须经编排者审核。
这不是渐进信任可以放开的，因为外化内容一旦发布即不可撤回（与资源管理/交易不同，没有回滚）。

#### `content-status` — 发布状态查询

```json
{
  "tool": "content-status",
  "requireApproval": false,
  "trust_phase_min": 1,
  "params": {
    "publish_id": "string (optional)",
    "platform": "string (optional)"
  },
  "returns": {
    "items": [
      {
        "publish_id": "string",
        "platform": "string",
        "status": "drafted | pending_approval | published | rejected",
        "published_at": "<ISO-8601> | null",
        "platform_url": "string | null"
      }
    ]
  }
}
```

---

## 五、谱系读写 Agent 工具接口

### 5.1 工具定义

#### `genealogy-read` — 谱系条目读取

```json
{
  "tool": "genealogy-read",
  "requireApproval": false,
  "trust_phase_min": 0,
  "params": {
    "entry_id": "string (e.g. '001', '089')",
    "include_body": "boolean (default false)"
  },
  "returns": {
    "entry": {
      "id": "string",
      "title": "string",
      "status": "settled | 生成态 | superseded",
      "created_at": "<ISO-8601>",
      "settled_at": "<ISO-8601> | null",
      "body": "string | null (if include_body)"
    }
  }
}
```

#### `genealogy-query` — 谱系检索

```json
{
  "tool": "genealogy-query",
  "requireApproval": false,
  "trust_phase_min": 0,
  "params": {
    "status_filter": "settled | 生成态 | all",
    "keyword": "string (optional, fulltext search)",
    "referenced_by": "string (optional, entry_id — find entries referencing this one)",
    "limit": "number (default 50, max 200)"
  },
  "returns": {
    "entries": [
      {
        "id": "string",
        "title": "string",
        "status": "string",
        "references": ["string (entry_ids)"]
      }
    ],
    "total_count": "number"
  }
}
```

#### `genealogy-write` — 谱系条目写入

```json
{
  "tool": "genealogy-write",
  "requireApproval": true,
  "trust_phase_min": 0,
  "params": {
    "action": "create | settle | append",
    "entry_id": "string (required for settle/append)",
    "title": "string (required for create)",
    "body": "string",
    "references": ["string (entry_ids)"],
    "status": "生成态 (create default) | settled (only via settle action)"
  },
  "pre_checks": [
    "action == settle → entry.status == '生成态'",
    "action == append → entry.status != 'settled' (settled entries are immutable)",
    "action == create → entry_id is auto-generated (monotonic)"
  ],
  "returns": {
    "entry_id": "string",
    "status": "string",
    "action_performed": "string"
  }
}
```

**关键约束**：
- 已结算（settled）的谱系条目不可修改——这是区块拓扑的不可变性保证
- 写入操作要求 `requireApproval: true`——谱系写入是不可逆的
- 存储路径由 Gateway 配置绑定到 `.chanlun/genealogy/`，Agent 无权访问其他路径

---

## 六、资源管理协议

### 6.1 扩张执行流程

```
[核心回路检测到利润信号]
    |
    v
[resource-agent 调用 budget-query]
    |
    v
[计算 cost_delta vs 月预算百分比]
    |
    +--- < 5% ──→ [resource-scale(scale_up)] ──→ 自动执行 ──→ 事件通知
    |
    +--- 5%-20% ──→ [resource-scale(scale_up)] ──→ Gateway 暂停 ──→ 审批请求发送到紧急通道
    |                                                                    |
    |                                                    编排者批准 ──→ 执行 ──→ 事件通知
    |                                                    编排者拒绝 ──→ rejected 响应
    |
    +--- >= 20% ──→ [resource-scale(scale_up)] ──→ Gateway 暂停 ──→ 审批请求 + 24h 冷却
    |                                                                    |
    |                                              冷却期过 + 编排者批准 ──→ 执行
    |
    +--- 新服务/新区域 ──→ Gateway 层直接拒绝（forbidden）
```

### 6.2 收缩执行流程

```
[核心回路检测到亏损信号 OR 资源利用率低于阈值]
    |
    v
[resource-agent 调用 resource-query 评估当前资源]
    |
    v
[确认目标资源不是谱系存储（genealogy_storage_immutable 检查）]
    |
    +--- 是谱系存储 ──→ 拒绝，谱系数据不可收缩
    |
    +--- 非谱系存储 ──→ [触发备份]
                            |
                            v
                        [备份完成 + 校验通过]
                            |
                            v
                        [resource-scale(scale_down)] ──→ Gateway 暂停 ──→ 审批请求
                                                                            |
                                                            编排者批准 ──→ 执行 ──→ 事件通知
                                                            编排者拒绝 ──→ rejected 响应
```

**收缩不可逆性保护**：
- 备份必须在独立存储位置（非待收缩资源本身）
- 备份校验必须包含完整性检查（checksum 比对）
- Gateway 记录 `scale_down` 操作的完整快照（资源状态 before/after），支持恢复审计

### 6.3 月度预算硬限制实现

```yaml
# Gateway 配置（Agent 不可访问此配置文件）
resource_management:
  monthly_budget_limit: 100.00       # USD, hardcoded
  single_operation_cap_percent: 10   # 单次操作不超过月预算的 10%
  budget_warning_threshold: 0.20     # 剩余 20% 时告警
  currency: USD
  genealogy_storage_paths:           # 不可收缩的路径
    - ".chanlun/genealogy/"
    - ".chanlun/gangmu.yaml"
```

预算限制的执行链：
1. Agent 调用 `resource-scale` → Gateway 查询当前 `current_spend`
2. `current_spend + estimated_cost > monthly_budget_limit` → 拒绝，返回 `BUDGET_EXCEEDED`
3. `estimated_cost > monthly_budget_limit * single_operation_cap_percent / 100` → 拒绝，返回 `SINGLE_OP_CAP_EXCEEDED`
4. Agent 无法修改 `monthly_budget_limit`——API 不暴露该字段的写入接口，配置文件以只读挂载

---

## 七、错误处理策略

### 7.1 Gateway 不可达

| 场景 | 检测方式 | 降级行为 |
|------|---------|---------|
| Gateway 进程崩溃 | 心跳超时（30s 无响应） | 核心回路切换为只读模式，所有写操作暂停 |
| 网络断开 | TCP 连接失败 | 同上 |
| Gateway 启动中 | HTTP 503 | 指数退避重试（1s, 2s, 4s, 8s, max 60s），5 分钟后告警 |
| TLS 证书过期 | TLS handshake failure | 拒绝连接，告警到紧急通道，不降级为 HTTP |

**降级模式（只读）**：
- 核心回路继续运行（缠论分析、谱系推理不依赖 Gateway）
- 所有外部写操作暂停（不下单、不扩缩资源、不发布内容）
- 已有持仓的止损/止盈由交易所侧条件单保护（不依赖 Gateway）
- Gateway 恢复后，核心回路不自动恢复写操作——需编排者显式确认

### 7.2 Agent 异常熔断

```json
{
  "circuit_breaker_config": {
    "resource-agent": {
      "max_errors_per_hour": 5,
      "error_types": ["PROVIDER_ERROR", "TIMEOUT", "UNEXPECTED_RESPONSE"],
      "action": "suspend_agent",
      "notification": "emergency_channel",
      "resume": "orchestrator_manual"
    },
    "trade-agent": {
      "max_errors_per_hour": 3,
      "error_types": ["EXCHANGE_ERROR", "ORDER_REJECTED", "ANOMALY_DETECTED"],
      "action": "suspend_agent + cancel_pending_orders",
      "notification": "emergency_channel",
      "resume": "orchestrator_manual"
    },
    "publish-agent": {
      "max_errors_per_hour": 10,
      "error_types": ["PLATFORM_ERROR", "CONTENT_REJECTED"],
      "action": "suspend_agent",
      "notification": "log_only",
      "resume": "auto_after_cooldown(1h)"
    },
    "genealogy-agent": {
      "max_errors_per_hour": 5,
      "error_types": ["FS_ERROR", "CORRUPTION_DETECTED"],
      "action": "suspend_agent + readonly_fallback",
      "notification": "emergency_channel",
      "resume": "orchestrator_manual"
    }
  }
}
```

**熔断层级**：
1. **工具级熔断**：单个工具连续失败 N 次 → 该工具暂停，Agent 其他工具可用
2. **Agent 级熔断**：Agent 小时内错误超限 → 整个 Agent 暂停
3. **Gateway 级熔断**：Gateway 自身异常 → 所有 Agent 暂停，降级为只读

### 7.3 交易执行失败回滚

交易操作的回滚策略取决于失败时机：

| 失败阶段 | 状态 | 回滚策略 |
|---------|------|---------|
| 下单前（pre_check 失败） | 未提交 | 无操作——订单从未到达交易所 |
| 下单中（网络超时） | 未知 | 查询订单状态：存在则记录，不存在则标记失败 |
| 下单后（交易所拒绝） | 拒绝 | 记录拒绝原因，分析是否为风控触发 |
| 部分成交 | 部分填充 | 不自动撤销剩余——剩余部分按原策略处理 |
| 成交后（止损触发） | 已填充 | 止损由交易所条件单执行，不经过 Gateway |

**关键原则**：交易操作不存在"通用回滚"——每笔交易的状态转换是不可逆的。
回滚 = 开反向仓位，这本身是一个新的交易决策，必须重新经过信号验证和风控层。

---

## 八、渐进信任模型 API 层实现

### 8.1 信任阶段与工具权限映射

信任阶段由 Gateway 配置 `trust_phase` 控制，Agent 无权修改。

| 阶段 | trust_phase | 可用工具 | requireApproval 覆盖 |
|------|------------|---------|---------------------|
| 0: 只读 | 0 | resource-query, budget-query, market-data, position-query, genealogy-read, genealogy-query | 所有写工具 blocked |
| 1: 监控 | 1 | 阶段0 + content-draft, content-submit, genealogy-write | content-submit: true, genealogy-write: true |
| 2: 小额交易 | 2 | 阶段1 + order-submit, order-cancel | order-submit: true (single_order_limit 取最小值) |
| 3: 交易自动化 | 3 | 阶段2 全部 | order-submit: dynamic (小额auto, 大额approval) |
| 4: 资源自动化 | 4 | 阶段3 + resource-scale | resource-scale: dynamic (见 §2.2 分级) |
| 5: 完全自动化 | 5 | 全部 | 仅硬限制生效 (budget cap, rate limit, etc.) |

### 8.2 阶段转换 API

阶段转换不通过 Agent 工具执行——它是 Gateway 管理接口的一部分，仅编排者可访问。

```
POST /api/v1/admin/trust-phase
Authorization: Bearer <admin-token>
{
  "action": "upgrade | downgrade",
  "target_phase": 3,
  "reason": "string",
  "evidence": {
    "days_without_incident": 60,
    "audit_log_reviewed": true,
    "orchestrator_approval": true
  }
}
```

**升级条件（Gateway 强制检查）**：
- `days_without_incident >= phase_min_duration[target_phase]`
- `audit_log_reviewed == true`
- `orchestrator_approval == true`
- 不可跳级：`target_phase == current_phase + 1`

**降级条件**：
- 任何安全事件 → 立即降至阶段 0（Gateway 自动执行，不需编排者触发）
- 降级可跳级（阶段 4 直接降到阶段 0）

### 8.3 每阶段最短持续时间

```yaml
phase_min_duration:
  0: 0    # 审计通过即可升级
  1: 14   # 至少 2 周监控
  2: 28   # 至少 4 周小额交易
  3: 56   # 至少 8 周交易自动化
  4: 112  # 至少 16 周资源自动化
  5: null # 无上限——达到后持续
```

---

## 九、紧急通道

独立于 OpenClaw Gateway 的紧急通道，确保 Gateway 失效时编排者仍可控制系统。

### 9.1 通道定义

```
[编排者设备]
    |
    +--- 紧急通道 A: 独立 HTTP endpoint (不走 Gateway)
    |       |
    |       +--- POST /emergency/halt-all    → 停止所有 Agent
    |       +--- POST /emergency/halt-trade  → 仅停止交易
    |       +--- GET  /emergency/status      → 查询系统状态
    |
    +--- 紧急通道 B: 交易所 Web UI (手动操作)
    |       |
    |       +--- 手动撤销所有挂单
    |       +--- 手动平仓
    |
    +--- 紧急通道 C: 云服务商控制台 (手动操作)
            |
            +--- 停止所有实例
            +--- 撤销 STS token
```

**通道 A 的独立性要求**：
- 不依赖 OpenClaw Gateway 进程
- 不依赖同一 Docker 网络
- 独立认证（不共享 Gateway token）
- 每日心跳检测可达性

---

## 十、审计日志接口

### 10.1 日志写入

所有 Agent 工具调用由 Gateway 自动记录，Agent 无需主动写入。

```json
{
  "log_entry": {
    "timestamp": "<ISO-8601>",
    "agent": "resource-agent | trade-agent | publish-agent | genealogy-agent",
    "tool": "string",
    "params": { "...": "..." },
    "result": { "success": "boolean", "...": "..." },
    "trust_phase": "number",
    "request_id": "string",
    "duration_ms": "number"
  }
}
```

### 10.2 日志存储

- 日志存储独立于 Gateway（安全审计 §4.3 要求）
- Agent 无权读取或修改审计日志
- 操作日志、交易日志、资源变更日志保留期：永久
- 日志不可删除标记由存储层强制执行

---

## 十一、Gateway 配置骨架

以下为 Gateway 配置文件的架构骨架（不含具体 credential 值）：

```yaml
# openclaw-gateway-config.yaml
# Agent 无权访问此文件——以只读方式挂载到容器外部

gateway:
  bind: "loopback"
  port: 18789
  auth:
    mode: "token"
    token: { "$ref": "exec:///path/to/token-provider" }
  tls:
    enabled: true
    cert: "/secrets/tls/cert.pem"
    key: "/secrets/tls/key.pem"
  mdns: "off"
  trustedProxies: []
  allowRealIpFallback: false

agents:
  resource-agent:
    sandbox:
      scope: "agent"
    tools:
      profile: "custom"
      allow: ["resource-query", "budget-query", "resource-scale"]
      deny: ["group:runtime", "group:fs", "group:automation", "group:browser"]
    credentials:
      cloud_provider:
        type: "secretref"
        provider: "exec"
        command: "/secrets/scripts/get-sts-token.sh"

  trade-agent:
    sandbox:
      scope: "agent"
    tools:
      profile: "custom"
      allow: ["market-data", "position-query", "order-submit", "order-cancel"]
      deny: ["group:runtime", "group:fs", "group:automation", "group:browser"]
    credentials:
      exchange:
        type: "secretref"
        provider: "exec"
        command: "/secrets/scripts/get-exchange-key.sh"

  publish-agent:
    sandbox:
      scope: "agent"
    tools:
      profile: "custom"
      allow: ["content-draft", "content-submit", "content-status"]
      deny: ["group:runtime", "group:fs", "group:automation", "group:browser"]
    credentials:
      platform:
        type: "secretref"
        provider: "exec"
        command: "/secrets/scripts/get-platform-token.sh"

  genealogy-agent:
    sandbox:
      scope: "agent"
    tools:
      profile: "custom"
      allow: ["genealogy-read", "genealogy-query", "genealogy-write"]
      deny: ["group:runtime", "group:fs", "group:automation", "group:browser"]
    credentials: {}
    filesystem:
      allow: [".chanlun/genealogy/"]
      mode: "rw"

trust:
  current_phase: 0
  phase_min_duration: { 0: 0, 1: 14, 2: 28, 3: 56, 4: 112 }
  auto_downgrade_on_incident: true
  downgrade_target: 0

risk_management:
  resource:
    monthly_budget_limit: 100.00
    single_operation_cap_percent: 10
    budget_warning_threshold: 0.20
    currency: "USD"
    genealogy_storage_immutable: true

  trade:
    single_order_limit: 100.00
    daily_limit: 500.00
    rate_limit_per_minute: 5
    rate_limit_per_hour: 30
    max_leverage: 1
    max_consecutive_losses_before_halt: 5
    max_daily_loss: 200.00
    approved_strategies: []

  publish:
    require_approval_always: true

circuit_breaker:
  resource-agent: { max_errors_per_hour: 5, resume: "manual" }
  trade-agent: { max_errors_per_hour: 3, resume: "manual" }
  publish-agent: { max_errors_per_hour: 10, resume: "auto_1h" }
  genealogy-agent: { max_errors_per_hour: 5, resume: "manual" }

logging:
  storage: "independent"
  retention:
    operations: "permanent"
    trades: "permanent"
    resource_changes: "permanent"
    auth: "180d"
    anomalies: "permanent"
  immutable: true
```

---

## 十二、边界条件

以下条件下本设计需要修订：

1. OpenClaw Gateway API 协议发生重大变更（当前基于 v2026.2.x）
2. 从单操作者扩展到多操作者（当前设计假设单编排者模型）
3. 需要跨 Agent 协作的新场景出现（当前设计禁止 Agent 间直接通信）
4. 谱系存储从本地文件系统迁移到分布式网络（总方针阶段 E）——需要重新设计 genealogy-agent 的工具接口
5. 交易所 API 权限模型变更导致现有隔离策略失效

## 十三、影响声明

- 本文档满足 gangmu.yaml 中 `openclaw-api-design` 的 `completion_check`
- 解除 `openclaw-resource-protocol` 的 `blocked_by: openclaw-api-design` 阻塞
- 不引入任何 API key 或 credential
- 不修改任何现有代码或配置
- 不产生任何可执行代码——仅做架构设计

## 十四、谱系引用

- 总方针 S3.4：OpenClaw 资源管理层——扩张方向（利润→算力/存储）和收缩方向（亏损→资源收缩）
- 总方针 S八.六：世界接口层分层——OpenClaw 的系统定位
- 总方针 S八.七 阶段 G：资源管理自主化——编排者失去资源管理控制权的路径
- 安全审计 §2.2：权限模型（deny-by-default + 白名单）
- 安全审计 §2.5：分级审批门槛（5%/20% 阈值）
- 安全审计 §4.4：渐进信任模型（6 阶段）
- gangmu `world-interface` gang：`openclaw-api-design` 工位
