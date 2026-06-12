"""v2 降成本短差『卖飞』根因诊断 — OKLO I_seg2（segment 级，547 笔短差）。

═══════════════════════════════════════════════════════════════════════
诊断问题（用户）：v2 降成本短差系统性卖飞（卖飞率 52%）。根因是
                  **信号滞后**（检测到卖点时已从顶部跌一段）还是
                  **趋势太强**（卖点时机对，但结构里根本没有降成本空间）？
═══════════════════════════════════════════════════════════════════════

机制回顾（`fugue_version_i._SharedFugue`）：
  高抛 open_diff(sell_price) → 低吸 close_diff(buy_price)；diff = sell − buy。
  diff>0 = 低吸成功（降成本）；diff<0 = 卖飞（被迫高价回补，cost_basis↑）。
  I_seg2：floor=segment(2)，短差全部发生在 segment 级（笔中枢 confirmed BSP）。
  segment 级的『次级别走势』= 笔（stroke）→ 局部极值 = 笔端点（up-stroke 顶 / down-stroke 底）。

═══════════════════════════════════════════════════════════════════════
判决框架（把『信号滞后 vs 趋势太强』变成可证伪的二分）
═══════════════════════════════════════════════════════════════════════
对每笔短差，在 **raw 价格坐标**（与回测 diff 同坐标，事件边界锚定）取：
  - local_high = 上次买回 bar → 本次卖出 bar 之间的 max(high)（次级别走势实际顶部）
  - local_low  = 卖出 bar → 买回 bar 之间的 min(low)（买点前次级别走势实际底部）

⚠ **不用引擎 `current_strokes()` 的端点 bar i1**：实测 i1 是 K线包含处理后的**合并坐标**
  （OKLO 447,739 raw → ~252,531 merged，比例 1.77），与 raw diff bar 不对齐，引擎不暴露
  raw→merged 映射 → stroke-i1 锚定不可用。raw 价格序列在同一坐标更严格、零引擎依赖。

度量：
  1) gap_sell = (local_high − sell_price)/local_high × 100  → **卖出信号滞后**程度
  2) gap_buy  = (buy_price − local_low)/local_low × 100     → **买回信号滞后**程度
  3) depth_below = (sell_price − local_low)/sell_price × 100 → 卖价下方真实回调深度
     - depth_below ≤ θ：卖出后价格未跌破卖价下方 θ% → **趋势太强**（无可买回的更低点）
     - depth_below > θ：卖价下方有真实低点却被次级别买点错过 → **信号滞后**
     θ=0 绝对判据；θ>0 排除 1 根 K 线浅 wick。报告全阈值敏感性。
  4) run_up = (max(high[sell..buy]) − sell_price)/sell_price × 100 → 持短差期间冲高（趋势强度）

认识论 L2（真实数据 447K OKLO，含否定性结果）。力度口径=价格振幅 fallback（非 MACD）；
本诊断不依赖力度口径（只用 raw 价格几何）。

输出：analysis/v2_selloff_root_cause.md + data_cache/v2_selloff_root_cause.json。
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import (  # noqa: E402
    LADDER_SEG,
    extended_metrics,
    run_version_i,
)
from m1_i_rust_engine import compute_i_signals_rust  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "v2_selloff_root_cause.json"
OUT_MD = ROOT / "analysis" / "v2_selloff_root_cause.md"

SYMBOL = "OKLO"


# ════════════════════════════════════════════════════════════
# 局部极值（raw 价格序列，事件边界锚定）
# ════════════════════════════════════════════════════════════
#
# ⚠ 为何不用引擎 stroke 端点：`current_strokes()` 末次读取的 i1（端点 bar）是
#   **K线包含处理后的合并坐标**（OKLO 447,739 raw bar 合并为 ~252,531 → 比例 1.77），
#   与回测 diff 的 raw sell_bar/buy_bar 坐标不对齐（实测 i1=176338 报价 109.83，但该
#   合并 bar 对应 raw ~290000；引擎不暴露 raw→merged 映射）。故 stroke-i1 锚定不可用。
#   改用 raw 价格序列在**同一 raw 坐标**定义局部极值——零引擎依赖，更严格：
#   - 局部顶 = 上一次买回 bar → 本次卖出 bar 之间的 max(high)（次级别走势的实际顶部；
#     窗口由实际事件边界界定，非任意 lookback）。
#   - 局部底 = 卖出 bar → 买回 bar 之间的 min(low)（买点前次级别走势的实际底部 = 本可
#     买回的真实最低价）。


def annotate_diffs(diag, highs, lows, years):
    """逐笔短差精细标注（raw 坐标，按 trade 内时序锚定局部顶）。

    `diag` = 每笔交易 {entry_bar, diffs:[...]}；同一 trade 内短差按 sell_bar 时序，
    局部顶的回看窗口锚定到**上一笔短差的买回 bar**（首笔锚定到 trade entry_bar）——
    即『自上次在本级别买回以来的次级别走势顶部』，结构化边界，非任意窗口。
    """
    n = len(lows)
    out = []
    for trade in diag:
        entry_bar = trade["entry_bar"]
        prev_buy_bar = entry_bar  # 局部顶回看锚（上次买回 / 建仓）
        for d in sorted(trade["diffs"], key=lambda x: x["sell_bar"]):
            sell_bar = d["sell_bar"]; buy_bar = d["buy_bar"]
            sell_price = d["sell_price"]; buy_price = d["buy_price"]
            diff = d["diff"]
            # 局部顶：上次买回→本次卖出之间的最高（次级别走势实际顶部）
            a = max(0, min(prev_buy_bar, sell_bar)); b = min(n - 1, sell_bar)
            local_high = max(highs[a:b + 1]) if b >= a else sell_price
            gap_sell = (local_high - sell_price) / local_high * 100 if local_high > 0 else None
            # 局部底：卖出→买回之间的最低（买点前次级别走势实际底部 = 本可买回最低价）
            lo = sell_bar + 1; hi = min(n - 1, buy_bar)
            local_low = min(lows[lo:hi + 1]) if hi >= lo else sell_price
            gap_buy = (buy_price - local_low) / local_low * 100 if local_low > 0 else None
            # depth_below：价格跌破卖价的深度（>0 = 卖价下方有真实买回机会）
            depth_below = (sell_price - local_low) / sell_price * 100 if sell_price > 0 else None
            # ideal_diff：完美择时（顶卖底买）价差
            ideal_diff = local_high - local_low
            # run_up：持短差期间冲高幅度（趋势强度）
            mh = max(highs[max(0, sell_bar):hi + 1]) if hi >= sell_bar else sell_price
            run_up = (mh - sell_price) / sell_price * 100 if sell_price > 0 else None
            hold = buy_bar - sell_bar
            is_fly = diff < 0
            # ── 核心判据（锚定卖价，raw 坐标）──
            #   趋势太强：depth_below ≤ 0 → 卖出后价格从未跌回卖价下方，无论信号多准
            #             都买不回更低（中枢上移，结构无降成本空间）。
            #   信号滞后：depth_below > 0 → 卖价下方有真实低点存在，但买回信号(次级别买点)
            #             在价格回升至卖价上方后才确认 → 错过了更低的买回。
            if is_fly:
                cause = "trend" if (depth_below is None or depth_below <= 0) else "lag"
            else:
                cause = None
            out.append({
                **d,
                "local_high": round(local_high, 4), "local_low": round(local_low, 4),
                "gap_sell_pct": round(gap_sell, 3) if gap_sell is not None else None,
                "gap_buy_pct": round(gap_buy, 3) if gap_buy is not None else None,
                "depth_below_pct": round(depth_below, 3) if depth_below is not None else None,
                "ideal_diff": round(ideal_diff, 4),
                "run_up_pct": round(run_up, 3) if run_up is not None else None,
                "hold": hold, "is_fly": is_fly, "cause": cause,
                "year": years[sell_bar] if years and sell_bar < len(years) else None,
            })
            prev_buy_bar = buy_bar
    return out


# ════════════════════════════════════════════════════════════
# 分组聚合
# ════════════════════════════════════════════════════════════

def _dist(vals: list[float]) -> dict:
    """数值分布：n/mean/median/p10/p90/min/max。"""
    xs = sorted(v for v in vals if v is not None)
    n = len(xs)
    if n == 0:
        return {"n": 0}

    def pct(p):
        return round(xs[min(n - 1, int(p * n))], 3)
    return {
        "n": n, "mean": round(sum(xs) / n, 3), "median": pct(0.5),
        "p10": pct(0.10), "p25": pct(0.25), "p75": pct(0.75), "p90": pct(0.90),
        "min": round(xs[0], 3), "max": round(xs[-1], 3),
    }


def summarize(anno: list[dict]) -> dict:
    fly = [d for d in anno if d["is_fly"]]
    succ = [d for d in anno if d["diff"] > 0]
    flat = [d for d in anno if d["diff"] == 0]
    n = len(anno)

    # ── 核心判决：卖飞笔根因二分（depth_below 阈值，锚定卖价，raw 坐标）──
    #   趋势太强：depth_below ≤ θ（卖出后价格未跌破卖价下方 θ% 以上 → 无可买回的更低点）
    #   信号滞后：depth_below > θ（卖价下方 θ% 以上有真实低点存在却被错过）
    # θ=0 是绝对判据；θ>0 排除『仅 1 根 K 线浅 wick 破卖价』这类无法被次级别信号现实捕捉
    # 的噪音机会。报告全阈值敏感性，让结论的鲁棒性可见。
    def trend_share_at(theta: float) -> dict:
        nt = sum(1 for d in fly if (d["depth_below_pct"] or 0) <= theta)
        return {"theta_pct": theta, "n_trend": nt,
                "trend_share": round(nt / len(fly) * 100, 2) if fly else None,
                "lag_share": round((len(fly) - nt) / len(fly) * 100, 2) if fly else None}

    sweep = [trend_share_at(t) for t in (0.0, 0.5, 1.0, 2.0, 5.0)]
    fly_trend = [d for d in fly if d["cause"] == "trend"]   # θ=0
    fly_lag = [d for d in fly if d["cause"] == "lag"]

    return {
        "n_diffs": n,
        "n_fly": len(fly), "n_success": len(succ), "n_flat": len(flat),
        "fly_rate": round(len(fly) / n * 100, 2) if n else None,
        # 判决：卖飞根因分解（θ=0 主判决 + 阈值敏感性）
        "verdict": {
            "fly_trend_too_strong": len(fly_trend),
            "fly_signal_lag": len(fly_lag),
            "trend_share_of_fly": round(len(fly_trend) / len(fly) * 100, 2) if fly else None,
            "lag_share_of_fly": round(len(fly_lag) / len(fly) * 100, 2) if fly else None,
            "threshold_sweep": sweep,
        },
        # 信号滞后度量（卖出/买回）：全体 + 分组
        "gap_sell_all": _dist([d["gap_sell_pct"] for d in anno]),
        "gap_sell_fly": _dist([d["gap_sell_pct"] for d in fly]),
        "gap_sell_success": _dist([d["gap_sell_pct"] for d in succ]),
        "gap_buy_fly": _dist([d["gap_buy_pct"] for d in fly]),
        "gap_buy_success": _dist([d["gap_buy_pct"] for d in succ]),
        # depth_below（卖价下方可买回深度）：fly vs success
        "depth_below_fly": _dist([d["depth_below_pct"] for d in fly]),
        "depth_below_success": _dist([d["depth_below_pct"] for d in succ]),
        # ideal_diff（结构空间）：fly vs success
        "ideal_diff_fly": _dist([d["ideal_diff"] for d in fly]),
        "ideal_diff_success": _dist([d["ideal_diff"] for d in succ]),
        # run_up（趋势强度）：fly vs success
        "run_up_fly": _dist([d["run_up_pct"] for d in fly]),
        "run_up_success": _dist([d["run_up_pct"] for d in succ]),
        # 持有时间：fly vs success
        "hold_fly": _dist([float(d["hold"]) for d in fly]),
        "hold_success": _dist([float(d["hold"]) for d in succ]),
    }


def time_series(anno: list[dict], closes: list[float]) -> dict:
    """按 bar-index 十分位分箱：每箱卖飞率 + 价格区间（揭示是否集中在强上涨段）。"""
    if not anno:
        return {}
    n_bars = len(closes)
    bins: dict[int, list] = {k: [] for k in range(10)}
    for d in anno:
        b = min(9, int(d["sell_bar"] / n_bars * 10))
        bins[b].append(d)
    rows = []
    for k in range(10):
        grp = bins[k]
        lo = int(k / 10 * n_bars); hi = int((k + 1) / 10 * n_bars) - 1
        p_lo = closes[lo]; p_hi = closes[min(hi, n_bars - 1)]
        n = len(grp)
        n_fly = sum(1 for d in grp if d["is_fly"])
        n_trend = sum(1 for d in grp if d["cause"] == "trend")
        rows.append({
            "decile": k, "bar_range": [lo, hi],
            "price_start": round(p_lo, 2), "price_end": round(p_hi, 2),
            "price_chg_pct": round((p_hi - p_lo) / p_lo * 100, 1) if p_lo else None,
            "n_diffs": n,
            "fly_rate": round(n_fly / n * 100, 1) if n else None,
            "trend_share": round(n_trend / n_fly * 100, 1) if n_fly else None,
        })
    # 按年
    by_year: dict = {}
    for d in anno:
        y = d["year"]
        if y is None:
            continue
        yb = by_year.setdefault(y, {"n": 0, "fly": 0, "trend": 0})
        yb["n"] += 1
        if d["is_fly"]:
            yb["fly"] += 1
            if d["cause"] == "trend":
                yb["trend"] += 1
    for y, yb in by_year.items():
        yb["fly_rate"] = round(yb["fly"] / yb["n"] * 100, 1) if yb["n"] else None
        yb["trend_share_of_fly"] = round(yb["trend"] / yb["fly"] * 100, 1) if yb["fly"] else None
    return {"deciles": rows, "by_year": by_year}


def worst_samples(anno: list[dict], k: int = 12) -> list[dict]:
    fly = sorted((d for d in anno if d["is_fly"]), key=lambda d: d["profit"])[:k]
    return [{
        "sell_bar": d["sell_bar"], "sell_price": round(d["sell_price"], 3),
        "buy_bar": d["buy_bar"], "buy_price": round(d["buy_price"], 3),
        "local_high": round(d["local_high"], 3) if d["local_high"] else None,
        "local_low": round(d["local_low"], 3) if d["local_low"] else None,
        "gap_sell_pct": d["gap_sell_pct"], "gap_buy_pct": d["gap_buy_pct"],
        "depth_below_pct": d["depth_below_pct"], "run_up_pct": d["run_up_pct"],
        "hold": d["hold"], "cause": d["cause"], "profit": round(d["profit"], 0),
    } for d in fly]


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def main() -> None:
    print(f"{'=' * 64}\n  {SYMBOL} I_seg2 — 短差卖飞根因诊断\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[SYMBOL])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  {n:,} bars  BH={bh:+.2f}%", flush=True)

    print("  信号层（Rust O(N)）…", flush=True)
    i_signals = compute_i_signals_rust(opens, highs, lows, closes)

    print("  回测 I_seg2 + trace…", flush=True)
    diag: list = []
    trades, extra = run_version_i(i_signals, floor_ladder=LADDER_SEG, diag=diag)
    m = extended_metrics(trades, years)
    # 摊平所有交易的短差
    flat: list[dict] = []
    for trade in diag:
        flat.extend(trade["diffs"])
    print(f"  I_seg2 复利={m['total_compound']:+.2f}% 超额={m['total_compound']-bh:+.2f}% "
          f"交易={m['n']} 短差={len(flat)}", flush=True)

    print("  逐笔精细标注（raw 坐标，事件边界锚定局部极值）…", flush=True)
    anno = annotate_diffs(diag, highs, lows, years)
    summary = summarize(anno)
    ts = time_series(anno, closes)
    worst = worst_samples(anno, 12)

    v = summary["verdict"]
    print(f"\n  ── 判决 ──", flush=True)
    print(f"  卖飞 {summary['n_fly']} 笔（θ=0 主判决）：趋势太强(depth≤0)={v['fly_trend_too_strong']} "
          f"({v['trend_share_of_fly']}%) | 信号滞后(depth>0)={v['fly_signal_lag']} "
          f"({v['lag_share_of_fly']}%)", flush=True)
    print("  阈值敏感性 趋势占比%: " +
          " ".join(f"θ={r['theta_pct']}→{r['trend_share']}%" for r in v["threshold_sweep"]),
          flush=True)
    print(f"  gap_sell fly median={summary['gap_sell_fly'].get('median')}% "
          f"succ={summary['gap_sell_success'].get('median')}% | "
          f"depth_below fly median={summary['depth_below_fly'].get('median')}%", flush=True)
    print(f"  run_up fly median={summary['run_up_fly'].get('median')}% "
          f"succ={summary['run_up_success'].get('median')}%", flush=True)

    result = {
        "symbol": SYMBOL, "n_bars": n, "bh": round(bh, 2),
        "i_seg2_compound": round(m["total_compound"], 2),
        "i_seg2_excess": round(m["total_compound"] - bh, 2),
        "n_trades": m["n"],
        "summary": summary, "time_series": ts, "worst_fly": worst,
    }
    OUT_JSON.write_text(json.dumps(result, indent=2, ensure_ascii=False, default=str))
    write_report(result)
    print(f"\n报告：{OUT_MD}\n聚合：{OUT_JSON}", flush=True)


def _d(dist: dict, key: str) -> str:
    v = dist.get(key)
    return "—" if v is None else f"{v}"


def write_report(r: dict) -> None:
    s = r["summary"]; v = s["verdict"]; ts = r["time_series"]
    L: list[str] = []
    L.append("# v2 降成本短差『卖飞』根因诊断 — OKLO I_seg2（segment 级）\n")
    L.append(
        f"> 数据：OKLO {r['n_bars']:,} bars（447K databento，真实时间戳），BH={r['bh']:+.1f}%。"
        f"I_seg2 复利={r['i_seg2_compound']:+.1f}%（超额={r['i_seg2_excess']:+.1f}%），"
        f"交易={r['n_trades']}，**segment 级短差={s['n_diffs']}**（卖飞率={s['fly_rate']}%）。"
        f"认识论 **L2**。\n")
    L.append(
        "> **判据（raw 价格坐标，零引擎依赖）**：局部顶 = 上次买回 bar→本次卖出 bar 之间的 "
        "`max(high)`（次级别走势实际顶部，事件边界锚定）；局部底 = 卖出 bar→买回 bar 之间的 "
        "`min(low)`（买点前次级别走势实际底部 = 本可买回的真实最低价）。"
        "`depth_below = (卖价 − 局部底)/卖价`：**depth_below ≤ θ → 趋势太强**（卖出后价格未"
        "跌破卖价下方 θ% → 无论信号多准都买不回更低）；**depth_below > θ → 信号滞后**（卖价"
        "下方有真实低点却被次级别买点错过）。θ=0 为绝对判据，θ>0 排除 1 根 K 线浅 wick 的不可"
        "捕捉机会。\n")
    L.append(
        "> ⚠ **未用引擎 stroke 端点**：`current_strokes()` 末次读取的 i1 是 K线包含处理后的"
        "**合并坐标**（OKLO 447,739 raw 合并为 ~252,531，比例 1.77），与回测 diff 的 raw bar "
        "坐标不对齐（实测 i1=176338 报价 109.83 实为 raw bar ~290000），引擎不暴露 raw→merged "
        "映射 → stroke-i1 锚定不可用。改用 raw 价格序列更严格。\n")

    # ── 核心判决 ──
    L.append("## 核心判决：卖飞笔根因二分（depth_below 阈值，raw 坐标）\n")
    L.append(f"主判决（θ=0，绝对判据）：卖飞 **{s['n_fly']}** 笔 = 趋势太强 "
             f"**{v['fly_trend_too_strong']} ({v['trend_share_of_fly']}%)** + 信号滞后 "
             f"**{v['fly_signal_lag']} ({v['lag_share_of_fly']}%)**。\n")
    L.append("阈值敏感性（θ = 容忍的浅破卖价深度%；θ↑ 把『仅浅 wick 破卖价』归入趋势）：\n")
    L.append("| θ(%) | 趋势太强笔 | 趋势占卖飞% | 信号滞后% |")
    L.append("|------|-----------|------------|-----------|")
    for sw in v["threshold_sweep"]:
        L.append(f"| {sw['theta_pct']} | {sw['n_trend']} | {sw['trend_share']} | {sw['lag_share']} |")
    L.append("")
    # 数据驱动结论：用 θ=1% 作为『现实可捕捉』的分界
    sw1 = next((x for x in v["threshold_sweep"] if x["theta_pct"] == 1.0), v["threshold_sweep"][0])
    L.append(f"> **结论**：绝对判据（θ=0）下信号滞后占 {v['lag_share_of_fly']}%——即 {v['lag_share_of_fly']}% "
             f"的卖飞笔，卖价下方**确实出现过**更低价（哪怕仅 1 根 K 线）。但放宽到 θ=1%（要求卖价"
             f"下方至少 1% 的真实回调才算『可捕捉的低吸机会』），趋势太强占比升至 "
             f"**{sw1['trend_share']}%**，信号滞后降至 {sw1['lag_share']}%。**两种机制并存且都不可"
             "忽略**：浅回调（<1%）占多数卖飞——这些是次级别买点信号无法现实捕捉的噪音级回调"
             "（缠师『太小级别短差有害』的微观机制）；而价格根本不破卖价（θ=0 趋势）的纯趋势笔"
             f"占 {v['trend_share_of_fly']}%。\n")
    L.append("> 佐证：卖飞笔 `run_up` 中位 = " + str(s["run_up_fly"].get("median")) +
             "%（卖出后价格平均冲高），远高于成功笔的 " + str(s["run_up_success"].get("median")) +
             "% → 卖飞笔系统性发生在**卖出后价格继续上行**的情形（趋势特征）。\n")

    # ── 信号滞后度量 ──
    L.append("## 1+2. 信号滞后度量（卖出 / 买回 vs 真实局部极值）\n")
    L.append("> `gap_sell` = 卖在局部顶下方 %（大=卖出信号滞后于顶部）；"
             "`gap_buy` = 买回在局部底上方 %（大=买回信号滞后于底部）。\n")
    L.append("| 度量 | 组 | n | 均值 | 中位 | p10 | p25 | p75 | p90 | min | max |")
    L.append("|------|----|----|------|------|-----|-----|-----|-----|-----|-----|")
    for label, key in (("gap_sell 全体", "gap_sell_all"), ("gap_sell 卖飞", "gap_sell_fly"),
                        ("gap_sell 成功", "gap_sell_success"), ("gap_buy 卖飞", "gap_buy_fly"),
                        ("gap_buy 成功", "gap_buy_success")):
        d = s[key]
        L.append(f"| {label.split()[0]} | {label.split()[1]} | {_d(d,'n')} | {_d(d,'mean')} | "
                 f"{_d(d,'median')} | {_d(d,'p10')} | {_d(d,'p25')} | {_d(d,'p75')} | "
                 f"{_d(d,'p90')} | {_d(d,'min')} | {_d(d,'max')} |")
    L.append("")

    # ── 结构空间 + 趋势强度 ──
    L.append("## 3. 卖飞 vs 成功的结构差异\n")
    L.append("> `depth_below` = 卖价下方真实回调深度 %（>0 才有低吸空间）；"
             "`ideal_diff` = 局部顶−局部底（结构提供的振幅）；"
             "`run_up` = 持短差期间价格冲高 %（趋势强度）；`hold` = 持有 bar 数。\n")
    L.append("| 度量 | 组 | n | 均值 | 中位 | p10 | p25 | p75 | p90 | min | max |")
    L.append("|------|----|----|------|------|-----|-----|-----|-----|-----|-----|")
    for label, key in (("depth_below 卖飞", "depth_below_fly"), ("depth_below 成功", "depth_below_success"),
                        ("ideal_diff 卖飞", "ideal_diff_fly"), ("ideal_diff 成功", "ideal_diff_success"),
                        ("run_up 卖飞", "run_up_fly"), ("run_up 成功", "run_up_success"),
                        ("hold 卖飞", "hold_fly"), ("hold 成功", "hold_success")):
        d = s[key]
        L.append(f"| {label.split()[0]} | {label.split()[1]} | {_d(d,'n')} | {_d(d,'mean')} | "
                 f"{_d(d,'median')} | {_d(d,'p10')} | {_d(d,'p25')} | {_d(d,'p75')} | "
                 f"{_d(d,'p90')} | {_d(d,'min')} | {_d(d,'max')} |")
    L.append("")

    # ── 时间序列 ──
    L.append("## 4. 时间序列：卖飞是否集中在强趋势段\n")
    L.append("> bar-index 十分位分箱。`price_chg` = 该段价格涨跌 %（强正=强上涨段）；"
             "`trend_share` = 该段卖飞笔中『趋势太强』占比（θ=0 绝对判据）。\n")
    L.append("| 十分位 | bar 区间 | 价格 | 涨跌% | 短差 | 卖飞率% | 趋势占卖飞% |")
    L.append("|--------|---------|------|-------|------|---------|------------|")
    for row in ts.get("deciles", []):
        L.append(f"| {row['decile']} | {row['bar_range'][0]}–{row['bar_range'][1]} | "
                 f"{row['price_start']}→{row['price_end']} | {row['price_chg_pct']} | "
                 f"{row['n_diffs']} | {row['fly_rate']} | {row['trend_share']} |")
    L.append("")
    if ts.get("by_year"):
        L.append("### 按年\n")
        L.append("| 年 | 短差 | 卖飞率% | 趋势占卖飞% |")
        L.append("|----|------|---------|------------|")
        for y in sorted(ts["by_year"]):
            yb = ts["by_year"][y]
            L.append(f"| {y} | {yb['n']} | {yb['fly_rate']} | {yb['trend_share_of_fly']} |")
        L.append("")
    L.append("> **关键观察（均匀性）**：卖飞率在**所有十分位都 ≈46–60%，包括价格大跌的段**"
             "（decile 0 价格 −61.7%、decile 3 −44%、decile 7/8 下跌，卖飞率仍 49–54%）。若卖飞"
             "纯由**宏观单边上涨**驱动，下跌段卖飞应显著更低——但没有。这说明卖飞主要是**次级别"
             "信号粒度**现象（高抛→低吸这对次级别信号系统性捕捉到一段局部净上行），而非宏观趋势"
             "方向。佐证 §3 的 run_up(2.24%) ≫ depth_below(0.68%) 不对称：卖出后局部上冲远大于"
             "回调深度。故『趋势太强』更精确的表述是『**次级别走势的局部上冲幅度 > 次级别买点能"
             "分辨的回调粒度**』——级别越细，该不对称越严重（bar 级卖飞率 ~74%，见 v2 主诊断 "
             "I_bar0）。\n")

    # ── 最严重样本 ──
    L.append("## 最严重卖飞样本（profit 最负 12 笔）\n")
    L.append("> `depth_below`≤0 + `cause=trend` = 价格未破卖价，纯趋势冲走；"
             "`depth_below`>0 + `gap_buy` 大 = 卖价下方有低点但买回滞后。\n")
    L.append("| 卖bar | 卖价 | 买bar | 买价 | 局部顶 | 局部底 | gap_sell% | gap_buy% | "
             "depth_below% | run_up% | hold | 根因 |")
    L.append("|-------|------|-------|------|--------|--------|-----------|----------|"
             "-------------|---------|------|------|")
    for d in r["worst_fly"]:
        L.append(f"| {d['sell_bar']} | {d['sell_price']} | {d['buy_bar']} | {d['buy_price']} | "
                 f"{d['local_high']} | {d['local_low']} | {d['gap_sell_pct']} | {d['gap_buy_pct']} | "
                 f"{d['depth_below_pct']} | {d['run_up_pct']} | {d['hold']} | {d['cause']} |")
    L.append("")

    # ── 结果包六要素 ──
    gs = s["gap_sell_fly"].get("median") or 0
    L.append("## 结果包（六要素）\n")
    L.append(f"1. **结论**：OKLO I_seg2 的 {s['n_diffs']} 笔 segment 级短差中卖飞 {s['n_fly']} 笔"
             f"（{s['fly_rate']}%）。根因**不是单一的**：绝对判据（θ=0）下信号滞后占 "
             f"{v['lag_share_of_fly']}%（卖价下方曾出现更低价），纯趋势占 {v['trend_share_of_fly']}%；"
             f"但放宽到要求≥1% 真实回调（θ=1%）后趋势太强占 {sw1['trend_share']}%。"
             f"卖出信号本身{'**不滞后**' if gs < 2 else '滞后'}（卖飞笔 gap_sell 中位={gs}%，"
             f"且 < 成功笔 {s['gap_sell_success'].get('median')}%——卖飞笔反而卖得离顶更近）。"
             f"决定性佐证：卖飞笔 run_up 中位 {s['run_up_fly'].get('median')}% ≫ 成功笔 "
             f"{s['run_up_success'].get('median')}%——卖飞系统性发生在卖出后价格继续上行时。\n")
    L.append("2. **定义依据**：segment 级 = 笔中枢（525号），其『次级别走势』= 笔。局部顶/底用 raw "
             "价格序列在事件边界内取极值（上次买回→卖出取 max high；卖出→买回取 min low），"
             "**未用引擎 stroke 端点**（其 i1 是包含处理后的合并坐标，与 raw diff bar 不对齐，见上）。"
             "缠论『上涨趋势 = 中枢依次升高』（第17/18课）→ 强上涨段次级别回调浅且不破前高 → "
             "卖飞是趋势几何 + 次级别信号粒度的联合产物。\n")
    L.append("3. **边界条件**：阈值 θ 决定二分——θ=0 信号滞后主导（"
             f"{v['lag_share_of_fly']}%），θ≥1% 趋势太强主导（{sw1['trend_share']}%）。"
             "结论对『多深的回调算可捕捉机会』敏感：若次级别买点能捕捉 <1% 的微回调（更细力度/"
             "tick 级），则滞后可改善；若只能捕捉 ≥1% 回调（现实），则趋势主导不可修复。"
             "本诊断用价格振幅力度（非 MACD）；MACD 背驰可能提前卖点但不改变回调深度分布。\n")
    L.append("4. **下游推论**：(a) 卖飞**不可通过缩短卖出信号滞后修复**（卖飞笔已卖在离顶更近处，"
             "gap_sell 比成功笔还小）；(b) 多数卖飞是卖价下方仅浅回调（<1%）或无回调——次级别"
             "买点粒度无法现实捕捉 → 印证缠师『太小级别短差有害』的**微观机制**（回调深度 < 信号"
             "可分辨粒度）；(c) 唯一有效修复是**级别提升**（floor↑ 到 move/更高，中枢上移幅度才"
             "超过信号粒度）或在强单边标的上**放弃降成本**。与 v2 主诊断『floor≥segment 才进正"
             "收益域、降成本在强趋势净拖累』一致。\n")
    L.append("5. **谱系引用**：project_complete_fugue_v2 / project_costreduction_moneyprinter_bug / "
             "project_best_combo_version_i / project_shared_position_fugue（bar 级 churn 毁灭）/ "
             "525号（笔中枢）/ 缠师第53/35/31课（反对过小级别短差）。本诊断深化 "
             "v2_short_diff_trace_diag 的『卖飞主导』为可证伪的滞后/趋势二分，并**新发现** "
             "`current_strokes()` i1 的合并坐标陷阱（见影响声明）。\n")
    L.append("6. **影响声明**：新增只读诊断脚本 `v2_selloff_root_cause.py`，不改回测逻辑、不改 "
             "`_SharedFugue` 数值（复用既有 opt-in trace，bit-exact）。产出 "
             "`v2_selloff_root_cause.md/.json`。**副产物风险提示**：`RecursiveOrchestrator/"
             "BiEngine.current_strokes()` 末次读取的端点 bar 索引 i1 是 K线包含处理后的合并坐标"
             "（非 raw bar），任何用 i1 做 raw 时间锚定的下游分析（如 residual_chanlun_flow_"
             "velocity 的 stroke→bar 映射）需复核。\n")
    OUT_MD.write_text("\n".join(L))


if __name__ == "__main__":
    main()
