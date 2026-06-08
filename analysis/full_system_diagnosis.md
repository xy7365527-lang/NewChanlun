# 全市场端到端回测：组合 vs 单标的不一致诊断

## 症状

| 标的 | 单标的回测 | 组合回测 | 差距 |
|------|-----------|---------|------|
| OKLO | +934.5%（做空版）/ +652.3%（纯做多版） | -140.6% | >1000pp |
| BTC | +486.6%（3年） | -99.9% | >580pp |

## 根因分析

### Bug 1：空头降成本方向错误（确认根因）

**位置**：`full_system_backtest.py` → `run_fsm()` → COST_REDUCING / PRINCIPAL_WITHDRAWN 状态

**现象**：降成本利润对 `cost_basis` 的修改方向不区分多空：

```python
# 修复前（错误）
cost_basis -= profit / total_shares  # 多空通用

# 修复后（正确）
if direction == 1:
    cost_basis -= profit / total_shares   # 多头：降低成本基础 → 更多盈利
else:
    cost_basis += profit / total_shares   # 空头：提高有效入场价 → 更多盈利
```

**机制**：
- 多头 PnL = `(exit_price - cost_basis) / entry_price * 100`：cost_basis ↓ = PnL ↑ ✓
- 空头 PnL = `(cost_basis - exit_price) / entry_price * 100`：cost_basis ↓ = PnL ↓ ✗

每次"成功"的降成本操作反而**恶化**空头持仓的盈亏。在强势上涨标的（OKLO、BTC）上做空本就困难，bug 导致降成本机制反向运作，加速亏损。

**验证**：单标的版本 `fugue_with_short_backtest_1min.py` 的 `_close_short()` 正确使用 `cost_basis += profit / short_shares`（line 1103）。

### 结构差异 2：FSM 架构完全不同（设计差异，非 bug）

| 维度 | 组合版 `full_system_backtest.py` | 单标的版 `fugue_with_short_backtest_1min.py` |
|------|--------------------------------|---------------------------------------------|
| PH 级别 | 仅 L0（tree_down/tree_up） | L0 + L1 + L2 三级别 |
| 方向控制 | 无（L0 rank-1 settle 即可入场） | L2 PH 方向翻转（l2_direction） |
| 段间比较 | 无 | 38课 S7（不创新高/盘整背驰） |
| 区间套 | 无 | WAIT_SUB_ENTRY / WAIT_SUB_EXIT |
| 降成本触发 | L0 non-rank-1 settle | L1 non-rank-1 settle |
| 走势完成判断 | 无 | MoveSettleV1 + persistence 门槛 |
| BSP 管线 | 手动 BiEngine + 重算窗口 | RecursiveOrchestrator 增量 |
| 状态数 | 4 (SCANNING/OPEN/REDUCING/PW) | 14 (WAIT_ENTRY→EXIT + 5个 SHORT_*) |

**影响**：没有 L2 方向控制意味着组合版 FSM 可以在强上升趋势中自由进入空头——这在 OKLO（长期暴涨）和 BTC（3年大牛市）上是灾难性的。

### 结构差异 3：BSP 管线产出差异

组合版使用手动 BSP 管线（BiEngine → zhongshu → moves → divergences → buysellpoints），每次 stroke 数量变化时重新计算整个窗口。单标的版使用 RecursiveOrchestrator 增量产出 BSP 事件。两者产出的买卖点时机可能不同。

## 修复内容

1. **已修复**：`cost_basis` 的方向性调整（2处）
2. **未修改**（设计层面）：FSM 架构差异属于两版本不同的策略选择，不是 bug

## 修复后结果

| 标的 | 修复前 | 修复后 | BH | 交易数 |
|------|--------|--------|-----|--------|
| OKLO | -140.6% | **+65178.29%** | +251.3% | 144 (60多/84空) |
| BTC | -99.9% | **+15021.13%** | +345.3% | 333 (115多/218空) |
| QQQ (WITH K4) | — | +86.91% | +174.6% | 311 (311多/0空) |
| QQQ (WITHOUT K4) | — | +86.91% | +174.6% | 311 (311多/0空) |
| HK700 (WITH K4) | — | +1.32% | +51.6% | 52 (52多/0空) |
| HK700 (WITHOUT K4) | — | +108.88% | +51.6% | 130 (46多/84空) |

**修复效果确认**：OKLO 从 -140.6% → +65178%，BTC 从 -99.9% → +15021%。负收益彻底消除。

### 新观察：L0 级别降成本导致收益膨胀

修复后 OKLO +65178% 和 BTC +15021% 虽然方向正确，但数值异常高。原因：

1. **组合版使用 L0 PH 触发降成本**（每 1min bar 更新），单标的版使用 L1 PH（仅在线段 settle 时更新）
2. L0 级别事件密度极高 → 每笔交易可产生数十到数百次降成本循环（如 OKLO 第7笔交易：155次短差）
3. 无交易成本/滑点假设下，这些微利润复利膨胀

**示例**：OKLO 第7笔交易，做多 $9.99 入场，$7.86 出场（价格下跌 21%），但 155 次降成本循环将 cost_basis 压低到约 $7.04，最终 PnL 为 **+8.20%**。

**认识论评估**：这一观察不改变修复的正确性（bug 确实存在），但提示 L0 级别降成本在零成本假设下产出的数字不可作为策略评估依据。如需可比性，应统一 PH 触发级别或加入交易成本。

### K4 选股效果

| 对比 | WITH K4 | WITHOUT K4 | K4 增量 |
|------|---------|-----------|---------|
| QQQ | +86.91% | +86.91% | 0%（σ(E/$) 全程 ≥0） |
| HK700 | +1.32% | +108.88% | -107.56%（K4 过度限制） |
| 组合 | +20071.91% | +20098.80% | -26.89% |

K4 对 QQQ 无影响（ES 全程看多）。K4 严重限制了 HK700（从 +108.88% 降到 +1.32%）。

## 结果包（简化版）

**结论**：组合回测严重负收益的主要根因是空头降成本方向错误（Bug 1），叠加无 L2 方向过滤（设计差异 2）导致大量逆势空单。修复后收益恢复为正，但 L0 级别降成本产出的数值需加交易成本才可评估。

**边界条件**：
- 修复后数值在零成本假设下显著膨胀，不可直接与单标的版（L1 级别降成本）比较
- K4 选股层对 HK700 过度限制，需调参或加入更多配置状态

**影响声明**：修改 `full_system_backtest.py` 中 `run_fsm()` 的 2 处 `cost_basis` 调整逻辑，不影响其他模块。修复后回测详细报告见 `analysis/full_system_backtest_v2.md`。
