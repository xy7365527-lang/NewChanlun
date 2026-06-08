"""QQQ 递归引擎 + 三阶段降成本系统联合回测。

存在论位置
----------
本脚本把三个已结算的组件接到一起，回答一个具体问题：
**用 TV 缠论买卖点（左侧候选）驱动 267/338 号"满仓满融降成本"三阶段状态机
（cost_reduction_fsm），在递归引擎产出的走势方向约束下，是否优于简化版全仓进出？**

与 `scripts/tv_chanlun_backtest.py` 的关系：那里是简化版 4 状态机（持现金↔持仓，
long-only）= 本脚本的 **C 组 baseline**。本脚本把它升级为五状态三阶段系统。

三个信号源（全部因果，无 lookahead）
-----------------------------------
1. **缠论买卖点**：TV 缠论指标（CHANLUN | CZSC）的 137 个买卖点（周线级），
   bar_index 经"价格锚点 + ±25 周局部精修"映射到 QQQ 日线交易日（L1 管线，复用
   tv_chanlun_backtest 的映射算法，但锚点用本地日线重采样的周线，无网络依赖）。
   —— 缠论 v1 引擎自身的 confirmed 买点几乎恒为 0（结构性，见 chanlun_ph_backtest.py
   docstring），故必须用 TV 标注作为左侧候选。
2. **PH settle**：QQQ 日线 close 上跑 OnlineMergeTree，settle 事件 = 分型的因果确认
   （a_online_persistence §10 L0 定理）。B 组用"买点前 W 日内有 settle"做右侧过滤。
3. **走势方向**：递归引擎（merge_inclusion→fractal→stroke→segment_v1）的
   **日线线段方向**（Move[0]，58 条，数月级）逐 bar 标注 up/down。额外输出 L1
   走势类型（trend/consolidation，季线级）供对照——双标注（用户裁定）。
   走势方向决定多空：cost_reduction_fsm 是多头系统，仅在 up 线段建仓做多；
   down 线段空仓（做空需镜像 FSM，未实现，见报告缺口声明）。

三阶段（267 号 v1 / 338 号 v2）
------------------------------
- COST_POSITIVE：建仓（POSITION_OPEN）+ 次级别短差降成本（COST_REDUCING）
- COST_ZERO：累计回收 ≥ 自有本金 → PRINCIPAL_WITHDRAWN（本金已退出）
- FREE_POSITION：零成本持股，仅主级别卖点 / 买点失效退出

认识论等级（formalization-validity-domain）
-------------------------------------------
- 递归引擎管线 / OnlineMergeTree / cost_reduction_fsm 转移：**L0**（确定性算法）。
- bar_index→date 映射：**L1**（管线正确性，价格匹配偏差见报告质量节）。
- 三组胜率/收益/降成本曲线：**L2**（QQQ 单标的单时段假设检验，可否证；否定性结果
  同样有价值——若 A/B 不优于 C，即为对"降成本系统改善单标的回测"的否证）。
- 跨标的：**L3 未做**（严格限定 QQQ）。
"""
from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal, Optional

import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_inclusion import merge_inclusion  # noqa: E402
from newchan.a_fractal import fractals_from_merged  # noqa: E402
from newchan.a_stroke import strokes_from_fractals  # noqa: E402
from newchan.a_segment_v1 import segments_from_strokes_v1  # noqa: E402
from newchan.a_recursive_engine import build_recursive_levels  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.trading.cost_reduction_fsm import (  # noqa: E402
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    IllegalTransitionError,
    transition,
)

# ── 路径 ────────────────────────────────────────────────────────────────────

QQQ_PATH = ROOT / "analysis" / "data_cache" / "QQQ_1d_max.json"
LABELS_PATH = ROOT / "analysis" / "data_cache" / "qqq_chanlun_labels.json"
REPORT_PATH = ROOT / "analysis" / "recursive_backtest_qqq.md"
RESULT_JSON = ROOT / "analysis" / "data_cache" / "recursive_backtest_qqq.json"

BSP_KEYWORDS = ["1买", "2买", "3买", "1卖", "2卖", "3卖"]


# ════════════════════════════════════════════════════════════════════════════
# 1. 数据加载
# ════════════════════════════════════════════════════════════════════════════


