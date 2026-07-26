"""BTC 近两周 1s 数据 × fusion_tr 回测（当前最优配置的秒级近窗实测）。

数据：data_cache/btc_1s_2week.json（Binance 现货 BTCUSDT 1s 日度归档，
2026-05-29 → 2026-06-11 UTC，1,209,600 bar 满秒零缺）。

臂位：fusion_tr = kind 时钟 × R2 削减位置门（BTC 1min 全史在册最优
+4298.4%，见 p7_conj_summary.json）；hold26 / fusion_t 并跑作上下文基线。
a0 = 1s K线（与在册 1min 口径不同——级别塔整体下移，见报告口径声明）。

摩擦三口径：
  零摩擦 close（主口径，与全史在册可比）；
  0.05%/侧（在册摩擦面，analyze fa_ 字段）；
  maker 0.02%/侧（Binance USDⓈ-M **永续** VIP0 maker 档——本脚本自述的
    目标执行场所；数据为现货价格序列，maker 成交假设按 trading_system
    保守口径属上界乐观面，报告中标注）。

    ★venue 口径落差（#303 编排者裁定 2026-07-26 切 spot，照实登记未改数值）：
    #303 把 rust 侧 CostModel 的 venue 假设裁为 Binance **现货**。本脚本这一
    摩擦面仍是永续 maker 档（0.02%/侧），未随之切换——现货 VIP0 maker/taker
    是 0.1%/侧（venue-fee-source-research-20260726.md §2.1），量级差 5 倍。
    数值未改（改动会翻转本口径的数值结论，属 CostModel 之外的独立数值改动，
    #303 明示本票只做声明面 + rust CostModel 口径）；改为**回收有效域**：见
    下方等级声明。

认识论等级：
  零摩擦 close / 0.05%侧 两口径 = L2（单标的/单时段真实数据）。两周窗口
  n=1，不构成 fusion_tr 有效域的扩展或否证——近窗表现观测，非假设检验。
  maker 0.02%/侧 口径 = **有效域悬置**（#303 起）：其 venue 假设（永续）与
  本仓已裁定的现货口径不一致，费率来源与数据来源不同 venue ⟹ 该口径的
  数值结论**不作任何等级的结论依据**，直到 venue 假设与费率一并重裁。

用法：PYTHONPATH=src .venv/bin/python analysis/btc_2week_1s_backtest.py
输出：data_cache/btc_1s_2week_result.json（含全部交易明细）
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_version_i import LADDER_SEG, MAX_LEVELS  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402
from p6_phase_machine_backtest import analyze, bh_mdd  # noqa: E402

DATA = ROOT / "analysis" / "data_cache" / "btc_1s_2week.json"
OUT = ROOT / "analysis" / "data_cache" / "btc_1s_2week_result.json"
MODES = ["hold26", "fusion_t", "fusion_tr"]
MAKER_SIDE = 0.0002   # Binance USDⓈ-M 永续 VIP0 maker（venue 落差见模块 docstring，#303 未改）
BOOK_SIDE = 0.0005    # 在册摩擦面（analyze 内部常数，此处仅作文档对照）
INIT = 100_000.0


def friction_face(trades: list, closes: list, side: float) -> dict:
    """给定单侧摩擦率的 NAV 重建：终值 + MDD（与 analyze 同构，摩擦率参数化）。"""
    n = len(closes)
    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    for (_lad, eb, ep, xb, xp, sh, _w, _dfr, _part, _reason, _pol) in trades:
        dshares[eb] += sh
        dshares[xb] -= sh
        dcash[eb] -= sh * ep * (1 + side)
        dcash[xb] += sh * xp * (1 - side)
    shares = 0.0
    pool = INIT
    peak = mdd = 0.0
    for i in range(n):
        shares += dshares[i]
        pool += dcash[i]
        nav = pool + shares * closes[i]
        peak = max(peak, nav)
        mdd = min(mdd, nav / peak - 1.0)
    return {"strat_pct": round((nav / INIT - 1) * 100, 2),
            "mdd_pct": round(mdd * 100, 2)}


def structure_counts(opens, highs, lows, closes) -> dict:
    """裸 orchestrator pass：级别涌现普查（笔/段/中枢/走势 + 递归层）。"""
    orch = nr.RecursiveOrchestrator(max_levels=MAX_LEVELS)
    for i in range(len(closes)):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
    rec = [{"level_id": lid, "zhongshus": len(zhs), "moves": len(mvs)}
           for (lid, zhs, mvs) in orch.current_recursive()]
    return {"strokes": orch.stroke_count(),
            "segments": len(orch.current_segments()),
            "zhongshus": len(orch.current_zhongshus()),
            "moves": len(orch.current_moves()),
            "recursive_levels": rec}


def main() -> None:
    raw = json.loads(DATA.read_text())
    opens, highs, lows, closes = (raw["opens"], raw["highs"],
                                  raw["lows"], raw["closes"])
    dates = raw["dates"]
    n = len(closes)
    years = [int(d[:4]) for d in dates]
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    print(f"[BTC-1s] bars={n:,} 范围 {dates[0]} → {dates[-1]} "
          f"BH={bh_pct:+.2f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    trend_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips)
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"[BTC-1s] 信号层 {time.time() - t0:.1f}s fp={fp}", flush=True)

    t0 = time.time()
    structs = structure_counts(opens, highs, lows, closes)
    structs["max_ladder_final"] = max(s.max_ladder for s in tape)
    print(f"[BTC-1s] 结构普查 {time.time() - t0:.1f}s {structs}", flush=True)

    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)
    out = {"symbol": "BTC-1s-2week", "n_bars": n, "tape_fp": fp,
           "range": [dates[0], dates[-1]],
           "bh_pct": round(bh_pct, 2),
           "bh_mdd_pct": round(bh_mdd(closes) * 100, 2),
           "structures": structs,
           "friction": {"book_side": BOOK_SIDE, "maker_side": MAKER_SIDE},
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=LADDER_SEG, mode=mode)
        a = analyze(res, closes, years)
        a["maker_face"] = friction_face(res["trades"], closes, MAKER_SIDE)
        a["trades_detail"] = [
            {"ladder": lad, "entry_ts": dates[eb], "entry_px": ep,
             "exit_ts": dates[xb] if xb < n else dates[-1], "exit_px": xp,
             "shares": round(sh, 6), "weight": w,
             "pnl_cash": round(sh * (xp - ep), 2),
             "pnl_pct": round((xp / ep - 1) * 100, 3),
             "held_bars": xb - eb, "reason": reason}
            for (lad, eb, ep, xb, xp, sh, w, _dfr, _part, reason)
            in res["trades"]]
        out["modes"][mode] = a
        print(f"[BTC-1s][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.2f}% fa(0.05%)={a['fa_strat_pct']:+.2f}% "
              f"maker(0.02%)={a['maker_face']['strat_pct']:+.2f}% "
              f"trades={a['n_trades']} mdd={a['mdd_pct']}% "
              f"reasons={a['exit_reasons']}", flush=True)

    OUT.write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print(f"✓ 落盘 {OUT}", flush=True)


if __name__ == "__main__":
    main()
