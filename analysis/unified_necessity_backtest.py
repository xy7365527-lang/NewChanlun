"""统一必然性引擎（unn，mode="unn"）八标的必然性检验 + 回测。

上游：编排者 2026-06-14"你要严格验证必然性，这是通过推论证明的……然后严格实装，
绝不允许近似"。把分散在 URS/iso/nif/pcf 的 8 条必然性（概念运动链 N1-N8）叠加到
一个引擎：`rust/src/trading/unified_necessity.rs`。

══════════════ 8 条必然性（验收标准——先于回测，prove 函数 violation = panic）══════════════

引擎内 8 个 prove 函数运行时证明（违反即 panic）：
  N1 逐仓独立森林（第20环）：prove_n1_forest——单根 + 无孤儿 + children 一致。
     观测 nrf_max_children >1 = 森林实证（栈不可能）。
  N2 per-voice 独立操作（第18环）：prove_n2_per_voice——本 bar 操作 voice id 无重复。
  N3 全三类 BSP 消费（第12环）：prove_n3_type2——type2 事件出现数 == 处理数（无 continue）。
  N4 成本门终止递归（第16环）：prove_n4——floor_stop 恒 0（纯 θ=None ∨ θ<friction）。
  N5 区间套自上而下定位（第14环）：prove_chain/prove_n5_cascade——级联连续前缀 +
     source_ladder ≥ k（自上而下，非自下而上独立武装）。
  N6 先势后定位时序（540号）：prove_chain——compress_bar ≤ confirm_bar ≤ bar。
  N7 降成本不需 pending（第17环）：prove_n7_spawn_self_level——E 触发层 == voice 层
     （自层 nf，非全局 located 链、不 prove_chain）。
  N8 双层会计多空嵌套守恒（第22环）：prove_n8_conservation——Σunits=N_base + NAV 价值中性。
⇒ 8 标的真实数据跑通**无 panic** = N1-N8 在 ~25M bar 上成立（L2 必然性验证）。

══════════════ 回测判据（必然性通过后；L2/L3——非验收标准）══════════════

P1（对比）：unn ≥ BH 逐标的 + 对比 URS/pcf 基线（缓存 urs_<SYM>.json / pcf_<SYM>.json）。
   **回测结果不是验收标准**（编排者：必然性检验才是）——P1 是有效域读数。

用法：PYTHONPATH=src .venv/bin/python analysis/unified_necessity_backtest.py [SYM ...]
输出：analysis/data_cache/unn_<SYM>.json + unn_summary.json
"""

from __future__ import annotations

import json
import sys
import time
import traceback
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


def _load_baseline(sym: str, tag: str) -> dict | None:
    p = DATA_DIR / f"{tag}_{sym}.json"
    if not p.exists():
        return None
    d = json.loads(p.read_text())
    if "failed" in d:
        return None
    blk = d.get(tag, {})
    return {
        "strat_pct": blk.get("strat_pct"),
        "mdd_pct": blk.get("mdd_pct"),
        "sellpt": blk.get("exit_reasons", {}).get("sellpt", 0),
    }


