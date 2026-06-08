"""M1 Version I 完整版回测 — 7 标的 1min 10 年期货全量（Rust 驱动）。

E 基线（`run_swing_trading(MODE_NONE)`）+ Version I 完整版（多 FSM 多重赋格 + 笔中枢
segment 级 BSP + 三 floor 变体）。信号层全部由 Rust 引擎驱动：
  - E：`m1_e_rust_engine.compute_e_signals_rust`（逐位等价，已验证）。
  - I：`m1_i_rust_engine.compute_i_signals_rust`（逐位等价于 `compute_signals_i` 的
        i_signals，differential test 在 OKLO 120k 上 PnL 字段 + max_ladder 全 PASS）。

认识论等级：移植正确性 L0/L1（管线 bit-exact）；回测结论 L2（真实数据，7 期货 1min 10y）。

性能口径（透明声明，非降级）：bi-zhongshu BSP（ladder 2）每笔全量重算 = O(strokes²)，
process_bar = O(N²)——二者均为引擎/算法固有复杂度（项目已接受 process_bar 的 O(N²)）。
Rust 合并全链 + 零 stroke marshalling 把常数压到极限（OKLO 447k：105s），但复杂度不变，
故最大标的（ES 5.6M ≈ 330k strokes）单标的耗时数小时。28 核并行，墙钟由最大标的决定。
彻底消除 O(strokes²) 需增量流式 bi-zhongshu 重写（未做——会冒非 bit-exact 风险）。
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
    LADDER_MOVE,
    LADDER_SEG,
    extended_metrics,
    ladder_name,
    run_version_i,
)
from m1_i_rust_engine import compute_i_signals_rust  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "m1_i_rust_backtest_results.json"
OUT_MD = ROOT / "analysis" / "m1_i_rust_backtest_results.md"

SYMBOL_FILES = {
    "ES": DATA_DIR / "es_1m_databento_10y.json",
    "GC": DATA_DIR / "gc_1m_databento_10y.json",
    "CL": DATA_DIR / "cl_1m_databento_10y.json",
    "ZN": DATA_DIR / "zn_1m_databento_10y.json",
    "BRN": DATA_DIR / "brn_1m_databento_10y.json",
    "6E": DATA_DIR / "usd6e_1m_databento_10y.json",
    "DX": DATA_DIR / "dx_1m_databento_10y.json",
}

# I 三 floor 变体（同一信号 pass）：含 bar / 线段起 / 缠师走势级口径。
_VARIANTS = (("I_bar0", 0), ("I_seg2", LADDER_SEG), ("I_move3", LADDER_MOVE))
# 硬止损模式（用户指令，2%）：none=无 / A=整体仓位入场价 / B=每FSM切片短差独立。
_STOP_MODES = ("none", "A", "B")


def load_ohlc(path: Path):
    """加载并清洗：删除任一 OHLC 为 nan 的 bar，dates 同步对齐（by_year 需要）。"""
    raw = json.loads(path.read_text())
    o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    d_in = raw.get("dates")
    _max = int(os.environ.get("BT_MAX_BARS", "0"))
    if _max > 0:  # 冒烟测试用：限制 bar 数
        o_in, h_in, l_in, c_in = o_in[:_max], h_in[:_max], l_in[:_max], c_in[:_max]
        if d_in is not None:
            d_in = d_in[:_max]
    opens: list[float] = []
    highs: list[float] = []
    lows: list[float] = []
    closes: list[float] = []
    years: list[int] = []
    for idx in range(len(c_in)):
        o, h, l, c = float(o_in[idx]), float(h_in[idx]), float(l_in[idx]), float(c_in[idx])
        # 删除 nan 与非正价格（≤0 为无效数据：污染 PH/MACD 下游，且令价格阈值止损误触发）。
        # DX 含 2 根 close=0 垃圾 bar——nan 清洗漏网，会令 c<entry×0.98 误判为暴跌触发止损。
        if math.isnan(o) or math.isnan(h) or math.isnan(l) or math.isnan(c):
            continue
        if o <= 0 or h <= 0 or l <= 0 or c <= 0:
            continue
        opens.append(o)
        highs.append(h)
        lows.append(l)
        closes.append(c)
        if d_in is not None:
            years.append(int(str(d_in[idx])[:4]))
    return opens, highs, lows, closes, (years if d_in is not None else None)


def _metric_row(m: dict) -> dict:
    return {
        "n": m["n"],
        "win_rate": m["win_rate"],
        "compound": m["total_compound"],
        "sharpe": m["sharpe"],
        "max_dd": m["max_dd"],
        "profit_factor": (None if m["profit_factor"] == float("inf")
                          else round(m["profit_factor"], 3)),
        "n_with_cr": m.get("n_with_cr", 0),
    }


def run_symbol(symbol: str) -> tuple[str, dict]:
    path = SYMBOL_FILES[symbol]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100

    # ── E 基线（Rust 驱动）──
    t0 = time.time()
    e_signals = RE.compute_e_signals_rust(opens, highs, lows, closes)
    e_trades, _ = F.run_swing_trading(e_signals, F.MODE_NONE)
    em = extended_metrics(e_trades, years)
    e_s = time.time() - t0
    del e_signals
    print(
        f"  [{symbol:4s}/E] {n:>9,} bars BH={bh:+9.2f}% | E复利={em['total_compound']:+11.2f}% "
        f"超额={em['total_compound']-bh:+11.2f}% 交易={em['n']:4d} 夏普={em['sharpe']:+.3f} "
        f"MDD={em['max_dd']:+7.2f}% | {e_s:.0f}s",
        flush=True,
    )

    # ── Version I 完整版（Rust 驱动）：单次信号 pass → 三 floor 变体 ──
    t1 = time.time()
    i_signals = compute_i_signals_rust(opens, highs, lows, closes)
    sig_s = time.time() - t1

    # 每个 floor 跑 3 止损模式（none/A/B）——run_version_i 廉价，i_signals 单次 pass 复用。
    variants: dict[str, dict] = {}
    for tag, floor in _VARIANTS:
        for sm in _STOP_MODES:
            key = tag if sm == "none" else f"{tag}_stop{sm}"
            i_trades, i_extra = run_version_i(
                i_signals, floor_ladder=floor, stop_mode=sm)
            im = extended_metrics(i_trades, years)
            variants[key] = {
                "floor": ladder_name(floor),
                "stop_mode": sm,
                "metrics": _metric_row(im),
                "excess": round(im["total_compound"] - bh, 4),
                "fsm_contribution": i_extra["fsm_contribution"],
                "ladder_attribution": i_extra["ladder_attribution"],
                "addon_2buy_marks": i_extra["addon_2buy_marks"],
                "n_core_stops": i_extra["n_core_stops"],
                "n_voice_stops": i_extra["n_voice_stops"],
            }
            print(
                f"  [{symbol:4s}/{tag} floor={ladder_name(floor):7s} stop={sm:4s}] "
                f"复利={im['total_compound']:+11.2f}% 超额={im['total_compound']-bh:+11.2f}% "
                f"交易={im['n']:4d} 夏普={im['sharpe']:+.3f} MDD={im['max_dd']:+7.2f}% "
                f"核心止损={i_extra['n_core_stops']} voice止损={i_extra['n_voice_stops']}",
                flush=True,
            )
    i_s = time.time() - t1
    del i_signals

    return symbol, {
        "n_bars": n,
        "bh": round(bh, 4),
        "first": closes[0],
        "last": closes[-1],
        "E": {"metrics": _metric_row(em), "excess": round(em["total_compound"] - bh, 4),
              "by_year": em.get("by_year", {})},
        "variants": variants,
        "e_seconds": round(e_s, 1),
        "i_seconds": round(i_s, 1),
    }


def _write_report(results: dict) -> None:
    L: list[str] = []
    L.append("# M1 Version I 完整版回测 — 7 标的 1min 10 年期货（Rust 驱动）\n")
    L.append(
        "> E 基线（背驰定位器进出场，无降成本）+ Version I 完整版（多 FSM 多重赋格降成本 + "
        "笔中枢 segment 级真实 BSP + 三 floor 变体）。信号层全部 Rust 引擎驱动，逐位等价于 "
        "Python `compute_signals_i`（differential test PASS）。认识论 **L2**（真实数据，含否定性结果）。\n")
    L.append(
        "> 力度口径=价格振幅 fallback（非 MACD，O(N²) 不可行）；bi-zhongshu O(strokes²) + "
        "process_bar O(N²) 为固有复杂度，Rust 仅压常数。\n")
    L.append(
        "> **硬止损（2%）4 组对比**：E（纯信号）/ I（多FSM降成本无止损）/ I+止损A（整体仓位"
        "基于入场价 entry×0.98 全仓出）/ I+止损B（每FSM切片：核心仓 entry×0.98 全出＋各声部"
        "开放短差逆向亏2%回补冻结该级别）。每 floor 变体（bar0/seg2/move3）各跑 none/A/B。\n")
    L.append("## E vs I × 止损（none/A/B）对照\n")
    L.append("| 标的 | bars | BH% | 策略 | 止损 | 复利% | 超额% | 夏普 | MDD% | 交易 | 核心止损 | voice止损 |")
    L.append("|------|------|-----|------|------|-------|-------|------|------|------|---------|----------|")
    for s, r in results.items():
        e = r["E"]["metrics"]
        L.append(
            f"| {s} | {r['n_bars']:,} | {r['bh']:+.1f} | E基线 | — | {e['compound']:+.1f} | "
            f"{r['E']['excess']:+.1f} | {e['sharpe']:+.2f} | {e['max_dd']:+.1f} | {e['n']} | — | — |")
        for tag, _ in _VARIANTS:
            for sm in _STOP_MODES:
                key = tag if sm == "none" else f"{tag}_stop{sm}"
                v = r["variants"].get(key)
                if not v:
                    continue
                m = v["metrics"]
                L.append(
                    f"| {s} | | | {tag}({v['floor']}) | {sm} | {m['compound']:+.1f} | "
                    f"{v['excess']:+.1f} | {m['sharpe']:+.2f} | {m['max_dd']:+.1f} | {m['n']} | "
                    f"{v['n_core_stops']} | {v['n_voice_stops']} |")
    L.append("")
    L.append("## 每级 FSM 独立贡献度（多重赋格各声部）— I_bar0\n")
    L.append("| 标的 | 级别 | 回收现金 | 短差次数 | slice净增益 |")
    L.append("|------|------|---------|---------|-----------|")
    for s, r in results.items():
        v = r["variants"].get("I_bar0")
        if not v:
            continue
        for lvl, d in v["fsm_contribution"].items():
            L.append(f"| {s} | {lvl} | {d['recovered_cash']:+.0f} | {d['n_short_diffs']} | "
                     f"{d['slice_net_gain']:+.0f} |")
    L.append("")
    L.append("## 级别归属分布（揭示高层稀疏有效域）— I_bar0\n")
    L.append("| 标的 | 归属(级别→笔数) | 2买标记 |")
    L.append("|------|----------------|--------|")
    for s, r in results.items():
        v = r["variants"].get("I_bar0")
        if not v:
            continue
        L.append(f"| {s} | {v['ladder_attribution']} | {v['addon_2buy_marks']} |")
    L.append("")
    L.append("## 耗时\n")
    L.append("| 标的 | bars | E耗时s | I耗时s |")
    L.append("|------|------|--------|--------|")
    for s, r in results.items():
        L.append(f"| {s} | {r['n_bars']:,} | {r.get('e_seconds','-')} | {r.get('i_seconds','-')} |")
    OUT_MD.write_text("\n".join(L))


def main() -> None:
    symbols = [s for s in SYMBOL_FILES if SYMBOL_FILES[s].exists()]
    only = os.environ.get("BT_SYMBOLS")
    if only:
        symbols = [s.strip().upper() for s in only.split(",") if s.strip()]
    workers = int(os.environ.get("BT_WORKERS", "0")) or min(len(symbols), os.cpu_count() or 2)

    print(f"M1 Version I 回测：{len(symbols)} 标的，{workers} 进程并行")
    print(f"标的：{', '.join(symbols)}\n", flush=True)

    results: dict[str, dict] = {}
    # 增量加载已有结果（断点续跑）
    if OUT_JSON.exists():
        try:
            results = json.loads(OUT_JSON.read_text())
        except Exception:
            results = {}
    pending = [s for s in symbols if s not in results]
    print(f"待跑：{pending}（已完成：{list(results)}）\n", flush=True)

    t_all = time.time()
    if workers > 1 and len(pending) > 1:
        from concurrent.futures import ProcessPoolExecutor

        with ProcessPoolExecutor(max_workers=workers) as ex:
            futs = {ex.submit(run_symbol, s): s for s in pending}
            from concurrent.futures import as_completed
            for fut in as_completed(futs):
                sym, out = fut.result()
                results[sym] = out
                OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False, default=str))
                _write_report(results)
                print(f"  ✓ {sym} 完成并已写入（{time.time()-t_all:.0f}s）", flush=True)
    else:
        for s in pending:
            _, out = run_symbol(s)
            results[s] = out
            OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False, default=str))
            _write_report(results)
            print(f"  ✓ {s} 完成并已写入（{time.time()-t_all:.0f}s）", flush=True)

    print(f"\n总耗时 {time.time()-t_all:.0f}s（{len(pending)} 标的）")
    print(f"报告：{OUT_MD}\n结果：{OUT_JSON}")


if __name__ == "__main__":
    main()
