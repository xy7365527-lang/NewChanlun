"""区间套递归赋格（nif，mode="nif{N}"）八标的回测——交易 floor 消费规则实验。

上游：`analysis/why_not_profitable.md` alpha 瓶颈 L3 诊断（编排者 2026-06-14）。
诊断结论：信号方向有效（z≤80）、会计正确（25M bar 零守恒违反），瓶颈在
**操作语义层**——引擎 99% 的交易在 segment 级别（close 幅度 0.02–0.19% < 摩擦
地板 0.2%），alpha 集中在递归高级别（recL2 +2~25%、recL3 +12~75%/swing）但稀有。

实装：`rust/src/trading/nested_interval_fugue.rs`（URS 的构成性推广——加
`min_trade_ladder` 交易 floor；第16环"成本门=递归终止"的结构形态）。三层改动：
  ① segment 级别不独立开仓（F/E/C 落点 ≥ min_trade_ladder，低层仅区间套定位）；
  ② 仓位集中高级别 + 区间套定位（F 最高 θ 涌现层满仓 + nf 定位，E θ 配额）；
  ③ 出场对齐 E* 涌现层（≥根入场级别）反向 BSP，低级别反向 → E 降成本非平仓。

退化锚（L0，构造保证 + Rust 单测 `nif_floor_equals_urs`）：
  min_trade_ladder = FIRST_BSP_LADDER(2) ⇒ nif bit-exact = URS。

信号层与 URS 基线**逐字一致**（require_settled=True，仅 dir_flips）——保证 nif vs
URS 的唯一变量 = 引擎 min_trade_ladder，非信号层漂移。

══════════════ 预注册判据（先于运行声明；L3）══════════════

P1（任务判据）：nif ≥ BH 逐标的。对比基线 URS（缓存 urs_<SYM>.json）。
   8/8 = 任务达成线（在册先验：零标的级参数约束下从未达成）。
M1（机制活性）：n_nrf_root_entries > 0（零入场 = 交易 floor 过高吞掉所有势 ⇒
   读数无效非否证；需降 min_trade_ladder）。
M2（segment 消除）：min_trade_ladder=k ⇒ held_bars_by_ladder[<k] 全零 ∧
   n_entries_by_ladder[<k] 全零（改动1 的可观测形式——交易层零接触 segment）。
ΔURS（增量定位）：strat_nif − strat_urs 逐标的；正域 = 抬高交易 floor 改善的
   标的集，负域 = 抬高 floor 踏空高级别 alpha 入场点的标的集。
R_mdd：MDD 逐标的报告（不预设方向——少交易可能加深 or 减轻回撤）。

否证价值：若 nif（任意 min）P1 ≤ URS 且无正域 ⇒ 坐实"摩擦地板 gate 不改善
   alpha"（§6 #1 可操作建议被 L3 否证——级别选择不是普适杠杆，与
   `project_backtest_benchmark_falsifiability` 共振）。

用法：PYTHONPATH=src .venv/bin/python analysis/nested_interval_fugue_backtest.py [SYM ...]
输出：analysis/data_cache/nif_<SYM>.json + nif_summary.json
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
    LADDER_NAMES,
    analyze,
    bh_mdd,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = sys.argv[1:] or ["OKLO", "QQQ", "BRN", "DX", "ES", "GC", "CL", "BTC"]
# 摩擦地板候选（why_not_profitable §3/§4）：move(L1)=3、recL2=4。
MIN_LADDERS = (3, 4)


def _load_urs(sym: str) -> dict | None:
    """读 URS 基线（同信号层口径——urs 与 nif 都 settle on + dir_flips）。"""
    p = DATA_DIR / f"urs_{sym}.json"
    if not p.exists():
        return None
    d = json.loads(p.read_text())
    if "failed" in d:
        return None
    blk = d.get("urs", {})
    return {
        "strat_pct": blk.get("strat_pct"),
        "mdd_pct": blk.get("mdd_pct"),
        "sellpt": blk.get("exit_reasons", {}).get("sellpt", 0),
        "p1": d.get("preregistered", {}).get("P1_ge_bh", [None, None, None])[2],
    }


def prereg(a: dict, bh_pct: float, bh_dd: float, n: int, min_lad: int,
           urs: dict | None) -> dict:
    """nif 预注册判据裁决（P1/M1/M2/ΔURS/R_mdd）。"""
    res_entries = a["nrf_counters"]["root_entries"]
    held = a.get("held_bars_by_ladder", [])
    ent = a.get("entries_by_ladder", [])
    # M2：交易层零接触 < min_lad 的级别（segment 消除的可观测形式）。
    below = list(range(FIRST := FLOOR, min_lad))
    held_below = sum(held[k] for k in below) if held else 0
    ent_below = sum(ent[k] for k in below) if ent else 0
    out = {
        "min_trade_ladder": min_lad,
        "min_trade_ladder_name": LADDER_NAMES.get(min_lad, str(min_lad)),
        "P1_ge_bh": [a["strat_pct"], round(bh_pct, 1), a["strat_pct"] >= bh_pct],
        "M1_active": [sum(res_entries), sum(res_entries) > 0],
        "M2_segment_eliminated": {
            "below_ladders": below, "held_bars_below": held_below,
            "entries_below": ent_below,
            "pass": held_below == 0 and ent_below == 0,
        },
        "R_mdd_ge_bh": [a["mdd_pct"], round(bh_dd, 1), a["mdd_pct"] >= bh_dd],
        "sellpt": a["exit_reasons"].get("sellpt", 0),
        "spawns": sum(a["nrf_counters"]["spawns"]),
        "phys_long_frac": round(a["nrf_counters"]["phys_long_bars"] / n, 3),
    }
    _ = FIRST
    if urs is not None:
        out["vs_urs"] = {
            "strat_urs": urs["strat_pct"], "strat_nif": a["strat_pct"],
            "delta_pp": round(a["strat_pct"] - (urs["strat_pct"] or 0), 1),
            "mdd_urs": urs["mdd_pct"], "mdd_nif": a["mdd_pct"],
            "sellpt_urs": urs["sellpt"], "sellpt_nif": out["sellpt"],
            "p1_urs": urs["p1"], "p1_nif": out["P1_ge_bh"][2],
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
    # ★ 与 URS 基线逐字一致：settle on + 仅 dir_flips（保证唯一变量 = 引擎）。
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips, require_settled=True)
    print(f"[{sym}] 信号层(settle on) {time.time() - t0:.1f}s flips={len(dir_flips)}",
          flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)
    urs = _load_urs(sym)

    out: dict = {
        "symbol": sym, "n_bars": n, "bh_pct": round(bh_pct, 1),
        "bh_mdd_pct": round(bh_dd, 1),
        "design": "区间套递归赋格 nif：URS + min_trade_ladder 交易 floor"
                  "（三层 BSP 消费改动；min=2 退化 = URS bit-exact）",
    }
    for min_lad in MIN_LADDERS:
        t1 = time.time()
        res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode=f"nif{min_lad}")
        a = analyze(res, closes, years)
        # analyze 不带 held/entries by ladder（M2 判据需要）——直接从 res 取。
        a["held_bars_by_ladder"] = res["held_bars_by_ladder"]
        a["entries_by_ladder"] = res["n_entries_by_ladder"]
        a["nrf_counters"]["deep_fires"] = res["n_nrf_deep_fires_by_ladder"]
        pr = prereg(a, bh_pct, bh_dd, n, min_lad, urs)
        # held/entries 仅供 M2 判据，分析块内删除避免冗长。
        del a["held_bars_by_ladder"]
        del a["entries_by_ladder"]
        out[f"nif{min_lad}"] = a
        out[f"preregistered_nif{min_lad}"] = pr
        dpp = pr.get("vs_urs", {}).get("delta_pp", "NA")
        print(f"[{sym}][nif{min_lad}] {time.time() - t1:.1f}s "
              f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
              f"trades={a['n_trades']} sellpt={pr['sellpt']} "
              f"roots={pr['M1_active'][0]} M2={pr['M2_segment_eliminated']['pass']} "
              f"P1={pr['P1_ge_bh'][2]} ΔURS={dpp}pp", flush=True)
    return out


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离，不吞错误细节
            import traceback
            traceback.print_exc()
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED {out['failed']}", flush=True)
        (DATA_DIR / f"nif_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            row = {"symbol": sym, "bh_pct": out["bh_pct"]}
            for min_lad in MIN_LADDERS:
                pr = out[f"preregistered_nif{min_lad}"]
                row[f"nif{min_lad}_pct"] = out[f"nif{min_lad}"]["strat_pct"]
                row[f"nif{min_lad}_mdd"] = out[f"nif{min_lad}"]["mdd_pct"]
                row[f"nif{min_lad}_P1"] = pr["P1_ge_bh"][2]
                row[f"nif{min_lad}_dURS"] = pr.get("vs_urs", {}).get("delta_pp")
            summary.append(row)
        (DATA_DIR / "nif_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
