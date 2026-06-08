"""线段瓶颈诊断：dump 每根笔 + 每个线段 + 特征序列触发轨迹。

复用 streaming 路径（RecursiveOrchestrator），从末态 snapshot 取 L1 笔/线段，
再脱离 orchestrator 单独跑 segments_from_strokes_v1 做白盒 instrument。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

from _qqq_data import load_qqq_last_n, to_bars  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan import a_segment_v1 as seg_mod  # noqa: E402


def main() -> None:
    data = load_qqq_last_n(1000)
    bars = to_bars(data)
    orch = RecursiveOrchestrator(
        stream_id="diag", max_levels=6, stroke_mode="new",
        reset_dir_on_fractal=True, new_raw_gap_min=3,
        enable_macd_divergence=True,
    )
    snap = None
    for bar in bars:
        snap = orch.process_bar(bar)

    strokes = snap.bi_snapshot.strokes
    segs = snap.seg_snapshot.segments
    print(f"=== L1: {len(strokes)} 笔, {len(segs)} 线段 ===\n")

    print("--- 每根笔 (idx, dir, i0..i1, p0->p1, high/low, conf) ---")
    for i, s in enumerate(strokes):
        print(f"{i:3d} {s.direction:4s} i[{s.i0:4d}..{s.i1:4d}] "
              f"p[{s.p0:7.2f}->{s.p1:7.2f}] hl[{s.high:7.2f}/{s.low:7.2f}] "
              f"{'C' if s.confirmed else '.'}")

    print("\n--- 每个线段 (idx, dir, s0..s1=笔范围, 含笔数, conf, gap) ---")
    for i, sg in enumerate(segs):
        nb = sg.s1 - sg.s0 + 1
        ge = sg.break_evidence.gap_type if sg.break_evidence else "-"
        print(f"{i:3d} {sg.direction:4s} s[{sg.s0:3d}..{sg.s1:3d}] "
              f"笔数={nb:3d} {'C' if sg.confirmed else '.'} gap={ge} "
              f"hl[{sg.high:7.2f}/{sg.low:7.2f}]")

    # 白盒：直接对 L1 笔跑 segment v1，instrument scan_trigger
    print("\n=== 白盒 instrument: segments_from_strokes_v1 over L1 strokes ===")
    instrument(strokes)


def instrument(strokes: list) -> None:
    """复刻主循环，记录每次 append 后 scan_trigger 的结果与拒绝原因。"""
    n = len(strokes)
    seg_start = seg_mod._find_overlap_start(strokes, 0)
    print(f"first overlap start: {seg_start}")
    if seg_start is None:
        return
    seg_dir = strokes[seg_start].direction
    feat = seg_mod._FeatureSeqState(seg_dir)
    cursor = seg_start
    trig_count = 0
    reject_minlen = 0
    reject_anchor = 0
    reject_gap2 = 0
    emit = 0

    while cursor < n:
        sk = strokes[cursor]
        opposite = "down" if seg_dir == "up" else "up"
        if sk.direction != opposite:
            cursor += 1
            continue
        feat.append(cursor, sk.high, sk.low)
        trig = feat.scan_trigger(seg_dir, strokes)
        if trig is not None:
            trig_count += 1
            k, abc, gap = trig
            end_stroke = k - 1
            if end_stroke - seg_start < 3 - 1:
                reject_minlen += 1
                feat.skip_trigger(k)
                cursor += 1
                continue
            if k + 2 >= n or not seg_mod._three_stroke_overlap(
                strokes[k], strokes[k + 1], strokes[k + 2]
            ):
                reject_anchor += 1
                feat.skip_trigger(k)
                cursor += 1
                continue
            emit += 1
            print(f"  EMIT seg dir={seg_dir} s0={seg_start} s1={end_stroke} "
                  f"trigger_k={k} gap={gap} feat_len={len(feat.std)}")
            seg_start, seg_dir = k, opposite
            feat.reset(seg_dir)
            cursor = k
            continue
        cursor += 1

    print(f"\nscan_trigger 命中={trig_count} | EMIT={emit} | "
          f"拒绝(minlen)={reject_minlen} 拒绝(anchor)={reject_anchor}")
    # 看看 down 段从未触发？统计方向
    print(f"末段 seg_start={seg_start} seg_dir={seg_dir} 剩余笔={n-seg_start}")


if __name__ == "__main__":
    main()
