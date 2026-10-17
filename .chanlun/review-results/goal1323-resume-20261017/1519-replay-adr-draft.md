# #1519 ADR 草稿：D4 回放验收系统架构决策

**日期**：2026-10-17  
**状态**：DRAFT  
**绑定票**：#1519（#1304 Phase 0-C）

---

## 背景与动机

### 问题

回测（backtest）最初设计为生产策略的离线验证工具，但在 M1（Milestone 1）范围内遇到确定性障碍：
- 市场数据的时序依赖（订单簿快照、成交时间戳）
- 外部系统状态（IBKR 连接、账户余额）
- 浮点运算的平台差异

这些障碍导致回测无法作为生产验证的可靠 oracle。

### 决策

将回测**重定位为历史回放（replay）**：
- 不追求完全确定性，而是追求**可审计性**
- 回放消费历史 feed（K 线、tick、订单簿快照），重建决策过程
- 对拍回放结果与生产结果，识别偏差

---

## 确定性障碍清单（M1 范围绕行）

| 障碍 | 绕行方案 | 后续 Milestone |
|-----|---------|---------------|
| 订单簿快照缺失 | M1 只回放 K 线级别决策，不回放订单簿 | M2：接入历史订单簿 |
| IBKR 连接状态 | 回放假设连接恒可用，不模拟断线 | M3：fault injection |
| 浮点精度差异 | 对拍时容差 1e-6（资金）/ 1 tick（价格） | M4：固定精度算术 |
| 成交时间戳 | 回放只记录决策时间，不记录成交时间 | M2：tick 级回放 |

---

## 历史 feed 适配方案

### 数据源优先级

1. **已归档 K 线**（`/Volumes/AgentStorage/market-data/klines/`）- 主源
2. **IBKR 历史数据 API**（`reqHistoricalData`）- 补缺
3. **合成 K 线**（从 tick 聚合）- 最后手段

### 适配接口

```rust
pub trait HistoricalFeed {
    fn fetch_klines(&self, symbol: &str, from: DateTime, to: DateTime) -> Vec<Kline>;
    fn fetch_ticks(&self, symbol: &str, from: DateTime, to: DateTime) -> Vec<Tick>;
}
```

实现：
- `ArchivedKlinesFeed`（读 AgentStorage）
- `IbkrHistoricalFeed`（调 IBKR API）
- `SyntheticKlinesFeed`（从 tick 聚合）

---

## 快照/恢复设计（联动 #378/#372）

### 快照内容

```rust
pub struct ReplayCheckpoint {
    pub timestamp: DateTime,
    pub bar_index: usize,
    pub account_book: AccountBook,  // #1517 导出
    pub tower_state: TowerState,    // 缠论结构状态
    pub order_state: OrderState,    // 未成交订单
}
```

### 恢复流程

1. 加载 checkpoint（从 AgentStorage）
2. 恢复 tower_state（通过 #378 接口）
3. 恢复 order_state（通过 #372 接口）
4. 从 checkpoint.bar_index + 1 继续回放

---

## 对拍策略

### 三种对拍模式

| 模式 | 对拍对象 | 用途 |
|-----|---------|-----|
| **Replay vs Backtest** | 回放 vs 旧回测 | 过渡期验证（M1） |
| **Replay vs Live** | 回放 vs 生产 | 偏差检测（M2+） |
| **Replay vs Replay** | 两次回放 | 确定性验证（M3+） |

### 对拍字段（#1518 实施）

```bash
# scripts/replay_compare.sh
diff <(replay export_book) <(backtest export_book)
```

对拍字段：
- 资金（现金、持仓市值、总资产）- 容差 1e-6
- 持仓（symbol、数量、均价）- 容差 1 tick
- 成交（时间、价格、数量、方向）- 精确匹配
- 决策时间戳 - 精确匹配

---

## 决策记录

### Phase 0（当前）

**范围**：
- #1517：export_book() 接口
- #1518：replay vs backtest 对拍脚本
- #1519（本文档）：架构决策记录

**不包括**：
- 快照/恢复实现（归 Phase 1）
- 历史 feed 适配器（归 Phase 1）
- 浏览器可视化（归 Phase 2）

### Phase 1（D4 生产级升级）

- 实现 `ReplayCheckpoint` 序列化/反序列化
- 实现 `ArchivedKlinesFeed`
- 集成 #378（tower 恢复）和 #372（order 恢复）
- 100 bars → 10,000 bars 对拍

### Phase 2（生产集成）

- Replay vs Live 对拍
- 浏览器可视化（对拍结果、偏差报告）
- 自动化回归测试（CI 每日运行）

---

## 关联决策

- **ADR-0010**：Chong 持久操作单元（回放也是一个 Chong）
- **ADR-0017**：#1304 frontier scan（本 ADR 的前置研究）
- **#378**：Tower 恢复接口（回放需要）
- **#372**：Order 恢复接口（回放需要）

---

## 后果

### 正面

- 回测重定位后，不再追求不可达的确定性
- 回放可作为生产验证的**辅助 oracle**（非唯一 oracle）
- 历史数据归档后，可随时回放任意时间段

### 负面

- 对拍容差引入不确定性（1e-6 资金差异可能累积）
- 快照/恢复增加存储开销（每 1,000 bars 一个 checkpoint ≈ 10 MB）
- 历史 feed 适配器维护成本（IBKR API 变更）

### 缓解措施

- Phase 2 引入 Replay vs Replay 对拍（确定性基线）
- checkpoint 自动清理策略（保留最近 30 天）
- IBKR API 适配器加版本检测（API 变更时告警）

---

**最后更新**：2026-10-17  
**下一步**：等待 Phase 0-A（#1517）和 Phase 0-B（#1518）完成后，进入 Phase 1 实施
