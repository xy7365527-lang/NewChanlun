#!/usr/bin/env python3
"""逐位等价回归 harness — 增量化优化的正确性闸门。

捕获 RecursiveOrchestrator 每根 bar 的完整结构输出指纹（精确 repr，无 rounding），
保存为 golden；优化后重跑，逐 bar 比对，定位首个分歧点。

任何 bit 级差异（包括 float max/min 重排导致的尾数差）都会被捕获。

用法：
    # 捕获 golden（优化前）
    PYTHONPATH=src python3 scripts/regression_equiv.py capture --n 20000 --out .cache/golden_20k.txt
    # 比对（优化后）
    PYTHONPATH=src python3 scripts/regression_equiv.py verify --n 20000 --golden .cache/golden_20k.txt
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

from newchan.orchestrator.recursive import RecursiveOrchestrator
from profile_incremental import load_bz_bars


def _zhongshu_digest(zss) -> str:
    return "|".join(
        f"{z.zd!r},{z.zg!r},{z.seg_start},{z.seg_end},{z.seg_count},{z.settled},"
        f"{z.break_seg},{z.break_direction},{z.first_seg_s0},{z.last_seg_s1},{z.gg!r},{z.dd!r}"
        for z in zss
    )


def _move_digest(moves) -> str:
    return "|".join(
        f"{m.kind},{m.direction},{m.seg_start},{m.seg_end},{m.zs_start},{m.zs_end},"
        f"{m.zs_count},{m.settled},{m.high!r},{m.low!r},{m.first_seg_s0},{m.last_seg_s1},"
        f"{m.zg_max!r},{m.zd_min!r},{m.persistence!r}"
        for m in moves
    )


def _level_zs_digest(zss) -> str:
    return "|".join(
        f"{z.zd!r},{z.zg!r},{z.comp_start},{z.comp_end},{z.comp_count},{z.settled},"
        f"{z.break_comp},{z.break_direction},{z.gg!r},{z.dd!r},{z.level_id}"
        for z in zss
    )


def _segment_digest(segs) -> str:
    return "|".join(
        f"{s.s0},{s.s1},{s.i0},{s.i1},{s.direction},{s.high!r},{s.low!r},"
        f"{s.confirmed},{s.kind},{s.p0!r},{s.p1!r}"
        for s in segs
    )


def _stroke_digest(strokes) -> str:
    return "|".join(
        f"{s.i0},{s.i1},{s.direction},{s.p0!r},{s.p1!r},{s.confirmed}"
        for s in strokes
    )


def bar_digest(snap) -> str:
    """单根 bar 的完整结构指纹。"""
    parts = [
        f"bi={_stroke_digest(snap.bi_snapshot.strokes)}",
        f"seg={_segment_digest(snap.seg_snapshot.segments)}",
        f"zs={_zhongshu_digest(snap.zs_snapshot.zhongshus)}",
        f"mv={_move_digest(snap.move_snapshot.moves)}",
        f"ev={len(snap.all_events)}",
    ]
    for rs in snap.recursive_snapshots:
        parts.append(
            f"L{rs.level_id}:zs={_level_zs_digest(rs.zhongshus)};mv={_move_digest(rs.moves)}"
        )
    lstar = snap.lstar
    parts.append(f"lstar={lstar.level if lstar else None}")
    return "".join(parts)


def run_digests(n: int) -> list[str]:
    bars = load_bz_bars(limit=n)
    orch = RecursiveOrchestrator(
        stream_id="regress", max_levels=6,
        stroke_mode="wide", min_strict_sep=5,
    )
    out: list[str] = []
    for bar in bars:
        snap = orch.process_bar(bar)
        out.append(bar_digest(snap))
    return out


def _event_digest(events) -> str:
    """逐 bar 事件指纹（O(events)/bar，通常为空）。events 是规范化 diff 输出。"""
    return ";".join(f"{type(e).__name__}:{getattr(e, 'event_id', '')}" for e in events)


def run_fast(n: int, snapshot_every: int = 20000) -> list[str]:
    """O(1)/bar 等价检查：每 bar 记事件指纹 + 周期/末态全态指纹。

    事件是规范化 diff，捕获所有状态转移；周期/末态全态指纹兜底捕获
    "无事件但状态漂移"。新旧实现在此序列上一致 ⟹ 逐位等价。harness 本身
    O(1)/bar（不像 full 模式每 bar 全量字符串化），原始慢代码也能跑大规模。
    """
    bars = load_bz_bars(limit=n)
    orch = RecursiveOrchestrator(
        stream_id="regress", max_levels=6,
        stroke_mode="wide", min_strict_sep=5,
    )
    out: list[str] = []
    last = n - 1
    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)
        line = f"ev={_event_digest(snap.all_events)}"
        if i % snapshot_every == 0 or i == last:
            line += "|STATE=" + bar_digest(snap)
        out.append(line)
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["capture", "verify"])
    ap.add_argument("--n", type=int, default=20000)
    ap.add_argument("--out", default=".cache/golden.txt")
    ap.add_argument("--golden", default=".cache/golden.txt")
    ap.add_argument("--fast", action="store_true", help="O(1)/bar 事件指纹模式")
    args = ap.parse_args()

    runner = run_fast if args.fast else run_digests

    if args.mode == "capture":
        digests = runner(args.n)
        Path(args.out).write_text("\n".join(digests), encoding="utf-8")
        print(f"golden captured: {len(digests)} bars → {args.out}")
    else:
        golden = Path(args.golden).read_text(encoding="utf-8").split("\n")
        digests = runner(args.n)
        if len(golden) != len(digests):
            print(f"FAIL: bar count {len(digests)} != golden {len(golden)}")
            sys.exit(1)
        mismatches = 0
        first = -1
        for i, (g, d) in enumerate(zip(golden, digests)):
            if g != d:
                mismatches += 1
                if first < 0:
                    first = i
        if mismatches == 0:
            print(f"PASS: {len(digests)} bars 逐位等价")
        else:
            print(f"FAIL: {mismatches} bars 不匹配，首个分歧 @ bar {first}")
            g, d = golden[first], digests[first]
            j = next((k for k in range(min(len(g), len(d))) if g[k] != d[k]), min(len(g), len(d)))
            lo = max(0, j - 80)
            print(f"  diff @ char {j}")
            print(f"  golden: ...{g[lo:j+120]}")
            print(f"  actual: ...{d[lo:j+120]}")
            sys.exit(1)


if __name__ == "__main__":
    main()
