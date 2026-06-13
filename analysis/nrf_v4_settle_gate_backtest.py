"""NRF v4 + settle 门（Piece 2C）八标的回测——信号层 settle 门消融。

═══════════════════════════════════════════════════════════════════════
设计（编排者 2026-06-13 区间套三层架构）
═══════════════════════════════════════════════════════════════════════
交易基座 = **v4**（已回退 v5 清仓修复，`nested_fugue.rs` mode="nrf"；v5
清仓参照系修复已 L3 否证 `nrf_v5_clearance_fix.md`，v4 最优基座结算）。
NRF v4 严格会计（`docs/nested_fugue_accounting.md`）不变。

信号修正（正交于交易逻辑，Piece 1/2）：
  - **settle 门开启**（require_settled=True）：Level-0 段 BSP（ladder3 走势级）
    confirmed 加「次级别走势(线段)已 settle」合取——生长期 pending 伪背驰
    降级为 candidate（OKLO 447K 消融：type2 29.6%/合计 7.4%；§7.3）。
  - **递归层 BSP 定位**（已存在）：ladder≥4 = 严格走势类型，confirmed 定位
    （nest 区间套窗口消费，candidate 武装→confirmed 定位→deep_fire）。

对照 = 在册 `nrf_v4_<SYM>.json`（settle off，P1 3/8）。本脚本 = 实验臂
（settle on）；两臂同交易基座 v4，唯一差 = 信号层 settle 门。

═══════════════════════════════════════════════════════════════════════
预注册判据（先于运行声明；L3）
═══════════════════════════════════════════════════════════════════════
P1（任务判据）：settle-on strat_pct ≥ BH 逐标的；判据线 = ≥ v4 的 3/8，
   理想 8/8。
G1（CL 清仓解冻 = 任务"CL 清仓自然增加"）：CL sellpt 从 settle-off 的 3 次
   增加——假设 settle 门去伪信号 ⇒ 清仓判据(sell1@top∧located)用更干净信号
   ⇒ 油崩域该清多清。判据：CL sellpt(on) > 3 ∧ CL strat_pct(on) > +2.7%(off)。
G2（OKLO 保持 = 任务"不过度清仓"）：OKLO sellpt 保持低位（v4=3；v5 过度
   清仓 27 次→+52.6% 是反面教材）。判据：OKLO sellpt(on) ≤ 8 ∧
   OKLO strat_pct(on) ≥ +400%（不坍塌）。
G3（SCm 否证判据，承 `sublevel_confirmation_recursive.md §4` +
   `bsp_engine_sublevel_fix.md §7.4`）：settle 门移除 type2 29.6% 卖侧
   confirmed = 延迟卖侧信号。若强趋势标的（OKLO/BTC/QQQ）strat_pct 因出场
   延迟显著下降（Δ(on−off) < −100pp），则 settle 门在信号生成层重演 SCm
   确认滞后（−1189pp 机制）⇒ settle 门作 NRF 信号修正被否证。
   定位型消费（candidate 已触发方向，confirmed 只精化）应规避此延迟——
   G3 是定位型 vs 信号型的判据。
B1（指纹守卫）：settle-on tape_fp 记录入 json（settle-on 自身基线，**不**
   对照 settle-off 的 p7_conj 指纹——settle 门改变 confirmed ⟹ 指纹必漂移
   是有意修正）。同标的重跑指纹一致 = 管线稳定。

═══════════════════════════════════════════════════════════════════════
用法：PYTHONPATH=src .venv/bin/python analysis/nrf_v4_settle_gate_backtest.py [SYM ...]
输出：analysis/data_cache/nrf_v4sg_<SYM>.json + nrf_v4sg_summary.json
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

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from nested_recursive_fugue_final_backtest import (  # noqa: E402
    FLOOR,
    analyze,
    bh_mdd,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = sys.argv[1:] or ["OKLO", "QQQ", "BRN", "DX", "ES", "GC", "CL", "BTC"]


def load_v4_off(sym: str) -> dict | None:
    """在册 v4（settle off）对照读数。"""
    p = DATA_DIR / f"nrf_v4_{sym}.json"
    if not p.exists():
        return None
    d = json.loads(p.read_text())
    if "failed" in d:
        return None
    pr = d.get("preregistered", {})
    return {
        "strat_pct": d["nrf_v4"]["strat_pct"],
        "mdd_pct": d["nrf_v4"]["mdd_pct"],
        "sellpt": pr.get("N1_liquidations_vs_spawns", {}).get("sellpt_closes", None),
        "spawns": pr.get("N1_liquidations_vs_spawns", {}).get("spawns", None),
        "deep_fires": pr.get("N2_deep_fires", [None])[0],
        "phys_long_frac": pr.get("N3_phys_long_frac", None),
        "tape_fp": d.get("tape_fp"),
    }


def prereg(a: dict, bh_pct: float, n_bars: int, off: dict | None,
           sym: str) -> dict:
    """settle-on 预注册判据裁决（P1/G1/G2/G3/B1）。"""
    c = a["nrf_counters"]
    sellpt_on = a["exit_reasons"].get("sellpt", 0)
    strat_on = a["strat_pct"]
    out = {
        "P1_ge_bh": [strat_on, round(bh_pct, 1), strat_on >= bh_pct],
        "sellpt_on": sellpt_on,
        "spawns_on": sum(c["spawns"]),
        "deep_fires_on": sum(c["deep_fires"]),
        "phys_long_frac_on": round(c["phys_long_bars"] / n_bars, 3),
        "liquidations_on": sum(c["liquidations"]),
        "earning_adds_on": sum(c["earning_adds"]),
    }
    if off is not None:
        delta = round(strat_on - off["strat_pct"], 1)
        out["vs_v4_off"] = {
            "strat_off": off["strat_pct"], "strat_on": strat_on,
            "delta_pp": delta,
            "sellpt_off": off["sellpt"], "sellpt_on": sellpt_on,
            "mdd_off": off["mdd_pct"], "mdd_on": a["mdd_pct"],
        }
        # G3 SCm 否证判据：强趋势标的 Δ < −100pp ⇒ settle 门重演确认滞后
        if sym in ("OKLO", "BTC", "QQQ", "ES"):
            out["G3_scm_check"] = {
                "symbol_class": "strong_trend",
                "delta_pp": delta,
                "scm_replay": delta < -100.0,  # True = settle 门否证
            }
    # G1（CL 清仓解冻）/ G2（OKLO 保持）
    if sym == "CL" and off is not None:
        out["G1_cl_thaw"] = {
            "sellpt_off": off["sellpt"], "sellpt_on": sellpt_on,
            "thawed": sellpt_on > (off["sellpt"] or 0),
            "strat_improved": strat_on > off["strat_pct"],
        }
    if sym == "OKLO" and off is not None:
        out["G2_oklo_hold"] = {
            "sellpt_on": sellpt_on,
            "not_over_clearing": sellpt_on <= 8,
            "strat_on": strat_on,
            "not_collapsed": strat_on >= 400.0,
        }
    return out


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
    # ★ settle 门开启（Piece 2C 实验臂）
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips,
                                   require_settled=True)
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"[{sym}] 信号层(settle on) {time.time() - t0:.1f}s fp={fp}", flush=True)

    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)

    t1 = time.time()
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="nrf")
    a = analyze(res, closes, years)
    a["nrf_counters"]["deep_fires"] = res["n_nrf_deep_fires_by_ladder"]
    a["nrf_counters"]["phys_short_bars"] = res["nrf_phys_short_bars"]
    a["nrf_counters"]["earning_adds"] = res["n_nrf_earning_adds_by_ladder"]
    a["nrf_counters"]["earning_units"] = round(res["nrf_earning_units"], 3)
    a["nrf_counters"]["short_earning_hits"] = res["nrf_short_earning_hits"]
    a["nrf_counters"]["shrink_units"] = round(res["nrf_shrink_units"], 3)
    off = load_v4_off(sym)
    pr = prereg(a, bh_pct, n, off, sym)
    print(f"[{sym}][v4+settle] {time.time() - t1:.1f}s strat={a['strat_pct']:+.1f}% "
          f"mdd={a['mdd_pct']}% trades={a['n_trades']} "
          f"sellpt={pr['sellpt_on']} spawns={pr['spawns_on']} "
          f"deep={pr['deep_fires_on']} Lfrac={pr['phys_long_frac_on']} "
          f"P1={pr['P1_ge_bh'][2]} "
          f"Δoff={pr.get('vs_v4_off', {}).get('delta_pp', 'NA')}pp", flush=True)
    return {"symbol": sym, "n_bars": n, "tape_fp": fp,
            "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
            "design": "NRF v4（回退v5清仓修复）+ settle门(require_settled=True)；"
                      "Level-0 candidate(settle区分) + Level-1+ confirmed(递归层定位)",
            "nrf_v4_settle": a, "preregistered": pr}


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"nrf_v4sg_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            summary.append({"symbol": sym, "bh_pct": out["bh_pct"],
                            "on_pct": out["nrf_v4_settle"]["strat_pct"],
                            "on_mdd": out["nrf_v4_settle"]["mdd_pct"],
                            "preregistered": out["preregistered"]})
        (DATA_DIR / "nrf_v4sg_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
