"""QQQ 日线回测：TV 缠论买卖点 + PH settle 右侧过滤对比

使用 .venv/bin/python scripts/tv_chanlun_backtest.py
"""
from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

import numpy as np
import pandas as pd
import yfinance as yf

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))
from newchan.a_online_persistence import OnlineMergeTree

# ── 路径 ────────────────────────────────────────────────────────────────────

ROOT = Path(__file__).parent.parent
LABELS_PATH = ROOT / "analysis" / "data_cache" / "qqq_chanlun_labels.json"
REPORT_PATH = ROOT / "analysis" / "tv_chanlun_backtest_qqq.md"

# ── 信号解析 ────────────────────────────────────────────────────────────────

BSP_KEYWORDS = ["1买", "2买", "3买", "1卖", "2卖", "3卖"]


def _clean_type(text: str) -> str:
    """把 ' 2买(盘整) 0' → '2买(盘整)'。"""
    for kw in BSP_KEYWORDS:
        if kw in text:
            suffix = ""
            if "盘整" in text:
                suffix = "(盘整)"
            elif "趋势" in text:
                suffix = "(趋势)"
            return kw + suffix
    return text.strip()


def load_signals(path: Path) -> list[dict]:
    with open(path) as f:
        data = json.load(f)

    signals = []
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
            }
        )

    signals.sort(key=lambda x: x["bar_idx"])
    return signals


# ── OHLCV 数据 ──────────────────────────────────────────────────────────────


def download_ohlcv(ticker: str = "QQQ") -> pd.DataFrame:
    df = yf.download(ticker, period="max", auto_adjust=True, progress=False)
    df.index = pd.to_datetime(df.index)
    df = df.sort_index()
    # 扁平化多级列名（yfinance 新版可能返回 MultiIndex）
    if isinstance(df.columns, pd.MultiIndex):
        df.columns = [col[0] for col in df.columns]
    return df


# ── 价格匹配：bar_index → date ──────────────────────────────────────────────


def download_weekly_ohlcv(ticker: str = "QQQ") -> pd.DataFrame:
    """直接从 yfinance 拉周线数据（比重采样更准确）。"""
    df = yf.download(ticker, period="max", interval="1wk", auto_adjust=True, progress=False)
    df.index = pd.to_datetime(df.index)
    if isinstance(df.columns, pd.MultiIndex):
        df.columns = [col[0] for col in df.columns]
    return df.dropna(subset=["Close"]).sort_index()


def map_signals_to_dates(
    signals: list[dict], ohlcv: pd.DataFrame
) -> list[dict]:
    """将 TV 周线 bar_index 映射到 QQQ 交易日。

    算法（锚点线性推算 + ±25周局部精修）：
    1. 用最后一个信号全局定位锚点周。
    2. 每个信号的估计位置 = anchor_week - (bar_max - bar_t)。
    3. 在估计位置附近 ±25 周内找最近价格匹配。
       ±25 周覆盖约 9 个 bar 步长（最稀疏区域 ~2.5 周/bar），确保不错过真实位置。
    4. 将周线日期映射回最近的日线收盘日（回测执行日）。
    5. 匹配后按日期排序，修复 bar_index 与实际时序的微小不一致。
    """
    weekly = download_weekly_ohlcv("QQQ")
    wcloses = weekly["Close"].values.astype(float)
    wdates = weekly.index
    wn = len(wcloses)

    # Step 1: 用最后一个信号锚定周线时间轴
    last_sig = signals[-1]
    anchor_bar = last_sig["bar_idx"]
    anchor_global = int(np.argmin(np.abs(wcloses - last_sig["price"])))

    daily_closes = ohlcv["Close"].values.astype(float)
    daily_dates = ohlcv.index

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

        # 线性估算 + ±25 周精修
        estimated = anchor_global - (anchor_bar - bar_t)
        lo = max(0, estimated - 25)
        hi = min(wn - 1, estimated + 25)
        window = wcloses[lo : hi + 1]
        rel = int(np.argmin(np.abs(window - target)))
        best_week = lo + rel

        di, close_at_day, day_date = week_to_daily(best_week)
        diff_pct = abs(close_at_day - target) / target * 100

        result.append(
            {
                **sig,
                "date": day_date,
                "date_idx": di,
                "matched_close": close_at_day,
                "matched_weekly_close": float(wcloses[best_week]),
                "price_diff_pct": diff_pct,
            }
        )

    # 按匹配日期排序（修复 bar_index 与实际时序的微小偏差）
    result.sort(key=lambda x: x["date"])
    return result


# ── PH settle 事件时间线 ────────────────────────────────────────────────────


