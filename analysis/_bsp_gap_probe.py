"""引擎 BSP 缺口诊断探针 — OKLO 447K 实证（只读，不改引擎）。

诊断两个问题：
  1. buy1 ladder 恒为 {2,3}：ladder≥4 的 bsp 事件 kind/side/confirmed 分布？
     是 type1 不产生（无 trend 背驰）还是产生了但 confirmed=False（force 比率）？
  2. type2 完全缺失：各 ladder 的 type2 计数？若为 0，终态结构里 type1 之后
     的段序列是否满足 rebound→callback 存在性？

输出：stdout 统计 + analysis/data_cache/_bsp_gap_probe.json
用法：PYTHONPATH=src .venv/bin/python analysis/_bsp_gap_probe.py
"""
from __future__ import annotations

import json
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import MAX_LEVELS  # noqa: E402
from organic_signals import compute_organic_signals, _level_bsps_with_divs  # noqa: E402
from per_level_bsp import BI_ZHONGSHU_LEVEL_ID  # noqa: E402

OUT = ROOT / "analysis" / "data_cache" / "_bsp_gap_probe.json"


def main() -> None:
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES["OKLO"])
    n = len(closes)
    print(f"OKLO bars={n:,}")

    # ── pass 1：全量信号层，聚合事件流 ──
    t0 = time.time()
    sigs = compute_organic_signals(opens, highs, lows, closes)
    print(f"signal pass {time.time() - t0:.1f}s")

    ev_stats: Counter = Counter()          # (ladder, kind, side, confirmed) → n
    buy1_bars: dict[int, int] = defaultdict(int)
    l4_events: list = []                   # ladder4 全事件（含 bar）
    l5_events: list = []
    for i, s in enumerate(sigs):
        for lad in range(len(s.buy1)):
            if s.buy1[lad]:
                buy1_bars[lad] += 1
        if s.bsp_events:
            for lad, evs in enumerate(s.bsp_events):
                for (kind, side, seg_idx, confirmed, css, zd, zg, price) in evs:
                    ev_stats[(lad, kind, side, confirmed)] += 1
                    if lad == 4:
                        l4_events.append((i, kind, side, seg_idx, confirmed, price))
                    elif lad == 5:
                        l5_events.append((i, kind, side, seg_idx, confirmed, price))
    del sigs

    print("\n== 事件流聚合 (ladder, kind, side, confirmed) → n ==")
    for k in sorted(ev_stats):
        print(f"  ladder{k[0]} {k[1]:6s} {k[2]:4s} confirmed={k[3]!s:5s} : {ev_stats[k]}")
    print("\n== buy1 触发 bar 数 per ladder ==")
    for lad in sorted(buy1_bars):
        print(f"  ladder{lad}: {buy1_bars[lad]}")
    print(f"\nladder4 事件总数 {len(l4_events)}, ladder5 事件总数 {len(l5_events)}")

    # ── pass 2：终态结构深挖（重跑引擎一遍取终态） ──
    t0 = time.time()
    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)
    for i in range(n):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
    print(f"\nfinal-state pass {time.time() - t0:.1f}s")

    report: dict = {"ev_stats": {str(k): v for k, v in ev_stats.items()},
                    "buy1_bars": dict(buy1_bars),
                    "l4_events": l4_events, "l5_events": l5_events}

    # ladder2 笔中枢终态 BSP 全量
    bsps2 = orch.current_bi_zhongshu_buysellpoints_inc(BI_ZHONGSHU_LEVEL_ID)
    c2 = Counter((b[0][0], b[0][1], b[0][5]) for b in bsps2)
    print(f"\n== ladder2 笔中枢终态 BSP（共{len(bsps2)}）(kind, side, confirmed) ==")
    for k in sorted(c2):
        print(f"  {k}: {c2[k]}")
    report["ladder2_final_bsp"] = {str(k): v for k, v in c2.items()}

    # ladder2 背驰 kind 分布
    divs2 = orch.current_bi_zhongshu_divergences_inc(BI_ZHONGSHU_LEVEL_ID)
    cd2 = Counter(d[0] for d in divs2)
    print(f"  ladder2 背驰 kind 分布: {dict(cd2)}  (共{len(divs2)})")
    report["ladder2_div_kinds"] = dict(cd2)

    # ladder3 走势级终态 BSP
    bsps3 = orch.current_buysellpoints()
    c3 = Counter((b[0][0], b[0][1], b[0][5]) for b in bsps3)
    print(f"\n== ladder3 走势级终态 BSP（共{len(bsps3)}）==")
    for k in sorted(c3):
        print(f"  {k}: {c3[k]}")
    report["ladder3_final_bsp"] = {str(k): v for k, v in c3.items()}

    # 递归层终态：moves 结构 + 背驰 + BSP
    recursive = orch.current_recursive()
    level_moves_map = {lid: mvs for (lid, _z, mvs) in recursive}
    moves_l1 = orch.current_moves()
    rec_report = {}
    for (lid, zhs, mvs) in recursive:
        ladder = lid + 2
        prev = moves_l1 if lid - 1 == 1 else level_moves_map.get(lid - 1, [])
        mv_stats = Counter((m[0][0], m[0][7], m[0][6]) for m in mvs)  # (kind, settled, zs_count)
        bsps_l, div_rows_l = _level_bsps_with_divs(prev, zhs, mvs, lid)
        cb = Counter((b[0][0], b[0][1], b[0][5]) for b in bsps_l)
        cdv = Counter(d[0] for d in div_rows_l)
        n_trend_settled2 = sum(1 for m in mvs
                               if m[0][0] == "trend" and m[0][6] >= 2)
        print(f"\n== ladder{ladder} (level_id={lid}) 终态 ==")
        print(f"  下层组件段数(prev moves)={len(prev)}  本层中枢={len(zhs)}  本层moves={len(mvs)}")
        print(f"  moves (kind, settled, zs_count) 分布: {dict(mv_stats)}")
        print(f"  trend moves with zs_count>=2: {n_trend_settled2}")
        print(f"  背驰 kind 分布: {dict(cdv)} (共{len(div_rows_l)})")
        print(f"  BSP (kind, side, confirmed) 分布: {dict(cb)} (共{len(bsps_l)})")
        # 若有 trend 背驰，打印 force 比率
        trend_divs = [(d[2], d[3], d[4]) for d in div_rows_l if d[0] == "trend"]
        for (idx, fa, fc) in trend_divs:
            ratio = fc / fa if fa > 0 else float("nan")
            print(f"    trend div @seg_c_end={idx}: force_c/force_a={ratio:.4f} "
                  f"(confirmed需≤0.9: {ratio <= 0.9})")
        rec_report[ladder] = {
            "n_prev_segs": len(prev), "n_zs": len(zhs), "n_moves": len(mvs),
            "mv_stats": {str(k): v for k, v in mv_stats.items()},
            "div_kinds": dict(cdv),
            "bsp": {str(k): v for k, v in cb.items()},
            "trend_div_ratios": [(idx, fa, fc) for (idx, fa, fc) in trend_divs],
        }
    report["recursive_final"] = rec_report

    # type2 存在性剖析（ladder2）：终态 type1 之后是否有 rebound→callback 段
    segs2 = orch.stroke_count()
    t1s = [b for b in bsps2 if b[0][0] == "type1"]
    t2s = [b for b in bsps2 if b[0][0] == "type2"]
    print(f"\n== ladder2 type2 存在性 ==")
    print(f"  终态 confirmed 笔数(段数)≈{segs2}, type1={len(t1s)}, type2={len(t2s)}")
    if t1s:
        tail = t1s[-3:]
        for b in tail:
            print(f"  type1 tail: side={b[0][1]} seg_idx={b[0][3]} "
                  f"confirmed={b[0][5]} price={b[1][2]}")
    report["ladder2_type1_n"] = len(t1s)
    report["ladder2_type2_n"] = len(t2s)

    OUT.write_text(json.dumps(report, indent=1, default=str))
    print(f"\n→ {OUT}")


if __name__ == "__main__":
    main()
