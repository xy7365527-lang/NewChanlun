"""嵌套递归赋格 v3（根翻转 + 递归区间套）八标的回测。

上游：2026-06-12 编排者任务"在 NRF v2 基础上补完两个概念链缺口"。
v2 死因定位：① 根 confirmed 卖后空仓等待（物理暴露 0.37-0.53，BH 在涨）
——概念链第23环"出场=翻转=新建仓"未实现；② nest_forward 只下探一层
——第14环要求递归到 a0。

v3 实装（`rust/src/trading/nested_fugue.rs`，引擎 mode 不变 = "nrf"）：
1. **根翻转（第23环字面）**：根走势完美 ⇒ 全链清算 ⇒ 立即按新势方向在
   当前最高 θ 涌现层满仓重建根（保留根爬升）。根空头 = 现金等待回补
   （零杠杆；P&L 物理实现 = 翻回多头时股数变化），与子空头同律（正则
   化）：可 spawn 子多 voice 降成本、受 1x 逐仓强平兜底。
2. **递归区间套（第14环完整）**：candidate@k 触发证据从 k−1 起逐层下探
   ——直接证据 ⇒ 触发；该层同侧窗口武装（嵌套链不断）⇒ 继续下探，
   直到 a0（bi 层方向翻转沿）。最低层证据最先出现 ⇒ 触发时点尽可能早。

[镜像推导] 声明（090号）：根/子空头均无原文净空头锚——38:36 镜像授权止
于判断-动作序列；空头相位物理形态 = 现金（NAV 持平），增益体现为再入
场股数（挣股数视角，零新载体）。

══════════════ 预注册判据（先于运行声明）══════════════

P1（任务判据）：v3 ≥ BH 逐标的；8/8 = 任务达成线。
   在册先验：8/8 从未达成（v2 2/8，正域 {CL,DX} = BH 弱域）。
N1 根翻转活性：n_nrf_root_flips > 0（零翻转 = 第23环词汇空集 ⇒ 修改1
   无活性，判决无效非否证）。
N2 递归下探活性：n_nrf_deep_fires > 0（零深触发 = 第14环递归从未越过
   k−1 ⇒ 修改2无活性）。
N3 暴露修复：链活跃 bar 占比 = 1 − depth_bars[0]/n 应接近 1.0（v2 物理
   多头暴露 0.37-0.53；v3 含空头相位的市场参与度）。
N4 尾部风险：n_short_liquidations 计数 + MDD 对照 v2（v2 MDD 8/8 全优
   BH——根翻空引入下跌方向暴露，MDD 面可能恶化，必须显式报告）。
B1 守卫：tape_fp 与 p7_conj_<SYM>.json 在册指纹逐项一致；漂移 ⇒ 读数作废。

════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/nrf_v3_root_flip_recursive_nest_backtest.py [SYM ...]
输出：analysis/data_cache/nrf_v3_<SYM>.json + nrf_v3_summary.json
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


def prereg_v3(a: dict, bh_pct: float, n_bars: int, v2: dict | None) -> dict:
    """v3 预注册判据裁决（P1/N1-N4，先于运行声明在模块 docstring）。"""
    c = a["nrf_counters"]
    depth_bars = c["depth_bars"]
    active = 1.0 - depth_bars[0] / n_bars
    out = {
        "P1_ge_bh": [a["strat_pct"], round(bh_pct, 1), a["strat_pct"] >= bh_pct],
        "N1_root_flips": [sum(c["root_flips"]), sum(c["root_flips"]) > 0],
        "N2_deep_fires": [sum(c["deep_fires"]), sum(c["deep_fires"]) > 0],
        "N3_active_frac": round(active, 3),
        "N3_phys_long_frac": round(c["phys_long_bars"] / n_bars, 3),
        "N3_phys_short_frac": round(c["phys_short_bars"] / n_bars, 3),
        "N4_liquidations": sum(c["liquidations"]),
    }
    if v2 is not None:
        out["vs_v2"] = {
            "v2_pct": v2["nrf"]["strat_pct"],
            "delta_pp": round(a["strat_pct"] - v2["nrf"]["strat_pct"], 1),
            "v2_mdd": v2["nrf"]["mdd_pct"],
            "mdd_delta_pp": round(a["mdd_pct"] - v2["nrf"]["mdd_pct"], 1),
            "v2_fires": sum(v2["nrf"]["nrf_counters"]["nest_fire_sell"])
            + sum(v2["nrf"]["nrf_counters"]["nest_fire_buy"]),
            "v3_fires": sum(c["nest_fire_sell"]) + sum(c["nest_fire_buy"]),
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

    v2_path = DATA_DIR / f"nrf_final_{sym}.json"
    v2 = json.loads(v2_path.read_text()) if v2_path.exists() else None
    if v2 is not None and ("failed" in v2 or v2.get("tape_fp") != fp):
        v2 = None  # v2 读数不可用 ⇒ 只报 v3 vs BH

    t1 = time.time()
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="nrf")
    a = analyze(res, closes, years)
    a["nrf_counters"]["root_flips"] = res["n_nrf_root_flips_by_ladder"]
    a["nrf_counters"]["deep_fires"] = res["n_nrf_deep_fires_by_ladder"]
    a["nrf_counters"]["phys_short_bars"] = res["nrf_phys_short_bars"]
    pr = prereg_v3(a, bh_pct, n, v2)
    print(f"[{sym}][nrf_v3] {time.time() - t1:.1f}s strat={a['strat_pct']:+.1f}% "
          f"mdd={a['mdd_pct']}% trades={a['n_trades']} "
          f"rootflips={pr['N1_root_flips'][0]} deep={pr['N2_deep_fires'][0]} "
          f"active={pr['N3_active_frac']} liq={pr['N4_liquidations']} "
          f"P1={pr['P1_ge_bh'][2]}", flush=True)
    return {"symbol": sym, "n_bars": n, "tape_fp": fp,
            "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
            "design": "NRF v3：根翻转（第23环）+ 递归区间套（第14环）；"
                      "判据见模块 docstring，空头侧 [镜像推导]",
            "nrf_v3": a, "preregistered": pr}


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"nrf_v3_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            summary.append({"symbol": sym, "bh_pct": out["bh_pct"],
                            "v3_pct": out["nrf_v3"]["strat_pct"],
                            "v3_mdd": out["nrf_v3"]["mdd_pct"],
                            "preregistered": out["preregistered"]})
        (DATA_DIR / "nrf_v3_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
