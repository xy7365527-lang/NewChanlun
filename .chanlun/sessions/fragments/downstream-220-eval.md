# 220号下游推论实现评估

date: 2026-02-26
source: .chanlun/definitions/chanlun-trading-system.md

## 推论1：risk/ 模块不应存在

**状态：resolved**

`src/newchan/risk/` 目录已删除（commit c6a83c9）。Glob 确认无残留文件。
Grep 确认 `stop_loss|StopLoss|止损` 仅出现在 `backtest.py`（见推论3）。

---

## 推论2：回测引擎需支持短差程序模拟、成本归零追踪、多级别联动

**状态：未实现，且当前代码存在缠论语言封闭性违反**

### 当前状态诊断

`src/newchan/backtest.py` 当前能力：
- 单级别 BSP 信号消费（仅 level=1）
- 简单开平仓（一个 BSP 开仓，反向 BSP 或固定百分比止损平仓）
- `BacktestConfig.stop_loss_pct` — **外部金融工程概念**（固定百分比止损）
- `Trade.exit_reason` 包含 `"stop_loss"` — **外部概念**

缺失能力：
1. **短差程序模拟**：无次级别 BSP 消费，无部分仓位操作
2. **成本归零追踪**：无持仓成本跟踪，无阶段状态机
3. **多级别联动**：`RecursiveOrchestratorSnapshot.recursive_snapshots` 已提供多级别数据，但 `BacktestEngine` 未消费

### 代码设计方案

#### 2.1 新增类型（`src/newchan/backtest.py` 内部）

```python
from enum import Enum
from typing import Literal

class PositionPhase(Enum):
    """仓位阶段（220号概念2）"""
    COST_REDUCTION = "cost_reduction"   # 成本>0，短差降成本
    COST_ZERO = "cost_zero"             # 成本归零（翻倍出半仓触发）
    EARN_SHARES = "earn_shares"         # 成本=0，短差挣股票


@dataclass(frozen=True, slots=True)
class ShortDiffRecord:
    """一次短差操作记录"""
    bar_idx: int
    side: Literal["reduce", "replenish"]  # 减仓 / 回补
    price: float
    quantity: float
    sub_bsp_kind: str       # 次级别 BSP 类型
    sub_bsp_level: int      # 次级别 level_id
    profit: float           # 本次短差利润（reduce时计算）
```

#### 2.2 BacktestConfig 修改

```python
@dataclass(frozen=True, slots=True)
class BacktestConfig:
    operation_level: int = 1          # 操作级别（大级别买卖点）
    short_diff_level: int | None = None  # 短差级别（次级别），None=不启用短差
    allow_short: bool = False
    initial_capital: float = 100_000.0
    position_fraction: float = 0.1    # 短差用机动资金比例（原文1/10）
```

删除项：`stop_loss_pct`（外部概念）

#### 2.3 持仓状态重设计

```python
@dataclass
class _OpenPosition:
    """当前持仓（内部可变）"""
    side: Literal["long", "short"]
    entry_bsp: BuySellPoint           # 完整的入场 BSP（用于退出条件判定）
    entry_bar: int
    entry_price: float
    quantity: float
    cost_basis: float                 # 当前成本基础
    phase: PositionPhase              # 当前阶段
    short_diffs: list[ShortDiffRecord]  # 短差记录
    # 退出条件所需的结构参数（从 entry_bsp 提取）
    exit_params: _ExitParams


@dataclass(frozen=True, slots=True)
class _ExitParams:
    """退出条件参数（按 BSP 类型不同）"""
    bsp_kind: Literal["type1", "type2", "type3"]
    # Type1: 需要监控上涨中是否形成新同级别中枢
    entry_move_zs_count: int | None = None
    # Type2: 需要监控是否跌破前一下跌趋势最低点
    prev_downtrend_low: float | None = None
    # Type3: 需要监控回试是否跌破 ZG
    center_zg: float | None = None
```

#### 2.4 退出条件判定（推论3 的实现位置）

退出条件在 `BacktestEngine.process_snapshot` 中判定，但判定逻辑基于 BSP 系统的结构条件，不是价格百分比：

```python
def _check_exit_condition(
    self, pos: _OpenPosition, snapshot: RecursiveOrchestratorSnapshot, price: float,
) -> Literal["bsp_negated", "reverse_bsp"] | None:
    """检查退出条件（220号概念1）。

    退出条件 = 买入程序的判断条件被否定，不是价格偏离度量。
    """
    ep = pos.exit_params

    if ep.bsp_kind == "type1" and pos.side == "long":
        # Type1 买点退出：上涨中再次形成同级别中枢
        # → 背驰判断的"最后一个缠绕"前提被否定
        current_zs_count = len(snapshot.zs_snapshot.zhongshus)
        if (ep.entry_move_zs_count is not None
                and current_zs_count > ep.entry_move_zs_count):
            return "bsp_negated"

    elif ep.bsp_kind == "type2" and pos.side == "long":
        # Type2 买点退出：跌破前一下跌趋势最低点
        if ep.prev_downtrend_low is not None and price < ep.prev_downtrend_low:
            return "bsp_negated"

    elif ep.bsp_kind == "type3" and pos.side == "long":
        # Type3 买点退出：回试跌破 ZG
        if ep.center_zg is not None and price < ep.center_zg:
            return "bsp_negated"

    return None
```

#### 2.5 短差程序模拟

