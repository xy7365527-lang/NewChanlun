# OpenClaw 资源管理协议

## 协议范围

本文档定义**利润→扩张算力/存储**和**亏损→收缩资源**的形式化协议。
协议覆盖：触发阈值、执行步骤、回滚机制、与实盘闭环的信号对接、降级策略。

**认识论等级**：L0（纯协议设计，不涉及数据验证）

**前置依赖**：
- `docs/architecture/openclaw-security-audit.md`（安全审计 §2.5 分级门槛）
- `docs/architecture/openclaw-api-design.md`（API 架构 §2/§6 资源管理接口）

---

## 一、信号源定义

### 1.1 利润/亏损信号格式

资源管理协议的输入是来自实盘闭环的利润/亏损信号。信号格式：

```json
{
  "signal_type": "pnl_report",
  "timestamp": "<ISO-8601>",
  "period": "daily | weekly | monthly",
  "data": {
    "realized_pnl": "number (已实现盈亏，含手续费)",
    "unrealized_pnl": "number (未实现盈亏)",
    "total_equity": "number (总权益)",
    "equity_peak": "number (历史权益峰值)",
    "drawdown_pct": "number (当前回撤百分比 = (peak - current) / peak)",
    "consecutive_loss_days": "number (连续亏损天数)",
    "win_rate_30d": "number (30日胜率)"
  },
  "source": "trade-agent (via Gateway event stream)"
}
```

### 1.2 资源利用率信号格式

收缩决策的另一输入——资源利用率低于阈值时触发收缩评估：

```json
{
  "signal_type": "utilization_report",
  "timestamp": "<ISO-8601>",
  "period": "daily",
  "data": {
    "resources": [
      {
        "id": "string",
        "type": "compute | storage",
        "utilization_pct": "number (0-100)",
        "cost_per_hour": "number",
        "idle_hours": "number (连续低利用率小时数)"
      }
    ],
    "total_monthly_cost": "number",
    "budget_remaining": "number"
  },
  "source": "resource-agent (via resource-query)"
}
```

### 1.3 信号缺失处理

| 场景 | 检测方式 | 降级行为 |
|------|---------|---------|
| 日报未到达（超过 26h） | 定时检查最后信号时间戳 | 暂停所有扩张决策，保持当前资源规模 |
| 周报/月报未到达 | 同上，按周期计 | 同上 |
| 信号格式异常（字段缺失/类型错误） | JSON schema 校验 | 拒绝该信号，记录错误，等待下一个合法信号 |
| 信号数值异常（equity < 0 或 drawdown > 100%） | 范围校验 | 拒绝该信号 + 发送告警到紧急通道 |

**核心原则**：信号缺失时**不做任何资源变更**——沉默 = 保持现状，不是沉默 = 一切正常。

---

## 二、触发阈值

### 2.1 扩张触发条件

扩张不是对单次利润的反应，而是对**持续利润趋势**的响应。

| 条件编号 | 条件 | 计算方式 | 含义 |
|---------|------|---------|------|
| E1 | 月度已实现利润 > 0 | `sum(daily_realized_pnl, 30d) > 0` | 最低门槛——亏钱不扩张 |
| E2 | 30日滚动权益新高 | `current_equity > equity_peak_30d` | 权益在扩张而非回撤中 |
| E3 | 当前回撤 < 10% | `drawdown_pct < 0.10` | 不在回撤中扩张 |
| E4 | 预算有余量 | `budget_remaining > monthly_budget_limit * 0.30` | 至少 30% 预算余量 |

**扩张触发 = E1 AND E2 AND E3 AND E4**。四个条件全部满足才触发扩张评估。

### 2.2 扩张规模计算

扩张规模与利润幅度关联，但受安全审计 §2.5 的分级门槛约束：

```
expansion_budget = min(
    monthly_realized_pnl * reinvestment_ratio,    # 利润的一部分用于扩张
    budget_remaining * 0.50,                       # 不超过剩余预算的50%
    monthly_budget_limit * single_operation_cap    # 不超过单次操作上限
)
```

其中 `reinvestment_ratio` 由 Gateway 配置，建议初始值 0.20（利润的 20% 用于再投入算力/存储）。

### 2.3 收缩触发条件

收缩是对**持续亏损或资源闲置**的响应。两条独立触发路径：

**路径 A：亏损触发**

