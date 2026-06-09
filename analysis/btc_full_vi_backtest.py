"""BTC/USDT 1min 全历史 — Version I 回测 4 变体（Rust 驱动）。

4 变体（用户指令）：
  - E：背驰定位器进出场（无降成本），`compute_e_signals_rust` + run_swing_trading(MODE_NONE)。
  - I：Version I 完整版（多 FSM 多重赋格降成本，无止损），floor=segment（数据裁决最优 floor）。
  - I+stopA：I 叠加整体仓位硬止损（entry×0.98 全仓出）。
  - I+stopB：I 叠加每 FSM 切片止损（核心仓 entry×0.98 全出＋各声部短差逆向亏 2% 回补冻结）。

floor 固定为 segment（LADDER_SEG）——见记忆 [版本I完整版重写]：OKLO 447k 数据裁决
segment 为最优 floor（bar 级 churn 毁 PnL，move 级过稀疏）。

认识论：移植正确性 L0/L1（bit-exact，已在 OKLO/DX 上 differential test PASS）；
回测结论 L2（真实数据，BTC 单标的 1min 全历史，可产生否定性结果）。

性能口径：bi-zhongshu BSP O(strokes²) + process_bar O(N²) 为固有复杂度。BTC 全历史
≈4.6M bar，量级接近 ES（数小时），后台运行。
"""

from __future__ import annotations

import json
import math
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as F  # noqa: E402
import m1_e_rust_engine as RE  # noqa: E402
from fugue_version_i import (  # noqa: E402
    LADDER_SEG,
    extended_metrics,
    ladder_name,
    run_version_i,
)
from m1_i_rust_engine import compute_i_signals_rust  # noqa: E402

DATA = ROOT / "analysis" / "data_cache" / "btc_1m_full.json"
OUT_MD = ROOT / "analysis" / "btc_full_vi_results.md"
OUT_JSON = ROOT / "analysis" / "data_cache" / "btc_full_vi_results.json"

FLOOR = LADDER_SEG  # 数据裁决最优 floor（segment）


def load_ohlc(path: Path):
    """加载并清洗：删除任一 OHLC 为 nan 或非正的 bar，years 同步对齐。"""
    raw = json.loads(path.read_text())
    o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    d_in = raw.get("dates")
    _max = int(os.environ.get("BT_MAX_BARS", "0"))
    if _max > 0:
        o_in, h_in, l_in, c_in = o_in[:_max], h_in[:_max], l_in[:_max], c_in[:_max]
        if d_in is not None:
            d_in = d_in[:_max]
    opens: list[float] = []
    highs: list[float] = []
    lows: list[float] = []
    closes: list[float] = []
    years: list[int] = []
    first_date: str | None = None
    last_date: str | None = None
    for idx in range(len(c_in)):
        o, h, l, c = float(o_in[idx]), float(h_in[idx]), float(l_in[idx]), float(c_in[idx])
        if math.isnan(o) or math.isnan(h) or math.isnan(l) or math.isnan(c):
            continue
        if o <= 0 or h <= 0 or l <= 0 or c <= 0:
            continue
        opens.append(o)
        highs.append(h)
        lows.append(l)
        closes.append(c)
        if d_in is not None:
            d = str(d_in[idx])
            years.append(int(d[:4]))
            if first_date is None:
                first_date = d
            last_date = d
    return (opens, highs, lows, closes,
            (years if d_in is not None else None), first_date, last_date)


def _metric_row(m: dict) -> dict:
    return {
        "n": m["n"],
        "win_rate": m["win_rate"],
        "compound": m["total_compound"],
        "sharpe": m["sharpe"],
        "max_dd": m["max_dd"],
        "profit_factor": (None if m["profit_factor"] == float("inf")
                          else round(m["profit_factor"], 3)),
    }


def main() -> None:
    opens, highs, lows, closes, years, first_date, last_date = load_ohlc(DATA)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    date_range = (f"{first_date} → {last_date}" if first_date else None)
    print(f"BTC 全历史 V-I 回测：{n:,} bar，BH={bh:+.2f}%，{date_range}", flush=True)

    # ── E 基线 ──
    t0 = time.time()
    e_signals = RE.compute_e_signals_rust(opens, highs, lows, closes)
    e_trades, _ = F.run_swing_trading(e_signals, F.MODE_NONE)
    em = extended_metrics(e_trades, years)
    e_s = time.time() - t0
    del e_signals
    print(f"  [E] 复利={em['total_compound']:+.2f}% 超额={em['total_compound']-bh:+.2f}% "
          f"交易={em['n']} 夏普={em['sharpe']:+.3f} MDD={em['max_dd']:+.2f}% | {e_s:.0f}s",
          flush=True)

    # ── Version I 完整版（单次信号 pass → 3 止损模式复用）──
    t1 = time.time()
    i_signals = compute_i_signals_rust(opens, highs, lows, closes)
    sig_s = time.time() - t1
    print(f"  [I信号] compute_i_signals_rust 完成 | {sig_s:.0f}s", flush=True)

    variants: dict[str, dict] = {}
    for sm in ("none", "A", "B"):
        key = "I" if sm == "none" else f"I_stop{sm}"
        i_trades, i_extra = run_version_i(i_signals, floor_ladder=FLOOR, stop_mode=sm)
        im = extended_metrics(i_trades, years)
        variants[key] = {
            "floor": ladder_name(FLOOR),
            "stop_mode": sm,
            "metrics": _metric_row(im),
            "excess": round(im["total_compound"] - bh, 4),
            "fsm_contribution": i_extra["fsm_contribution"],
            "ladder_attribution": i_extra["ladder_attribution"],
            "addon_2buy_marks": i_extra["addon_2buy_marks"],
            "n_core_stops": i_extra["n_core_stops"],
            "n_voice_stops": i_extra["n_voice_stops"],
        }
        print(f"  [{key}] 复利={im['total_compound']:+.2f}% 超额={im['total_compound']-bh:+.2f}% "
              f"交易={im['n']} 夏普={im['sharpe']:+.3f} MDD={im['max_dd']:+.2f}% "
              f"核心止损={i_extra['n_core_stops']} voice止损={i_extra['n_voice_stops']}", flush=True)
    i_s = time.time() - t1
    del i_signals

    result = {
        "symbol": "BTCUSDT",
        "n_bars": n,
        "bh": round(bh, 4),
        "date_range": date_range,
        "first": closes[0],
        "last": closes[-1],
        "E": {"metrics": _metric_row(em), "excess": round(em["total_compound"] - bh, 4),
              "by_year": em.get("by_year", {})},
        "variants": variants,
        "e_seconds": round(e_s, 1),
        "i_seconds": round(i_s, 1),
    }
    OUT_JSON.write_text(json.dumps(result, indent=2, ensure_ascii=False, default=str))
    _write_report(result, years)
    print(f"\n报告：{OUT_MD}\n结果：{OUT_JSON}", flush=True)


