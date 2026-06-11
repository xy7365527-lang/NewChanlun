"""41课域腿门 regime 分段分析：拦截率/osc 盈亏按牛熊震荡阶段分解。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-11 追加质询：41课门在牛市/牛熊转换/熊市的表现是否一样）
═══════════════════════════════════════════════════════════════════════
方向语义的代码层事实（不依赖回测，先于本脚本成立）：
1. 门只消费 up_unexhausted（Up 侧判据）——熊市中父级别无"相邻 Up 段
   创新高"，判据恒 false → 门自动放行；
2. voice（含 step_osc）只在 master LONG 块内运行——熊市 master 出场后
   连门的求值都不发生；
3. 账本 osc 腿只有"先卖后买"（持多仓内 Short 短差）一种形态——
   "熊市 osc 买开"与"做空 osc 卖开"在当前账本不可表示。

本脚本提供经验面：按年分段（year 粒度，外生日历标注）统计
- 该年 BH 收益 → regime 标签（bull > +15% / bear < −15% / range 其间）；
- master 暴露（持仓 bar 占比）——区分"门放行"与"无持仓"两种零拦截；
- osc 腿数/盈亏（diag leg trace，按 sell_bar 归年，两变体对照）；
- o41 拒开数（osc_l41_reject_log 按 bar 归年）与拦截率。

点名窗口：BTC 2022 熊、GC 2022 回调、ES 2022 熊 + 2023 反弹。

输出：analysis/data_cache/osc_l41_regime_{SYM}.json
用法：BT_SYMBOLS=BTC,GC,ES PYTHONPATH=src .venv/bin/python \
      analysis/osc_l41_regime_analysis.py
"""

from __future__ import annotations

import json
import os
import sys
import time
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG
SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "BTC").split(",")]
BULL_TH, BEAR_TH = 0.15, -0.15  # 年 BH 收益的 regime 阈值（外生标注）


def year_regimes(closes: list[float], years: list[int]) -> dict[int, dict]:
    """逐年 BH 收益 → regime 标签。年边界 = year 值变化点（bar 粒度）。"""
    spans: dict[int, list[int]] = defaultdict(list)
    for i, y in enumerate(years):
        spans[y].append(i)
    out = {}
    for y, idxs in sorted(spans.items()):
        i0, i1 = idxs[0], idxs[-1]
        ret = closes[i1] / closes[i0] - 1
        label = ("bull" if ret > BULL_TH else
                 "bear" if ret < BEAR_TH else "range")
        out[y] = {"i0": i0, "i1": i1, "bh_ret_pct": round(ret * 100, 1),
                  "regime": label, "n_bars": len(idxs)}
    return out


def held_bars_by_year(trades_raw: list, years: list[int]) -> dict[int, int]:
    """master 持仓 bar 数按年分解（trade 跨年时逐年切分）。"""
    held: dict[int, int] = defaultdict(int)
    for t in trades_raw:
        e, x = int(t[0]), int(t[2])
        for i in range(e, min(x, len(years))):
            held[years[i]] += 1
    return dict(held)


def osc_by_year(diag: list, years: list[int]) -> dict[int, dict]:
    """osc 域腿按 sell_bar 归年：腿数/盈亏。"""
    agg: dict[int, dict] = defaultdict(lambda: {"n": 0, "profit": 0.0})
    for _h, diffs in diag:
        for (_key, kind, sb, _sp, _bb, _bp), (_sh, _df, profit, *_r) in diffs:
            if kind != "osc":
                continue
            y = years[int(sb)] if int(sb) < len(years) else None
            if y is None:
                continue
            agg[y]["n"] += 1
            agg[y]["profit"] += profit
    return {y: {"n": v["n"], "profit": round(v["profit"], 1)}
            for y, v in agg.items()}


def process_symbol(symbol: str) -> None:
    out_json = DATA_DIR / f"osc_l41_regime_{symbol}.json"
    if out_json.exists() and os.environ.get("BT_FORCE", "0") != "1":
        print(f"[skip] {symbol} 已有产出", flush=True)
        return
    print(f"\n{'=' * 72}\n  {symbol} — 41课域腿门 regime 分段（floor={FLOOR}）"
          f"\n{'=' * 72}", flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    if not years:
        print(f"  [{symbol}] 无日期数据，regime 分段不可定义——跳过", flush=True)
        return
    regimes = year_regimes(closes, years)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips)
    print(f"  信号层 {time.time() - t0:.1f}s", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)

    cells = {}
    for variant in ("V2oa25_ht", "V2oa25_ht_o41"):
        res = nr.run_organic_rust(rtape, variant, floor_ladder=FLOOR,
                                  stop_mode="none", diag=True)
        cells[variant] = {
            "osc_year": osc_by_year(res["diag"], years),
            "held_year": held_bars_by_year(res["trades"], years),
            "rejects_year": dict(sorted(
                _count_by_year(res["counters"]["osc_l41_reject_log"], years)
                .items())),
        }

    ht, o41 = cells["V2oa25_ht"], cells["V2oa25_ht_o41"]
    rows = []
    print(f"  {'年':>5s} {'regime':>6s} {'BH%':>7s} {'暴露ht':>7s} "
          f"{'osc腿ht':>7s} {'osc盈亏ht':>11s} {'osc腿o41':>8s} "
          f"{'osc盈亏o41':>11s} {'拒':>5s} {'拦截率':>7s}")
    for y, r in regimes.items():
        oh = ht["osc_year"].get(y, {"n": 0, "profit": 0.0})
        oo = o41["osc_year"].get(y, {"n": 0, "profit": 0.0})
        nrej = o41["rejects_year"].get(y, 0)
        att = nrej + oo["n"]
        expo = ht["held_year"].get(y, 0) / r["n_bars"]
        row = {"year": y, **r,
               "exposure_ht": round(expo, 3),
               "osc_ht": oh, "osc_o41": oo,
               "n_rejects": nrej,
               "reject_rate": round(nrej / att, 4) if att else None,
               "osc_profit_delta": round(oo["profit"] - oh["profit"], 1)}
        rows.append(row)
        print(f"  {y:>5d} {r['regime']:>6s} {r['bh_ret_pct']:>+7.1f} "
              f"{expo:>7.1%} {oh['n']:>7d} {oh['profit']:>+11.1f} "
              f"{oo['n']:>8d} {oo['profit']:>+11.1f} {nrej:>5d} "
              f"{(nrej / att if att else 0):>7.1%}", flush=True)

    out = {"symbol": symbol, "floor": FLOOR,
           "regime_thresholds": {"bull": BULL_TH, "bear": BEAR_TH},
           "rows": rows}
    out_json.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print(f"  [{symbol}] 已落盘 {out_json.name}", flush=True)


def _count_by_year(reject_log: list, years: list[int]) -> dict[int, int]:
    cnt: dict[int, int] = defaultdict(int)
    for _lad, bar in reject_log:
        if int(bar) < len(years):
            cnt[years[int(bar)]] += 1
    return dict(cnt)


def main() -> None:
    for sym in SYMBOLS:
        process_symbol(sym)
    print("\n[done]", flush=True)


if __name__ == "__main__":
    main()
