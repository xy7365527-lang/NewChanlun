"""双书独立逐仓 voice + 区间套递归到 a0（fusion_vd/vn/vdn）八标的 L3 回测。

上游：
- `rust/src/trading/dual_voice.rs`（实装：fusion_v 基座两正交轴）
- `analysis/bidirectional_nested_accounting.md` v2 §4.1（m>N 承载位）
- `analysis/interval_nesting_forward_positioning.md` §4.2（多层递归开放轴）
- 同源分岔声明：与 nrf（`nested_fugue.rs`，合一读法——区间套递归 = voice
  spawn）是同一编排者决断的两种读法，本脚本只裁决分离读法的 2×2 矩阵；
  与 nrf 的跨读法对比在判决文档层进行。

══════════════ 预注册判据（先于数据声明）══════════════

2×2 消融矩阵 {fusion_v 基线, fusion_vd, fusion_vn, fusion_vdn}：

D1（dual_book 主增量）：fusion_vd ≥ fusion_v，逐标的符号。
   双书拆解翻转断面：多头出清与空头开仓解耦（θ 配额 vs M=N），同级别
   多空可共存。若 D1 为负，归因分解：(a) 共存期对冲拖累（both_held_bars
   × 该期 P&L）；(b) 空头量从 M=N 改为 θ 配额的暴露差。
N1（nest_deep 主增量）：fusion_vn ≥ fusion_v，逐标的符号。
   链式贯通收紧触发（∀中间层窗口活动 ∧ bi 翻转沿）——预期 fire 数下降、
   高层 fire 占比上升；若 N1 为负且 fire 数趋零 ⇒ 链贯通概率过低 =
   严格区间套在事件稀疏磁带上的可达性否证（有价值的否定性结果）。
X1（交互项，VLg 负交互先例强制预注册）：
   (fusion_vdn − fusion_v) − (fusion_vd − fusion_v) − (fusion_vn − fusion_v)。
P1（编排者基准）：max(fusion_vd, fusion_vn, fusion_vdn) ≥ BH，逐标的。

机制可观测（非零判别义务）：
- fusion_vd：n_dual_short_opens / dual_both_held_bars 非零（恒零 ⇒ 双书
  路径不可达 = 实装失败而非机制否证）；
- fusion_vn：n_nest_fire_* 与 fusion_v 的逐层差分（链收紧的判别力）。

guard-v：fusion_v 复现在册 v2 值（`data_cache/unified_voice_v2_<SYM>.json`
   modes.fusion_v.strat_pct，0.05pp）——磁带代次 + 在册路径零接触双验证。

诚实声明（090号）：
- a0 端递归下界 = bi 层翻转沿（a0 序列上的笔转折）；bar 层无结构词汇。
- 空头载体 = 1x 虚拟逐仓；同级别双书共存 = 双份配额占用（逐仓本质）。
- 认识论等级：L3（八标的 1min 全史真实数据，可否证）。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/dual_voice_backtest.py [SYM ...]
输出：analysis/data_cache/dual_voice_<SYM>.json + dual_voice_summary.json
"""

from __future__ import annotations

import json
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
from universal_combination_backtest import analyze, bh_mdd  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
FLOOR = LADDER_SEG

MODES = ["fusion_v", "fusion_vd", "fusion_vn", "fusion_vdn"]
SYMBOLS = sys.argv[1:] or ["BTC", "OKLO", "BRN", "CL", "ES", "GC", "QQQ", "DX"]


def gates_dual(res: dict) -> dict:
    """双书/递归 nest 观测面（非零判别义务）。"""
    return {
        "dual_short_opens": res["n_dual_short_opens_by_ladder"],
        "dual_short_open_blocks": res["n_dual_short_open_blocks_by_ladder"],
        "both_held_bars": res["dual_both_held_bars_by_ladder"],
        "short_covers": res["n_short_covers_by_ladder"],
        "short_moveup_covers": res["n_short_moveup_covers_by_ladder"],
        "short_liquidations": res["n_short_liquidations_by_ladder"],
        "short_held_bars": res["short_held_bars_by_ladder"],
        "short_net_cash": [round(x, 0) for x in res["short_net_cash_by_ladder"]],
        "movedown_holds": res["n_short_trend_holds_by_ladder"],
        "restore_r2_blocks": res["n_short_r2_blocks_by_ladder"],
        "nest_arms": res["n_nest_arms_by_ladder"],
        "nest_fire_sell": res["n_nest_fire_sell_by_ladder"],
        "nest_fire_buy": res["n_nest_fire_buy_by_ladder"],
        "nest_breaks": res["n_nest_breaks_by_ladder"],
        "nest_lead_bars_sum": res["nest_lead_bars_sum"],
        "nest_lead_n": res["nest_lead_n"],
        "flip_shorts": res["n_flip_shorts_by_ladder"],
        "freeze_up_bars": res["freeze_up_bars_by_ladder"],
        "freeze_dn_bars": res["freeze_dn_bars_by_ladder"],
        "t2w_fires": res["n_t2w_fires_by_ladder"],
        "osc_net_cash": [round(x, 0) for x in res["osc_net_cash_by_ladder"]],
    }