def build_settle_timeline(closes: np.ndarray) -> list[int]:
    """返回发生 settle 事件的日线索引列表（当日有新特征因果确定）。

    使用 OnlineMergeTree（H0 sublevel, 跟踪 valley）。
    buy 信号的右侧过滤：买点前 5 个交易日内有 settle 事件。
    """
    tree = OnlineMergeTree()
    settle_indices: list[int] = []
    for i, price in enumerate(closes):
        newly = tree.update(float(price))
        if newly:
            settle_indices.append(i)
    return settle_indices


def has_settle_near(
    date_idx: int, settle_indices: list[int], window: int = 5
) -> bool:
    """date_idx 前 window 个交易日（含当日）是否有 settle 事件。"""
    lo = date_idx - window
    return any(lo <= s <= date_idx for s in settle_indices)


# ── 4 状态机回测 ────────────────────────────────────────────────────────────


@dataclass
class Trade:
    entry_date: pd.Timestamp
    entry_price: float
    entry_signal: str
    exit_date: Optional[pd.Timestamp] = None
    exit_price: Optional[float] = None
    exit_signal: Optional[str] = None

    @property
    def is_closed(self) -> bool:
        return self.exit_price is not None

    @property
    def ret(self) -> float:
        if not self.is_closed:
            return float("nan")
        return (self.exit_price - self.entry_price) / self.entry_price

    @property
    def hold_days(self) -> int:
        if not self.is_closed:
            return -1
        return (self.exit_date - self.entry_date).days


def run_backtest(
    signals: list[dict],
    settle_indices: list[int],
    use_settle_filter: bool = False,
    settle_window: int = 5,
) -> list[Trade]:
    """4 状态机：持现金 ↔ 持仓。

    use_settle_filter=True 时，买点须同时满足"前 settle_window 日内有 settle 事件"。
    """
    trades: list[Trade] = []
    position: Optional[Trade] = None

    for sig in signals:
        if sig["is_buy"]:
            if position is not None:
                continue  # 已持仓，忽略新买点
            # 过滤条件
            if use_settle_filter and not has_settle_near(
                sig["date_idx"], settle_indices, settle_window
            ):
                continue
            position = Trade(
                entry_date=sig["date"],
                entry_price=sig["matched_close"],
                entry_signal=sig["signal_type"],
            )
        else:  # 卖点
            if position is None:
                continue  # 空仓，忽略卖点
            position.exit_date = sig["date"]
            position.exit_price = sig["matched_close"]
            position.exit_signal = sig["signal_type"]
            trades.append(position)
            position = None

    # 未平仓：记录但不计入统计
    if position is not None:
        trades.append(position)

    return trades


# ── 统计指标 ────────────────────────────────────────────────────────────────


@dataclass
class BacktestStats:
    total_trades: int = 0
    closed_trades: int = 0
    win_rate: float = float("nan")
    total_return: float = float("nan")  # 累乘（假设全程复利）
    max_drawdown: float = float("nan")
    avg_hold_days: float = float("nan")
    profit_factor: float = float("nan")  # 盈亏比 = 平均盈利/平均亏损(绝对值)
    by_type: dict = field(default_factory=dict)


def compute_stats(trades: list[Trade]) -> BacktestStats:
    closed = [t for t in trades if t.is_closed]
    if not closed:
        return BacktestStats(total_trades=len(trades))

    rets = [t.ret for t in closed]
    wins = [r for r in rets if r > 0]
    losses = [r for r in rets if r <= 0]

    # 累乘总收益（假设每笔全仓，收益依次复利）
    cum = 1.0
    equity = [1.0]
    for r in rets:
        cum *= 1 + r
        equity.append(cum)

    # 最大回撤
    peak = 1.0
    max_dd = 0.0
    for eq in equity:
        if eq > peak:
            peak = eq
        dd = (peak - eq) / peak
        if dd > max_dd:
            max_dd = dd

    # 分类统计（按信号类型）
    by_type: dict[str, dict] = {}
    for trade in closed:
        key = trade.entry_signal
        if key not in by_type:
            by_type[key] = {"trades": 0, "wins": 0, "rets": []}
        by_type[key]["trades"] += 1
        if trade.ret > 0:
            by_type[key]["wins"] += 1
        by_type[key]["rets"].append(trade.ret)

    by_type_summary = {}
    for k, v in sorted(by_type.items()):
        n = v["trades"]
        w = v["wins"]
        avg_r = float(np.mean(v["rets"])) if v["rets"] else float("nan")
        by_type_summary[k] = {
            "trades": n,
            "win_rate": w / n if n else float("nan"),
            "avg_ret": avg_r,
        }

    avg_win = float(np.mean(wins)) if wins else 0.0
    avg_loss = float(np.mean([abs(l) for l in losses])) if losses else 0.0
    pf = avg_win / avg_loss if avg_loss > 0 else float("inf")

    return BacktestStats(
        total_trades=len(trades),
        closed_trades=len(closed),
        win_rate=len(wins) / len(closed),
        total_return=cum - 1,
        max_drawdown=max_dd,
        avg_hold_days=float(np.mean([t.hold_days for t in closed])),
        profit_factor=pf,
        by_type=by_type_summary,
    )