| 条件编号 | 条件 | 计算方式 | 含义 |
|---------|------|---------|------|
| C1 | 月度已实现亏损 | `sum(daily_realized_pnl, 30d) < 0` | 在亏钱 |
| C2 | 连续亏损超过阈值 | `consecutive_loss_days >= 7` | 不是偶然波动 |
| C3 | 回撤超过警戒线 | `drawdown_pct >= 0.15` | 回撤较深 |

**亏损收缩触发 = C1 AND (C2 OR C3)**。

**路径 B：闲置触发**

| 条件编号 | 条件 | 计算方式 | 含义 |
|---------|------|---------|------|
| U1 | 资源利用率持续低于阈值 | `utilization_pct < 20% 连续 72h` | 资源闲置 |
| U2 | 该资源不是谱系存储 | `resource.type != genealogy_storage` | 谱系不可收缩 |

**闲置收缩触发 = U1 AND U2**。

### 2.4 阈值性质

所有阈值是**Gateway 配置参数**，Agent 无权修改。编排者可以在 Gateway 配置中调整：

```yaml
# Gateway 配置（Agent 无权访问）
resource_protocol:
  expansion:
    reinvestment_ratio: 0.20
    min_budget_remaining_pct: 0.30
    max_drawdown_for_expansion: 0.10
    require_equity_new_high: true
  contraction:
    consecutive_loss_days_threshold: 7
    drawdown_warning_pct: 0.15
    idle_utilization_pct: 20
    idle_hours_threshold: 72
```

阈值**不是动态自适应的**——资源管理是保守操作，阈值由编排者根据运行经验手动调整。自适应阈值在资源管理域引入了额外的不确定性（Agent 可能"学会"放松阈值），这与安全审计的保守原则矛盾。

---

## 三、扩张执行流程

### 3.1 完整流程

```
[实盘闭环] ──→ pnl_report 信号
    |
    v
[Step 1: 条件评估]
    resource-agent 收到 pnl_report
    检查 E1-E4 是否全部满足
    |
    +── 任一条件不满足 ──→ 记录"扩张条件未达" ──→ 结束
    |
    v
[Step 2: 需求计算]
    调用 budget-query 获取当前预算状态
    计算 expansion_budget
    确定扩张目标：
      - 算力扩张（增加实例/升级规格）？
      - 存储扩张（增加磁盘空间）？
    选择依据：当前瓶颈资源（utilization_report 中利用率最高的类型）
    |
    v
[Step 3: 分级审批]
    计算 cost_delta = expansion_budget 折算为月度增量
    cost_delta_pct = cost_delta / monthly_budget_limit
    |
    +── cost_delta_pct < 5% ──→ auto 层级 ──→ Step 4
    +── 5% <= cost_delta_pct < 20% ──→ orchestrator 层级 ──→ 发送审批请求到紧急通道
    |                                                           |
    |                                           编排者批准 ──→ Step 4
    |                                           编排者拒绝 ──→ Step 7 (记录)
    |
    +── cost_delta_pct >= 20% ──→ orchestrator_with_cooldown ──→ 审批请求 + 24h 冷却
    |                                                               |
    |                                             冷却期过 + 批准 ──→ Step 4
    |                                             编排者拒绝 ──→ Step 7 (记录)
    |
    v
[Step 4: 执行]
    调用 resource-scale(action=scale_up, specs=..., justification=...)
    Gateway 执行前二次校验：
      - budget_remaining >= estimated_cost
      - estimated_cost <= monthly_budget_limit * single_operation_cap_percent / 100
    |
    +── Gateway 拒绝（预算不足/超单次上限）──→ Step 7 (记录拒绝原因)
    |
    v
[Step 5: 验证]
    等待 resource.scale_up.completed 事件
    调用 resource-query 确认新资源状态
    |
    +── 资源状态异常 ──→ Step 6 (回滚)
    |
    v
[Step 6-success: 记录]
    记录扩张操作到审计日志：
      - 触发信号（pnl_report 摘要）
      - 扩张目标（资源类型 + 规格）
      - 成本变化（月度增量）
      - 审批层级 + 审批结果
    发送事后通知（auto 层级）或确认完成（orchestrator 层级）
    结束
    |
[Step 6-rollback: 回滚]
    见 §5.1
    |
[Step 7: 记录拒绝/失败]
    记录完整上下文：
      - 触发信号
      - 失败阶段 + 失败原因
      - 当时的预算状态
    结束
```

### 3.2 每步的输入/输出

