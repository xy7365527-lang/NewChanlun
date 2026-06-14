"""区间套定位链驱动赋格（pcf）1s a0 接入 + source 坍缩复检（唯一变量 = a0）。

上游：编排者 2026-06-14"我们的 nautilus 有这个框架，在这个框架下搞就行了""严格实现"。
pcf v2 判决（`positioning_chain_fugue_verdict.md` §v2）的决定性 L2 发现：1min a0 上
source 83-87% 坍缩到 segment，根因被精确隔离为「高级别 candidate 本身稀缺」（与武装
方向无关），其**边界条件**明确列「改 a0 粒度」为**未检验开放轴**。用户切到 1s a0
正是要检验这条开放轴的另一方向（更细）。

★ 关键架构事实（印证"在这个框架下搞就行了"）：a0 粒度 = 输入 bar 分辨率。引擎
（buysellpoint.rs / nested_fugue 原语 / positioning_chain_fugue.rs）自底向上从喂入的
bar 构建塔（bi→segment→...→递归层）——**零引擎改动**。切到 1s a0 = 喂 1s bar。

══════════════ 严格设计：唯一变量 = a0（formalization-validity-domain）══════════════

1min ES 在册 pcf 结果是 **10 年** 5.59M bar；本 1s 数据是 **1 年** 11.77M bar——窗口
不同会把 a0 与 regime 混淆。故本脚本用同一份 ES 1s 序列构造两塔：
  - **1s 塔**：raw 1s bar → compute_organic_signals → pcf
  - **1min 塔**：墙钟分钟桶聚合（first/max/min/last）→ compute_organic_signals → pcf
两塔同窗、同数据源、同信号层、同引擎、同 settle 门——**唯一变量 = a0**
（旧 nest_coverage_1s_a0.py 同款对照构造）。

══════════════ 验收标准（必然性检验，先于回测）══════════════

引擎内 prove_chain（F/C/D/E 每操作点）+ 守恒律（§8.1 每 bar）违反即 panic。
两塔跑通无 panic ⇒ N1（完整级联链）∧ N2（因果）∧ N3（链顶一致）∧ 守恒成立（L2）。

══════════════ 关键验证点（回测；L2 有效域读数）══════════════

source_ladder 分布：1s 塔的 source 是否仍坍缩到 segment？
  - 假设 H_救（用户希望）：1s 塔更高 ⇒ 高级别 candidate 绝对数更多 ⇒ source 上移
    （segment 占比下降，recL2+ 入场出现）。
  - 假设 H_尺度不变（旧实验 §5a 先验）：candidate 各层存活率尺度不变 ⇒ source 仍
    坍缩到塔底（现在是更微观的 1s-segment）⇒ 1s 也救不了，反而更碎。
判决 = 两塔 entry/spawn 的 segment 占比 + 塔高（最高涌现 source 层）对比。

用法：PYTHONPATH=src .venv/bin/python analysis/positioning_chain_fugue_1s_a0.py
输出：analysis/data_cache/pcf_1s_a0_ES.json
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
from positioning_chain_fugue_backtest import necessity  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SRC_1S = DATA_DIR / "es_1s_databento_1y.json"
OUT = DATA_DIR / "pcf_1s_a0_ES.json"


def aggregate_1min(o, h, l, c, minutes):
    """墙钟分钟桶聚合（同数据源唯一变量 = a0；桶内 first/max/min/last）。

    与 nest_coverage_1s_a0.aggregate_1min 逐字一致（同对照构造）。
    """
    ao, ah, al, ac = [], [], [], []
    cur = None
    for i in range(len(c)):
        if minutes[i] != cur:
            cur = minutes[i]
            ao.append(o[i])
            ah.append(h[i])
            al.append(l[i])
            ac.append(c[i])
        else:
            ah[-1] = max(ah[-1], h[i])
            al[-1] = min(al[-1], l[i])
            ac[-1] = c[i]
    return ao, ah, al, ac


def run_arm(label: str, o, h, l, c) -> dict:
    """单塔：信号层 → pcf 引擎 → 必然性检验读数 + source 分布。"""
    n = len(c)
    bh_pct = (c[-1] / c[0] - 1) * 100
    bh_dd = bh_mdd(c) * 100
    print(f"[{label}] bars={n:,} BH={bh_pct:+.1f}% BH_MDD={bh_dd:.1f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(o, h, l, c, dir_flips=dir_flips, require_settled=True)
    t_sig = time.time() - t0
    print(f"[{label}] 信号层(settle on) {t_sig:.1f}s flips={len(dir_flips):,}", flush=True)

    rtape = pack_tape(tape, dir_flips=dir_flips)
    t1 = time.time()
    # 必然性检验：prove_chain / 守恒 violation = panic ⇒ 此调用跑通即 N1/N2/N3/守恒成立。
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="pcf")
    t_eng = time.time() - t1
    a = analyze(res, c, None)
    a["nrf_counters"]["deep_fires"] = res["n_nrf_deep_fires_by_ladder"]
    nec = necessity(a, res, n)

    # 塔高 + source 坍缩量：最高涌现操作层 + segment 占比。
    root_ent = res["n_nrf_root_entries_by_ladder"]
    spawns = res["n_nrf_spawns_by_ladder"]
    held = res["held_bars_by_ladder"]
    seg = FLOOR  # FIRST_BSP_LADDER = segment = 2
    tot_root = sum(root_ent)
    tot_spawn = sum(spawns)
    max_active_ladder = max((k for k in range(len(held)) if held[k] > 0), default=seg)
    seg_root_pct = round(100 * root_ent[seg] / tot_root, 1) if tot_root else None
    seg_spawn_pct = round(100 * spawns[seg] / tot_spawn, 1) if tot_spawn else None

    print(
        f"[{label}][pcf] {t_eng:.1f}s strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% "
        f"trades={a['n_trades']} roots={tot_root} spawns={tot_spawn} "
        f"sellpt={nec['sellpt_clear']} src_levels={nec['N_source_dynamic']} "
        f"seg_root%={seg_root_pct} seg_spawn%={seg_spawn_pct} "
        f"max_ladder={max_active_ladder}({LADDER_NAMES.get(max_active_ladder)})",
        flush=True,
    )

    return {
        "label": label,
        "n_bars": n,
        "bh_pct": round(bh_pct, 1),
        "bh_mdd_pct": round(bh_dd, 1),
        "sig_seconds": round(t_sig, 1),
        "eng_seconds": round(t_eng, 1),
        "flips": len(dir_flips),
        "pcf": a,
        "necessity": nec,
        "source_collapse": {
            "seg_root_pct": seg_root_pct,
            "seg_spawn_pct": seg_spawn_pct,
            "max_active_ladder": [max_active_ladder, LADDER_NAMES.get(max_active_ladder)],
            "root_entries_by_ladder": [
                [k, LADDER_NAMES.get(k, str(k)), root_ent[k]]
                for k in range(len(root_ent)) if root_ent[k] > 0
            ],
            "spawns_by_ladder": [
                [k, LADDER_NAMES.get(k, str(k)), spawns[k]]
                for k in range(len(spawns)) if spawns[k] > 0
            ],
            "held_bars_by_ladder": [
                [k, LADDER_NAMES.get(k, str(k)), held[k]]
                for k in range(len(held)) if held[k] > 0
            ],
        },
        "P1_ge_bh": [a["strat_pct"], round(bh_pct, 1), a["strat_pct"] >= bh_pct],
    }


def main() -> None:
    if not SRC_1S.exists():
        sys.exit(f"ES 1s 数据缺失：{SRC_1S}（先跑 analysis/_fetch_es_1s_1y.py）")
    raw = json.loads(SRC_1S.read_text())
    o, h, l, c = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    ts = raw["timestamps_ns"]
    print(f"加载 ES 1s {len(c):,} bars 区间 {raw['start']}..{raw['end']}", flush=True)

    out: dict = {
        "symbol": "ES",
        "schema": "ohlcv-1s",
        "window": [raw["start"], raw["end"]],
        "design": "唯一变量=a0：同一份 1s 序列构造 1s 塔 vs 墙钟分钟桶聚合 1min 塔",
        "necessity_gate": "prove_chain(N1/N2/N3)+守恒 violation=panic；两塔跑通无 panic=PASS",
        "arms": {},
    }

    # 1s 塔（raw a0）。
    out["arms"]["1s"] = run_arm("1s塔", o, h, l, c)
    OUT.write_text(json.dumps(out, ensure_ascii=False, indent=1))

    # 1min 塔（墙钟分钟桶聚合，唯一变量 = a0）。
    minutes = [t // 60_000_000_000 for t in ts]
    ao, ah, al, ac = aggregate_1min(o, h, l, c, minutes)
    out["arms"]["1min"] = run_arm("1min塔", ao, ah, al, ac)
    OUT.write_text(json.dumps(out, ensure_ascii=False, indent=1))

    # 判决：source 坍缩对比。
    s1, m1 = out["arms"]["1s"]["source_collapse"], out["arms"]["1min"]["source_collapse"]
    verdict = {
        "necessity_PASS": "两塔跑通无 panic ⇒ N1∧N2∧N3∧守恒成立（L2）",
        "1s_seg_root_pct": s1["seg_root_pct"],
        "1min_seg_root_pct": m1["seg_root_pct"],
        "1s_max_ladder": s1["max_active_ladder"],
        "1min_max_ladder": m1["max_active_ladder"],
        "collapse_persists": (
            None if s1["seg_root_pct"] is None
            else s1["seg_root_pct"] >= 70.0
        ),
        "1s_救得了吗": (
            "无入场（链未成立）" if s1["seg_root_pct"] is None
            else ("否：1s 塔 source 仍坍缩 segment≥70%（H_尺度不变成立）"
                  if s1["seg_root_pct"] >= 70.0
                  else "可能：1s 塔 segment 占比 <70%，source 上移（H_救候选）")
        ),
    }
    out["verdict"] = verdict
    OUT.write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print("\n=== 判决 ===", flush=True)
    print(json.dumps(verdict, ensure_ascii=False, indent=1), flush=True)


if __name__ == "__main__":
    main()
