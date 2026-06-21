"""螺旋引擎 v2（spiral）真流式回测 + unn 同数据差异对照（Step 6+7 验证）。

═══════════════ 范畴位置（与 backtest_unn_stream.py 对偶）═══════════════

`backtest_unn_stream.py` = unn × NautilusTrader 真流式。本模块 = spiral 真流式，
**复用同一 `StreamingSignalReader`** 逐 bar 产出的 sig，同时喂给 `UnnStream` 与
`SpiralStream`（apples-to-apples，两者 `push_bar` 签名相同 ⇒ `push_signal` 通用）。

为何同一脚本驱动两引擎：spiral 的验收命题是「操作=群作用」的 L2 验证 = spiral
**逐位复现** unn 的全部已验证操作（除 A 强平非群 gap G2）。bit-exact 对账只有在
**同一 sig 流**下才成立（信号层是共因，不能两次独立跑信号层引入浮点/状态差）。

═══════════════ 三个验收判据（架构 §2.4）═══════════════

1. **bit-exact（L2 表达力）**：spiral.trades == unn.trades ∧ final_nav 逐位等 ⇒
   群作用抽象足以表达 unn 全部操作。**失败**才有信息——落在群作用层=否证。
2. **必然性（prove panic）**：spiral 的 step 内 N1–N8/群关系/cross_level 每 bar
   panic 守卫。跑通无 panic ⇒ 必然性在群作用驱动下成立。panic = 进程崩溃 ⇒
   「panic 数」只有 0（完成）/ crash（未完成）两态。
3. **P-close（H¹ 闭合）观测**：spiral 的 `cross_level_closures` 计数 = Δr=−1
   兑现次数。这是 spiral 相对 unn 的**唯一新增可观测量**——但因 bit-exact，它
   **不改变 P&L**（P-close 是 unn 既有跨级别闭合的群论 re-description，扬弃=否定+
   保留+提升，非新机制 ⇒ 表达力 L2 非 alpha）。

用法（仓库根目录）：
    PYTHONPATH=src .venv/bin/python trading_system/backtest_spiral_stream.py --symbol CL
    PYTHONPATH=src .venv/bin/python trading_system/backtest_spiral_stream.py --symbol BTC --bars 0
输出：trading_system/data_cache/spiral_stream_<SYM>.json
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(REPO_ROOT / "src"))
sys.path.insert(0, str(REPO_ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402
from nautilus_trader.model.data import BarType  # noqa: E402

from nested_recursive_fugue_final_backtest import FLOOR, analyze, bh_mdd  # noqa: E402
from organic_signals import StreamingSignalReader, push_signal  # noqa: E402

from trading_system.backtest_unn import load_bars  # noqa: E402
from trading_system.config.instruments import INSTRUMENTS, make_instrument  # noqa: E402

OUT_DIR = REPO_ROOT / "trading_system" / "data_cache"

LADDER_NAMES = {2: "segment", 3: "move(L1)", 4: "recL2", 5: "recL3",
                6: "recL4", 7: "recL5", 8: "recL6"}


def analyze_spiral(res: dict, closes: list) -> dict:
    """spiral result dict → 指标（spiral 键名，区别于 analyze 的 unn `n_nrf_*` 键名）。

    strat_pct / mdd / by_ladder 来自 trades+equity+final_nav（spiral 与 unn 同名键），
    spiral 专属计数（cross_level_closures / liquidations / spawns）来自 spiral 键。
    """
    trades = res["trades"]
    strat_pct = (res["final_nav"] / 100_000.0 - 1) * 100
    equity = res["equity"]
    assert abs(equity[-1][1] - res["final_nav"]) < 1e-3, \
        f"equity 末值漂移：{equity[-1][1]} ≠ {res['final_nav']}"
    peak = mdd = 0.0
    for (_b, nav_i) in equity:
        peak = max(peak, nav_i)
        if peak > 0:
            mdd = min(mdd, nav_i / peak - 1.0)

    by_ladder: dict = {}
    reason_counts: dict[str, int] = {}
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        reason_counts[reason] = reason_counts.get(reason, 0) + 1
        pnl = sh * (xp - ep) if pol == "long" else sh * (ep - xp)
        d = by_ladder.setdefault((lad, pol), {"n": 0, "pnl_cash": 0.0, "wins": 0, "held": 0})
        d["n"] += 1
        d["pnl_cash"] += pnl
        d["wins"] += pnl > 0
        d["held"] += xb - eb
    for d in by_ladder.values():
        d["pnl_cash"] = round(d["pnl_cash"], 0)
        d["avg_held_bars"] = round(d["held"] / d["n"], 0)
        del d["held"]

    return {
        "strat_pct": round(strat_pct, 1),
        "mdd_pct": round(mdd * 100, 1),
        "n_trades": len(trades),
        "exit_reasons": reason_counts,
        "by_ladder": {f"{LADDER_NAMES.get(k, str(k))}/{pol}": v
                      for (k, pol), v in sorted(by_ladder.items())},
        # ── spiral 专属（架构 §6 H¹ 闭合观测）──
        "cross_level_closures": res["cross_level_closures"],
        "max_children": res["max_children"],
        "liquidations_by_ladder": res["n_liquidations_by_ladder"],
        "spawns_by_ladder": res["n_spawns_by_ladder"],
        "root_entries_by_ladder": res["n_root_entries_by_ladder"],
        "root_flips_by_ladder": res["n_root_flips_by_ladder"],
        "cascade_closes_by_ladder": res["n_cascade_closes_by_ladder"],
        "short_net_cash_by_ladder": [round(x, 0) for x in res["short_net_cash_by_ladder"]],
        # ── T 观测（群关系运行时读数）──
        "t50_monotone_violations": res["t50_monotone_violations"],
        "t56_radial_coverage": res["t56_radial_coverage"],
        "t57_onesided_layers": res["t57_onesided_layers"],
        "t58_active_levels": res["t58_active_levels"],
        "t59_degenerate_layers": res["t59_degenerate_layers"],
    }


def short_wins_summary(by_ladder: dict) -> dict:
    """空头声部胜负汇总（Part 3：BTC E spawn 子空头是否 wins>0）。"""
    out = {}
    tot_n = tot_w = 0
    tot_cash = 0.0
    for key, d in by_ladder.items():
        if key.endswith("/short"):
            out[key] = {"n": d["n"], "wins": d["wins"], "pnl_cash": d["pnl_cash"]}
            tot_n += d["n"]
            tot_w += d["wins"]
            tot_cash += d["pnl_cash"]
    out["_total_short"] = {"n": tot_n, "wins": tot_w, "pnl_cash": round(tot_cash, 0)}
    return out


def run(sym: str, max_bars: int | None) -> dict:
    spec = INSTRUMENTS[sym]
    instrument = make_instrument(spec)
    bar_type = BarType.from_str(f"{instrument.id}-1-MINUTE-LAST-EXTERNAL")
    print(f"加载 {sym}（max_bars={max_bars or '全量'}）...")
    t0 = time.time()
    _bars, opens, highs, lows, closes = load_bars(
        sym, bar_type, spec.price_precision, max_bars, instrument.size_precision)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"  {n:,} bar  {time.time() - t0:.1f}s  BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%")

    # ── 单一 sig 流 → 同时喂 unn + spiral（apples-to-apples，共因隔离）──
    dir_flips: list = []
    reader = StreamingSignalReader(dir_flips=dir_flips, require_settled=True)
    unn = nr.UnnStream(floor_ladder=FLOOR)
    spiral = nr.SpiralStream(floor_ladder=FLOOR)

    n_bar_div = 0
    first_div = None
    print("逐 bar 真流式（同一 sig → UnnStream + SpiralStream，prove 每 bar panic 守卫）...")
    t0 = time.time()
    for i in range(n):
        n_before = len(dir_flips)
        sig = reader.process_bar(i, opens[i], highs[i], lows[i], closes[i])
        flip_rows = [(lad, d) for (_b, lad, d) in dir_flips[n_before:]]
        tu = push_signal(unn, sig, flip_rows)
        ts = push_signal(spiral, sig, flip_rows)
        if tu != ts:
            n_bar_div += 1
            if first_div is None:
                first_div = (i, tu, ts)
    print(f"  逐 bar 完成 {time.time() - t0:.1f}s（无 panic ⇒ N1–N8/群关系 L2 守卫通过）")

    res_u = unn.finish()
    res_s = spiral.finish()

    a_u = analyze(res_u, closes, years=None)        # unn（n_nrf_* 键）
    a_s = analyze_spiral(res_s, closes)             # spiral（spiral 键）

    # ── bit-exact（L2 表达力）──
    trade_match = res_u["trades"] == res_s["trades"]
    nav_match = res_u["final_nav"] == res_s["final_nav"]
    bit_exact = trade_match and nav_match and n_bar_div == 0

    # ── Part 3：P-close 差异 = 0（bit-exact ⇒ 结构性恒等）──
    short_u = short_wins_summary(a_u["by_ladder"])
    short_s = short_wins_summary(a_s["by_ladder"])
    liq_u = sum(a_u["nrf_counters"]["liquidations"])
    liq_s = sum(a_s["liquidations_by_ladder"])

    print("\n========== spiral v2 真流式回测 + unn 差异对照 ==========")
    print(f"标的            : {sym}（{instrument.id}）  bars={n:,}")
    print(f"spiral strat_pct: {a_s['strat_pct']:+.1f}%   (BH {bh_pct:+.1f}%, "
          f"P1={'PASS' if a_s['strat_pct'] >= bh_pct else 'fail'})")
    print(f"spiral MDD      : {a_s['mdd_pct']:.1f}%   (BH_MDD {bh_dd:.1f}%)")
    print(f"spiral n_trades : {a_s['n_trades']}")
    print(f"cross_level_closures (Δr=−1 H¹ 兑现): {a_s['cross_level_closures']}")
    print(f"liquidations    : spiral={liq_s}  unn={liq_u}  Δ={liq_s - liq_u}")
    print(f"空头总计 spiral : {short_s.get('_total_short')}")
    print(f"空头总计 unn    : {short_u.get('_total_short')}")
    print(f"prove panic     : 0（进程完成 ⇒ N1–N8/群关系/cross_level 每 bar 守卫无违反）")
    print(f"bit-exact vs unn: {'✅ PASS' if bit_exact else '❌ MISMATCH'}  "
          f"(trade={'=' if trade_match else '≠'} nav={'=' if nav_match else '≠'} "
          f"bar_div={n_bar_div})")
    if first_div is not None:
        bar, tud, tsd = first_div
        print(f"  第一处分歧 @ bar {bar}: unn={tud}  spiral={tsd}")

    out = {
        "symbol": sym, "instrument_id": str(instrument.id), "n_bars": n,
        "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
        "max_bars": max_bars,
        "spiral": a_s,
        "P1_ge_bh": a_s["strat_pct"] >= bh_pct,
        "prove_panic": 0,
        "part3_pclose_vs_unn": {
            "bit_exact": bit_exact,
            "trade_match": trade_match,
            "nav_match": nav_match,
            "bar_divergences": n_bar_div,
            "spiral_strat_pct": a_s["strat_pct"],
            "unn_strat_pct": a_u["strat_pct"],
            "strat_delta": round(a_s["strat_pct"] - a_u["strat_pct"], 4),
            "liquidations_spiral": liq_s,
            "liquidations_unn": liq_u,
            "liquidations_delta": liq_s - liq_u,
            "short_spiral": short_s,
            "short_unn": short_u,
            "cross_level_closures": a_s["cross_level_closures"],
            "note": (
                "bit-exact ⇒ P-close（Δr=−1）是 unn 跨级别闭合的群论 re-description，"
                "非新机制；P&L/强平/空头胜负逐位恒等（表达力 L2，非 alpha L3）。"
                if bit_exact else
                "P-close 已激活（闭合下沉到次级别 r=k−1，NR-1 否定同级别 σ=e 闭合）⇒ "
                "spiral 与 unn 行为分歧（非 bit-exact）：D 回补改读次级别 nf[k−1]，闭合时机"
                "改变 ⇒ 空头胜负/强平/P&L 偏离 unn 基线。这是 L3 行为差异（regime 依赖），"
                "非纯 L2 re-description。"
            ),
        },
        "architecture": "单一 StreamingSignalReader 逐 bar → 同 sig 喂 UnnStream+SpiralStream；"
                        "spiral 复用 unn CenterBook/DepthRef 成本门，群作用层（C ChiralSeam/"
                        "E σ⁻¹∘τ/closure Δr=−1）为类型化路由 ⇒ bit-exact 由构造保证。",
    }
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / f"spiral_stream_{sym}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print(f"\n结果写入 {OUT_DIR / f'spiral_stream_{sym}.json'}")
    return out


def main() -> int:
    p = argparse.ArgumentParser(description="spiral v2 真流式回测 + unn 差异对照")
    p.add_argument("--symbol", default="CL")
    p.add_argument("--bars", type=int, default=0, help="最大 bar 数（0=全量）")
    args = p.parse_args()
    if args.symbol not in INSTRUMENTS:
        print(f"标的 {args.symbol} 未注册（可用: {list(INSTRUMENTS)}）", file=sys.stderr)
        return 1
    out = run(args.symbol, args.bars or None)
    return 0 if out["part3_pclose_vs_unn"]["bit_exact"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