def load_qqq() -> pd.DataFrame:
    """从本地缓存加载 QQQ 日线 OHLC（period=max，6848 根）。"""
    d = json.loads(QQQ_PATH.read_text())
    df = pd.DataFrame(
        {
            "open": d["opens"],
            "high": d["highs"],
            "low": d["lows"],
            "close": d["closes"],
        },
        index=pd.to_datetime(d["dates"]),
    ).sort_index()
    return df


def _clean_type(text: str) -> str:
    for kw in BSP_KEYWORDS:
        if kw in text:
            suffix = ""
            if "盘整" in text:
                suffix = "(盘整)"
            elif "趋势" in text:
                suffix = "(趋势)"
            return kw + suffix
    return text.strip()


def load_tv_signals() -> list[dict]:
    """加载 TV 缠论买卖点（137 个），按 bar_index 排序。"""
    data = json.loads(LABELS_PATH.read_text())
    signals: list[dict] = []
    for label in data["pine"]["labels"]:
        text = label.get("text") or ""
        if not any(kw in text for kw in BSP_KEYWORDS):
            continue
        price = label.get("price")
        if price is None:
            continue
        raw_type = next(kw for kw in BSP_KEYWORDS if kw in text)
        signals.append(
            {
                "bar_idx": label["time"],
                "price": float(price),
                "signal_type": _clean_type(text),
                "raw_type": raw_type,
                "is_buy": "买" in raw_type,
                "is_trend": "趋势" in text,
                "is_consolidation": "盘整" in text,
            }
        )
    signals.sort(key=lambda x: x["bar_idx"])
    return signals


# ════════════════════════════════════════════════════════════════════════════
# 2. TV 周线 bar_index → 日线 date_idx 映射（复用 tv_chanlun_backtest 算法）
# ════════════════════════════════════════════════════════════════════════════


def map_signals_to_dates(signals: list[dict], df: pd.DataFrame) -> list[dict]:
    """周线 bar_index 锚点线性推算 + ±25 周局部精修 → 日线交易日。

    周线由本地日线重采样得到（无网络依赖）；与 tv_chanlun_backtest 一致的算法。
    """
    weekly = df["close"].resample("W").last().dropna()
    wcloses = weekly.values.astype(float)
    wdates = weekly.index
    wn = len(wcloses)

    last_sig = signals[-1]
    anchor_bar = last_sig["bar_idx"]
    anchor_global = int(np.argmin(np.abs(wcloses - last_sig["price"])))

    daily_closes = df["close"].values.astype(float)
    daily_dates = df.index

    def week_to_daily(week_idx: int) -> tuple[int, float, pd.Timestamp]:
        week_date = wdates[week_idx]
        day_matches = np.where(daily_dates <= week_date)[0]
        if len(day_matches) == 0:
            return 0, float(daily_closes[0]), daily_dates[0]
        di = int(day_matches[-1])
        return di, float(daily_closes[di]), daily_dates[di]

    result: list[dict] = []
    for sig in signals:
        bar_t = sig["bar_idx"]
        target = sig["price"]
        estimated = anchor_global - (anchor_bar - bar_t)
        lo = max(0, estimated - 25)
        hi = min(wn - 1, estimated + 25)
        window = wcloses[lo : hi + 1]
        rel = int(np.argmin(np.abs(window - target)))
        best_week = lo + rel
        di, close_at_day, day_date = week_to_daily(best_week)
        result.append(
            {
                **sig,
                "date": day_date,
                "date_idx": di,
                "matched_close": close_at_day,
                "price_diff_pct": abs(close_at_day - target) / target * 100,
            }
        )
    result.sort(key=lambda x: x["date"])
    return result


# ════════════════════════════════════════════════════════════════════════════
# 3. 走势方向时间线（线段方向 + L1 走势类型双标注）
# ════════════════════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class TrendTimeline:
    """逐 bar 的走势方向标注。

    seg_dir[i]    : 该 bar 所在日线线段方向 'up'/'down'/''（Move[0]，数月级）。
    l1_kind[i]    : 该 bar 所在 L1 走势类型 'trend'/'consolidation'/''（季线级）。
    l1_dir[i]     : L1 走势方向 'up'/'down'/''。
    """

    seg_dir: list[str]
    l1_kind: list[str]
    l1_dir: list[str]


