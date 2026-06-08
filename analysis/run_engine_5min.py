"""Run the chanlun engine on QQQ 5min data and dump recursive structures.

Output: analysis/data_cache/engine_5min_output.json
"""
import json
import sys
import time
from datetime import datetime

sys.path.insert(0, "src")

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar


def load_5min_bars(path: str) -> list[Bar]:
    with open(path) as f:
        data = json.load(f)
    bars = []
    for b in data["bars"]:
        bars.append(Bar(
            ts=datetime.fromisoformat(b["ts"]),
            open=b["open"],
            high=b["high"],
            low=b["low"],
            close=b["close"],
            volume=b["volume"],
        ))
    return bars


def serialize_strokes(strokes):
    return [
        {
            "i0": s.i0, "i1": s.i1,
            "direction": s.direction,
            "high": s.high, "low": s.low,
            "p0": s.p0, "p1": s.p1,
            "confirmed": s.confirmed,
        }
        for s in strokes
    ]


def serialize_segments(segments):
    return [
        {
            "start_bi": s.start_bi, "end_bi": s.end_bi,
            "direction": s.direction,
            "high": s.high, "low": s.low,
            "confirmed": s.confirmed,
            "kind": getattr(s, "kind", ""),
        }
        for s in segments
    ]


def serialize_zhongshus(zss):
    return [
        {
            "zd": z.zd, "zg": z.zg,
            "seg_start": z.seg_start, "seg_end": z.seg_end,
            "seg_count": z.seg_count, "settled": z.settled,
            "break_direction": z.break_direction,
            "gg": z.gg, "dd": z.dd,
            "first_seg_s0": z.first_seg_s0,
            "last_seg_s1": z.last_seg_s1,
        }
        for z in zss
    ]


def serialize_bsps(bsps):
    return [
        {
            "kind": b.kind, "side": b.side,
            "level_id": b.level_id, "seg_idx": b.seg_idx,
            "price": b.price, "bar_idx": b.bar_idx,
            "confirmed": b.confirmed, "settled": b.settled,
        }
        for b in bsps
    ]


def serialize_moves(moves):
    return [
        {
            "direction": m.direction,
            "seg_start": m.seg_start, "seg_end": m.seg_end,
            "high": m.high, "low": m.low,
            "settled": m.settled,
        }
        for m in moves
    ]


def main():
    print("Loading 5min bars...")
    bars = load_5min_bars("analysis/data_cache/qqq_5m_2y.json")
    print(f"Loaded {len(bars)} bars: {bars[0].ts} to {bars[-1].ts}")

    orch = RecursiveOrchestrator(
        stream_id="qqq_5m",
        stroke_mode="new",
        enable_macd_divergence=False,
        max_levels=6,
    )

    t0 = time.time()
    snap = None
    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)
        if (i + 1) % 20000 == 0:
            elapsed = time.time() - t0
            print(f"  Processed {i+1}/{len(bars)} bars ({elapsed:.1f}s)")
            print(f"    L1 strokes={len(snap.bi_snapshot.strokes)}, "
                  f"segs={len(snap.seg_snapshot.segments)}, "
                  f"zs={len(snap.zs_snapshot.zhongshus)}, "
                  f"moves={len(snap.move_snapshot.moves)}, "
                  f"bsp={len(snap.bsp_snapshot.buysellpoints)}")
            print(f"    Recursive levels: {len(snap.recursive_snapshots)}")

    elapsed = time.time() - t0
    print(f"\nDone in {elapsed:.1f}s")
    assert snap is not None

    # Collect stroke timestamps for date mapping
    stroke_ts_map = {}
    merged_bars = snap.bi_snapshot.merged_bars
    if merged_bars is not None and len(merged_bars) > 0:
        for s in snap.bi_snapshot.strokes:
            if s.i0 < len(merged_bars):
                stroke_ts_map[s.i0] = str(merged_bars[s.i0].ts) if hasattr(merged_bars[s.i0], 'ts') else str(s.i0)
            if s.i1 < len(merged_bars):
                stroke_ts_map[s.i1] = str(merged_bars[s.i1].ts) if hasattr(merged_bars[s.i1], 'ts') else str(s.i1)

    output = {
        "bars_processed": len(bars),
        "time_range": [str(bars[0].ts), str(bars[-1].ts)],
        "elapsed_seconds": round(elapsed, 1),
        "level1": {
            "strokes": serialize_strokes(snap.bi_snapshot.strokes),
            "stroke_count": len(snap.bi_snapshot.strokes),
            "confirmed_strokes": sum(1 for s in snap.bi_snapshot.strokes if s.confirmed),
            "segments": serialize_segments(snap.seg_snapshot.segments),
            "segment_count": len(snap.seg_snapshot.segments),
            "zhongshus": serialize_zhongshus(snap.zs_snapshot.zhongshus),
            "zhongshu_count": len(snap.zs_snapshot.zhongshus),
            "moves": serialize_moves(snap.move_snapshot.moves),
            "move_count": len(snap.move_snapshot.moves),
            "buy_sell_points": serialize_bsps(snap.bsp_snapshot.buysellpoints),
            "bsp_count": len(snap.bsp_snapshot.buysellpoints),
            "merged_bar_count": len(merged_bars) if merged_bars is not None else 0,
        },
        "recursive_levels": [],
    }

    for rs in snap.recursive_snapshots:
        level_data = {
            "level_id": rs.level_id,
            "zhongshus": serialize_zhongshus(rs.zhongshus),
            "zhongshu_count": len(rs.zhongshus),
            "moves": serialize_moves(rs.moves),
            "move_count": len(rs.moves),
        }
        output["recursive_levels"].append(level_data)
        print(f"  Level {rs.level_id}: {len(rs.zhongshus)} zhongshus, {len(rs.moves)} moves")

    # Print summary
    print(f"\n=== Level 1 Summary ===")
    print(f"  Merged bars: {output['level1']['merged_bar_count']}")
    print(f"  Strokes: {output['level1']['stroke_count']} ({output['level1']['confirmed_strokes']} confirmed)")
    print(f"  Segments: {output['level1']['segment_count']}")
    print(f"  Zhongshus: {output['level1']['zhongshu_count']}")
    print(f"  Moves: {output['level1']['move_count']}")
    print(f"  BSP: {output['level1']['bsp_count']}")

    # Save
    with open("analysis/data_cache/engine_5min_output.json", "w") as f:
        json.dump(output, f, indent=2, default=str)
    print(f"\nSaved to analysis/data_cache/engine_5min_output.json")


if __name__ == "__main__":
    main()