def necessity(a: dict, res: dict) -> dict:
    """脚本侧必然性读数。引擎内 8 个 prove（N1-N8）已在跑通无 panic 时成立——本
    函数读 res 验证 source 分布、森林实证、N4 无 floor_stop 等可观测证据。"""
    root_ent = res["n_nrf_root_entries_by_ladder"]
    spawns = res["n_nrf_spawns_by_ladder"]
    flips = res["n_nrf_root_flips_by_ladder"]
    negate_hedges = res["n_nrf_negate_hedges_by_ladder"]
    floor_stops = sum(res["n_nrf_floor_stops_by_ladder"])
    entry_levels = [k for k in range(len(root_ent)) if root_ent[k] > 0]
    spawn_levels = [k for k in range(len(spawns)) if spawns[k] > 0]
    return {
        "N1_N8_runtime_proof": "PASS（引擎跑通无 panic ⇒ 8 条必然性全 bar 成立 L2）",
        "N1_forest_max_children": res.get("nrf_max_children", 0),
        "N4_floor_stops": floor_stops,  # 必为 0（prove_n4 否则 panic）
        "entry_source_levels": [(k, LADDER_NAMES.get(k, str(k)), root_ent[k]) for k in entry_levels],
        "spawn_target_levels": [(k, LADDER_NAMES.get(k, str(k)), spawns[k]) for k in spawn_levels],
        "N_source_dynamic": len(set(entry_levels) | set(spawn_levels)),
        "flips_by_ladder": [(k, flips[k]) for k in range(len(flips)) if flips[k] > 0],
        "sellpt_clear": a["exit_reasons"].get("sellpt", 0),
        # B 规则：否定线触发 = 势减弱 ⇒ 加速降成本对冲（根多头否定 ⇒ 次级别 spawn 子空头，
        # 根不清仓；删观测态）。total_negate_hedges = B 机制在真实数据上的激活计数。
        "total_negate_hedges": sum(negate_hedges),
        "negate_hedge_levels": [(k, LADDER_NAMES.get(k, str(k)), negate_hedges[k])
                                for k in range(len(negate_hedges)) if negate_hedges[k] > 0],
        "total_root_entries": sum(root_ent),
        "total_spawns": sum(spawns),
        "deep_fires": sum(res["n_nrf_deep_fires_by_ladder"]),
        "earning_adds": sum(res["n_nrf_earning_adds_by_ladder"]),
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
    # ★ 与 URS/pcf 基线逐字一致：settle on + 仅 dir_flips（唯一变量 = 引擎层选择机制）。
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips, require_settled=True)
    print(f"[{sym}] 信号层(settle on) {time.time() - t0:.1f}s flips={len(dir_flips)}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)
    urs = _load_baseline(sym, "urs")
    pcf = _load_baseline(sym, "pcf")

    t1 = time.time()
    # 必然性检验：引擎内 8 个 prove 违反即 panic——此调用跑通即 N1-N8 成立。
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="unn")
    a = analyze(res, closes, years)
    a["nrf_counters"]["deep_fires"] = res["n_nrf_deep_fires_by_ladder"]
    nec = necessity(a, res)

    p1 = a["strat_pct"] >= bh_pct
    d_urs = round(a["strat_pct"] - (urs["strat_pct"] or 0), 1) if urs else None
    d_pcf = round(a["strat_pct"] - (pcf["strat_pct"] or 0), 1) if pcf else None
    print(f"[{sym}][unn] {time.time() - t1:.1f}s strat={a['strat_pct']:+.1f}% "
          f"mdd={a['mdd_pct']}% trades={a['n_trades']} "
          f"roots={nec['total_root_entries']} spawns={nec['total_spawns']} "
          f"neg_hedges={nec['total_negate_hedges']} "
          f"maxkids={nec['N1_forest_max_children']} floor_stops={nec['N4_floor_stops']} "
          f"sellpt={nec['sellpt_clear']} src_lvls={nec['N_source_dynamic']} "
          f"P1={p1} ΔURS={d_urs} Δpcf={d_pcf}pp", flush=True)

    return {
        "symbol": sym, "n_bars": n, "bh_pct": round(bh_pct, 1), "bh_mdd_pct": round(bh_dd, 1),
        "design": "统一必然性引擎 unn：8 条必然性叠加（N1 森林 + N2 per-voice + N3 type2 + "
                  "N4 纯成本门 + N5/N6 级联 pending_locate + N7 E 自层 nf + N8 双层会计守恒）",
        "unn": a,
        "necessity": nec,
        "P1_ge_bh": [a["strat_pct"], round(bh_pct, 1), p1],
        "vs_urs": ({"strat_urs": urs["strat_pct"], "strat_unn": a["strat_pct"], "delta_pp": d_urs}
                   if urs else None),
        "vs_pcf": ({"strat_pcf": pcf["strat_pct"], "strat_unn": a["strat_pct"], "delta_pp": d_pcf}
                   if pcf else None),
    }


def main() -> None:
    summary = []
    for sym in SYMBOLS:
        try:
            out = run_symbol(sym)
        except Exception as e:  # 标的隔离；prove panic 在此显形为必然性失败
            traceback.print_exc()
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{sym}] FAILED（必然性检验失败 or 数据缺失）{out['failed']}", flush=True)
        (DATA_DIR / f"unn_{sym}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"symbol": sym, "failed": out["failed"]})
        else:
            summary.append({
                "symbol": sym, "bh_pct": out["bh_pct"],
                "unn_pct": out["unn"]["strat_pct"], "unn_mdd": out["unn"]["mdd_pct"],
                "P1": out["P1_ge_bh"][2],
                "dURS": (out["vs_urs"] or {}).get("delta_pp"),
                "dpcf": (out["vs_pcf"] or {}).get("delta_pp"),
                "max_children": out["necessity"]["N1_forest_max_children"],
                "floor_stops": out["necessity"]["N4_floor_stops"],
                "roots": out["necessity"]["total_root_entries"],
                "spawns": out["necessity"]["total_spawns"],
                "neg_hedges": out["necessity"]["total_negate_hedges"],
                "sellpt": out["necessity"]["sellpt_clear"],
            })
        (DATA_DIR / "unn_summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