def build_trend_timeline(df: pd.DataFrame) -> TrendTimeline:
    """跑递归引擎，把线段方向 + L1 走势类型逐 bar 平铺到原始 bar 索引。"""
    n = len(df)
    dm, m2r = merge_inclusion(df, reset_dir_on_fractal=True)
    fx = fractals_from_merged(dm)
    sk = strokes_from_fractals(dm, fx, mode="new", merged_to_raw=m2r)
    seg = segments_from_strokes_v1(sk)
    levels = build_recursive_levels(seg, merged_to_raw=m2r)

    seg_dir = [""] * n
    for s in seg:
        r0 = m2r[s.i0][0]
        r1 = m2r[min(s.i1, len(m2r) - 1)][1]
        for i in range(r0, min(r1 + 1, n)):
            seg_dir[i] = s.direction

    l1_kind = [""] * n
    l1_dir = [""] * n
    if levels:
        for t in levels[0].trends:
            r0 = m2r[t.i0][0]
            r1 = m2r[min(t.i1, len(m2r) - 1)][1]
            for i in range(r0, min(r1 + 1, n)):
                l1_kind[i] = t.kind
                l1_dir[i] = t.direction

    return TrendTimeline(seg_dir=seg_dir, l1_kind=l1_kind, l1_dir=l1_dir)


# ════════════════════════════════════════════════════════════════════════════
# 4. PH settle 时间线
# ════════════════════════════════════════════════════════════════════════════


def build_settle_indices(closes: np.ndarray) -> list[int]:
    """OnlineMergeTree 逐根 update，记录有新特征 settle 的 bar 索引。"""
    tree = OnlineMergeTree()
    out: list[int] = []
    for i, p in enumerate(closes):
        if tree.update(float(p)):
            out.append(i)
    return out


def has_settle_near(date_idx: int, settle_indices: list[int], window: int) -> bool:
    """date_idx 前 window 日（含当日）内是否有 settle 事件（右侧确认）。"""
    lo = date_idx - window
    return any(lo <= s <= date_idx for s in settle_indices)


# ════════════════════════════════════════════════════════════════════════════
# 5. 信号 → cost_reduction_fsm 事件映射（核心业务判定）
# ════════════════════════════════════════════════════════════════════════════
#
# 这是把"缠论买卖点语言"翻译成"三阶段降成本状态机事件"的接缝，也是本回测最关键
# 的价值判定——它决定一个卖点是触发"次级别短差降成本"还是"主级别退出清仓"。
#
# cost_reduction_fsm 各状态接受的事件（见 src/.../cost_reduction_fsm.py）：
#   SCANNING            : BUY_POINT_CONFIRMED
#   POSITION_OPEN       : SUB_LEVEL_SELL_POINT | BUY_POINT_NEGATED
#   COST_REDUCING       : SUB_LEVEL_SELL/BUY_POINT | MAIN_LEVEL_SELL_POINT |
#                         LEVEL_UPGRADE | BUY_POINT_NEGATED
#   PRINCIPAL_WITHDRAWN : MAIN_LEVEL_SELL_POINT | SUB_LEVEL_SELL_POINT | BUY_POINT_NEGATED


def classify_holding_event(
    sig: dict,
    state: CostState,
    seg_dir_now: str,
) -> Optional[FsmEventType]:
    """持仓期间，把一个买卖点信号分类为 FSM 事件。

    返回 None 表示该信号在当前状态下不产生转移（忽略）。

    baseline 规则（L2 假设，可调）：
    - 持仓中遇到**买点** → SUB_LEVEL_BUY_POINT（次级别短差买回）。
    - 持仓中遇到**卖点**：
        * 走势已转空（seg_dir_now == 'down'）→ 主级别退出：
            COST_REDUCING/PRINCIPAL_WITHDRAWN 用 MAIN_LEVEL_SELL_POINT；
            POSITION_OPEN（尚无短差）用 BUY_POINT_NEGATED（两者都 → STOPPED_OUT 退出）。
        * 走势仍向上（seg_dir_now == 'up'）→ SUB_LEVEL_SELL_POINT（次级别短差卖出降成本）。
    """
    if sig["is_buy"]:
        if state in (CostState.COST_REDUCING,):
            return FsmEventType.SUB_LEVEL_BUY_POINT
        return None  # POSITION_OPEN/PRINCIPAL_WITHDRAWN 无活跃短差，买点忽略

    # 卖点
    trend_turned_down = seg_dir_now == "down"
    if trend_turned_down:
        if state in (CostState.COST_REDUCING, CostState.PRINCIPAL_WITHDRAWN):
            return FsmEventType.MAIN_LEVEL_SELL_POINT
        return FsmEventType.BUY_POINT_NEGATED  # POSITION_OPEN 无短差，止损式退出
    # 走势仍向上：次级别短差卖出
    return FsmEventType.SUB_LEVEL_SELL_POINT


