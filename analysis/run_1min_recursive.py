"""QQQ 1min 递归引擎运行脚本。

从 1 分钟 K 线自下而上递归：
  1min K线 → 笔 → 线段 → 中枢 → 走势 → RecursiveStack → 更高级别

数据源：analysis/data_cache/qqq_1m_3mo.json (Polygon.io)
输出：  analysis/data_cache/recursive_1min_result.json

用法：
  python analysis/run_1min_recursive.py
  python analysis/run_1min_recursive.py --max-bars 5000   # 快速测试
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from datetime import datetime
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "src"))

from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.types import Bar  # noqa: E402

DATA_PATH = PROJECT_ROOT / "analysis" / "data_cache" / "qqq_1m_3mo.json"
OUT_PATH = PROJECT_ROOT / "analysis" / "data_cache" / "recursive_1min_result.json"


def load_bars(path: Path, max_bars: int | None = None) -> tuple[list[Bar], dict]:
    with open(path) as f:
        raw = json.load(f)
    bars_data = raw["bars"]
    if max_bars:
        bars_data = bars_data[:max_bars]
    bars = [
        Bar(
            ts=datetime.fromisoformat(b["ts"]),
            open=b["open"],
            high=b["high"],
            low=b["low"],
            close=b["close"],
            volume=b.get("volume"),
        )
        for b in bars_data
    ]
    return bars, raw


def run_engine(bars: list[Bar]) -> "RecursiveOrchestratorSnapshot":  # noqa: F821
    orch = RecursiveOrchestrator(
        stream_id="QQQ_1min",
        max_levels=8,
        stroke_mode="new",
        min_strict_sep=5,
    )
    t0 = time.time()
    snap = None
    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)
        if (i + 1) % 5000 == 0:
            e = time.time() - t0
            bi = len(snap.bi_snapshot.strokes)
            sg = len(snap.seg_snapshot.segments)
            zs = len(snap.zs_snapshot.zhongshus)
            mv = len(snap.move_snapshot.moves)
            rc = len(snap.recursive_snapshots)
            print(
                f"  [{i+1}/{len(bars)}] {e:.1f}s | "
                f"bi={bi} seg={sg} zs={zs} mv={mv} rec={rc}",
                flush=True,
            )
    elapsed = time.time() - t0
    print(f"\nDone in {elapsed:.1f}s", flush=True)
    return snap, elapsed


def extract_results(
    snap, bars: list[Bar], raw_meta: dict, elapsed: float,
) -> dict:
    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves
    rec_snaps = snap.recursive_snapshots
    bsp_points = snap.bsp_snapshot.buysellpoints

    def safe_ts(idx: int) -> str:
        return bars[min(idx, len(bars) - 1)].ts.isoformat()

    stroke_data = [
        {
            "n": i,
            "dir": s.direction,
            "p0": round(s.p0, 2),
            "p1": round(s.p1, 2),
            "ts0": safe_ts(s.i0),
            "ts1": safe_ts(s.i1),
        }
        for i, s in enumerate(strokes)
    ]

    seg_data = [
        {
            "n": i,
            "dir": sg.direction,
            "s0": sg.s0,
            "s1": sg.s1,
            "p0": round(sg.p0, 2),
            "p1": round(sg.p1, 2),
            "h": round(sg.high, 2),
            "l": round(sg.low, 2),
            "ts0": safe_ts(strokes[min(sg.s0, len(strokes) - 1)].i0),
            "ts1": safe_ts(strokes[min(sg.s1, len(strokes) - 1)].i1),
            "bi_cnt": sg.s1 - sg.s0 + 1,
        }
        for i, sg in enumerate(segments)
    ]

    zs_data = [
        {
            "n": i,
            "zg": round(z.zg, 2),
            "zd": round(z.zd, 2),
            "gg": round(z.gg, 2),
            "dd": round(z.dd, 2),
            "seg_s": z.seg_start,
            "seg_e": z.seg_end,
            "cnt": z.seg_count,
            "settled": z.settled,
        }
        for i, z in enumerate(zhongshus)
    ]

    mv_data = [
        {
            "n": i,
            "kind": m.kind,
            "dir": m.direction,
            "zs_cnt": m.zs_count,
            "settled": m.settled,
            "h": round(m.high, 2),
            "l": round(m.low, 2),
            "seg_s": m.seg_start,
            "seg_e": m.seg_end,
        }
        for i, m in enumerate(moves)
    ]

    rec_data = {}
    for rs in rec_snaps:
        zs_list = [
            {
                "n": i,
                "zg": round(z.zg, 2),
                "zd": round(z.zd, 2),
                "cnt": z.comp_count,
                "settled": z.settled,
            }
            for i, z in enumerate(rs.zhongshus)
        ]
        mv_list = [
            {
                "n": i,
                "dir": m.direction,
                "kind": m.kind,
                "zs_cnt": m.zs_count,
                "settled": m.settled,
                "h": round(m.high, 2),
                "l": round(m.low, 2),
            }
            for i, m in enumerate(rs.moves)
        ]
        rec_data[f"L{rs.level_id}"] = {"zhongshus": zs_list, "moves": mv_list}

    bsp_data = [
        {
            "kind": b.kind,
            "side": b.side,
            "price": round(b.price, 2),
            "bar_idx": b.bar_idx,
            "ts": safe_ts(b.bar_idx),
            "confirmed": b.confirmed,
            "settled": b.settled,
            "seg_idx": b.seg_idx,
        }
        for b in bsp_points
    ]

    return {
        "meta": {
            "symbol": "QQQ",
            "base_tf": "1min",
            "bars": len(bars),
            "date_range": raw_meta["date_range"],
            "elapsed_s": round(elapsed, 1),
        },
        "L1": {
            "strokes": len(stroke_data),
            "segments": len(seg_data),
            "zhongshus": len(zs_data),
            "moves": len(mv_data),
            "settled_moves": sum(1 for m in moves if m.settled),
            "stroke_data": stroke_data,
            "seg_data": seg_data,
            "zs_data": zs_data,
            "mv_data": mv_data,
        },
        "recursive": rec_data,
        "bsp": bsp_data,
    }


def print_summary(result: dict) -> None:
    l1 = result["L1"]
    print(f"\n{'='*60}")
    print(f"L1: {l1['strokes']} strokes, {l1['segments']} segs, "
          f"{l1['zhongshus']} zs, {l1['moves']} moves "
          f"({l1['settled_moves']} settled)")
    print(f"BSP: {len(result['bsp'])} points")
    for k, v in result["recursive"].items():
        print(f"{k}: {len(v['zhongshus'])} zs, {len(v['moves'])} moves")

    print(f"\n--- L1 Moves ---")
    for m in l1["mv_data"]:
        print(f"  Move#{m['n']:2d} {m['kind']:14s} {m['dir']:4s} "
              f"zs={m['zs_cnt']} [{m['l']:.2f}~{m['h']:.2f}] "
              f"settled={m['settled']}")

    print(f"\n--- Recursive ---")
    for k, v in result["recursive"].items():
        print(f"  {k}:")
        for z in v["zhongshus"]:
            print(f"    ZS#{z['n']} [{z['zd']:.2f}~{z['zg']:.2f}] "
                  f"{z['cnt']}comp settled={z['settled']}")
        for m in v["moves"]:
            print(f"    Move#{m['n']} {m['kind']} {m['dir']} "
                  f"zs={m['zs_cnt']} [{m['l']:.2f}~{m['h']:.2f}] "
                  f"settled={m['settled']}")

    print(f"\n--- BSP (last 10) ---")
    for b in result["bsp"][-10:]:
        print(f"  {b['kind']:5s} {b['side']:4s} @{b['price']:>7.2f} "
              f"{b['ts'][:16]} conf={b['confirmed']} settled={b['settled']}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Run 1min recursive engine on QQQ")
    parser.add_argument("--max-bars", type=int, default=None,
                        help="Limit bar count for quick testing")
    args = parser.parse_args()

    print(f"Loading data from {DATA_PATH}...", flush=True)
    bars, raw = load_bars(DATA_PATH, max_bars=args.max_bars)
    print(f"  {len(bars)} bars loaded", flush=True)

    snap, elapsed = run_engine(bars)

    result = extract_results(snap, bars, raw, elapsed)

    with open(OUT_PATH, "w") as f:
        json.dump(result, f, indent=2, default=str)
    print(f"\nSaved to {OUT_PATH}")

    print_summary(result)


if __name__ == "__main__":
    main()
