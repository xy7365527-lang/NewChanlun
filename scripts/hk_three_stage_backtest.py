#!/usr/bin/env python3
"""三阶段持仓回测 — HSI / SHCOMP / BRN 日线 C/D 组 + A/B 对比。

四组对比：
  A组：纯缠论全仓进出（lag=1，无杠杆，无短差）
  B组：A + PH settle 5-bar 门控（同 hk_index_backtest.py B 组语义）
  C组：cost_reduction_fsm 三阶段持仓（lag=1，2x 杠杆，次级别短差）
  D组：C + PH settle 门控（sublevel settle 确认建仓）

信号来源：analysis/data_cache/{sym}_daily_chanlun.json
价格序列：从 labels 重建（bar_idx → price，100% 覆盖，与买卖点天然对齐）
yfinance：仅用于 buy-and-hold 基准（无需逐 bar 对齐）

级别划分（缠论语义）：
  1买/1卖 = 操作级别（主级别，level=0）
  2买/3买/2卖/3卖 = 次级别（level=1）

认识论等级：L2（真实数据，单标的单时段，可产生否定性结果）

谱系引用：
  267号 操作方法论 v1（满仓满融降成本）
  cost_reduction_fsm.py §CostReductionFSM
  a_online_persistence.py OnlineMergeTree（PH settle 门控）
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

_REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_REPO / "src"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.trading.cost_reduction_fsm import (  # noqa: E402
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
)

# ════════════════════════════════════════════════════════════════
# 配置
# ════════════════════════════════════════════════════════════════

DATA_DIR = _REPO / "analysis" / "data_cache"
OUTPUT_MD = _REPO / "analysis" / "hk_three_stage_backtest.md"

EXECUTION_LAG = 1       # 因果延迟（pivot 后第 1 根收盘成交）
OWN_CAPITAL = 100_000.0
LEVERAGE = 2.0          # 满仓满融
SUB_RATIO = 0.3         # 短差仓位比例
FRICTION = 0.0005       # 单次摩擦成本（5bp）
SETTLE_WINDOW = 5       # PH settle 门控窗口（bar 数）

SYMBOLS = [
    {"name": "hsi",    "label": "恒生指数",  "yf_ticker": "^HSI",       "currency": "HKD"},
    {"name": "shcomp", "label": "上证指数",  "yf_ticker": "000001.SS",  "currency": "CNY"},
    {"name": "brn",    "label": "Brent原油", "yf_ticker": "BZ=F",       "currency": "USD"},
]


# ════════════════════════════════════════════════════════════════
# 1. 数据加载与价格序列重建
# ════════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class BspSignal:
    bar: int
    price: float         # pivot bar 收盘（TV 标注价）
    side: str            # "buy" / "sell"
    level: int           # 0=主级别, 1=次级别
    raw_type: str        # "1买" / "2卖" 等


_BSP_KEYWORDS = ["1买", "2买", "3买", "1卖", "2卖", "3卖"]


def load_labels(sym_name: str) -> list[dict]:
    path = DATA_DIR / f"{sym_name}_daily_chanlun.json"
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    return data.get("pine", {}).get("labels", [])


def reconstruct_prices(labels: list[dict]) -> list[float]:
    """从全部带价 labels 重建逐 bar 收盘序列（前值填充）。"""
    by_bar: dict[int, float] = {}
    for lbl in labels:
        if lbl.get("price") is not None and lbl.get("time") is not None:
            by_bar[int(lbl["time"])] = float(lbl["price"])
    if not by_bar:
        raise ValueError("无有效价格标签")
    n = max(by_bar) + 1
    first = by_bar[min(by_bar)]
    series: list[float] = []
    last = first
    for t in range(n):
        if t in by_bar:
            last = by_bar[t]
        series.append(last)
    return series


def parse_bsps(labels: list[dict]) -> list[BspSignal]:
    """解析 labels → BspSignal 列表，按 bar 排序。"""
    bsps: list[BspSignal] = []
    for lbl in labels:
        text = lbl.get("text") or ""
        price = lbl.get("price")
        bar = lbl.get("time")
        if price is None or bar is None:
            continue
        matched = next((k for k in _BSP_KEYWORDS if k in text), None)
        if matched is None:
            continue
        side = "buy" if "买" in matched else "sell"
        level = 0 if matched.startswith("1") else 1
        bsps.append(BspSignal(
            bar=int(bar), price=float(price),
            side=side, level=level, raw_type=matched,
        ))
    bsps.sort(key=lambda b: (b.bar, b.level))
    return bsps


def diag_bsps(bsps: list[BspSignal], prices: list[float]) -> dict:
    return {
        "total": len(bsps),
        "main": sum(1 for b in bsps if b.level == 0),
        "sub": sum(1 for b in bsps if b.level == 1),
        "type_counts": dict(Counter(b.raw_type for b in bsps)),
        "bar_range": f"{min(b.bar for b in bsps)}~{max(b.bar for b in bsps)}",
        "price_range": f"{min(prices):.1f}~{max(prices):.1f}",
        "n_bars": len(prices),
    }


# ════════════════════════════════════════════════════════════════
# 2. PH settle 时间线（A/B/D 组共用）
# ════════════════════════════════════════════════════════════════


def build_settle_timeline(prices: list[float]) -> list[int]:
    """逐 bar 推进 OnlineMergeTree，记录 merge 事件的 bar 索引。"""
    tree = OnlineMergeTree()
    indices: list[int] = []
    for i, p in enumerate(prices):
        if tree.update(float(p)):
            indices.append(i)
    return indices


def has_settle_near(bar: int, settle_indices: list[int], window: int = SETTLE_WINDOW) -> bool:
    lo = bar - window
    return any(lo <= s <= bar for s in settle_indices)


# ════════════════════════════════════════════════════════════════
# 3. A/B 组：简化全仓进出
# ════════════════════════════════════════════════════════════════


@dataclass
class AbTrade:
    entry_bar: int
    entry_price: float
    entry_type: str
    exit_bar: int = -1
    exit_price: float = 0.0
    exit_type: str = ""

    @property
    def closed(self) -> bool:
        return self.exit_bar >= 0

    @property
    def ret(self) -> float:
        if not self.closed:
            return float("nan")
        return (self.exit_price - self.entry_price) / self.entry_price


def run_ab_group(
    bsps: list[BspSignal],
    prices: list[float],
    settle_indices: list[int] | None = None,
    lag: int = EXECUTION_LAG,
) -> list[AbTrade]:
    """A/B 组：任意买点全仓进、任意卖点全仓出。

    settle_indices=None → A 组（不过滤）。
    settle_indices 非空 → B 组（PH settle 门控建仓）。
    """
    n = len(prices)

    def exec_price(bar: int) -> float:
        return prices[min(max(bar + lag, 0), n - 1)]

    trades: list[AbTrade] = []
    position: AbTrade | None = None

    for b in bsps:
        if b.side == "buy":
            if position is not None:
                continue
            if settle_indices is not None and not has_settle_near(b.bar, settle_indices):
                continue
            position = AbTrade(
                entry_bar=b.bar, entry_price=exec_price(b.bar), entry_type=b.raw_type,
            )
        else:
            if position is None:
                continue
            position.exit_bar = b.bar
            position.exit_price = exec_price(b.bar)
            position.exit_type = b.raw_type
            trades.append(position)
            position = None

    # 末尾未平仓：MTM 最后价
    if position is not None:
        position.exit_bar = n - 1
        position.exit_price = prices[-1]
        position.exit_type = "mtm"
        trades.append(position)

    return trades


def metrics_ab(trades: list[AbTrade]) -> dict:
    closed = [t for t in trades if t.closed and t.exit_type != "mtm"]
    all_t = [t for t in trades if t.closed]
    if not all_t:
        return {"n_trades": 0}
    rets = [t.ret for t in all_t]
    wins = [r for r in rets if r > 0]
    compound = 1.0
    equity = [1.0]
    for r in rets:
        compound *= (1 + r)
        equity.append(compound)
    peak, max_dd = 1.0, 0.0
    for e in equity:
        peak = max(peak, e)
        max_dd = max(max_dd, (peak - e) / peak)
    return {
        "n_trades": len(all_t),
        "n_closed": len(closed),
        "win_rate": len(wins) / len(all_t) * 100,
        "compound_return_pct": (compound - 1.0) * 100,
        "avg_ret_pct": sum(rets) / len(rets) * 100,
        "best_pct": max(rets) * 100,
        "worst_pct": min(rets) * 100,
        "max_drawdown_pct": max_dd * 100,
    }


# ════════════════════════════════════════════════════════════════
# 4. C/D 组：cost_reduction_fsm 三阶段持仓
# ════════════════════════════════════════════════════════════════


@dataclass
class CdCycle:
    """一个完整的 FSM 降成本循环。"""
    entry_bar: int
    entry_price: float
    own_capital: float
    leverage: float

    # 跟踪现金 + 持仓
    total_shares: float = field(init=False)
    debt: float = field(init=False)
    cash: float = 0.0                  # 短差利润累计（现金）
    active_short_shares: float = 0.0
    active_short_sell_price: float = 0.0
    short_diff_count: int = 0
    deepest_state: str = "POSITION_OPEN"

    exit_bar: int = -1
    exit_price: float = 0.0

    def __post_init__(self) -> None:
        total_capital = self.own_capital * self.leverage
        self.total_shares = total_capital / self.entry_price
        self.debt = self.own_capital * (self.leverage - 1.0)
        # Entry friction (negative cash = cost paid at open)
        self.cash = -total_capital * FRICTION

    @property
    def remaining_shares(self) -> float:
        return self.total_shares - self.active_short_shares

    @property
    def closed(self) -> bool:
        return self.exit_bar >= 0

    def on_sub_sell(self, sell_price: float) -> None:
        """次级别卖点：开启短差，卖出 sub_ratio * total_shares。"""
        if self.active_short_shares > 0:
            return  # 已有活跃短差，跳过
        sub_shares = self.total_shares * SUB_RATIO
        self.active_short_shares = sub_shares
        self.active_short_sell_price = sell_price
        self.cash += sub_shares * sell_price * (1 - FRICTION)

    def on_sub_buy(self, buy_price: float) -> None:
        """次级别买点：买回，完成短差。"""
        if self.active_short_shares <= 0:
            return  # 无活跃短差
        self.cash -= self.active_short_shares * buy_price * (1 + FRICTION)
        self.short_diff_count += 1
        self.active_short_shares = 0.0
        self.active_short_sell_price = 0.0

    def on_exit(self, exit_bar: int, exit_price: float) -> None:
        """平仓（止损或主级别卖点）。"""
        self.exit_bar = exit_bar
        self.exit_price = exit_price

    @property
    def net_equity(self) -> float:
        """最终权益（扣除融资）。"""
        # 平仓时：remaining_shares + active_short_shares 全部以 exit_price 变现
        # （active_short_shares 若未买回，也在此价平仓）
        liquidation = self.total_shares * self.exit_price * (1 - FRICTION)
        return liquidation + self.cash - self.debt

    @property
    def return_mult(self) -> float:
        return self.net_equity / self.own_capital


def _exec_price(bar: int, prices: list[float], lag: int) -> float:
    n = len(prices)
    return prices[min(max(bar + lag, 0), n - 1)]


def run_cd_group(
    bsps: list[BspSignal],
    prices: list[float],
    settle_indices: list[int] | None = None,
    lag: int = EXECUTION_LAG,
) -> list[CdCycle]:
    """C/D 组：cost_reduction_fsm 三阶段持仓回测。

    settle_indices=None → C 组（不过滤）。
    settle_indices 非空 → D 组（PH settle 门控 1买建仓）。
    """
    n = len(prices)
    fsm = CostReductionFSM.create(
        own_capital=OWN_CAPITAL,
        margin_amount=OWN_CAPITAL * (LEVERAGE - 1.0),
        sub_ratio=SUB_RATIO,
    )
    cycles: list[CdCycle] = []
    current: CdCycle | None = None

    for b in bsps:
        ep = _exec_price(b.bar, prices, lag)

        # ── 主级别买点（1买）→ 建仓 ──
        if b.level == 0 and b.side == "buy":
            if fsm.state != CostState.SCANNING:
                continue  # 已持仓，跳过
            if settle_indices is not None and not has_settle_near(b.bar, settle_indices):
                continue  # D 组 PH 门控不放行
            event = FsmEvent(FsmEventType.BUY_POINT_CONFIRMED, ep, "main")
            fsm = transition(fsm, event)
            current = CdCycle(entry_bar=b.bar, entry_price=ep,
                              own_capital=OWN_CAPITAL, leverage=LEVERAGE)

        # ── 主级别卖点（1卖）→ 退出 ──
        elif b.level == 0 and b.side == "sell":
            if fsm.state == CostState.SCANNING or current is None:
                continue
            current.on_exit(b.bar, ep)
            if fsm.state in (CostState.POSITION_OPEN, CostState.COST_REDUCING):
                # 成本归零前的主级别卖点 = 买入程序否定（267号§四）
                current.deepest_state = fsm.state.name
                event = FsmEvent(FsmEventType.BUY_POINT_NEGATED, ep, "main")
                fsm = transition(fsm, event)
            else:
                # PRINCIPAL_WITHDRAWN → 本金已退出，主级别卖点清仓
                current.deepest_state = "PRINCIPAL_WITHDRAWN"
                event = FsmEvent(FsmEventType.MAIN_LEVEL_SELL_POINT, ep, "main")
                fsm = transition(fsm, event)
            cycles.append(current)
            current = None
            # 自动 RESET（进入下一个 SCANNING 循环）
            if fsm.state == CostState.STOPPED_OUT:
                # 止损后，下个循环本金按上一循环最终净值（此处简化：用固定 OWN_CAPITAL）
                reset_evt = FsmEvent(FsmEventType.RESET, ep, "main",
                                     new_own_capital=OWN_CAPITAL)
                fsm = transition(fsm, reset_evt)

        # ── 次级别卖点（2卖/3卖）→ 开启短差 ──
        elif b.level == 1 and b.side == "sell":
            if fsm.state not in (CostState.POSITION_OPEN, CostState.COST_REDUCING,
                                  CostState.PRINCIPAL_WITHDRAWN) or current is None:
                continue
            try:
                event = FsmEvent(FsmEventType.SUB_LEVEL_SELL_POINT, ep, "sub")
                fsm = transition(fsm, event)
                current.on_sub_sell(ep)
                current.deepest_state = max(current.deepest_state, "COST_REDUCING",
                                            key=_state_rank)
            except Exception:
                pass  # 已有活跃短差，FSM 拒绝，跳过

        # ── 次级别买点（2买/3买）→ 买回短差 ──
        elif b.level == 1 and b.side == "buy":
            if fsm.state not in (CostState.COST_REDUCING,) or current is None:
                continue
            try:
                event = FsmEvent(FsmEventType.SUB_LEVEL_BUY_POINT, ep, "sub")
                fsm = transition(fsm, event)
                current.on_sub_buy(ep)
                if fsm.state == CostState.PRINCIPAL_WITHDRAWN:
                    current.deepest_state = "PRINCIPAL_WITHDRAWN"
            except Exception:
                pass  # 无活跃短差，跳过

    # 末尾未平仓：MTM
    if current is not None and not current.closed:
        current.on_exit(n - 1, prices[-1])
        current.deepest_state = current.deepest_state or fsm.state.name
        cycles.append(current)

    return cycles


def _state_rank(s: str) -> int:
    rank = {"POSITION_OPEN": 0, "COST_REDUCING": 1, "PRINCIPAL_WITHDRAWN": 2,
            "STOPPED_OUT": -1, "idle": -2}
    return rank.get(s, 0)


def metrics_cd(cycles: list[CdCycle]) -> dict:
    if not cycles:
        return {"n_cycles": 0}
    closed = [c for c in cycles if c.closed]
    mults = [c.return_mult for c in closed]
    if not mults:
        return {"n_cycles": 0}
    wins = [m for m in mults if m > 1.0]
    compound = 1.0
    equity = [1.0]
    for m in mults:
        compound *= m
        equity.append(compound)
    peak, max_dd = 1.0, 0.0
    for e in equity:
        peak = max(peak, e)
        max_dd = max(max_dd, (peak - e) / peak)
    reached_cr = sum(1 for c in closed if c.deepest_state in ("COST_REDUCING", "PRINCIPAL_WITHDRAWN"))
    reached_pw = sum(1 for c in closed if c.deepest_state == "PRINCIPAL_WITHDRAWN")
    return {
        "n_cycles": len(closed),
        "win_rate": len(wins) / len(closed) * 100,
        "compound_return_pct": (compound - 1.0) * 100,
        "avg_ret_pct": sum((m - 1.0) for m in mults) / len(mults) * 100,
        "best_pct": (max(mults) - 1.0) * 100,
        "worst_pct": (min(mults) - 1.0) * 100,
        "max_drawdown_pct": max_dd * 100,
        "reached_cost_reducing": reached_cr,
        "reached_principal_withdrawn": reached_pw,
        "total_short_diffs": sum(c.short_diff_count for c in closed),
    }


# ════════════════════════════════════════════════════════════════
# 5. Buy-and-hold 基准（yfinance）
# ════════════════════════════════════════════════════════════════


def yf_bah(ticker: str, n_bars: int) -> tuple[float, str]:
    """拉 yfinance 日线，返回 (总收益%, 说明)。"""
    try:
        import yfinance as yf
        hist = yf.Ticker(ticker).history(period="max", interval="1d", auto_adjust=True)
        closes = hist["Close"].dropna().tolist()
        if len(closes) < 2:
            return float("nan"), "数据不足"
        # 取与 TV bar 数接近的尾部窗口
        window = min(n_bars, len(closes))
        first, last = closes[-window], closes[-1]
        pct = (last / first - 1.0) * 100
        return pct, f"最近 {window} 根日线 ({hist.index[-window].date()} → {hist.index[-1].date()})"
    except Exception as e:
        return float("nan"), f"yfinance 失败: {e}"


# ════════════════════════════════════════════════════════════════
# 6. 报告生成
# ════════════════════════════════════════════════════════════════


def _pct(v: float, dec: int = 1) -> str:
    return "N/A" if v != v else f"{v:+.{dec}f}%"


def _fmt_pct(v: float) -> str:
    return "N/A" if v != v else f"{v:.1f}%"


def build_report(all_results: dict) -> str:
    lines: list[str] = []
    a = lines.append

    a("# 港股三指数 + BRN 三阶段持仓回测 — C/D 组 vs A/B 对比")
    a("")
    a(f"> 自动生成：`scripts/hk_three_stage_backtest.py`")
    a(f"> 认识论等级：**L2**（真实数据，三标的日线，单时段）")
    a(f"> 因果执行：lag={EXECUTION_LAG}（pivot 后第1根收盘成交）")
    a("")
    a("## 方法论说明")
    a("")
    a(f"| 组别 | 系统 | 杠杆 | 短差 | PH 门控 |")
    a(f"|------|------|------|------|--------|")
    a(f"| A | 纯缠论全仓进出（任意买卖点）| 1.0 | 无 | 无 |")
    a(f"| B | A + PH settle {SETTLE_WINDOW}-bar 门控建仓 | 1.0 | 无 | ✓ |")
    a(f"| C | cost_reduction_fsm 三阶段（1买建仓，次级别短差，1卖退出）| {LEVERAGE} | ✓ ({SUB_RATIO:.0%} 仓) | 无 |")
    a(f"| D | C + PH settle {SETTLE_WINDOW}-bar 门控（sublevel settle 确认建仓）| {LEVERAGE} | ✓ | ✓ |")
    a("")
    a(f"**摩擦成本**：{FRICTION*10000:.0f}bp（{FRICTION*100:.2f}%）/ 单次操作（C/D 组）；A/B 组无摩擦。")
    a("")
    a("---")
    a("")

    # 全局汇总表
    a("## 全局汇总")
    a("")
    a("| 标的 | 组别 | 杠杆 | 循环/笔 | 胜率 | 复利总收益 | 最大回撤 | B-H 基准 |")
    a("|------|------|------|---------|------|----------|---------|---------|")

    for sym in SYMBOLS:
        sym_name = sym["name"]
        r = all_results.get(sym_name, {})
        bah = r.get("bah", (float("nan"), "N/A"))
        bah_pct = _pct(bah[0])
        ma = r.get("metrics_a", {})
        mb = r.get("metrics_b", {})
        mc = r.get("metrics_c", {})
        md = r.get("metrics_d", {})

        def row(grp, m, lev):
            n = m.get("n_trades") or m.get("n_cycles") or 0
            wr = _fmt_pct(m.get("win_rate", float("nan")))
            cr = _pct(m.get("compound_return_pct", float("nan")))
            dd = _fmt_pct(m.get("max_drawdown_pct", float("nan")))
            bh = bah_pct if grp == "A" else "—"
            return f"| {sym['label']} | **{grp}** | {lev} | {n} | {wr} | {cr} | {dd} | {bh} |"

        a(row("A", ma, "1.0"))
        a(row("B", mb, "1.0"))
        a(row("C", mc, f"{LEVERAGE}x"))
        a(row("D", md, f"{LEVERAGE}x"))

    a("")
    a("---")
    a("")

    # 每标的详细
    for sym in SYMBOLS:
        sym_name = sym["name"]
        r = all_results.get(sym_name, {})
        if "error" in r:
            a(f"## {sym['label']} ({sym_name.upper()})")
            a(f"> **跳过**：{r['error']}")
            a("")
            continue

        diag = r.get("diag", {})
        bah_pct, bah_note = r.get("bah", (float("nan"), "N/A"))
        ma = r.get("metrics_a", {})
        mb = r.get("metrics_b", {})
        mc = r.get("metrics_c", {})
        md = r.get("metrics_d", {})

        a(f"## {sym['label']} ({sym_name.upper()})")
        a("")
        a(f"**数据**：{diag.get('n_bars')} bars，bar {diag.get('bar_range')}，"
          f"价格区间 {diag.get('price_range')}，货币={sym['currency']}")
        a(f"**买卖点**：{diag.get('total')} 个（主级别 {diag.get('main')} / 次级别 {diag.get('sub')}）")
        a(f"**类型分布**：`{diag.get('type_counts')}`")
        a(f"**settle 事件**：{r.get('settle_events', 0)} 个")
        a(f"**buy-and-hold**：{_pct(bah_pct)}（{bah_note}）")
        a("")
        a("### A/B/C/D 对比")
        a("")
        a("| 指标 | A组 | B组 | C组（2x杠杆） | D组（2x杠杆+PH） |")
        a("|------|-----|-----|------------|----------------|")

        rows_spec = [
            ("循环/笔数",   "n_trades",              "n_cycles",              "d"),
            ("胜率",        "win_rate",               "win_rate",              ".1f%"),
            ("复利总收益",  "compound_return_pct",    "compound_return_pct",   "+.1f%"),
            ("平均每笔",    "avg_ret_pct",            "avg_ret_pct",           "+.2f%"),
            ("最佳",        "best_pct",               "best_pct",              "+.1f%"),
            ("最差",        "worst_pct",              "worst_pct",             "+.1f%"),
            ("最大回撤",    "max_drawdown_pct",       "max_drawdown_pct",      ".1f%"),
        ]

        def fv(d: dict, key: str, fmt: str) -> str:
            key_ab = key if key in d else key.replace("n_cycles", "n_trades").replace("n_trades", "n_cycles")
            v = d.get(key) or d.get(key_ab)
            if v is None:
                return "—"
            if fmt == "d":
                return str(int(v))
            try:
                if fmt.endswith("%"):
                    return f"{v:{fmt[:-1]}}%"
                return f"{v:{fmt}}"
            except Exception:
                return str(v)

        for label, key_ab, key_cd, fmt in rows_spec:
            va = fv(ma, key_ab, fmt)
            vb = fv(mb, key_ab, fmt)
            vc = fv(mc, key_cd, fmt)
            vd = fv(md, key_cd, fmt)
            a(f"| {label} | {va} | {vb} | {vc} | {vd} |")

        # C/D 额外指标
        if mc.get("n_cycles", 0) > 0:
            a(f"| 达到COST_REDUCING | — | — | {mc.get('reached_cost_reducing', 0)} | {md.get('reached_cost_reducing', 0)} |")
            a(f"| 达到PRINCIPAL_WITHDRAWN | — | — | {mc.get('reached_principal_withdrawn', 0)} | {md.get('reached_principal_withdrawn', 0)} |")
            a(f"| 短差回补总次数 | — | — | {mc.get('total_short_diffs', 0)} | {md.get('total_short_diffs', 0)} |")

        a("")

    # 边界条件
    a("---")
    a("")
    a("## 边界条件（结论翻转条件）")
    a("")
    a("1. **C/D 组主级别卖点语义**：成本归零前（POSITION_OPEN/COST_REDUCING）的1卖触发"
      "BUY_POINT_NEGATED（买入程序被否定），不是正向清仓。若改为正向清仓（不止损），"
      "C/D 组收益结构翻转。（依据：267号§四；position_manager.py L430 同构）")
    a("2. **PH 门控方向**：D 组 has_settle_near 基于重建价格序列的 OnlineMergeTree，"
      f"窗口={SETTLE_WINDOW} bar。PH settle 的幅度必要条件适用于下跌腿的底部结构；"
      "若行情为单边上涨，settle 频率增高，D≈C（门控形同虚设）。")
    a("3. **价格序列**：重建自 TV labels（bar_idx→price，100% 覆盖）。与 yfinance 无逐 bar"
      "对齐关系，不受复权口径影响，但与 TV 指标定义的买卖点 bar 天然对齐。")
    a(f"4. **杠杆与摩擦**：C/D leverage={LEVERAGE}，friction={FRICTION*10000:.0f}bp"
      f"（{FRICTION*100:.2f}%）。满融放大双向波动；若摩擦升至 20bp（期货滑点），C/D 短差利润被显著侵蚀。")
    a("5. **短差完整性**：若序列末尾有未买回的活跃短差，以最后价平仓（MTM）。"
      "实战中短差未完成是否应持仓等待，依据行情判断。")
    a("")
    a("## 下游推论")
    a("")
    a("- 若 C > A（含摩擦）：满仓满融 + 次级别短差在日线级别有正风险调整贡献（L2 确认，"
      "不外推到其他标的/时段）。")
    a("- 若 D > C：PH settle 门控在此时段有效过滤假1买（否定性必要条件生效）。")
    a("- 若 C < A 或 D < C：杠杆回撤放大效果 > 短差降成本效果，或 PH 误杀真买点，"
      "是**否定性结果**（L2 更有价值）。")
    a("")
    a("## 谱系引用")
    a("")
    a("- 267号操作方法论 v1 / cost_reduction_fsm.py§CostReductionFSM")
    a("- a_online_persistence.py OnlineMergeTree（PH 在线 merge tree）")
    a("- 220号缠论交易策略体系")
    a("- 形式化有效域规则：本结论 L2，三标的日线，未交叉验证（L3 需多标的多时段）")
    a("")
    a("## 影响声明")
    a("")
    a("- 新增 `scripts/hk_three_stage_backtest.py`、`analysis/hk_three_stage_backtest.md`")
    a("- 未改动 cost_reduction_fsm.py / a_online_persistence.py / hk_index_backtest.py")

    return "\n".join(lines)


# ════════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════════


def run_sym(sym: dict) -> dict:
    sym_name = sym["name"]
    print(f"\n{'='*55}")
    print(f"[★] {sym['label']} ({sym_name})")

    try:
        labels = load_labels(sym_name)
    except Exception as e:
        return {"error": str(e)}

    prices = reconstruct_prices(labels)
    bsps = parse_bsps(labels)
    if len(bsps) < 4:
        return {"error": f"信号数不足: {len(bsps)}"}

    diag = diag_bsps(bsps, prices)
    settle_indices = build_settle_timeline(prices)

    print(f"  bars={diag['n_bars']}, bsp={diag['total']} "
          f"(主={diag['main']}/次={diag['sub']}), settle={len(settle_indices)}")

    # A/B 组
    trades_a = run_ab_group(bsps, prices)
    trades_b = run_ab_group(bsps, prices, settle_indices=settle_indices)
    ma = metrics_ab(trades_a)
    mb = metrics_ab(trades_b)

    # C/D 组
    cycles_c = run_cd_group(bsps, prices)
    cycles_d = run_cd_group(bsps, prices, settle_indices=settle_indices)
    mc = metrics_cd(cycles_c)
    md = metrics_cd(cycles_d)

    # Buy-and-hold
    bah = yf_bah(sym["yf_ticker"], diag["n_bars"])

    print(f"  A: trades={ma.get('n_trades',0)} win={ma.get('win_rate',0):.0f}% ret={ma.get('compound_return_pct',0):+.1f}%")
    print(f"  B: trades={mb.get('n_trades',0)} win={mb.get('win_rate',0):.0f}% ret={mb.get('compound_return_pct',0):+.1f}%")
    print(f"  C: cycles={mc.get('n_cycles',0)} win={mc.get('win_rate',0):.0f}% ret={mc.get('compound_return_pct',0):+.1f}%")
    print(f"  D: cycles={md.get('n_cycles',0)} win={md.get('win_rate',0):.0f}% ret={md.get('compound_return_pct',0):+.1f}%")
    print(f"  B-H: {bah[0]:+.1f}% ({bah[1]})")

    return {
        "diag": diag,
        "settle_events": len(settle_indices),
        "metrics_a": ma,
        "metrics_b": mb,
        "metrics_c": mc,
        "metrics_d": md,
        "bah": bah,
    }


def main() -> None:
    print("=== 港股三指数 + BRN 三阶段持仓回测 ===")
    all_results: dict = {}

    for sym in SYMBOLS:
        all_results[sym["name"]] = run_sym(sym)

    report = build_report(all_results)
    OUTPUT_MD.write_text(report, encoding="utf-8")
    print(f"\n[✓] 报告已写入 {OUTPUT_MD.relative_to(_REPO)}")


if __name__ == "__main__":
    main()