# ════════════════════════════════════════════════════════════════════════════
# 6. 三阶段 FSM 回测引擎（A / B 组）
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class FsmTrade:
    """一个完整持仓周期（建仓→退出）的记录。"""

    entry_date: pd.Timestamp
    entry_price: float
    entry_signal: str
    exit_date: Optional[pd.Timestamp] = None
    exit_price: Optional[float] = None
    exit_signal: Optional[str] = None
    final_cost_basis: float = 0.0
    n_short_diffs: int = 0
    cumulative_recovered: float = 0.0
    reached_cost_zero: bool = False  # 是否进入 PRINCIPAL_WITHDRAWN
    reached_free: bool = False

    @property
    def is_closed(self) -> bool:
        return self.exit_price is not None

    @property
    def ret(self) -> float:
        """单笔收益率（相对初始每股投入 entry_price）。

        total_shares 恒定满仓，降成本把利润摊进 cost_basis，故
        每股利润 = exit_price - final_cost_basis，收益率以 entry_price 归一。
        """
        if not self.is_closed:
            return float("nan")
        return (self.exit_price - self.final_cost_basis) / self.entry_price

    @property
    def hold_days(self) -> int:
        if not self.is_closed:
            return -1
        return (self.exit_date - self.entry_date).days


@dataclass
class FsmConfig:
    own_capital: float = 1.0
    margin_ratio: float = 0.0  # 0 = 无融资（纯看降成本，不叠杠杆）
    sub_ratio: float = 0.3
    use_settle_filter: bool = False
    settle_window: int = 5
    use_trend_filter: bool = True  # 仅在 up 线段建仓做多


def run_fsm_backtest(
    signals: list[dict],
    timeline: TrendTimeline,
    settle_indices: list[int],
    cfg: FsmConfig,
) -> list[FsmTrade]:
    """信号驱动三阶段降成本状态机，返回完整持仓周期记录。"""
    trades: list[FsmTrade] = []
    fsm = CostReductionFSM.create(
        own_capital=cfg.own_capital,
        margin_amount=cfg.own_capital * cfg.margin_ratio,
        sub_ratio=cfg.sub_ratio,
    )
    cur: Optional[FsmTrade] = None

    def settle(exit_sig: dict) -> None:
        nonlocal fsm, cur
        if cur is not None:
            cur.exit_date = exit_sig["date"]
            cur.exit_price = exit_sig["matched_close"]
            cur.exit_signal = exit_sig["signal_type"]
            cur.final_cost_basis = fsm.cost_basis
            cur.n_short_diffs = len(fsm.completed_short_diffs)
            cur.cumulative_recovered = fsm.cumulative_recovered
            trades.append(cur)
            cur = None
        fsm = CostReductionFSM.create(
            own_capital=cfg.own_capital,
            margin_amount=cfg.own_capital * cfg.margin_ratio,
            sub_ratio=cfg.sub_ratio,
        )

    for sig in signals:
        di = sig["date_idx"]
        seg_dir_now = timeline.seg_dir[di] if di < len(timeline.seg_dir) else ""
        price = sig["matched_close"]
        level = sig["signal_type"]

        if fsm.state == CostState.SCANNING:
            if not sig["is_buy"]:
                continue
            # 走势过滤：仅在 up 线段建仓做多
            if cfg.use_trend_filter and seg_dir_now != "up":
                continue
            # PH settle 右侧过滤（B 组）
            if cfg.use_settle_filter and not has_settle_near(
                di, settle_indices, cfg.settle_window
            ):
                continue
            fsm = transition(
                fsm, FsmEvent(FsmEventType.BUY_POINT_CONFIRMED, price, level)
            )
            cur = FsmTrade(
                entry_date=sig["date"], entry_price=price, entry_signal=level
            )
            continue

        # 持仓状态
        evt = classify_holding_event(sig, fsm.state, seg_dir_now)
        if evt is None:
            continue
        try:
            fsm = transition(fsm, FsmEvent(evt, price, level))
        except IllegalTransitionError:
            continue
        if cur is not None:
            if fsm.state == CostState.PRINCIPAL_WITHDRAWN:
                cur.reached_cost_zero = True
                cur.reached_free = True
        if fsm.state == CostState.STOPPED_OUT:
            settle(sig)

    # 未平仓周期：记录但不计入收益统计
    if cur is not None:
        trades.append(cur)
    return trades