def pnl_by_polarity(res: dict) -> dict:
    """多/空腿 P&L 分解（D1 失败语义归因基础）。"""
    agg: dict = defaultdict(lambda: {"n": 0, "pnl": 0.0, "win": 0})
    for t in res["trades"]:
        (lad, _eb, epx, _xb, xpx, sh, _w, _d, _p, reason, pol) = t
        pnl = sh * (epx - xpx) if pol == "short" else sh * (xpx - epx)
        key = f"{pol}:{reason}"
        agg[key]["n"] += 1
        agg[key]["pnl"] += pnl
        agg[key]["win"] += pnl > 0
        lkey = f"{pol}:lad{lad}"
        agg[lkey]["n"] += 1
        agg[lkey]["pnl"] += pnl
        agg[lkey]["win"] += pnl > 0
    return {k: {"n": v["n"], "pnl": round(v["pnl"], 0),
                "win_rate": round(v["win"] / v["n"], 3)}
            for k, v in sorted(agg.items())}


def fv2_in_book(sym: str) -> float | None:
    """fusion_v v2 在册值（guard-v 基线）。"""
    f = DATA_DIR / f"unified_voice_v2_{sym}.json"
    if not f.exists():
        return None
    d = json.loads(f.read_text())
    a = d.get("modes", {}).get("fusion_v")
    return a["strat_pct"] if isinstance(a, dict) and "strat_pct" in a else None


def verdicts(bh: float, modes: dict) -> dict:
    fv = modes["fusion_v"]["strat_pct"]
    vd = modes["fusion_vd"]["strat_pct"]
    vn = modes["fusion_vn"]["strat_pct"]
    vdn = modes["fusion_vdn"]["strat_pct"]
    d1 = vd - fv
    n1 = vn - fv
    return {
        "D1_dual_ge_v": vd >= fv, "D1_delta_pp": round(d1, 1),
        "N1_nest_ge_v": vn >= fv, "N1_delta_pp": round(n1, 1),
        "X1_interaction_pp": round((vdn - fv) - d1 - n1, 1),
        "P1_best_ge_bh": max(vd, vn, vdn) >= bh,
        "fv": fv, "vd": vd, "vn": vn, "vdn": vdn, "bh": round(bh, 1),
    }


def run_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    bh_dd = bh_mdd(closes) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%", flush=True)

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
    print(f"[{sym}] 信号层 {time.time() - t0:.1f}s fp={fp}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    out = {"symbol": sym, "n_bars": n, "tape_fp": fp,
           "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
           "design": "双书独立逐仓×递归区间套 2×2 消融（判据见 docstring）",
           "modes": {}}
    for mode in MODES:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=mode)
        a = analyze(res, closes, years)
        if mode != "fusion_v":
            a["gates"] = gates_dual(res)
            a["pnl_by_polarity"] = pnl_by_polarity(res)
        out["modes"][mode] = a
        print(f"[{sym}][{mode}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}%", flush=True)

    fv_book = fv2_in_book(sym)
    if fv_book is not None:
        now = out["modes"]["fusion_v"]["strat_pct"]
        out["guard_v"] = [now, fv_book, abs(now - fv_book) < 0.05]
        if not out["guard_v"][2]:
            out["failed"] = "guard_v_drift"
            return out
    else:
        out["guard_v"] = None  # 在册基线缺失——降级声明，不静默

    out["verdicts"] = verdicts(bh_pct, out["modes"])
    print(f"[{sym}] verdicts={json.dumps(out['verdicts'])}", flush=True)
    return out


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"dual_voice_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"],
                   "verdicts": out["verdicts"]}
            for mode in MODES:
                row[mode] = out["modes"][mode]["strat_pct"]
                row[f"{mode}_mdd"] = out["modes"][mode]["mdd_pct"]
            summary.append(row)
        (DATA_DIR / "dual_voice_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    ok = [r for r in summary if "failed" not in r]
    for key in ("D1_dual_ge_v", "N1_nest_ge_v", "P1_best_ge_bh"):
        hits = {r["symbol"]: r["verdicts"].get(key) for r in ok}
        npos = sum(bool(x) for x in hits.values())
        print(f"{key}: {npos}/{len(ok)} {hits}", flush=True)
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