| 步骤 | 输入 | 输出 | 失败处理 |
|------|------|------|---------|
| Step 1 | pnl_report | boolean (扩张条件是否满足) | 条件不满足 = 正常终止 |
| Step 2 | budget-query 响应 + utilization_report | expansion_budget + 目标资源类型 | budget-query 失败 → 暂停扩张评估 |
| Step 3 | cost_delta_pct | 审批层级 + 审批结果 | 审批请求发送失败 → 降级为 orchestrator 层级（保守方向） |
| Step 4 | resource-scale 参数 | operation_id + status | Gateway 拒绝 → 记录原因 |
| Step 5 | operation_id | 新资源状态 | 超时(10min) → 查询操作状态，仍 executing → 继续等待(max 30min) → 回滚 |
| Step 6 | 成功/失败 | 审计日志条目 | 日志写入失败 → 告警但不影响资源状态 |

---

## 四、收缩执行流程

### 4.1 完整流程

```
[实盘闭环] ──→ pnl_report (路径A) 或 utilization_report (路径B)
    |
    v
[Step 1: 条件评估]
    路径A：检查 C1 AND (C2 OR C3)
    路径B：检查 U1 AND U2
    |
    +── 条件不满足 ──→ 记录 ──→ 结束
    |
    v
[Step 2: 候选资源识别]
    调用 resource-query 获取全部资源清单
    排除不可收缩资源：
      - 谱系存储（genealogy_storage_immutable = true）
      - 核心回路运行所需最小资源
    |
    +── 无可收缩资源 ──→ 记录"已在最小配置" ──→ 结束
    |
    v
[Step 3: 收缩目标选择]
    路径A（亏损）：按 cost_per_hour 降序，从最贵的非核心资源开始
    路径B（闲置）：直接收缩 idle_hours 超阈值的资源
    计算 cost_saving = 收缩后月度节省
    |
    v
[Step 4: 备份]
    对目标资源上的数据执行备份：
      - 备份到独立存储位置（非目标资源本身）
      - 备份完成后执行完整性校验（checksum）
    |
    +── 备份失败 ──→ 记录 ──→ 终止收缩（不冒数据丢失风险）
    +── 校验失败 ──→ 记录 ──→ 终止收缩
    |
    v
[Step 5: 审批]
    所有收缩操作 requireApproval = true（安全审计 §2.5）
    发送审批请求到紧急通道：
      - 收缩目标（资源 ID + 类型 + 规格）
      - 备份位置 + checksum
      - 预计节省金额
      - 触发原因（亏损/闲置）
    |
    +── 编排者拒绝 ──→ 记录 ──→ 结束
    |
    v
[Step 6: 执行]
    调用 resource-scale(action=scale_down, target_id=..., justification=...)
    Gateway 执行前校验：
      - backup_verified == true
      - 目标不是谱系存储
    |
    +── Gateway 拒绝 ──→ 记录 ──→ 结束
    |
    v
[Step 7: 验证]
    等待 resource.scale_down.completed 事件
    确认资源已释放
    确认备份仍可访问（防止备份与资源在同一基础设施上）
    |
    +── 验证失败 ──→ Step 8 (回滚)
    |
    v
[Step 8-success: 记录]
    审计日志：触发信号、收缩目标、节省金额、备份位置
    结束
    |
[Step 8-rollback: 回滚]
    见 §5.2
```

### 4.2 每步的输入/输出

| 步骤 | 输入 | 输出 | 失败处理 |
|------|------|------|---------|
| Step 1 | pnl_report / utilization_report | boolean (收缩条件是否满足) | 条件不满足 = 正常终止 |
| Step 2 | resource-query 响应 | 可收缩资源列表 | 查询失败 → 暂停收缩评估 |
| Step 3 | 可收缩资源列表 | 收缩目标 + cost_saving | 无候选 = 正常终止 |
| Step 4 | 收缩目标 | 备份位置 + checksum | 备份失败 → 终止收缩 |
| Step 5 | 审批请求 | 批准/拒绝 | 审批请求发送失败 → 终止收缩（保守方向） |
| Step 6 | resource-scale 参数 | operation_id + status | Gateway 拒绝 → 记录原因 |
| Step 7 | operation_id | 资源释放确认 + 备份可达确认 | 超时 → 查询状态 → 回滚 |

---

## 五、回滚机制

### 5.1 扩张失败回滚

扩张失败 = 已申请的资源未正常就绪或成本异常。