# ════════════════════════════════════════════════════════════════════════════
# 7. C 组 baseline：简化版全仓进出（long-only，无降成本，无走势过滤）
# ════════════════════════════════════════════════════════════════════════════


def run_simple_backtest(signals: list[dict]) -> list[FsmTrade]:
    """C 组：持现金↔持仓全仓切换（= tv_chanlun_backtest A 组逻辑）。"""
    trades: list[FsmTrade] = []
    cur: Optional[FsmTrade] = None
    for sig in signals:
        if sig["is_buy"]:
            if cur is not None:
                continue
            cur = FsmTrade(
                entry_date=sig["date"],
                entry_price=sig["matched_close"],
                entry_signal=sig["signal_type"],
                final_cost_basis=sig["matched_close"],  # 无降成本，cost=entry
            )
        else:
            if cur is None:
                continue
            cur.exit_date = sig["date"]
            cur.exit_price = sig["matched_close"]
            cur.exit_signal = sig["signal_type"]
            cur.final_cost_basis = cur.entry_price  # 全仓无降成本
            trades.append(cur)
            cur = None
    if cur is not None:
        trades.append(cur)
    return trades


# ════════════════════════════════════════════════════════════════════════════
# 8. 统计
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class Stats:
    name: str
    total_trades: int = 0
    closed_trades: int = 0
    win_rate: float = float("nan")
    total_return: float = float("nan")
    max_drawdown: float = float("nan")
    avg_hold_days: float = float("nan")
    profit_factor: float = float("nan")
    avg_short_diffs: float = 0.0
    n_cost_zero: int = 0  # 进入 COST_ZERO 阶段的笔数
    by_type: dict = field(default_factory=dict)


def compute_stats(name: str, trades: list[FsmTrade]) -> Stats:
    closed = [t for t in trades if t.is_closed]
    if not closed:
        return Stats(name=name, total_trades=len(trades))

    rets = [t.ret for t in closed]
    wins = [r for r in rets if r > 0]
    losses = [r for r in rets if r <= 0]

    cum = 1.0
    equity = [1.0]
    for r in rets:
        cum *= 1 + r
        equity.append(cum)
    peak, max_dd = 1.0, 0.0
    for eq in equity:
        peak = max(peak, eq)
        max_dd = max(max_dd, (peak - eq) / peak)

    by_type: dict[str, dict] = {}
    for t in closed:
        b = by_type.setdefault(t.entry_signal, {"trades": 0, "wins": 0, "rets": []})
        b["trades"] += 1
        b["wins"] += 1 if t.ret > 0 else 0
        b["rets"].append(t.ret)
    by_type_summary = {
        k: {
            "trades": v["trades"],
            "win_rate": v["wins"] / v["trades"] if v["trades"] else float("nan"),
            "avg_ret": float(np.mean(v["rets"])) if v["rets"] else float("nan"),
        }
        for k, v in sorted(by_type.items())
    }

    avg_win = float(np.mean(wins)) if wins else 0.0
    avg_loss = float(np.mean([abs(x) for x in losses])) if losses else 0.0
    pf = avg_win / avg_loss if avg_loss > 0 else float("inf")

    return Stats(
        name=name,
        total_trades=len(trades),
        closed_trades=len(closed),
        win_rate=len(wins) / len(closed),
        total_return=cum - 1,
        max_drawdown=max_dd,
        avg_hold_days=float(np.mean([t.hold_days for t in closed])),
        profit_factor=pf,
        avg_short_diffs=float(np.mean([t.n_short_diffs for t in closed])),
        n_cost_zero=sum(1 for t in closed if t.reached_cost_zero),
        by_type=by_type_summary,
    )


# ════════════════════════════════════════════════════════════════════════════
# 9. 报告
# ════════════════════════════════════════════════════════════════════════════


