"""E 版本（背驰定位器进出场）逐笔交易统计精确报告。

E = swing_e（fugue_alpha_diagnosis 的 MODE_SWING_E）：
  进场（1买，敏感）= 次级别底背驰（down_move_settled ∧ entry_div_ok），满仓做多；
  持仓 = 默认满仓穿越所有 move 级波动；
  出场（1卖，鲁棒）= L2 趋势顶背驰（l2_flip_short ∧ exit_div_ok），清仓；
  无降成本短差（cost_mode=MODE_NONE）。
E 只做多，无做空腿。

本脚本复用 fugue_alpha_diagnosis 的信号管线（compute_signals，瓶颈，跑一次）与
run_swing_trading（重放磁带，近乎免费），并在 bar 区间 [entry_bar, exit_bar] 上
用 high/low 补算每笔的 MAE/MFE（CompletedTrade 不含），映射真实日历时间
（期货 10y databento 文件含 dates，QQQ/OKLO full 文件无 dates → 仅 bar 序号）。

认识论等级：L2（真实数据，单标的逐笔，可证伪）。
521 号限定：E 信号属 candidate 层（PH 门控 + MACD 面积代理），非 confirmed 买卖点。

用法：
    PYTHONPATH=src python3 analysis/trading_statistics.py QQQ OKLO
    PYTHONPATH=src python3 analysis/trading_statistics.py CL ES GC      # 期货 10y（慢）
"""
from __future__ import annotations

import json
import math
import sys
import time
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from fugue_alpha_diagnosis import (  # type: ignore
    MODE_NONE,
    CompletedTrade,
    compute_signals,
    run_swing_trading,
)

DATA_DIR = Path(__file__).parent / "data_cache"
OUTPUT_MD = Path(__file__).parent / "trading_statistics_report.md"


def _cache_path(symbol: str) -> Path:
    """逐标的结果缓存（算完即落地，报告从缓存聚合，可增量/可恢复）。"""
    return DATA_DIR / f"e_trade_{symbol.upper()}.json"

# 标的 → (文件名, 是否含真实 dates)。
# full 文件：仅 opens/highs/lows/closes（无 dates）。
# 10y databento：symbol/dates/opens/highs/lows/closes（dates 为真实日历时间）。
_FILES: dict[str, tuple[str, bool]] = {
    "QQQ": ("qqq_1m_databento_full.json", False),
    "OKLO": ("oklo_1m_databento_full.json", False),
    "CL": ("cl_1m_databento_10y.json", True),
    "ES": ("es_1m_databento_10y.json", True),
    "GC": ("gc_1m_databento_10y.json", True),
}


@dataclass(frozen=True)
class TradeRecord:
    """单笔交易的完整统计（E 版本，做多）。"""

    idx: int
    direction: str          # E 恒为 "long"
    entry_bar: int
    exit_bar: int
    entry_date: datetime | None
    exit_date: datetime | None
    entry_price: float
    exit_price: float
    hold_bars: int
    hold_days: float | None
    pnl_pct: float
    mfe_pct: float          # 最大浮盈（区间内 high 相对进场价）
    mae_pct: float          # 最大浮亏（区间内 low 相对进场价，≤0）
    entry_reason: str
    exit_reason: str


# ════════════════════════════════════════════════════════════
# 数据加载
# ════════════════════════════════════════════════════════════

def load_symbol(
    symbol: str,
) -> tuple[list[float], list[float], list[float], list[float], list[datetime] | None]:
    """加载 OHLC（+ dates，若文件含）。返回 (opens, highs, lows, closes, dates|None)。"""
    fname, has_dates = _FILES[symbol.upper()]
    path = DATA_DIR / fname
    if not path.exists():
        raise FileNotFoundError(f"{symbol}: 数据文件不存在 {path}")
    raw = json.loads(path.read_text())
    opens = [float(x) for x in raw["opens"]]
    highs = [float(x) for x in raw["highs"]]
    lows = [float(x) for x in raw["lows"]]
    closes = [float(x) for x in raw["closes"]]
    dates: list[datetime] | None = None
    if has_dates and "dates" in raw:
        dates = [datetime.fromisoformat(s) for s in raw["dates"]]
    return opens, highs, lows, closes, dates