```
[扩张执行异常]
    |
    v
[Step R1: 诊断]
    调用 resource-query 确认异常资源状态
    |
    +── 资源处于 provisioning（仍在初始化）──→ 等待（max 30min）──→ 超时则继续 R2
    +── 资源处于 error 状态 ──→ 继续 R2
    +── 资源已 running 但成本异常 ──→ 继续 R2
    |
    v
[Step R2: 释放]
    调用 resource-scale(action=scale_down, target_id=新申请的资源ID)
    此时 scale_down 的审批走快速通道（回滚操作 = 释放刚申请的资源，不涉及已有数据）
    |
    +── 释放成功 ──→ Step R3
    +── 释放失败 ──→ 告警到紧急通道 + 记录，编排者手动处理
    |
    v
[Step R3: 记录]
    审计日志：
      - 原始扩张操作 ID
      - 失败原因
      - 回滚操作 ID
      - 资源最终状态
      - 净成本影响（已产生的费用，即使资源已释放）
```

**扩张回滚的简单性**：扩张失败只需要释放新申请的资源。没有数据需要迁移（新资源上还没有数据），没有状态需要恢复。唯一的损失是短暂计费（provisioning 到释放之间的时间）。

### 5.2 收缩失败回滚

收缩失败 = 资源释放过程中出现异常，或释放后发现数据丢失。

```
[收缩执行异常]
    |
    v
[Step R1: 诊断]
    调用 resource-query 确认资源状态
    |
    +── 资源仍存在（scale_down 未完成）──→ 资源仍可用，无需恢复
    +── 资源已释放但备份完好 ──→ Step R2（从备份恢复）
    +── 资源已释放且备份异常 ──→ 告警到紧急通道（最严重场景）
    |
    v
[Step R2: 从备份恢复]
    申请同规格新资源（走扩张流程，但标记为 recovery 操作）
    从备份位置恢复数据
    校验恢复数据的完整性（checksum 比对）
    |
    +── 恢复成功 ──→ Step R3
    +── 恢复失败 ──→ 告警到紧急通道，编排者手动处理
    |
    v
[Step R3: 记录]
    审计日志：
      - 原始收缩操作 ID
      - 失败原因
      - 恢复操作 ID
      - 恢复后资源状态
      - 数据完整性校验结果
      - 净成本影响
```

### 5.3 不可回滚的保护边界

| 资源类型 | 回滚可能性 | 原因 |
|---------|-----------|------|
| 计算实例 | 完全可回滚 | 无状态，重新申请即可 |
| 块存储（有备份） | 可回滚 | 从备份恢复 |
| 块存储（备份失败） | 不可回滚 | 数据已丢失——这就是收缩前备份验证的意义 |
| 谱系存储 | 不允许收缩 | genealogy_storage_immutable = true，Gateway 层硬拒绝 |

---

## 六、谱系数据不可收缩保护

### 6.1 保护机制

谱系数据（`.chanlun/genealogy/` 和 `.chanlun/gangmu.yaml`）在资源管理域享有不可收缩保护：

1. **Gateway 配置层**：`genealogy_storage_paths` 列出所有谱系存储路径，标记为 immutable
2. **resource-scale 预检**：`action == scale_down → resource is NOT genealogy storage`
3. **resource-agent 工具白名单**：resource-agent 无权访问谱系存储路径（由 genealogy-agent 独占）

### 6.2 保护的存在论依据

谱系是区块拓扑的物质载体，区块拓扑是单调递增的。收缩谱系存储 = 删除已结算的概念演化记录 = 破坏系统的生成史。这不是"数据重要所以要保护"（成本收益论证），而是"谱系的存在方式要求单调递增"（存在论约束）。

---

## 七、与实盘闭环的对接协议

### 7.1 数据流

```
[交易执行]
    |
    v
[trade-agent 产生交易事件]
    trade.order.filled → pnl 数据
    |
    v
[Gateway 聚合]
    聚合每日/每周/每月 pnl_report
    推送到 event stream
    |
    v
[resource-agent 消费 pnl_report]
    评估扩张/收缩条件
    |
    v
[资源变更执行]
```

### 7.2 agent 间隔离

resource-agent 和 trade-agent 之间**不直接通信**（API 架构 §1.2 Agent 隔离拓扑）。
pnl_report 由 Gateway 聚合后通过 event stream 发布，resource-agent 作为订阅者消费。

