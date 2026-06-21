"""unn 引擎 × 1s a0 数据 严格区间套域空检验（通用 runner，commit 0c44be10a8）。

run_unn_es_1s.py 的多标的泛化：复用 unified_necessity_backtest.run_symbol
（compute_organic_signals settle on + run_positional_rust mode="unn"），仅注入
1s databento 真实数据文件（非 logic 改动）。

任务（2026-06-15）：1min strict located 8/8 域空（候选 ~95% 落 move(L1)=k=3，
第64课"低三级以上"无处落脚）。1s a0 在已有层级间插入更多中间层 ⇒ 候选可能上移到
recL3+（k≥5）⇒ strict located 可能有落脚 ⇒ 检验域空是否限 1min a0。

核心观测量：
  - nest_arms（候选武装层分布）：1s 是否把候选抬到 recL2/recL3/recL4（直接证据）。
  - cascades（located 确认）：strict 级联确认是否走通（域非空的充要条件）。
  - root_entries（F 入场）/ n_trades：实际是否有交易。
  - prove N1-N8 零 panic：必然性验收（跑通即 PASS）。

用法: PYTHONPATH=src .venv/bin/python analysis/unn_1s_a0_runner.py [KEY ...]
      KEY ∈ {ES1S, CL1S}（默认两者）。BT_MAX_BARS=N 截断冒烟。
输出: analysis/data_cache/unn_<KEY>.json + unn_1s_runner_summary.json
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

import unified_necessity_backtest as U  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES  # noqa: E402
from nested_recursive_fugue_final_backtest import LADDER_NAMES  # noqa: E402

DATA = ROOT / "analysis" / "data_cache"

# KEY → 1s databento 真实数据文件（仅这些有 1s 全年数据）。
FILES = {
    "ES1S": DATA / "es_1s_databento_1y.json",
    "CL1S": DATA / "cl_1s_databento_1y.json",
    "BRN1S": DATA / "brn_1s_databento_1y.json",
}
KEYS = sys.argv[1:] or ["ES1S", "CL1S"]


def ladder_dist(arr: list, label: str) -> str:
    items = [(k, LADDER_NAMES.get(k, str(k)), v) for k, v in enumerate(arr) if v]
    body = "  ".join(f"[{k}:{nm}]={v}" for k, nm, v in items) or "(全零)"
    return f"{label:22s}: {body}"


def run_one(key: str) -> dict:
    SYMBOL_FILES[key] = FILES[key]
    t0 = time.time()
    out = U.run_symbol(key)
    if "failed" in out:
        print(f"[{key}] FAILED:", out["failed"], file=sys.stderr)
        return out

    a = out["unn"]
    nec = out["necessity"]
    nc = a["nrf_counters"]

    print("\n" + "=" * 72)
    print(f"  {key} × unn 严格区间套（commit 0c44be10a8）—— 域空检验")
    print("=" * 72)
    print(f"bars          : {out['n_bars']:,}   BH={out['bh_pct']:+.1f}%  BH_MDD={out['bh_mdd_pct']:.1f}%")
    print(f"strat_pct     : {a['strat_pct']:+.1f}%   (P1_ge_bh={out['P1_ge_bh'][2]})")
    print(f"mdd_pct       : {a['mdd_pct']}%   n_trades(笔): {a['n_trades']}")
    print(f"入场(root_ent): {nec['total_root_entries']}   spawns={nec['total_spawns']}   "
          f"sellpt_clear={nec['sellpt_clear']}   floor_stops={nec['N4_floor_stops']}")
    print(f"exit_reasons  : {a['exit_reasons']}")
    print(f"by_ladder     : {json.dumps(a.get('by_ladder', {}), ensure_ascii=False)}")
    print("-" * 72)
    print("  source 分布（located arm 检验——关键：cascades 是否非零 + 候选上移到 recL2+）")
    print("-" * 72)
    for ck, lbl in [("nest_arms", "nest_arms(候选武装)"), ("cascades", "cascades(located确认)"),
                    ("root_entries", "root_entries(F入场)"), ("spawns", "spawns(E派生)"),
                    ("nest_fire_sell", "nest_fire_sell(nf卖)"), ("nest_fire_buy", "nest_fire_buy(nf买)"),
                    ("nest_breaks", "nest_breaks(破极值)"), ("cost_rejects", "cost_rejects(成本拒)"),
                    ("deep_fires", "deep_fires(深度fire)"), ("depth_bars", "depth_bars(各层bar)")]:
        print(ladder_dist(nc.get(ck, []), lbl))
    print("=" * 72)
    print(f"[{key}] 总耗时 {time.time() - t0:.1f}s")

    (DATA / f"unn_{key}.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
    return out


def main() -> int:
    summary = []
    for key in KEYS:
        try:
            out = run_one(key)
        except Exception as e:
            traceback.print_exc()
            out = {"symbol": key, "failed": f"{type(e).__name__}: {e}"}
        if "failed" in out:
            summary.append({"key": key, "failed": out["failed"]})
        else:
            a = out["unn"]
            nc = a["nrf_counters"]
            arms = nc.get("nest_arms", [])
            casc = nc.get("cascades", [])
            arms_hi = sum(arms[k] for k in range(4, len(arms))) if arms else 0  # recL2+ 候选
            summary.append({
                "key": key, "bh_pct": out["bh_pct"], "strat_pct": a["strat_pct"],
                "n_trades": a["n_trades"],
                "root_entries": out["necessity"]["total_root_entries"],
                "cascades_total": sum(casc),
                "nest_arms_total": sum(arms),
                "nest_arms_recL2plus": arms_hi,
                "max_arm_ladder": (max(k for k in range(len(arms)) if arms[k]) if any(arms) else -1),
                "empty_domain": (a["n_trades"] == 0),
                "cascade_fired": (sum(casc) > 0),
            })
        (DATA / "unn_1s_runner_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print("\n" + json.dumps(summary, ensure_ascii=False, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