def _write_report(r: dict, years) -> None:
    L: list[str] = []
    L.append("# BTC/USDT 1min 全历史 — Version I 回测（4 变体，Rust 驱动）\n")
    L.append("> 数据：Binance 公开归档（data.binance.vision）BTCUSDT 1min 现货全历史，"
             "24/7 连续无时段过滤。信号层全部 Rust 引擎驱动，逐位等价于 Python "
             "`compute_signals_i`（differential test PASS）。**认识论 L2**（真实数据，含否定性结果）。\n")
    L.append("> floor 固定 = segment（记忆 [版本I完整版重写] 数据裁决最优 floor）。"
             "力度口径=价格振幅 fallback（非 MACD，O(N²) 不可行）。\n")
    L.append("> **4 变体**：E（纯信号背驰进出场）/ I（多 FSM 多重赋格降成本无止损）/ "
             "I+止损A（整体仓位 entry×0.98 全出）/ I+止损B（每 FSM 切片独立止损）。硬止损 2%。\n")
    L.append(f"**标的**：BTCUSDT | **区间**：{r.get('date_range','?')} | **bars**：{r['n_bars']:,} | "
             f"**首价**：{r['first']:.2f} | **末价**：{r['last']:.2f} | **BH**：{r['bh']:+.2f}%\n")

    L.append("## 4 变体对照\n")
    L.append("| 变体 | floor | 止损 | 复利% | 超额(vs BH)% | 夏普 | MDD% | 交易 | 核心止损 | voice止损 |")
    L.append("|------|-------|------|-------|-------------|------|------|------|---------|----------|")
    e = r["E"]["metrics"]
    L.append(f"| E基线 | — | — | {e['compound']:+.2f} | {r['E']['excess']:+.2f} | "
             f"{e['sharpe']:+.3f} | {e['max_dd']:+.2f} | {e['n']} | — | — |")
    for key in ("I", "I_stopA", "I_stopB"):
        v = r["variants"].get(key)
        if not v:
            continue
        m = v["metrics"]
        L.append(f"| {key} | {v['floor']} | {v['stop_mode']} | {m['compound']:+.2f} | "
                 f"{v['excess']:+.2f} | {m['sharpe']:+.3f} | {m['max_dd']:+.2f} | {m['n']} | "
                 f"{v['n_core_stops']} | {v['n_voice_stops']} |")
    L.append("")

    L.append("## E 分年度收益（揭示 regime 依赖）\n")
    by = r["E"].get("by_year") or {}
    if by:
        L.append("| 年份 | 年内收益% | 交易 | 胜 | 胜率% |")
        L.append("|------|----------|------|----|------|")
        for y in sorted(by):
            d = by[y]
            if isinstance(d, dict):
                L.append(f"| {y} | {d.get('return_pct', 0):+.2f} | {d.get('n', '-')} | "
                         f"{d.get('wins', '-')} | {d.get('win_rate', 0):.1f} |")
            else:
                L.append(f"| {y} | {d} | - | - | - |")
        L.append("")

    L.append("## 每级 FSM 独立贡献度（多重赋格各声部）— I 无止损\n")
    v = r["variants"].get("I")
    if v:
        L.append("| 级别 | 回收现金 | 短差次数 | slice净增益 |")
        L.append("|------|---------|---------|-----------|")
        for lvl, d in v["fsm_contribution"].items():
            L.append(f"| {lvl} | {d['recovered_cash']:+.0f} | {d['n_short_diffs']} | "
                     f"{d['slice_net_gain']:+.0f} |")
        L.append("")
        L.append("## 级别归属分布（高层稀疏有效域）\n")
        L.append(f"归属(级别→笔数)：`{v['ladder_attribution']}`，2买标记：{v['addon_2buy_marks']}\n")

    L.append("## 耗时\n")
    L.append(f"E={r.get('e_seconds','-')}s，I（含信号 pass + 3 止损模式）={r.get('i_seconds','-')}s\n")
    OUT_MD.write_text("\n".join(L))


if __name__ == "__main__":
    main()