这意味着：
- resource-agent 看到的是**已聚合的 pnl 数据**，不是原始交易事件
- resource-agent 无法干预交易执行（无 order-submit 工具权限）
- trade-agent 无法干预资源管理（无 resource-scale 工具权限）

### 7.3 信号延迟处理

| 延迟类型 | 容忍度 | 处理 |
|---------|--------|------|
| 日报延迟 < 2h | 正常 | 等待 |
| 日报延迟 2h-26h | 异常 | 使用上一日数据，标记为"陈旧数据" |
| 日报延迟 > 26h | 信号缺失 | §1.3 信号缺失处理——暂停资源变更 |
| 利润/亏损数据明显异常 | 零容忍 | 拒绝信号 + 告警 |

### 7.4 月度预算硬限制的执行

月度预算硬限制由 Gateway 配置层强制执行，协议链：

1. `monthly_budget_limit` 硬编码于 Gateway 配置文件（Agent 无权访问）
2. 配置文件以只读方式挂载到容器（Docker `--read-only` + 只读 volume）
3. Gateway API 不暴露 `monthly_budget_limit` 的写入接口
4. resource-agent 每次调用 `resource-scale` 时，Gateway 校验 `current_spend + estimated_cost <= monthly_budget_limit`
5. 校验失败 → 返回 `BUDGET_EXCEEDED` 错误，操作不执行

**预算重置**：每月 1 日 00:00 UTC，Gateway 自动重置 `current_spend = 0`。重置是 Gateway 内部操作，不经过 Agent。

---

## 八、状态机

### 8.1 资源管理状态

```
                    ┌─────────────┐
                    │   IDLE      │ ← 初始状态，等待信号
                    └──────┬──────┘
                           │ pnl_report / utilization_report 到达
                           v
                    ┌─────────────┐
                    │ EVALUATING  │ ← 条件评估中
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │ 条件不满足  │ 条件满足    │
              v            v            v
        ┌─────────┐ ┌───────────┐
        │  IDLE   │ │ APPROVAL  │ ← 等待审批（非 auto 层级）
        └─────────┘ │ _PENDING  │   或直接执行（auto 层级）
                    └─────┬─────┘
                          │
              ┌───────────┼───────────┐
              │ 批准/auto  │ 拒绝      │
              v            v           v
        ┌───────────┐ ┌─────────┐
        │ EXECUTING │ │  IDLE   │
        └─────┬─────┘ └─────────┘
              │
              ├── 成功 ──→ RECORDING ──→ IDLE
              │
              └── 失败 ──→ ROLLING_BACK ──→ RECORDING ──→ IDLE
```

### 8.2 状态转换不变量

- IDLE → EVALUATING：仅由合法信号触发，信号缺失时停留在 IDLE
- EVALUATING → APPROVAL_PENDING / IDLE：评估结果二选一，不存在中间态
- APPROVAL_PENDING → EXECUTING / IDLE：审批结果二选一，超时视为拒绝
- EXECUTING → RECORDING / ROLLING_BACK：执行完成或失败，不存在无限等待（max 30min timeout）
- ROLLING_BACK → RECORDING：回滚完成后必须记录
- RECORDING → IDLE：记录后回到空闲

---

## 九、审批请求格式

### 9.1 扩张审批请求

发送到紧急通道（独立于 Gateway）的审批请求格式：

```json
{
  "type": "resource_approval_request",
  "action": "scale_up",
  "urgency": "normal | high",
  "request_id": "<uuid>",
  "timestamp": "<ISO-8601>",
  "details": {
    "resource_type": "compute | storage",
    "current_specs": { "...": "..." },
    "target_specs": { "...": "..." },
    "estimated_monthly_cost_delta": "number",
    "cost_delta_pct_of_budget": "number",
    "approval_tier": "orchestrator | orchestrator_with_cooldown",
    "cooldown_expires_at": "<ISO-8601> | null"
  },
  "justification": {
    "trigger": "pnl_report 摘要",
    "expansion_conditions": { "E1": true, "E2": true, "E3": true, "E4": true },
    "budget_status": { "remaining": "number", "limit": "number" }
  },
  "response_options": {
    "approve": "POST /api/v1/admin/approval/<request_id>/approve",
    "reject": "POST /api/v1/admin/approval/<request_id>/reject"
  }
}
```

### 9.2 收缩审批请求

