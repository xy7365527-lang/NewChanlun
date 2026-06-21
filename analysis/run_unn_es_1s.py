"""ES 1-second 数据 × unn 引擎（strict 区间套确认，commit 0c44be10a8）域空检验。

任务（2026-06-15）：commit 0c44be10a8 的 L2 读数是 8/8 标的（1min）strict located
域空（0 入场/0 笔），根因「候选 ~95% 落在 move(L1)=k=3，第64课低三级以上无处落脚」。
边界条件 #2 预言：信号层产出更高级别（recL2+）BSP ⇒ strict located 有效域非空。

ES 1s 比 1min 递归塔更高（recL3/lid4 涌现，见 project_second_bar_a0 / pcf_1s_a0）⇒
直接检验：更深的塔是否把候选层分布抬高到足以让 strict located 域非空。

复用 unified_necessity_backtest.run_symbol（compute_organic_signals + run_positional_rust
mode="unn"），仅注入 ES1S→es_1s_databento_1y.json（真实数据文件，非 logic 改动）。
输出 located arm 数、笔数、source 分布（nest_arms/root_entries/spawns/cascades by ladder）。
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import unified_necessity_backtest as U  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES  # noqa: E402
from nested_recursive_fugue_final_backtest import LADDER_NAMES  # noqa: E402

DATA = ROOT / "analysis" / "data_cache"
SYMBOL_FILES["ES1S"] = DATA / "es_1s_databento_1y.json"


def ladder_dist(arr: list, label: str) -> str:
    """source 分布：只打印非零层（ladder idx, 名称, 计数）。"""
    items = [(k, LADDER_NAMES.get(k, str(k)), v) for k, v in enumerate(arr) if v]
    body = "  ".join(f"[{k}:{nm}]={v}" for k, nm, v in items) or "(全零)"
    return f"{label:22s}: {body}"


def main() -> int:
    t0 = time.time()
    out = U.run_symbol("ES1S")
    if "failed" in out:
        print("FAILED:", out["failed"], file=sys.stderr)
        return 1

    a = out["unn"]
    nec = out["necessity"]
    nc = a["nrf_counters"]

    print("\n" + "=" * 72)
    print("  ES 1s × unn 严格区间套（commit 0c44be10a8）—— 域空检验")
    print("=" * 72)
    print(f"bars          : {out['n_bars']:,}   BH={out['bh_pct']:+.1f}%  BH_MDD={out['bh_mdd_pct']:.1f}%")
    print(f"strat_pct     : {a['strat_pct']:+.1f}%   (P1_ge_bh={out['P1_ge_bh'][2]})")
    print(f"mdd_pct       : {a['mdd_pct']}%")
    print(f"n_trades(笔)  : {a['n_trades']}")
    print(f"入场(root_ent): {nec['total_root_entries']}   spawns={nec['total_spawns']}")
    print(f"N_source_dyn  : {nec['N_source_dynamic']}   sellpt_clear={nec['sellpt_clear']}")
    print(f"max_children  : {nec['N1_forest_max_children']}   floor_stops={nec['N4_floor_stops']}")
    print(f"exit_reasons  : {a['exit_reasons']}")
    print(f"by_ladder     : {json.dumps(a.get('by_ladder', {}), ensure_ascii=False)}")
    print("-" * 72)
    print("  source 分布（located arm 检验——关键：recL2+ 是否非零）")
    print("-" * 72)
    print(ladder_dist(nc.get("nest_arms", []), "nest_arms(候选武装)"))
    print(ladder_dist(nc.get("cascades", []), "cascades(located确认)"))
    print(ladder_dist(nc.get("root_entries", []), "root_entries(F入场)"))
    print(ladder_dist(nc.get("spawns", []), "spawns(E派生)"))
    print(ladder_dist(nc.get("negates", []), "negates(否定)"))
    print(ladder_dist(nc.get("nest_fire_sell", []), "nest_fire_sell(nf卖)"))
    print(ladder_dist(nc.get("nest_fire_buy", []), "nest_fire_buy(nf买)"))
    print(ladder_dist(nc.get("nest_breaks", []), "nest_breaks(破极值)"))
    print(ladder_dist(nc.get("cost_rejects", []), "cost_rejects(成本拒)"))
    print(ladder_dist(nc.get("deep_fires", []), "deep_fires(深度fire)"))
    print(ladder_dist(nc.get("depth_bars", []), "depth_bars(各层bar)"))
    print("=" * 72)
    print(f"总耗时 {time.time() - t0:.1f}s")

    (DATA / "unn_ES1S.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print(f"结果写入 {DATA / 'unn_ES1S.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