# ════════════════════════════════════════════════════════════
# 逐笔记录（补 MAE/MFE + 时间映射）
# ════════════════════════════════════════════════════════════

def build_records(
    trades: list[CompletedTrade],
    highs: list[float],
    lows: list[float],
    dates: list[datetime] | None,
) -> list[TradeRecord]:
    """把 CompletedTrade 增补为 TradeRecord：MAE/MFE（high/low 区间极值）+ 日历时间。"""
    records: list[TradeRecord] = []
    for i, t in enumerate(trades):
        a, b = t.entry_bar, t.exit_bar
        # 区间 [entry_bar, exit_bar] 含端点：浮盈/浮亏极值用 bar 内 high/low。
        seg_high = max(highs[a : b + 1])
        seg_low = min(lows[a : b + 1])
        ep = t.entry_price
        mfe = (seg_high - ep) / ep * 100 if ep > 0 else 0.0
        mae = (seg_low - ep) / ep * 100 if ep > 0 else 0.0

        entry_date = dates[a] if dates is not None else None
        exit_date = dates[b] if dates is not None else None
        hold_days = (
            (exit_date - entry_date).total_seconds() / 86400.0
            if entry_date is not None and exit_date is not None
            else None
        )
        records.append(TradeRecord(
            idx=i + 1,
            direction="long",
            entry_bar=a,
            exit_bar=b,
            entry_date=entry_date,
            exit_date=exit_date,
            entry_price=ep,
            exit_price=t.exit_price,
            hold_bars=b - a,
            hold_days=hold_days,
            pnl_pct=t.pnl_pct,
            mfe_pct=round(mfe, 4),
            mae_pct=round(mae, 4),
            entry_reason="sub_level_bottom_divergence",  # 次级别底背驰（第一类买点）
            exit_reason=t.exit_reason,                    # l2_trend_top_divergence / eod_close
        ))
    return records


# ════════════════════════════════════════════════════════════
# 汇总统计
# ════════════════════════════════════════════════════════════

def _max_consecutive(pnls: list[float], win: bool) -> int:
    best = cur = 0
    for p in pnls:
        hit = p > 0 if win else p <= 0
        cur = cur + 1 if hit else 0
        best = max(best, cur)
    return best


def _percentile(sorted_vals: list[float], q: float) -> float:
    """线性插值分位数（q∈[0,1]）。"""
    if not sorted_vals:
        return 0.0
    if len(sorted_vals) == 1:
        return sorted_vals[0]
    pos = q * (len(sorted_vals) - 1)
    lo = int(math.floor(pos))
    hi = int(math.ceil(pos))
    frac = pos - lo
    return sorted_vals[lo] * (1 - frac) + sorted_vals[hi] * frac