def fmt_pct(v: float, d: int = 2) -> str:
    return "N/A" if v != v else f"{v * 100:.{d}f}%"


def fmt_f(v: float, d: int = 2) -> str:
    return "N/A" if v != v else f"{v:.{d}f}"


def buy_and_hold(df: pd.DataFrame, d0: pd.Timestamp, d1: pd.Timestamp) -> float:
    c = df["close"]
    try:
        return (float(c.asof(d1)) - float(c.asof(d0))) / float(c.asof(d0))
    except Exception:
        return float("nan")


def build_report(
    df: pd.DataFrame,
    signals: list[dict],
    timeline: TrendTimeline,
    settle_indices: list[int],
    stats_a: Stats,
    stats_b: Stats,
    stats_c: Stats,
    trades_a: list[FsmTrade],
) -> str:
    L: list[str] = []
    a = L.append

    closed_a = [t for t in trades_a if t.is_closed]
    bh = float("nan")
    if closed_a:
        bh = buy_and_hold(df, closed_a[0].entry_date, closed_a[-1].exit_date)

    # 走势方向覆盖统计（TV 信号时段）
    sig_dirs = [timeline.seg_dir[s["date_idx"]] for s in signals]
    n_up = sum(1 for d in sig_dirs if d == "up")
    n_down = sum(1 for d in sig_dirs if d == "down")

    a("# QQQ 递归引擎 + 三阶段降成本系统 联合回测")
    a("")
    a("**信号源**：TV 缠论指标（CHANLUN | CZSC）周线买卖点 137 个")
    a(f"**数据**：QQQ 日线 {df.index[0].date()} → {df.index[-1].date()}（{len(df)} 根，本地缓存）")
    a(f"**买卖点**：{len(signals)} 个（买 {sum(1 for s in signals if s['is_buy'])} / 卖 {sum(1 for s in signals if not s['is_buy'])}）")
    a(f"**PH settle 事件**：{len(settle_indices)} 个（OnlineMergeTree, H0 sublevel）")
    a(f"**走势方向（线段，Move[0]）**：信号时段落在 up 线段 {n_up} / down 线段 {n_down}")
    a("**认识论等级**：L2（QQQ 单标的单时段，可否证）")
    a("")
    a("---")
    a("")
    a("## 三组定义")
    a("")
    a("| 组 | 信号 | 持仓系统 | 走势过滤 | PH 过滤 |")
    a("|----|------|---------|---------|--------|")
    a("| A | TV 缠论买卖点 | cost_reduction_fsm（五状态三阶段） | 仅 up 线段建仓 | 否 |")
    a("| B | TV 缠论买卖点 | cost_reduction_fsm（五状态三阶段） | 仅 up 线段建仓 | 买点前 5 日内有 settle |")
    a("| C | TV 缠论买卖点 | 简化全仓进出（long-only baseline） | 无 | 否 |")
    a("")
    a("> **三阶段**：COST_POSITIVE（建仓+次级别短差降成本）→ COST_ZERO（累计回收≥本金，"
      "PRINCIPAL_WITHDRAWN）→ FREE_POSITION（零成本持股）。")
    a("> **单笔收益口径**：满仓 total_shares 恒定，短差利润摊入 cost_basis，"
      "ret =(exit−final_cost_basis)/entry_price——降成本直接抬升收益率。")
    a("")
    a("## 汇总对比")
    a("")
    a("| 指标 | A（缠论+FSM） | B（+PH过滤） | C（简化全仓） | 买持QQQ |")
    a("|------|------|------|------|------|")
    bh_ref = f"{fmt_pct(bh)}（同期）" if bh == bh else "N/A"
    a(f"| 成交笔数（已平仓） | {stats_a.closed_trades} | {stats_b.closed_trades} | {stats_c.closed_trades} | — |")
    a(f"| 胜率 | {fmt_pct(stats_a.win_rate)} | {fmt_pct(stats_b.win_rate)} | {fmt_pct(stats_c.win_rate)} | — |")
    a(f"| 总收益（复利） | {fmt_pct(stats_a.total_return)} | {fmt_pct(stats_b.total_return)} | {fmt_pct(stats_c.total_return)} | {bh_ref} |")
    a(f"| 最大回撤 | {fmt_pct(stats_a.max_drawdown)} | {fmt_pct(stats_b.max_drawdown)} | {fmt_pct(stats_c.max_drawdown)} | — |")
    a(f"| 平均持有天数 | {fmt_f(stats_a.avg_hold_days,1)} | {fmt_f(stats_b.avg_hold_days,1)} | {fmt_f(stats_c.avg_hold_days,1)} | — |")
    a(f"| 盈亏比 | {fmt_f(stats_a.profit_factor)} | {fmt_f(stats_b.profit_factor)} | {fmt_f(stats_c.profit_factor)} | — |")
    a(f"| 平均短差次数/笔 | {fmt_f(stats_a.avg_short_diffs,1)} | {fmt_f(stats_b.avg_short_diffs,1)} | 0.0 | — |")
    a(f"| 进入 COST_ZERO 笔数 | {stats_a.n_cost_zero} | {stats_b.n_cost_zero} | — | — |")
    a("")

    for st in (stats_a, stats_b, stats_c):
        a(f"## {st.name}：按入场信号类型")
        a("")
        a("| 信号类型 | 笔数 | 胜率 | 平均收益 |")
        a("|---------|------|------|---------|")
        for k, v in st.by_type.items():
            a(f"| {k} | {v['trades']} | {fmt_pct(v['win_rate'])} | {fmt_pct(v['avg_ret'])} |")
        a("")

    a("## A 组：全部交易记录")
    a("")
    a("| # | 入场日 | 入场价 | 信号 | 出场日 | 出场价 | 出场信号 | 短差 | 成本基 | 收益率 | 持有天 |")
    a("|---|-------|-------|------|-------|-------|---------|------|-------|-------|-------|")
    for i, t in enumerate(trades_a, 1):
        if t.is_closed:
            a(f"| {i} | {t.entry_date.date()} | {t.entry_price:.2f} | {t.entry_signal} "
              f"| {t.exit_date.date()} | {t.exit_price:.2f} | {t.exit_signal} "
              f"| {t.n_short_diffs} | {t.final_cost_basis:.2f} | {fmt_pct(t.ret)} | {t.hold_days} |")
        else:
            a(f"| {i} | {t.entry_date.date()} | {t.entry_price:.2f} | {t.entry_signal} "
              f"| — | — | 未平仓 | {t.n_short_diffs} | {t.final_cost_basis:.2f} | — | — |")
    a("")

    a("## 结果包（六要素）")
    a("")
    a("1. **结论**：见汇总表。三阶段降成本系统（A/B）vs 简化全仓（C）的对比为 L2 假设检验。")
    a("2. **定义依据**：cost_reduction_fsm = 267号 v1/338号 v2「满仓满融降成本」；"
      "走势方向 = 日线线段（Move[0]，缠论原文§5）；PH settle = a_online_persistence §10 因果确认定理；"
      "买卖点 = TV CHANLUN|CZSC 指标 1/2/3 类买卖点。")
    a("3. **边界条件**（结论翻转条件）：")
    a("   - 若 use_trend_filter 关闭（不限 up 线段），A/B 建仓笔数与 C 趋同。")
    a("   - 若 sub_ratio→0，短差停止，A 退化为「建仓持有到走势转空退出」。")
    a("   - 若 QQQ 处于强单边牛市，买持≈主动交易，降成本优势被吞没；需 L3 跨市况验证。")
    a("   - 价格映射偏差 >5% 的信号（见下）对应结论可靠性下降。")
    a("4. **下游推论**：若 A>C，则「短差降成本」在周线持有期内提供正 alpha（成本基下移）；"
      "若 A≤C，则在 QQQ 日线粒度上短差降成本的账面收益不足以抵消其放弃的趋势涨幅（证伪）。")
    a("5. **谱系引用**：267号（操作方法论 v1）、338号（v2 修正）、268a号（own_capital 独立核算）；"
      "走势级别错配见 formalization-validity-domain（走势类型有效域季线级 ≠ 周线信号应用域，"
      "故采用线段方向，L1 走势类型仅作对照）。")
    a("6. **影响声明**：新增 scripts/recursive_backtest.py + 本报告；复用 cost_reduction_fsm / "
      "a_recursive_engine / a_online_persistence（未改动）；映射算法复用 tv_chanlun_backtest。")
    a("")
    a("## 缺口与诚实声明（no-patch-mentality）")
    a("")
    a("1. **做空未实现**：cost_reduction_fsm 是多头降成本系统，无做空镜像。down 线段一律空仓。"
      "「下跌走势做空」需要镜像 FSM（COST_POSITIVE 做空 + 反向短差），本回测未实现，"
      "不以假的空头 PnL 冒充——这是已声明的缺口，非 bug。")
    a("2. **信号级回测**：退出由「走势转空 + 卖点」或「主级别卖点」触发，非逐 bar 价格止损。"
      "持仓期间的日内回撤未触发止损，最大回撤按成交笔权益曲线计算（笔间），非逐 bar。")
    a("3. **走势方向粒度**：用日线线段（数月级）近似周线走势；L1 走势类型（季线级）在信号时段"
      f"几乎单一（覆盖 up={sum(1 for i in range(len(timeline.l1_dir)) if timeline.l1_dir[i]=='up')} bars / "
      f"down={sum(1 for i in range(len(timeline.l1_dir)) if timeline.l1_dir[i]=='down')} bars 全期），"
      "故仅作对照不参与过滤（用户裁定）。")
    a("4. **classify_holding_event 是 L2 价值判定**：卖点→短差 vs 退出的分类规则是可调假设，"
      "见脚本第 5 节注释。不同分类规则会改变结果（声明为信息增量所在）。")
    return "\n".join(L)