```python
def _check_short_diff(
    self, pos: _OpenPosition, snapshot: RecursiveOrchestratorSnapshot, price: float,
) -> ShortDiffRecord | None:
    """检查次级别 BSP 信号，执行短差操作（220号概念3）。

    短差级别 = operation_level - 1 的 BSP 信号。
    成本>0阶段：减仓量=回补量（不改变总仓位）
    成本=0阶段：回补金额=减仓金额（仓位增加）
    """
    if self._config.short_diff_level is None:
        return None
    # 从 recursive_snapshots 中找到次级别的 BSP 快照
    # ... 消费次级别 BSP 信号
```

#### 2.6 Trade 类型修改

```python
@dataclass(frozen=True, slots=True)
class Trade:
    # ... 保留现有字段 ...
    exit_reason: Literal["reverse_bsp", "bsp_negated"]  # 删除 "stop_loss"
    phase_at_exit: PositionPhase | None = None
    short_diff_count: int = 0
    cost_reduction: float = 0.0  # 短差累计降成本金额
```

#### 2.7 多级别联动

`RecursiveOrchestratorSnapshot.recursive_snapshots` 已包含 level>=2 的数据。
当前 `BacktestEngine.process_snapshot` 仅消费 `snapshot.bsp_snapshot`（level=1）。

修改：
- 操作级别 BSP → 开平仓决策
- 次级别 BSP → 短差决策（从 `recursive_snapshots` 中按 `short_diff_level` 提取）

### 需要修改的文件

| 文件 | 修改内容 |
|------|---------|
| `src/newchan/backtest.py` | 重写 BacktestConfig（删除 stop_loss_pct）、重写 _OpenPosition、新增 PositionPhase/ShortDiffRecord/_ExitParams、重写 process_snapshot 退出逻辑、新增短差模拟 |
| `src/newchan/backtest.py` Trade | exit_reason 删除 "stop_loss"，新增 phase_at_exit 等字段 |

### 不需要修改的文件

| 文件 | 理由 |
|------|------|
| `src/newchan/a_buysellpoint_v1.py` | BSP 识别逻辑不变，退出条件是消费端逻辑 |
| `src/newchan/core/recursion/buysellpoint_engine.py` | BSP 引擎不变 |
| `src/newchan/orchestrator/recursive.py` | 已提供多级别数据，无需修改 |

---

## 推论3：退出条件应在 BuySellPoint 判定逻辑中

**状态：部分违反**

### 当前状态

- 无独立 StopLoss 类（合规）
- `backtest.py` 第249-256行：固定百分比止损逻辑（违反——外部金融工程概念）
- 退出条件未基于"买入程序判断条件被否定"

### 设计方案

退出条件判定在 `BacktestEngine._check_exit_condition` 中实现（见推论2的2.4节）。
这不是在 `a_buysellpoint_v1.py` 中实现，因为退出条件是走势演进过程中的 walk-forward 判定，
不是 BSP 识别时的静态计算。但退出条件的参数完全来自 BSP 系统（中枢数量、趋势低点、ZG），
不引入任何外部概念。

**关键删除项**：
- `BacktestConfig.stop_loss_pct`（固定百分比止损 → 外部概念）
- `Trade.exit_reason` 中的 `"stop_loss"` 字面量
- `process_snapshot` 中第249-256行的百分比止损逻辑

**替代项**：
- `_ExitParams` 记录每种 BSP 类型的结构性退出条件参数
- `_check_exit_condition` 基于走势结构判定退出
- `Trade.exit_reason` 新增 `"bsp_negated"`（买入程序判断条件被否定）

---

## 推论4：仓位管理应追踪"当前阶段"

**状态：未实现**

### 当前状态

`BacktestEngine` 仅做简单开平仓，无仓位阶段概念，无成本追踪。

### 设计方案

见推论2的2.1节（PositionPhase）和2.3节（_OpenPosition.phase / cost_basis）。

阶段转换逻辑：
```
COST_REDUCTION → COST_ZERO: 当 cost_basis 降至 0（翻倍出半仓或短差累计）
COST_ZERO → EARN_SHARES: 立即转换（归零即进入挣股票阶段）
```

短差行为随阶段变化：
- `COST_REDUCTION`: 减仓量 = 回补量（总仓位不变）
- `EARN_SHARES`: 回补金额 = 减仓金额（仓位增加，成本保持0）

终点：超大级别卖点出现时清仓（由操作级别的 reverse_bsp 触发）。

---

## 总结

| 推论 | 状态 | 行动 |
|------|------|------|
| 1. risk/ 删除 | resolved | 无 |
| 2. 回测引擎短差/归零/多级别 | 未实现 | 重写 backtest.py |
| 3. 退出条件基于BSP系统 | 部分违反（stop_loss_pct） | 删除外部概念，实现结构性退出 |
| 4. 仓位阶段追踪 | 未实现 | 新增 PositionPhase 状态机 |

推论2/3/4 的实现集中在 `src/newchan/backtest.py` 一个文件的重写。
上游引擎（BSP识别、递归编排器）无需修改。

### 缠论语言封闭性检查

当前 `backtest.py` 中的外部金融工程概念（需删除/替换）：
- `stop_loss_pct: float = 0.05` → 删除
- `exit_reason: Literal["reverse_bsp", "stop_loss"]` → 替换为 `Literal["reverse_bsp", "bsp_negated"]`
- `max_drawdown_pct` 属性 → 保留（这是统计指标，不是交易决策依据，不违反封闭性）

### 边界条件

1. 220号定义 status 为"生成态"——退出条件的具体判定逻辑（如 Type1 的"新中枢形成"如何精确检测）可能在后续谱系中修正
2. 短差程序依赖次级别 BSP 数据，而 `recursive_snapshots` 中是否包含次级别 BSP 取决于 RecursiveStack 的实现——需确认
3. T+1 制度下短差的隔日操作约束未在本设计中处理（220号边界条件已声明不覆盖）