def summarize(records: list[TradeRecord]) -> dict:
    """汇总统计。夏普/Sortino 为**按笔**口径（非年化）；年化需稳定的日历频率。"""
    if not records:
        return {"n": 0}
    pnls = [r.pnl_pct for r in records]
    wins = [p for p in pnls if p > 0]
    losses = [p for p in pnls if p <= 0]
    n = len(pnls)
    n_win, n_loss = len(wins), len(losses)
    win_rate = n_win / n
    loss_rate = n_loss / n

    avg = sum(pnls) / n
    sorted_pnls = sorted(pnls)
    median = _percentile(sorted_pnls, 0.5)
    avg_win = sum(wins) / n_win if n_win else 0.0
    avg_loss = sum(losses) / n_loss if n_loss else 0.0

    # 盈亏比（payoff ratio）= 平均盈利笔 / |平均亏损笔|（用户定义）。
    payoff = avg_win / abs(avg_loss) if avg_loss != 0 else float("inf")
    # profit factor = 总盈利 / |总亏损|（补充口径）。
    sum_win, sum_loss = sum(wins), sum(losses)
    profit_factor = sum_win / abs(sum_loss) if sum_loss != 0 else float("inf")

    # 期望值 = 胜率×平均盈利 + 败率×平均亏损（恒等于平均单笔回报 avg）。
    expectancy = win_rate * avg_win + loss_rate * avg_loss

    # 按笔夏普/Sortino（mean/std；Sortino 用下行 std，目标回报 0）。
    if n > 1:
        var = sum((p - avg) ** 2 for p in pnls) / (n - 1)
        std = var ** 0.5
    else:
        std = 0.0
    sharpe = avg / std if std > 0 else 0.0
    downside = [p for p in pnls if p < 0]
    if len(downside) >= 1:
        d_var = sum(p ** 2 for p in downside) / len(downside)
        d_std = d_var ** 0.5
    else:
        d_std = 0.0
    sortino = avg / d_std if d_std > 0 else float("inf") if avg > 0 else 0.0

    # 复利权益曲线 + 最大回撤（按笔串联）。
    eq = 1.0
    peak = 1.0
    max_dd = 0.0
    for p in pnls:
        eq *= 1 + p / 100
        peak = max(peak, eq)
        max_dd = min(max_dd, (eq - peak) / peak)
    compound = (eq - 1) * 100

    # 持仓 + 频率（bars；若有日历则附 days）。
    hold_bars = [r.hold_bars for r in records]
    avg_hold_bars = sum(hold_bars) / n
    has_dates = records[0].entry_date is not None
    avg_hold_days = None
    freq_days = None
    if has_dates:
        hd = [r.hold_days for r in records if r.hold_days is not None]
        avg_hold_days = sum(hd) / len(hd) if hd else None
        span = (records[-1].exit_date - records[0].entry_date).total_seconds() / 86400.0
        freq_days = span / n if n else None

    return {
        "n": n,
        "n_win": n_win,
        "n_loss": n_loss,
        "win_rate": win_rate * 100,
        "loss_rate": loss_rate * 100,
        "avg_pnl": avg,
        "median_pnl": median,
        "max_win": max(pnls),
        "max_loss": min(pnls),
        "avg_win": avg_win,
        "avg_loss": avg_loss,
        "payoff_ratio": payoff,
        "profit_factor": profit_factor,
        "expectancy": expectancy,
        "sharpe_per_trade": sharpe,
        "sortino_per_trade": sortino,
        "pnl_std": std,
        "compound": compound,
        "max_dd": max_dd * 100,
        "max_consec_win": _max_consecutive(pnls, True),
        "max_consec_loss": _max_consecutive(pnls, False),
        "avg_hold_bars": avg_hold_bars,
        "avg_hold_days": avg_hold_days,
        "freq_bars": sum(hold_bars) / n,  # 占位（见报告说明，bars 不连续）
        "freq_days": freq_days,
        "has_dates": has_dates,
        # MAE/MFE 汇总
        "avg_mfe": sum(r.mfe_pct for r in records) / n,
        "avg_mae": sum(r.mae_pct for r in records) / n,
        "max_mfe": max(r.mfe_pct for r in records),
        "max_mae": min(r.mae_pct for r in records),
    }


def by_year(records: list[TradeRecord]) -> dict[int, dict]:
    """按出场年份分组（仅含真实 dates 的标的）。"""
    if not records or records[0].exit_date is None:
        return {}
    groups: dict[int, list[TradeRecord]] = {}
    for r in records:
        y = r.exit_date.year
        groups.setdefault(y, []).append(r)
    return {y: summarize(rs) for y, rs in sorted(groups.items())}


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════