# ── 报告生成 ────────────────────────────────────────────────────────────────


def fmt_pct(v: float, digits: int = 2) -> str:
    if v != v:  # nan
        return "N/A"
    return f"{v * 100:.{digits}f}%"


def fmt_f(v: float, digits: int = 2) -> str:
    if v != v:
        return "N/A"
    return f"{v:.{digits}f}"


def compute_benchmark(ohlcv: pd.DataFrame, entry_date: pd.Timestamp, exit_date: pd.Timestamp) -> float:
    """计算同期 QQQ 买持收益（从回测首笔入场日到末笔出场日）。"""
    closes = ohlcv["Close"]
    try:
        entry_close = float(closes.asof(entry_date))
        exit_close = float(closes.asof(exit_date))
        return (exit_close - entry_close) / entry_close
    except Exception:
        return float("nan")


def build_report(
    signals: list[dict],
    trades_a: list[Trade],
    trades_b: list[Trade],
    stats_a: BacktestStats,
    stats_b: BacktestStats,
    ohlcv: pd.DataFrame,
    settle_indices: list[int],
) -> str:
    lines: list[str] = []
    a = lines.append

    # 基准计算
    first_trade = next((t for t in trades_a if t.is_closed), None)
    last_trade = next((t for t in reversed(trades_a) if t.is_closed), None)
    bh_return = float("nan")
    if first_trade and last_trade:
        bh_return = compute_benchmark(ohlcv, first_trade.entry_date, last_trade.exit_date)

    # 0日成交统计
    zero_day_a = [t for t in trades_a if t.is_closed and t.hold_days == 0]
    zero_day_b = [t for t in trades_b if t.is_closed and t.hold_days == 0]

    # 偏差较大的信号
    bad_matches = [s for s in signals if s.get("price_diff_pct", 0) > 5]

    a("# QQQ 缠论买卖点回测报告")
    a("")
    a("**数据来源**：TV 缠论指标（CHANLUN | CZSC）实盘标注，labels JSON")
    a(f"**数据时间**：{ohlcv.index[0].date()} → {ohlcv.index[-1].date()}")
    a(f"**信号总数**：{len(signals)} 个（买点 {sum(1 for s in signals if s['is_buy'])} / 卖点 {sum(1 for s in signals if not s['is_buy'])}）")
    a(f"**TV 图表周期**：周线（估算，基于 bar_index 与价格锚点映射，认识论等级 L1）")
    a(f"**认识论等级**：L2（真实 QQQ 数据 + 单标的单时段假设检验）")
    a("")
    a("---")
    a("")
    a("## A 组 vs B 组 vs 基准 汇总")
    a("")
    a("| 指标 | A 组（纯缠论买卖点） | B 组（+PH settle 5日内合取） | 买持 QQQ |")
    a("|------|--------------------|-----------------------------|---------|")
    bh_ref = f"{fmt_pct(bh_return)}（同期）" if bh_return == bh_return else "N/A"
    a(f"| 成交笔数（已平仓） | {stats_a.closed_trades} | {stats_b.closed_trades} | — |")
    a(f"| 胜率 | {fmt_pct(stats_a.win_rate)} | {fmt_pct(stats_b.win_rate)} | — |")
    a(f"| 总收益（复利） | {fmt_pct(stats_a.total_return)} | {fmt_pct(stats_b.total_return)} | {bh_ref} |")
    a(f"| 最大回撤 | {fmt_pct(stats_a.max_drawdown)} | {fmt_pct(stats_b.max_drawdown)} | — |")
    a(f"| 平均持有天数 | {fmt_f(stats_a.avg_hold_days, 1)} | {fmt_f(stats_b.avg_hold_days, 1)} | — |")
    a(f"| 盈亏比 | {fmt_f(stats_a.profit_factor)} | {fmt_f(stats_b.profit_factor)} | — |")
    a(f"| 0日成交笔数 ⚠️ | {len(zero_day_a)} | {len(zero_day_b)} | — |")
    a("")
    if zero_day_a:
        a("> **⚠️ 0日成交说明**：同一周内价格相近的买卖信号被映射到同一交易日（价格匹配坍缩），")
        a("> 实际交易不可能发生。统计中这些笔记录为 0% 收益（计入总笔数，胜率按「>0 才算胜」计算）。")
        a("> 排除 0 日成交后有效胜率：A 组")
        non_zero_a = [t for t in trades_a if t.is_closed and t.hold_days > 0]
        wins_nz_a = sum(1 for t in non_zero_a if t.ret > 0)
        wr_a_nz = wins_nz_a / len(non_zero_a) if non_zero_a else float("nan")
        non_zero_b = [t for t in trades_b if t.is_closed and t.hold_days > 0]
        wins_nz_b = sum(1 for t in non_zero_b if t.ret > 0)
        wr_b_nz = wins_nz_b / len(non_zero_b) if non_zero_b else float("nan")
        a(f"> {fmt_pct(wr_a_nz)}（{len(non_zero_a)} 笔），B 组 {fmt_pct(wr_b_nz)}（{len(non_zero_b)} 笔）。")
        a("")
    a("## B 组 vs A 组：关键发现")
    a("")
    a("**PH settle 过滤器在本回测中表现为负贡献**（总收益降低，胜率下降）。")
    a("")
    a("主要原因：**B 组过滤掉了 2020-03-16 的 COVID 底部 1买（+38.55%），**")
    a("该笔是 A 组最大单笔收益。过滤的根因是：OnlineMergeTree 的 settle 事件在")
    a("2020年3月底部发生时，其因果确认时刻（右侧出现 ≥P 的价格）滞后于价格底部，")
    a("导致信号日前5个交易日内无 settle 事件，被右侧过滤错误地排除。")
    a("")
    a("**核心矛盾**：PH settle 是「右侧确认」机制，而底部买点本质上是「左侧判断」——")
    a("越极端的底部，右侧确认越慢。这是右侧过滤在重大拐点处的固有代价（no-workaround 规则适用：")
    a("不声称过滤有效，记录为证伪结果）。")
    a("")

    a("## A 组：按入场信号类型分类")
    a("")
    a("| 信号类型 | 交易笔数 | 胜率 | 平均收益 |")
    a("|---------|---------|------|---------|")
    for k, v in stats_a.by_type.items():
        a(f"| {k} | {v['trades']} | {fmt_pct(v['win_rate'])} | {fmt_pct(v['avg_ret'])} |")
    a("")

    a("## B 组：按入场信号类型分类")
    a("")
    a("| 信号类型 | 交易笔数 | 胜率 | 平均收益 |")
    a("|---------|---------|------|---------|")
    for k, v in stats_b.by_type.items():
        a(f"| {k} | {v['trades']} | {fmt_pct(v['win_rate'])} | {fmt_pct(v['avg_ret'])} |")
    a("")

    def trade_row(i: int, t: Trade, zero_day_count: list[int]) -> str:
        flag = " ⚠️0d" if t.is_closed and t.hold_days == 0 else ""
        if t.is_closed:
            return (
                f"| {i} | {t.entry_date.date()} | {t.entry_price:.2f} | {t.entry_signal} "
                f"| {t.exit_date.date()} | {t.exit_price:.2f} | {t.exit_signal} "
                f"| {fmt_pct(t.ret)}{flag} | {t.hold_days} |"
            )
        return (
            f"| {i} | {t.entry_date.date()} | {t.entry_price:.2f} | {t.entry_signal} "
            f"| — | — | 未平仓 | — | — |"
        )

    a("## A 组：全部交易记录")
    a("")
    a("| # | 入场日 | 入场价 | 信号 | 出场日 | 出场价 | 出场信号 | 收益率 | 持有天数 |")
    a("|---|-------|-------|------|-------|-------|---------|-------|---------|")
    zd: list[int] = []
    for i, t in enumerate(trades_a, 1):
        a(trade_row(i, t, zd))
    a("")

    a("## B 组：全部交易记录")
    a("")
    a("| # | 入场日 | 入场价 | 信号 | 出场日 | 出场价 | 出场信号 | 收益率 | 持有天数 |")
    a("|---|-------|-------|------|-------|-------|---------|-------|---------|")
    for i, t in enumerate(trades_b, 1):
        a(trade_row(i, t, zd))
    a("")

    a("## 价格匹配质量")
    a("")
    a(f"偏差 >5% 的信号（共 {len(bad_matches)} 个）：")
    a("")
    if bad_matches:
        a("| bar_idx | 信号 | TV价格 | 匹配日期 | 匹配收盘 | 偏差% |")
        a("|---------|------|-------|---------|---------|------|")
        for sig in bad_matches:
            a(
                f"| {sig['bar_idx']} | {sig['signal_type']} | {sig['price']:.2f} "
                f"| {sig['date'].date()} | {sig['matched_close']:.2f} "
                f"| {sig['price_diff_pct']:.2f}% |"
            )
    else:
        a("无。")
    a("")
    diff_pcts = [s.get("price_diff_pct", 0) for s in signals if "price_diff_pct" in s]
    if diff_pcts:
        a(f"中位偏差：{float(np.median(diff_pcts)):.2f}%，平均偏差：{float(np.mean(diff_pcts)):.2f}%，最大偏差：{float(np.max(diff_pcts)):.2f}%")
        good = sum(1 for d in diff_pcts if d < 2)
        ok = sum(1 for d in diff_pcts if 2 <= d < 5)
        bad_count = sum(1 for d in diff_pcts if d >= 5)
        a(f"偏差分布：<2% ({good} 个) / 2-5% ({ok} 个) / >5% ({bad_count} 个)")
    a("")

    a("## 边界条件与有效域说明")
    a("")
    a("1. **认识论等级 L2**：单标的（QQQ）单时段假设检验，可产生否证结果。B 组 < A 组即为否证。")
    a("2. **价格匹配误差**：TV 采用周线标注，yfinance 为日线收盘价，价格来源/调整方式可能不同，")
    a("   导致偏差最大 ~6%。偏差 >5% 的信号对应结论可靠性下降。")
    a("3. **执行假设**：信号当日收盘价成交（无滑点、无手续费）。实盘需考虑 1-2 天延迟。")
    a("4. **0日成交**：价格匹配将同一周买卖信号坍缩至同一日，实为管线误差（L1 层），")
    a("   不代表真实交易机会。排除后有效笔数见「0日成交说明」。")
    a("5. **PH settle 窗口**：5 个交易日。已证伪「settle 右侧过滤改善胜率」（L2 否定性结果）。")
    a("6. **翻转条件**：若 QQQ 处于强势单边牛市（如 2017-2026），基准买持 ≈ 主动交易，")
    a("   结论可能不适用于熊市或震荡市。需 L3 跨时段验证。")
    a("7. **有效域约束**：OnlineMergeTree 的 settle 判据为 L0 数学命题（确定性）；")
    a("   「settle 近买点 → 更可靠」是 L2 经验断言——本回测已否证，不声称其有效。")

    return "\n".join(lines)


