"""OKLO 1min 递归引擎运行脚本。

从 1 分钟 K 线自下而上递归：
  1min K线 → 笔 → 线段 → 中枢 → 走势 → RecursiveStack → 更高级别

数据源：analysis/data_cache/oklo_1m_databento.json (Alpha Vantage)
输出：  analysis/data_cache/oklo_recursive_result.json

用法：
  PYTHONPATH=src python analysis/run_oklo_recursive.py
  PYTHONPATH=src python analysis/run_oklo_recursive.py --max-bars 5000
  PYTHONPATH=src python analysis/run_oklo_recursive.py --rth-only  # 仅正规交易时段
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

DATA_PATH = PROJECT_ROOT / "analysis" / "data_cache" / "oklo_1m_databento.json"
OUT_PATH = PROJECT_ROOT / "analysis" / "data_cache" / "oklo_recursive_result.json"


def load_bars(path: Path, max_bars: int | None = None, rth_only: bool = False) -> tuple[list[Bar], dict]:
    with open(path) as f:
        raw = json.load(f)
    bars_data = raw["bars"]

    if rth_only:
        bars_data = [b for b in bars_data if "09:30" <= b["ts"][11:16] < "16:00"]

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


def run_engine(bars: list[Bar]) -> tuple:
    orch = RecursiveOrchestrator(
        stream_id="OKLO_1min",
        max_levels=8,
        stroke_mode="new",
        min_strict_sep=5,
    )
    t0 = time.time()
    snap = None
    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)
        if (i + 1) % 10000 == 0:
            e = time.time() - t0
            bi = len(snap.bi_snapshot.strokes)
            sg = len(snap.seg_snapshot.segments)
            zs = len(snap.zs_snapshot.zhongshus)
            mv = len(snap.move_snapshot.moves)
            rc = len(snap.recursive_snapshots)
            print(
                f"  [{i+1:>6d}/{len(bars)}] {e:>6.1f}s | "
                f"bi={bi:>5d} seg={sg:>4d} zs={zs:>3d} mv={mv:>3d} rec_levels={rc}",
                flush=True,
            )
    elapsed = time.time() - t0
    print(f"\nDone in {elapsed:.1f}s ({len(bars)/elapsed:.0f} bars/s)", flush=True)
    return snap, elapsed


def extract_results(snap, bars: list[Bar], raw_meta: dict, elapsed: float) -> dict:
    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves
    rec_snaps = snap.recursive_snapshots
    bsp_points = snap.bsp_snapshot.buysellpoints

    def safe_ts(idx: int) -> str:
        return bars[min(idx, len(bars) - 1)].ts.isoformat()

    stroke_summary = [
        {
            "n": i, "dir": s.direction,
            "p0": round(s.p0, 2), "p1": round(s.p1, 2),
            "ts0": safe_ts(s.i0), "ts1": safe_ts(s.i1),
        }
        for i, s in enumerate(strokes[-50:])
    ]

    seg_data = [
        {
            "n": i, "dir": sg.direction,
            "s0": sg.s0, "s1": sg.s1,
            "p0": round(sg.p0, 2), "p1": round(sg.p1, 2),
            "h": round(sg.high, 2), "l": round(sg.low, 2),
            "ts0": safe_ts(strokes[min(sg.s0, len(strokes) - 1)].i0),
            "ts1": safe_ts(strokes[min(sg.s1, len(strokes) - 1)].i1),
            "bi_cnt": sg.s1 - sg.s0 + 1,
        }
        for i, sg in enumerate(segments)
    ]

    zs_data = [
        {
            "n": i,
            "zg": round(z.zg, 2), "zd": round(z.zd, 2),
            "gg": round(z.gg, 2), "dd": round(z.dd, 2),
            "seg_s": z.seg_start, "seg_e": z.seg_end,
            "cnt": z.seg_count, "settled": z.settled,
        }
        for i, z in enumerate(zhongshus)
    ]

    mv_data = [
        {
            "n": i, "kind": m.kind, "dir": m.direction,
            "zs_cnt": m.zs_count, "settled": m.settled,
            "h": round(m.high, 2), "l": round(m.low, 2),
            "seg_s": m.seg_start, "seg_e": m.seg_end,
        }
        for i, m in enumerate(moves)
    ]

    rec_data = {}
    for rs in rec_snaps:
        zs_list = [
            {
                "n": i,
                "zg": round(z.zg, 2), "zd": round(z.zd, 2),
                "cnt": z.comp_count, "settled": z.settled,
            }
            for i, z in enumerate(rs.zhongshus)
        ]
        mv_list = [
            {
                "n": i, "dir": m.direction, "kind": m.kind,
                "zs_cnt": m.zs_count, "settled": m.settled,
                "h": round(m.high, 2), "l": round(m.low, 2),
            }
            for i, m in enumerate(rs.moves)
        ]
        bsp_list = [
            {
                "kind": b.kind, "side": b.side,
                "price": round(b.price, 2),
                "confirmed": b.confirmed, "settled": b.settled,
            }
            for b in rs.buysellpoints
        ] if hasattr(rs, 'buysellpoints') else []
        rec_data[f"L{rs.level_id}"] = {
            "components": len(rs.components) if hasattr(rs, 'components') else 0,
            "zhongshus": zs_list,
            "moves": mv_list,
            "buysellpoints": bsp_list,
        }

    bsp_data = [
        {
            "kind": b.kind, "side": b.side,
            "price": round(b.price, 2),
            "bar_idx": b.bar_idx,
            "ts": safe_ts(b.bar_idx),
            "confirmed": b.confirmed, "settled": b.settled,
            "seg_idx": b.seg_idx,
        }
        for b in bsp_points
    ]

    return {
        "meta": {
            "symbol": "OKLO",
            "base_tf": "1min",
            "bars": len(bars),
            "date_range": raw_meta["date_range"],
            "elapsed_s": round(elapsed, 1),
            "bars_per_sec": round(len(bars) / elapsed),
        },
        "L1": {
            "strokes": len(strokes),
            "segments": len(segments),
            "zhongshus": len(zhongshus),
            "moves": len(moves),
            "settled_moves": sum(1 for m in moves if m.settled),
            "last_50_strokes": stroke_summary,
            "seg_data": seg_data,
            "zs_data": zs_data,
            "mv_data": mv_data,
        },
        "recursive": rec_data,
        "bsp": bsp_data,
    }


def print_summary(result: dict) -> None:
    meta = result["meta"]
    l1 = result["L1"]
    print(f"\n{'='*70}")
    print(f"  OKLO 1min Recursive Analysis")
    print(f"  {meta['bars']:,} bars | {meta['elapsed_s']}s ({meta['bars_per_sec']:,} bars/s)")
    print(f"  Date range: {meta['date_range']}")
    print(f"{'='*70}")

    print(f"\n  L1 (base level):")
    print(f"    Strokes: {l1['strokes']:>6,}")
    print(f"    Segments: {l1['segments']:>5,}")
    print(f"    Zhongshus: {l1['zhongshus']:>4,}")
    print(f"    Moves: {l1['moves']:>4,} ({l1['settled_moves']} settled)")
    print(f"    BSP: {len(result['bsp']):>4,}")

    print(f"\n  Recursive levels:")
    for k, v in result["recursive"].items():
        comps = v.get("components", "?")
        bsp_cnt = len(v.get("buysellpoints", []))
        print(f"    {k}: components={comps}, zs={len(v['zhongshus'])}, "
              f"moves={len(v['moves'])}, bsp={bsp_cnt}")

    print(f"\n  --- L1 Moves (last 10) ---")
    for m in l1["mv_data"][-10:]:
        print(f"    Move#{m['n']:3d} {m['kind']:14s} {m['dir']:4s} "
              f"zs={m['zs_cnt']} [{m['l']:.2f}~{m['h']:.2f}] "
              f"settled={m['settled']}")

    print(f"\n  --- Recursive Detail ---")
    for k, v in result["recursive"].items():
        if v["zhongshus"] or v["moves"]:
            print(f"    {k}:")
            for z in v["zhongshus"]:
                print(f"      ZS#{z['n']} [{z['zd']:.2f}~{z['zg']:.2f}] "
                      f"{z['cnt']}comp settled={z['settled']}")
            for m in v["moves"]:
                print(f"      Move#{m['n']} {m['kind']} {m['dir']} "
                      f"zs={m['zs_cnt']} [{m['l']:.2f}~{m['h']:.2f}] "
                      f"settled={m['settled']}")

    print(f"\n  --- BSP (last 15) ---")
    for b in result["bsp"][-15:]:
        print(f"    {b['kind']:5s} {b['side']:4s} @{b['price']:>8.2f} "
              f"{b['ts'][:16]} conf={b['confirmed']} settled={b['settled']}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Run 1min recursive engine on OKLO")
    parser.add_argument("--max-bars", type=int, default=None,
                        help="Limit bar count for quick testing")
    parser.add_argument("--rth-only", action="store_true",
                        help="Regular trading hours only (9:30-16:00 ET)")
    args = parser.parse_args()

    print(f"Loading data from {DATA_PATH}...", flush=True)
    bars, raw = load_bars(DATA_PATH, max_bars=args.max_bars, rth_only=args.rth_only)
    print(f"  {len(bars):,} bars loaded", flush=True)
    if bars:
        print(f"  Range: {bars[0].ts} to {bars[-1].ts}", flush=True)
        print(f"  Price: ${min(b.close for b in bars):.2f} - ${max(b.close for b in bars):.2f}", flush=True)

    snap, elapsed = run_engine(bars)
    result = extract_results(snap, bars, raw, elapsed)

    with open(OUT_PATH, "w") as f:
        json.dump(result, f, indent=2, default=str)
    print(f"\nSaved to {OUT_PATH}")

    print_summary(result)


if __name__ == "__main__":
    main()