def _fmt(v, spec="+.2f") -> str:
    if v is None:
        return "n/a"
    if isinstance(v, float) and math.isinf(v):
        return "∞"
    return format(v, spec)


def _trade_table(records: list[TradeRecord], has_dates: bool) -> list[str]:
    L: list[str] = []
    if has_dates:
        L.append("| # | 方向 | 进场时间 | 进场价 | 出场时间 | 出场价 | 出场原因 | 持仓天 | 回报% | MFE% | MAE% |")
        L.append("|---|------|----------|--------|----------|--------|----------|--------|-------|------|------|")
        for r in records:
            L.append(
                f"| {r.idx} | {r.direction} "
                f"| {r.entry_date:%Y-%m-%d %H:%M} | {r.entry_price:.2f} "
                f"| {r.exit_date:%Y-%m-%d %H:%M} | {r.exit_price:.2f} "
                f"| {r.exit_reason} | {_fmt(r.hold_days, '.1f')} "
                f"| {r.pnl_pct:+.2f} | {r.mfe_pct:+.2f} | {r.mae_pct:+.2f} |"
            )
    else:
        L.append("| # | 方向 | 进场bar | 进场价 | 出场bar | 出场价 | 出场原因 | 持仓bars | 回报% | MFE% | MAE% |")
        L.append("|---|------|---------|--------|---------|--------|----------|----------|-------|------|------|")
        for r in records:
            L.append(
                f"| {r.idx} | {r.direction} "
                f"| {r.entry_bar:,} | {r.entry_price:.2f} "
                f"| {r.exit_bar:,} | {r.exit_price:.2f} "
                f"| {r.exit_reason} | {r.hold_bars:,} "
                f"| {r.pnl_pct:+.2f} | {r.mfe_pct:+.2f} | {r.mae_pct:+.2f} |"
            )
    return L


def _summary_block(s: dict, has_dates: bool) -> list[str]:
    L: list[str] = []
    L.append("| 指标 | 值 |")
    L.append("|------|-----|")
    L.append(f"| 总交易数 | {s['n']} |")
    L.append(f"| 盈利笔 / 亏损笔 | {s['n_win']} / {s['n_loss']} |")
    L.append(f"| 胜率 / 败率 | {s['win_rate']:.1f}% / {s['loss_rate']:.1f}% |")
    L.append(f"| 平均单笔回报 | {s['avg_pnl']:+.2f}% |")
    L.append(f"| 中位单笔回报 | {s['median_pnl']:+.2f}% |")
    L.append(f"| 最大单笔盈利 | {s['max_win']:+.2f}% |")
    L.append(f"| 最大单笔亏损 | {s['max_loss']:+.2f}% |")
    L.append(f"| 平均盈利笔 / 平均亏损笔 | {s['avg_win']:+.2f}% / {s['avg_loss']:+.2f}% |")
    L.append(f"| 盈亏比（payoff，均盈/均亏） | {_fmt(s['payoff_ratio'], '.2f')} |")
    L.append(f"| 盈利因子（profit factor，总盈/总亏） | {_fmt(s['profit_factor'], '.2f')} |")
    L.append(f"| 期望值（胜率×均盈+败率×均亏） | {s['expectancy']:+.2f}% |")
    L.append(f"| 单笔回报标准差 | {s['pnl_std']:.2f}% |")
    L.append(f"| 夏普（按笔） | {_fmt(s['sharpe_per_trade'], '+.3f')} |")
    L.append(f"| Sortino（按笔，下行） | {_fmt(s['sortino_per_trade'], '+.3f')} |")
    L.append(f"| 复利总回报（按笔串联） | {s['compound']:+.2f}% |")
    L.append(f"| 最大回撤（按笔权益） | {s['max_dd']:+.2f}% |")
    L.append(f"| 最大连续盈利 | {s['max_consec_win']} 笔 |")
    L.append(f"| 最大连续亏损 | {s['max_consec_loss']} 笔 |")
    L.append(f"| 平均持仓 | {s['avg_hold_bars']:,.0f} bars |")
    if has_dates:
        L.append(f"| 平均持仓（日历） | {_fmt(s['avg_hold_days'], '.1f')} 天 |")
        L.append(f"| 交易频率 | 每 {_fmt(s['freq_days'], '.1f')} 天一笔 |")
    L.append(f"| 平均 MFE / MAE | {s['avg_mfe']:+.2f}% / {s['avg_mae']:+.2f}% |")
    L.append(f"| 最大 MFE / 最深 MAE | {s['max_mfe']:+.2f}% / {s['max_mae']:+.2f}% |")
    return L