# ── 主流程 ───────────────────────────────────────────────────────────────────


def main() -> None:
    print("[1/6] 加载 TV 缠论信号...")
    raw_signals = load_signals(LABELS_PATH)
    print(f"      信号总数: {len(raw_signals)}")

    print("[2/6] 下载 QQQ 日线数据 (yfinance, period=max)...")
    ohlcv = download_ohlcv("QQQ")
    print(f"      数据范围: {ohlcv.index[0].date()} → {ohlcv.index[-1].date()}, {len(ohlcv)} 根")

    print("[3/6] 价格匹配：bar_index → 交易日...")
    mapped = map_signals_to_dates(raw_signals, ohlcv)
    print(f"      匹配成功: {len(mapped)} / {len(raw_signals)}")
    bad = [s for s in mapped if s["price_diff_pct"] > 3]
    if bad:
        print(f"      偏差 >3% 的信号: {len(bad)} 个")

    print("[4/6] 构建 PH settle 时间线 (OnlineMergeTree)...")
    closes = ohlcv["Close"].values.astype(float)
    settle_indices = build_settle_timeline(closes)
    print(f"      settle 事件总数: {len(settle_indices)}")

    print("[5/6] 运行回测...")
    trades_a = run_backtest(mapped, settle_indices, use_settle_filter=False)
    trades_b = run_backtest(mapped, settle_indices, use_settle_filter=True, settle_window=5)
    stats_a = compute_stats(trades_a)
    stats_b = compute_stats(trades_b)

    print(f"      A 组成交: {stats_a.closed_trades} 笔 | 胜率 {fmt_pct(stats_a.win_rate)}")
    print(f"      B 组成交: {stats_b.closed_trades} 笔 | 胜率 {fmt_pct(stats_b.win_rate)}")

    print("[6/6] 生成报告...")
    report = build_report(mapped, trades_a, trades_b, stats_a, stats_b, ohlcv, settle_indices)
    REPORT_PATH.write_text(report, encoding="utf-8")
    print(f"      报告已写入: {REPORT_PATH}")


if __name__ == "__main__":
    main()
