"""嵌套递归赋格 v5（清仓参照系修复）八标的回测。

设计规格：`docs/nested_fugue_accounting.md` + 诊断 `analysis/cl_clearance_diagnosis.md`。
v4→v5 在 `rust/src/trading/nested_fugue.rs` **原位**演替（mode 仍 "nrf"）；
v4 读数从 `analysis/data_cache/nrf_v4_<SYM>.json` 缓存对照（指纹一致才可比）。

核心修正（A+B，对 v4 不清仓原则在深崩域坍塌的修复）：
- **修复A（清仓参照系解耦棘轮）**：清仓参照层从「历史累积≥10中枢的最高
  阶梯 top」（单调攀高棘轮）改为「当前 located 活跃的最高级别 top* =
  max{ k≥floor : located_sell[k] 活跃 }」；清仓 ⟺ confirmed Sell1@top*。
  ⇒ 清仓频率成为波动率的涌现函数（高波动多清/单边强牛少清），零 per-asset
  参数。入场层 top（棘轮）与清仓 top* 分离（诊断下游推论 3）。
- **修复B（清仓后 root 真正重置）**：清仓后 located + nest 区间套窗口全清
  ——旧走势完美即结束、新走势从 a0 重涌现。修复A 让清仓频繁化 ⇒ F 频繁
  在棘轮 top 重入 ⇒ 恢复高级别重涌现（v2 alpha 源：清仓→root→爬到 recL3）。
  depth_ref 不重置（级别涌现是市场因果性质；重置→root 落 floor→递归归零）。

══════════════ 预注册判据（先于运行声明）══════════════

任务判据（验收线）：
  R1：CL 从 +2.7%（v4）大幅改善（v2 +221.5% 是上界参照）。
  R2：OKLO 不恶化（≥ v4 读数 +695%）。
  R3：BTC 不恶化（≥ v4 读数 +970%）。
  R4：P1 ≥ 3/8（不低于 v4 的正域规模）。
  R5：MDD 8/8 仍全优于 BH。
诊断判据（机制验证，附带报告）：
  D-R1：CL 清仓次数 > OKLO 清仓次数（频率随波动率涌现；否则参照系未解耦）。
  D-R5：CL root_entries 高层（recL3 = ladder5）> 0（root 重涌现激活）。
B1 守卫：tape_fp 与 p7_conj_<SYM>.json 在册指纹逐项一致；漂移 ⇒ 作废。

认识论等级：L3（八标的真实数据交叉验证，可否证）。

════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/nrf_v5_clearance_fix_backtest.py [SYM ...]
输出：analysis/data_cache/nrf_v5_<SYM>.json + nrf_v5_summary.json
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

# recL3 = ladder5（区间套阶梯命名：2=segment,3=move(L1),4=recL2,5=recL3,6=recL4）。
RECL3_LADDER = 5


def load_prior(sym: str, fp: dict) -> dict:
    """读 v4/v2 在册读数（指纹一致才可比）。"""
    prior = {}
    for tag, fname, key in (("v4", f"nrf_v4_{sym}.json", "nrf_v4"),
                            ("v2", f"nrf_final_{sym}.json", "nrf")):
        p = DATA_DIR / fname
        if not p.exists():
            continue
        d = json.loads(p.read_text())
        if "failed" in d or d.get("tape_fp") != fp:
            continue
        prior[tag] = {"pct": d[key]["strat_pct"], "mdd": d[key]["mdd_pct"],
                      "sellpt": d[key]["exit_reasons"].get("sellpt", 0),
                      "root_recl3": d[key]["nrf_counters"]["root_entries"][RECL3_LADDER]
                      if len(d[key]["nrf_counters"]["root_entries"]) > RECL3_LADDER else 0}
    return prior


def prereg_v5(a: dict, bh_pct: float, bh_mdd_pct: float, n_bars: int,
              prior: dict) -> dict:
    """v5 预注册裁决（任务 R1-R5 + 诊断 D-R1/D-R5；先于运行声明在 docstring）。"""
    c = a["nrf_counters"]
    sellpt = a["exit_reasons"].get("sellpt", 0)
    root_entries = c["root_entries"]
    root_recl3 = root_entries[RECL3_LADDER] if len(root_entries) > RECL3_LADDER else 0
    out = {
        "P1_ge_bh": [a["strat_pct"], round(bh_pct, 1), a["strat_pct"] >= bh_pct],
        "sellpt_closes": sellpt,
        "spawns": sum(c["spawns"]),
        "deep_fires": sum(c["deep_fires"]),
        "root_entries": root_entries,
        "root_recl3": root_recl3,
        "mdd_vs_bh": [a["mdd_pct"], round(bh_mdd_pct, 1),
                      a["mdd_pct"] > bh_mdd_pct],  # MDD 为负，越大（越接近0）越优
        "N4": {
            "liquidations": sum(c["liquidations"]),
            "shrink_units": c["shrink_units"],
            "earning_adds": sum(c["earning_adds"]),
            "earning_units": c["earning_units"],
            "short_earning_hits": c["short_earning_hits"],
        },
        "vs_prior": {
            k: {"pct": v["pct"], "delta_pp": round(a["strat_pct"] - v["pct"], 1),
                "mdd": v["mdd"], "sellpt": v["sellpt"], "root_recl3": v["root_recl3"]}
            for k, v in prior.items()
        },
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
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips)
    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    print(f"[{sym}] 信号层 {time.time() - t0:.1f}s fp={fp}", flush=True)

    ref = json.loads((DATA_DIR / f"p7_conj_{sym}.json").read_text())
    if ref["tape_fp"] != fp:
        return {"symbol": sym, "failed": "tape_fp_drift",
                "fp": fp, "ref_fp": ref["tape_fp"]}
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
    pr = prereg_v5(a, bh_pct, bh_dd, n, load_prior(sym, fp))
    d4 = pr["vs_prior"].get("v4", {})
    print(f"[{sym}][nrf_v5] {time.time() - t1:.1f}s strat={a['strat_pct']:+.1f}% "
          f"mdd={a['mdd_pct']}% trades={a['n_trades']} "
          f"sellpt={pr['sellpt_closes']} spawns={pr['spawns']} "
          f"deep={pr['deep_fires']} root_recl3={pr['root_recl3']} "
          f"Δv4={d4.get('delta_pp', 'NA')}pp P1={pr['P1_ge_bh'][2]} "
          f"MDD>BH={pr['mdd_vs_bh'][2]}", flush=True)
    return {"symbol": sym, "n_bars": n, "tape_fp": fp,
            "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
            "design": "NRF v5 清仓参照系修复（修复A：top* = located 活跃最高层"
                      "解耦棘轮；修复B：清仓后区间套归零 root 重涌现）",
            "nrf_v5": a, "preregistered": pr}


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"nrf_v5_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            summary.append({"symbol": sym, "bh_pct": out["bh_pct"],
                            "v5_pct": out["nrf_v5"]["strat_pct"],
                            "v5_mdd": out["nrf_v5"]["mdd_pct"],
                            "preregistered": out["preregistered"]})
        (DATA_DIR / "nrf_v5_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
