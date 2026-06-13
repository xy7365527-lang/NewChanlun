"""嵌套递归赋格 v4（严格会计体系）八标的回测。

设计规格：`docs/nested_fugue_accounting.md`（2026-06-12 编排者）。
核心修正：**卖点不清仓**——绝大多数卖点只做降成本（θ 配额释放 m 给子
voice，根始终持有 N−m）；清仓只在最高涌现级别走势完美（confirmed
sell@top，极少发生）。v2 的 C 规则（根完美→全链清算）被否定（摧毁嵌套
递归）；v3 根翻转已被 L3 否证（P1 2/8，BTC 强平归零）。

实装（`rust/src/trading/nested_fugue.rs`，mode="nrf"）：
- 卖出原子：m = 在手 × θ_sub/θ_total（53课配额留白的 hold26 在册裁决
  形态）；35课成本门决定是否可释放。
- 回补原子：confirmed 反向点@own-level = 走势完美 ⇒ 平子 + 父回满 +
  cost_pool 传导（子 P&L ≡ 父降成本，同一数字存一次）。
- 词汇分工：confirmed@own = 回补关闭；nest 定位@own = spawn 子声部。
- earning（§7）：cost_pool ≤ 0 后纯利润在买点增仓 Δ，N 重定基；空头侧
  挣负股数 L0 不可构造（在册结算优先），命中计数观测。
- 全局不变量（§8.1）：每 bar Σ(在手) = N_base，违反即 Err（fail-fast）。
- 区间套递归（第14环）保留：candidate@k 证据下探到 a0。

[镜像推导] 声明（090号）：子层空头无原文净空头锚——38:36 镜像授权止于
判断-动作序列；空头 voice = 现金弹药 + 回补义务（1x 凸性，损失有界）。

══════════════ 预注册判据（先于运行声明）══════════════

P1（任务判据）：v4 ≥ BH 逐标的；8/8 = 任务达成线。
   在册先验：8/8 从未达成（v2 2/8、v3 2/8，正域均 {CL,DX}=BH 弱域）。
N1 不清仓活性：sellpt（清仓）次数 ≪ root_entries 在 v2 的水平——清仓
   应为个位数/标的（§6"十年1-2次"）；spawns > 0。
N2 递归下探活性：n_nrf_deep_fires > 0。
N3 暴露修复：物理多头占比应 ≈ 链活跃占比（根持 N−m 不清仓 ⇒ 长暴露
   连续）；对照 v2 的 0.37-0.53。
N4 守恒与尾部：§8.1 不变量零违反（违反即 run Err）；强平计数；
   shrink_units / earning_units 报告（重定基双向观测）。
B1 守卫：tape_fp 与 p7_conj_<SYM>.json 在册指纹逐项一致；漂移 ⇒ 作废。

════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/nrf_v4_strict_accounting_backtest.py [SYM ...]
输出：analysis/data_cache/nrf_v4_<SYM>.json + nrf_v4_summary.json
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


def prereg_v4(a: dict, bh_pct: float, n_bars: int, prior: dict) -> dict:
    """v4 预注册判据裁决（P1/N1-N4，先于运行声明在模块 docstring）。"""
    c = a["nrf_counters"]
    out = {
        "P1_ge_bh": [a["strat_pct"], round(bh_pct, 1), a["strat_pct"] >= bh_pct],
        "N1_liquidations_vs_spawns": {
            "sellpt_closes": a["exit_reasons"].get("sellpt", 0),
            "spawns": sum(c["spawns"]),
            "active": sum(c["spawns"]) > 0,
        },
        "N2_deep_fires": [sum(c["deep_fires"]), sum(c["deep_fires"]) > 0],
        "N3_phys_long_frac": round(c["phys_long_bars"] / n_bars, 3),
        "N4": {
            "liquidations": sum(c["liquidations"]),
            "shrink_units": c["shrink_units"],
            "earning_adds": sum(c["earning_adds"]),
            "earning_units": c["earning_units"],
            "short_earning_hits": c["short_earning_hits"],
        },
    }
    out["vs_prior"] = {
        k: {"pct": v["pct"], "delta_pp": round(a["strat_pct"] - v["pct"], 1),
            "mdd": v["mdd"]}
        for k, v in prior.items()
    }
    return out


def load_prior(sym: str, fp: dict) -> dict:
    """读 v2/v3 在册读数（指纹一致才可比）。"""
    prior = {}
    for tag, fname, key in (("v2", f"nrf_final_{sym}.json", "nrf"),
                            ("v3", f"nrf_v3_{sym}.json", "nrf_v3")):
        p = DATA_DIR / fname
        if not p.exists():
            continue
        d = json.loads(p.read_text())
        if "failed" in d or d.get("tape_fp") != fp:
            continue
        prior[tag] = {"pct": d[key]["strat_pct"], "mdd": d[key]["mdd_pct"]}
    return prior


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
    pr = prereg_v4(a, bh_pct, n, load_prior(sym, fp))
    print(f"[{sym}][nrf_v4] {time.time() - t1:.1f}s strat={a['strat_pct']:+.1f}% "
          f"mdd={a['mdd_pct']}% trades={a['n_trades']} "
          f"sellpt={pr['N1_liquidations_vs_spawns']['sellpt_closes']} "
          f"spawns={pr['N1_liquidations_vs_spawns']['spawns']} "
          f"deep={pr['N2_deep_fires'][0]} Lfrac={pr['N3_phys_long_frac']} "
          f"earn={pr['N4']['earning_adds']} liq={pr['N4']['liquidations']} "
          f"P1={pr['P1_ge_bh'][2]}", flush=True)
    return {"symbol": sym, "n_bars": n, "tape_fp": fp,
            "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
            "design": "NRF v4 严格会计（docs/nested_fugue_accounting.md：不清仓"
                      "+θ配额释放+五不变量+earning重定基）；空头侧 [镜像推导]",
            "nrf_v4": a, "preregistered": pr}


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"nrf_v4_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            summary.append({"symbol": sym, "bh_pct": out["bh_pct"],
                            "v4_pct": out["nrf_v4"]["strat_pct"],
                            "v4_mdd": out["nrf_v4"]["mdd_pct"],
                            "preregistered": out["preregistered"]})
        (DATA_DIR / "nrf_v4_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