def write_report(all_results: dict) -> None:
    L: list[str] = []
    L.append("# E 版本交易统计精确报告 — 背驰定位器进出场（逐笔）\n")
    L.append(
        "> 策略：**E = swing_e**（底背驰买入 / L2 趋势顶背驰卖出 / 满仓穿越 / 无降成本，纯做多）  |  "
        "数据：1min  |  脚本：`analysis/trading_statistics.py`  |  "
        "信号管线复用 `fugue_alpha_diagnosis`（compute_signals + run_swing_trading）。\n"
    )
    L.append(
        "> 认识论等级：**L2**（真实数据，单标的逐笔，可证伪）。"
        "521 号限定：E 信号属 candidate 层（PH 门控 + MACD 面积代理），非 confirmed 买卖点。\n"
    )
    L.append("## 口径说明\n")
    L.append(
        "- **方向**：E 仅做多（底背驰进场、顶背驰出场），无做空腿。\n"
        "- **MAE/MFE**：在 bar 区间 [进场bar, 出场bar]（含端点）上用 high/low 极值相对进场价计算。"
        "MFE=最大浮盈（区间最高价），MAE=最大浮亏（区间最低价，≤0）。\n"
        "- **进场原因**：全部为次级别底背驰（down_move_settled ∧ entry_div_ok，第一类买点）——"
        "E 进场信号唯一。出场原因：`l2_trend_top_divergence`（L2 趋势顶背驰清仓）或 `eod_close`（数据末尾强平）。\n"
        "- **时间戳**：QQQ/OKLO 的 `*_full.json` 仅含 OHLC 无日历时间 → 进出场以 **bar 序号**表示，"
        "无法换算日历天/按年份分组（如需真实时间须重拉带 dates 的数据）。"
        "CL/ES/GC 的 `*_10y.json` 含真实 `dates` → 给日历时间、持仓天数、按年份分组。\n"
        "- **夏普/Sortino**：**按笔**口径（单笔回报序列的 mean/std，目标回报 0），非年化——"
        "年化需稳定的日历交易频率，样本笔数过少时年化无统计意义。\n"
        "- **期望值**：用户定义 `胜率×平均盈利 + 败率×平均亏损`，数学上**恒等于平均单笔回报**（同义）。\n"
    )

    for sym, res in all_results.items():
        records = res["records"]
        s = res["summary"]
        has_dates = res["has_dates"]
        n_bars = res["n_bars"]
        bh = res["bh"]
        L.append(f"\n## {sym}\n")
        L.append(
            f"- 数据：**{n_bars:,}** bars（1min）  |  Buy-and-hold：**{bh:+.2f}%**  |  "
            f"E 复利：**{s.get('compound', 0):+.2f}%**  |  超额：**{s.get('compound', 0) - bh:+.2f}%**\n"
        )
        if s["n"] == 0:
            L.append("- **无交易**（信号未触发任何进场）。\n")
            continue
        L.append("### 汇总统计\n")
        L.extend(_summary_block(s, has_dates))
        L.append("")
        L.append("### 逐笔交易明细\n")
        L.extend(_trade_table(records, has_dates))
        L.append("")
        yr = res.get("by_year", {})
        if yr:
            L.append("### 按年份分组（出场年）\n")
            L.append("| 年份 | 交易数 | 胜率 | 平均回报% | 复利% | 最大回撤% | 平均持仓天 |")
            L.append("|------|--------|------|-----------|-------|-----------|------------|")
            for y, ys in yr.items():
                L.append(
                    f"| {y} | {ys['n']} | {ys['win_rate']:.0f}% "
                    f"| {ys['avg_pnl']:+.2f} | {ys['compound']:+.2f} "
                    f"| {ys['max_dd']:+.2f} | {_fmt(ys['avg_hold_days'], '.1f')} |"
                )
            L.append("")

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告已写入：{OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def _record_to_dict(r: TradeRecord) -> dict:
    d = r.__dict__.copy()
    d["entry_date"] = r.entry_date.isoformat() if r.entry_date else None
    d["exit_date"] = r.exit_date.isoformat() if r.exit_date else None
    return d


def _record_from_dict(d: dict) -> TradeRecord:
    d = dict(d)
    d["entry_date"] = datetime.fromisoformat(d["entry_date"]) if d["entry_date"] else None
    d["exit_date"] = datetime.fromisoformat(d["exit_date"]) if d["exit_date"] else None
    return TradeRecord(**d)


def save_cache(symbol: str, res: dict) -> None:
    payload = {
        "records": [_record_to_dict(r) for r in res["records"]],
        "has_dates": res["has_dates"],
        "n_bars": res["n_bars"],
        "bh": res["bh"],
    }
    _cache_path(symbol).write_text(json.dumps(payload, ensure_ascii=False, indent=2))


def load_cache(symbol: str) -> dict | None:
    path = _cache_path(symbol)
    if not path.exists():
        return None
    payload = json.loads(path.read_text())
    records = [_record_from_dict(d) for d in payload["records"]]
    return {
        "records": records,
        "summary": summarize(records),
        "by_year": by_year(records),
        "has_dates": payload["has_dates"],
        "n_bars": payload["n_bars"],
        "bh": payload["bh"],
    }


def process(symbol: str) -> dict:
    print(f"\n{'=' * 60}\n  {symbol} — E 版本逐笔统计\n{'=' * 60}")
    opens, highs, lows, closes, dates = load_symbol(symbol)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  [{symbol}] {n:,} bars  BH={bh:+.2f}%  dates={'有' if dates else '无'}")

    t0 = time.time()
    signals = compute_signals(opens, highs, lows, closes)
    print(f"  [{symbol}] 信号层 {time.time() - t0:.1f}s")

    trades, _ = run_swing_trading(signals, MODE_NONE)
    records = build_records(trades, highs, lows, dates)
    s = summarize(records)
    yr = by_year(records)
    print(
        f"  [{symbol}] 交易={s['n']} 胜率={s.get('win_rate', 0):.0f}% "
        f"复利={s.get('compound', 0):+.2f}% 超额={s.get('compound', 0) - bh:+.2f}%"
    )
    return {
        "records": records,
        "summary": s,
        "by_year": yr,
        "has_dates": dates is not None,
        "n_bars": n,
        "bh": bh,
    }


def main() -> None:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    force = "--force" in sys.argv
    report_only = "--report-only" in sys.argv
    symbols = args or ["QQQ", "OKLO"]
    results: dict = {}
    t_all = time.time()
    for sym in symbols:
        cached = None if (force and not report_only) else load_cache(sym)
        if cached is not None:
            print(f"  [{sym}] 从缓存加载（{cached['summary'].get('n', 0)} 笔）")
            results[sym] = cached
        elif report_only:
            print(f"  [{sym}] 无缓存，跳过（--report-only）")
            continue
        else:
            res = process(sym)
            save_cache(sym, res)
            results[sym] = res
    print(f"\n总耗时 {time.time() - t_all:.1f}s")
    if results:
        write_report(results)


if __name__ == "__main__":
    main()