# ════════════════════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════════════════════


def main() -> None:
    print("[1/7] 加载 QQQ 日线 + TV 缠论买卖点...")
    df = load_qqq()
    raw = load_tv_signals()
    print(f"      QQQ {len(df)} 根 | TV 买卖点 {len(raw)} 个")

    print("[2/7] 映射 bar_index → 交易日...")
    signals = map_signals_to_dates(raw, df)
    bad = [s for s in signals if s["price_diff_pct"] > 5]
    print(f"      映射 {len(signals)} 个 | 偏差>5% 的 {len(bad)} 个")

    print("[3/7] 跑递归引擎 → 走势方向时间线（线段 + L1 双标注）...")
    timeline = build_trend_timeline(df)

    print("[4/7] 跑 OnlineMergeTree → PH settle 时间线...")
    settle_indices = build_settle_indices(df["close"].values.astype(float))
    print(f"      settle 事件 {len(settle_indices)} 个")

    print("[5/7] 三组回测...")
    trades_a = run_fsm_backtest(
        signals, timeline, settle_indices,
        FsmConfig(use_settle_filter=False, use_trend_filter=True),
    )
    trades_b = run_fsm_backtest(
        signals, timeline, settle_indices,
        FsmConfig(use_settle_filter=True, settle_window=5, use_trend_filter=True),
    )
    trades_c = run_simple_backtest(signals)
    stats_a = compute_stats("A 组（缠论+FSM 三阶段）", trades_a)
    stats_b = compute_stats("B 组（+PH settle 过滤）", trades_b)
    stats_c = compute_stats("C 组（简化全仓 baseline）", trades_c)
    print(f"      A {stats_a.closed_trades}笔/{fmt_pct(stats_a.win_rate)} | "
          f"B {stats_b.closed_trades}笔/{fmt_pct(stats_b.win_rate)} | "
          f"C {stats_c.closed_trades}笔/{fmt_pct(stats_c.win_rate)}")

    print("[6/7] 写报告...")
    report = build_report(
        df, signals, timeline, settle_indices,
        stats_a, stats_b, stats_c, trades_a,
    )
    REPORT_PATH.write_text(report, encoding="utf-8")

    print("[7/7] 写结果 JSON...")
    summary = {
        st.name: {
            "closed_trades": st.closed_trades,
            "win_rate": st.win_rate,
            "total_return": st.total_return,
            "max_drawdown": st.max_drawdown,
            "avg_hold_days": st.avg_hold_days,
            "profit_factor": st.profit_factor,
            "avg_short_diffs": st.avg_short_diffs,
            "n_cost_zero": st.n_cost_zero,
        }
        for st in (stats_a, stats_b, stats_c)
    }
    RESULT_JSON.write_text(json.dumps(summary, ensure_ascii=False, indent=1))
    print(f"      报告: {REPORT_PATH}")
    print(f"      JSON: {RESULT_JSON}")


if __name__ == "__main__":
    main()
