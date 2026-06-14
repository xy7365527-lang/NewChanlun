"""区间套定位链驱动赋格（pcf）1s a0 回测——检验 source 坍缩是否限 1min a0。

═══════════════════════════════════════════════════════════════════════
上游（pcf verdict §边界条件①，2026-06-14）
═══════════════════════════════════════════════════════════════════════
pcf v2（自上而下级联）在 **1min a0** 上 source 仍 83-87% 坍缩 segment，根因被
隔离到"高级别 candidate 频率结构必然"。verdict 边界条件①明确：
  "若改 a0 粒度（更粗周期）⇒ 高级别 candidate 相对更频繁 ⇒ source 可能上移
   ⇒ 本'segment 坍缩是 candidate 频率必然'结论限 1min a0（未在粗周期检验）。"

本任务用 **1s a0**（更**细**周期，与边界条件①设想的"更粗"相反方向）检验：
  - 朴素假设（任务动机）：1s 塔多涌现层（lid4/lid5，urs_1s PR3 已测）⇒ 高级别
    candidate 的**绝对计数**可能增加 ⇒ source 可能上移 ⇒ 坍缩缓解。
  - 在册反假设（nest 覆盖率判决 §5a + 1s 加速测量发现一）：1s 塔的操作床位被
    推到 lad3+，高层结构事件墙钟坍缩 22-35× ⇒ source **占比**坍缩预期不缓解/加重。
  否证价值高：若 1s 下 segment 占比仍 ~85%（或更高）⇒ source 坍缩升格为
    **跨 a0 粒度的 candidate 频率结构必然**（有效域从 1min 扩到 1s）。

═══════════════════════════════════════════════════════════════════════
对照构造（先于数据声明的设计；唯一变量 = a0）
═══════════════════════════════════════════════════════════════════════
同一 1s 序列 → (a) 1s 塔（a0=1s 原始）；(b) 1min 桶塔（墙钟分钟桶聚合，同源同窗）。
两臂引擎 mode="pcf"、floor_ladder=FLOOR（=FIRST_BSP_LADDER=segment，结构常量非操作
floor）、信号层 require_settled=True——与 pcf verdict 在册口径逐字同一。唯一变量 = a0。

标的（仅这三个有 1s 全年 databento 数据）：
  - CL1Y：pcf 1min **正域**（油链，+93.0%>BH，ΔURS +88.3）— source 上移会否扩正域？
  - BRN1Y：pcf 1min **正域**（油链，+103.6%>BH，ΔURS +47.1）
  - ES1Y：pcf 1min **负域**（强牛，+176.9%≪BH+594.3，降成本 churn 死因）

═══════════════════════════════════════════════════════════════════════
必然性检验（验收标准——先于回测，L2）
═══════════════════════════════════════════════════════════════════════
引擎内 prove_chain（N1 完整级联链 / N2 因果 / N3 链顶一致）+ 守恒律（§8.1 每 bar）
违反即 panic。8 标的 25M bar 已在 1min 上 PASS；本任务验证 **1s a0** 上同样无 panic
（标的隔离捕获 panic 显形为必然性失败）。zero-fric + with-fric 双口径守卫秒级伪影。

用法: PYTHONPATH=src .venv/bin/python analysis/pcf_1s_a0_backtest.py [KEY ...]
输出: analysis/data_cache/pcf_1s_<KEY>.json + pcf_1s_summary.json
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

from nested_recursive_fugue_final_backtest import (  # noqa: E402
    FLOOR,
    LADDER_NAMES,
    analyze,
    bh_mdd,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402
from urs_1s_a0_backtest import bucket_1min, load_1s  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"

# KEY → (文件, 格式, maker 单侧摩擦率)。复用 urs_1s_a0 的 DATASETS 口径，新增 BRN1Y。
DATASETS = {
    "CL1Y": ("cl_1s_databento_1y.json", "databento", 0.0001),
    "BRN1Y": ("brn_1s_databento_1y.json", "databento", 0.0001),
    "ES1Y": ("es_1s_databento_1y.json", "databento", 0.00005),
    "CL1MO": ("cl_1s_databento_1mo.json", "databento", 0.0001),  # 冒烟用
}
KEYS = sys.argv[1:] or ["CL1Y", "BRN1Y", "ES1Y"]


def _load_1s_any(key):
    """加载 1s OHLCV，支持本模块 DATASETS（含 BRN/CL1MO）。返回 (ts, o, h, l, c, fric)。"""
    fname, _fmt, friction = DATASETS[key]
    raw = json.loads((DATA_DIR / fname).read_text())
    o, h, l, c = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    if "timestamps_ns" in raw:
        ts = [t // 1_000_000_000 for t in raw["timestamps_ns"]]
    elif "dates" in raw:
        from datetime import datetime
        ts = [int(datetime.fromisoformat(d).timestamp()) for d in raw["dates"]]
    else:
        raise KeyError(f"{fname} 无时间字段")
    return ts, o, h, l, c, friction


def _friction_adj(res, friction: float) -> float:
    """含摩擦 strat%：每笔 trade 双侧 friction（notional×friction×2）。"""
    cost = 0.0
    for t in res["trades"]:
        ep, xp, sh = t[2], t[4], t[5]
        cost += abs(sh) * (ep + xp) / 2 * friction * 2
    return (res["final_nav"] - cost) / 100000.0 * 100 - 100


def source_distribution(res: dict) -> dict:
    """source_ladder 分布（pcf 核心产出）：entry/spawn 按 ladder 计数 + segment 占比。

    pcf 的 source = chain_source（动态链顶），逐操作记入对应 ladder 桶。segment 占比
    高 = source 坍缩到最低层（区间套理想未达成）。verdict 1min：83-87% segment。
    """
    root_ent = res["n_nrf_root_entries_by_ladder"]
    spawns = res["n_nrf_spawns_by_ladder"]
    deep = res["n_nrf_deep_fires_by_ladder"]

    def _pct_seg(arr):
        tot = sum(arr)
        seg = arr[FLOOR] if len(arr) > FLOOR else 0
        return (round(100 * seg / tot, 1) if tot else None), tot

    entry_seg_pct, entry_tot = _pct_seg(root_ent)
    spawn_seg_pct, spawn_tot = _pct_seg(spawns)
    # entry+spawn 合并 = 全部 source 驱动操作
    merged = [(root_ent[k] if k < len(root_ent) else 0)
              + (spawns[k] if k < len(spawns) else 0)
              for k in range(max(len(root_ent), len(spawns)))]
    merged_seg_pct, merged_tot = _pct_seg(merged)

    def _named(arr):
        return [(k, LADDER_NAMES.get(k, str(k)), arr[k])
                for k in range(len(arr)) if arr[k] > 0]

    levels = sorted({k for k in range(len(merged)) if merged[k] > 0})
    return {
        "entry_by_ladder": _named(root_ent),
        "spawn_by_ladder": _named(spawns),
        "entry_segment_pct": entry_seg_pct, "entry_total": entry_tot,
        "spawn_segment_pct": spawn_seg_pct, "spawn_total": spawn_tot,
        "merged_segment_pct": merged_seg_pct, "merged_total": merged_tot,
        "source_levels_active": len(levels),
        "max_source_ladder": (max(levels) if levels else -1),
        "recL2plus_count": sum(merged[k] for k in range(FLOOR + 2, len(merged))),
        "deep_fires": sum(deep),
    }


def run_pcf(rtape, closes, friction: float) -> dict:
    """跑 pcf 引擎（prove_chain 违反即 panic = 必然性失败）。返回回测 + source 分布。"""
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="pcf")
    a = analyze(res, closes, None)
    return {
        "necessity_runtime": "PASS（引擎无 panic ⇒ N1∧N2∧N3∧守恒 在全 bar 成立）",
        "strat_zero_fric": a["strat_pct"],
        "strat_with_fric": round(_friction_adj(res, friction), 1),
        "mdd": a["mdd_pct"],
        "n_trades": a["n_trades"],
        "sellpt": a["exit_reasons"].get("sellpt", 0),
        "spawns": sum(res["n_nrf_spawns_by_ladder"]),
        "roots": sum(res["n_nrf_root_entries_by_ladder"]),
        "source_dist": source_distribution(res),
    }


def run_key(key: str) -> dict:
    ts, o, h, l, c, friction = _load_1s_any(key)
    n1s = len(c)
    bh = (c[-1] / c[0] - 1) * 100
    bh_dd = bh_mdd(c) * 100
    print(f"[{key}] 1s bars={n1s:,} BH={bh:+.1f}% BH_MDD={bh_dd:.1f}% fric={friction}",
          flush=True)

    # (a) 1s 塔信号层
    t0 = time.time()
    df1s: list = []
    tape_1s = compute_organic_signals(o, h, l, c, dir_flips=df1s, require_settled=True)
    rt_1s = pack_tape(tape_1s, dir_flips=df1s)
    print(f"[{key}] 1s 信号层 {time.time() - t0:.1f}s flips={len(df1s)}", flush=True)

    # (b) 1min 桶塔信号层（同源墙钟分钟聚合）
    t0 = time.time()
    bo, bh_, bl, bc = bucket_1min(ts, o, h, l, c)
    df1m: list = []
    tape_1m = compute_organic_signals(bo, bh_, bl, bc, dir_flips=df1m, require_settled=True)
    rt_1m = pack_tape(tape_1m, dir_flips=df1m)
    print(f"[{key}] 1min 桶 bars={len(bc):,} {time.time() - t0:.1f}s flips={len(df1m)}",
          flush=True)

    arms = {}
    t1 = time.time()
    arms["1s"] = run_pcf(rt_1s, c, friction)
    sd = arms["1s"]["source_dist"]
    print(f"[{key}][1s pcf] {time.time() - t1:.1f}s strat0={arms['1s']['strat_zero_fric']:+.1f}% "
          f"stratF={arms['1s']['strat_with_fric']:+.1f}% roots={arms['1s']['roots']} "
          f"spawns={arms['1s']['spawns']} sellpt={arms['1s']['sellpt']} "
          f"entry_seg%={sd['entry_segment_pct']} merged_seg%={sd['merged_segment_pct']} "
          f"maxsrc={sd['max_source_ladder']} recL2+={sd['recL2plus_count']}", flush=True)

    t1 = time.time()
    arms["1min"] = run_pcf(rt_1m, bc, friction)
    sdm = arms["1min"]["source_dist"]
    arms["1min"]["bh_pct"] = round((bc[-1] / bc[0] - 1) * 100, 1)
    print(f"[{key}][1min pcf] {time.time() - t1:.1f}s strat0={arms['1min']['strat_zero_fric']:+.1f}% "
          f"roots={arms['1min']['roots']} spawns={arms['1min']['spawns']} "
          f"sellpt={arms['1min']['sellpt']} entry_seg%={sdm['entry_segment_pct']} "
          f"merged_seg%={sdm['merged_segment_pct']} maxsrc={sdm['max_source_ladder']}",
          flush=True)

    # 核心裁决：1s vs 1min 桶塔 segment 占比对比
    seg_1s = sd["merged_segment_pct"]
    seg_1m = sdm["merged_segment_pct"]
    verdict = {
        "merged_seg_pct_1s": seg_1s, "merged_seg_pct_1min": seg_1m,
        "delta_seg_pct": (round(seg_1s - seg_1m, 1)
                          if seg_1s is not None and seg_1m is not None else None),
        "source_uplifted": (seg_1s is not None and seg_1m is not None and seg_1s < seg_1m - 5),
        "1s_max_source": sd["max_source_ladder"], "1min_max_source": sdm["max_source_ladder"],
        "1s_recL2plus": sd["recL2plus_count"], "1min_recL2plus": sdm["recL2plus_count"],
        "collapse_persists_1s": (seg_1s is not None and seg_1s >= 80),
    }
    return {
        "key": key, "n_1s": n1s, "n_1min": len(bc),
        "bh_pct": round(bh, 1), "bh_mdd_pct": round(bh_dd, 1), "friction": friction,
        "design": "pcf 1s a0：唯一变量=a0（1s 塔 vs 同源 1min 桶塔）；检验 source 坍缩是否限 1min",
        "arms": arms,
        "verdict": verdict,
    }


def main() -> None:
    summary = []
    for key in KEYS:
        try:
            out = run_key(key)
        except Exception as e:  # 标的隔离；prove_chain panic 在此显形为必然性失败
            import traceback
            traceback.print_exc()
            out = {"key": key, "failed": f"{type(e).__name__}: {e}"}
            print(f"[{key}] FAILED（必然性失败 or 数据缺失）{out['failed']}", flush=True)
        (DATA_DIR / f"pcf_1s_{key}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
        if "failed" in out:
            summary.append({"key": key, "failed": out["failed"]})
        else:
            v = out["verdict"]
            summary.append({
                "key": key, "bh": out["bh_pct"],
                "1s_strat0": out["arms"]["1s"]["strat_zero_fric"],
                "1s_stratF": out["arms"]["1s"]["strat_with_fric"],
                "1min_strat0": out["arms"]["1min"]["strat_zero_fric"],
                "seg%_1s": v["merged_seg_pct_1s"], "seg%_1min": v["merged_seg_pct_1min"],
                "Δseg%": v["delta_seg_pct"], "source_uplifted": v["source_uplifted"],
                "collapse_persists_1s": v["collapse_persists_1s"],
                "maxsrc_1s": v["1s_max_source"], "recL2+_1s": v["1s_recL2plus"],
            })
        (DATA_DIR / "pcf_1s_summary.json").write_text(
            json.dumps(summary, ensure_ascii=False, indent=1))
    print(json.dumps(summary, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