```json
{
  "type": "resource_approval_request",
  "action": "scale_down",
  "urgency": "normal",
  "request_id": "<uuid>",
  "timestamp": "<ISO-8601>",
  "details": {
    "target_resource_id": "string",
    "resource_type": "compute | storage",
    "current_specs": { "...": "..." },
    "estimated_monthly_saving": "number"
  },
  "justification": {
    "trigger": "loss (pnl_report) | idle (utilization_report)",
    "contraction_conditions": { "C1": true, "C2": false, "C3": true },
    "budget_status": { "remaining": "number", "limit": "number" }
  },
  "backup": {
    "location": "string",
    "checksum": "string",
    "verified": true,
    "backup_timestamp": "<ISO-8601>"
  },
  "response_options": {
    "approve": "POST /api/v1/admin/approval/<request_id>/approve",
    "reject": "POST /api/v1/admin/approval/<request_id>/reject"
  }
}
```

### 9.3 审批超时

| 审批层级 | 超时时间 | 超时行为 |
|---------|---------|---------|
| orchestrator（扩张） | 48h | 视为拒绝，记录 |
| orchestrator_with_cooldown（扩张） | 24h 冷却 + 48h | 冷却期后开始计时，超时视为拒绝 |
| orchestrator（收缩） | 72h | 视为拒绝，记录（收缩不紧急，给更长时间） |

**超时 = 拒绝**（保守原则）。不自动重发审批请求——如果编排者没回应，说明当前不适合做资源变更。

---

## 十、与安全审计分级门槛的一致性映射

本协议的分级门槛直接映射安全审计 §2.5 和 API 架构 §2.2，无偏差：

| 安全审计 §2.5 | API 架构 §2.2 | 本协议 |
|--------------|--------------|--------|
| 查询：全自动 | requireApproval: false | IDLE → EVALUATING 不需审批 |
| 小规模扩张 (< 5%)：自动 + 事后通知 | auto | Step 3 → auto 层级 |
| 中规模扩张 (5%-20%)：需人工确认 | orchestrator | Step 3 → orchestrator 层级 |
| 大规模扩张 (> 20%)：需确认 + 24h 冷却 | orchestrator_with_cooldown | Step 3 → orchestrator_with_cooldown 层级 |
| 任何收缩：需确认 + 备份验证 | orchestrator | Step 5 → 所有收缩走 orchestrator |
| 新服务/新区域：禁止自动化 | forbidden | 不在本协议范围——Gateway 层直接拒绝 |

---

## 十一、边界条件

以下条件下本协议需要修订：

1. **多交易策略并行**——当前假设单一策略的聚合 pnl，多策略需要按策略拆分 pnl 再聚合
2. **多云服务商**——当前假设单一云服务商，多服务商需要扩展 resource-query 的 provider 参数和预算分配逻辑
3. **币种变化**——当前假设 USD 单币种计价，涉及非 USD 资产时需要汇率转换层
4. **渐进信任阶段 < 4**——资源自动化在 trust_phase >= 4 时才可用（API 架构 §8.1），此前资源变更需编排者手动执行
5. **Gateway 不可达**——降级为只读模式（API 架构 §7.1），本协议的所有流程暂停

## 十二、影响声明

- 本文档满足 gangmu.yaml 中 `openclaw-resource-protocol` 的 `completion_check`（`file_exists: docs/architecture/openclaw-resource-protocol.md`）
- 不引入任何 API key 或 credential
- 不产生任何可执行代码——仅做协议定义
- 不修改任何现有文档或代码
- 协议中的所有阈值、参数、工具名称与 `openclaw-api-design.md` 的接口定义完全一致
- 协议中的所有分级门槛与 `openclaw-security-audit.md` §2.5 完全一致

## 十三、谱系引用

- 总方针 S3.4：OpenClaw 资源管理层——"扩张方向（利润→算力/存储）"和"收缩方向（亏损→资源收缩）"
- 总方针 S八.七 阶段 G：资源管理自主化——编排者失去资源管理控制权的路径
- 安全审计 §2.5：分级门槛（5%/20%/收缩=必须审批）
- API 架构 §2.2：分级审批协议（auto/orchestrator/orchestrator_with_cooldown）
- API 架构 §6：资源管理协议（扩张/收缩执行流程的 API 层实现）
- API 架构 §8.1：渐进信任模型——资源自动化在 trust_phase >= 4 时可用
- gangmu `openclaw-integration` mu：`openclaw-resource-protocol` 工位
