"""BZ Brent原油日线 缠论管线分析脚本"""

from __future__ import annotations

import sys
from datetime import datetime, timezone

import pandas as pd

sys.path.insert(0, "/Users/silencehan/Projects/NewChanlun/src")

from newchan.cache import load_df
from newchan.nested_pipeline import run_nested_search
from newchan.types import Bar


def df_to_bars(df: pd.DataFrame) -> list[Bar]:
    """DataFrame -> Bar list"""
    bars = []
    for ts, row in df.iterrows():
        bars.append(Bar(
            ts=ts.to_pydatetime(),
            open=float(row["open"]),
            high=float(row["high"]),
            low=float(row["low"]),
            close=float(row["close"]),
            volume=float(row["volume"]) if "volume" in row.index and pd.notna(row.get("volume")) else None,
        ))
    return bars


def main() -> None:
    # 1. 加载数据
    df = load_df("BZ_1day_raw")
    if df is None:
        print("ERROR: BZ_1day_raw.parquet not found in cache")
        return

    print(f"=== BZ Brent原油 日线数据 ===")
    print(f"数据范围: {df.index[0]} ~ {df.index[-1]}")
    print(f"总K线数: {len(df)}")
    print(f"列: {list(df.columns)}")
    print()

    # 2. 转 Bar
    bars = df_to_bars(df)

    # 3. 跑管线
    print("正在运行缠论管线 (stroke_mode=wide, max_levels=6) ...")
    results, snap = run_nested_search(bars=bars, stroke_mode="wide", max_levels=6)

    if snap is None:
        print("ERROR: 管线未产出快照")
        return

    # 4. Level 1 统计
    print("\n" + "=" * 60)
    print("Level 1 (日线基础级别)")
    print("=" * 60)

    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves
    bsps = snap.bsp_snapshot.buysellpoints

    print(f"  笔(Stroke): {len(strokes)}")
    print(f"  线段(Segment): {len(segments)}")
    print(f"    - confirmed: {sum(1 for s in segments if s.confirmed)}")
    print(f"    - candidate: {sum(1 for s in segments if not s.confirmed)}")
    print(f"  中枢(Zhongshu): {len(zhongshus)}")
    print(f"    - settled: {sum(1 for z in zhongshus if z.settled)}")
    print(f"    - unsettled: {sum(1 for z in zhongshus if not z.settled)}")
    print(f"  走势(Move): {len(moves)}")
    for i, m in enumerate(moves):
        print(f"    [{i}] {m.kind} {m.direction} seg[{m.seg_start}..{m.seg_end}] "
              f"zs_count={m.zs_count} settled={m.settled}")
    print(f"  买卖点(BSP): {len(bsps)}")
    for bp in bsps:
        print(f"    {bp.kind} {bp.side} level={bp.level_id} seg={bp.seg_idx} "
              f"price={bp.price:.2f} confirmed={bp.confirmed} settled={bp.settled}")

    # 5. 递归级别统计
    print("\n" + "=" * 60)
    print("递归级别 (Level >= 2)")
    print("=" * 60)

    for rsnap in snap.recursive_snapshots:
        lvl = rsnap.level_id
        print(f"\n  Level {lvl}:")
        print(f"    中枢(LevelZhongshu): {len(rsnap.zhongshus)}")
        for j, zs in enumerate(rsnap.zhongshus):
            print(f"      [{j}] ZD={zs.zd:.2f} ZG={zs.zg:.2f} "
                  f"comp[{zs.comp_start}..{zs.comp_end}] settled={zs.settled}")
        print(f"    走势(Move): {len(rsnap.moves)}")
        for j, m in enumerate(rsnap.moves):
            print(f"      [{j}] {m.kind} {m.direction} seg[{m.seg_start}..{m.seg_end}] "
                  f"zs_count={m.zs_count} settled={m.settled}")

    # 6. 区间套背驰搜索结果
    print("\n" + "=" * 60)
    print("区间套背驰搜索结果 (Nested Divergence)")
    print("=" * 60)

    if not results:
        print("  无背驰信号")
    else:
        for i, nd in enumerate(results):
            print(f"\n  [{i}] bar_range={nd.bar_range}")
            for lvl_id, div in nd.chain:
                if div is not None:
                    print(f"    Level {lvl_id}: {div.kind} {div.direction} "
                          f"A_seg=[{div.seg_a_start},{div.seg_a_end}] "
                          f"C_seg=[{div.seg_c_start},{div.seg_c_end}]")
                else:
                    print(f"    Level {lvl_id}: (无背驰)")

    # 7. 当前走势状态
    print("\n" + "=" * 60)
    print("当前走势状态")
    print("=" * 60)

    last_bar = bars[-1]
    print(f"  最新K线: {last_bar.ts.strftime('%Y-%m-%d')} "
          f"O={last_bar.open:.2f} H={last_bar.high:.2f} "
          f"L={last_bar.low:.2f} C={last_bar.close:.2f}")

    if moves:
        m = moves[-1]
        print(f"  当前走势(L1): {m.kind} {m.direction} "
              f"seg[{m.seg_start}..{m.seg_end}] zs_count={m.zs_count}")

    if snap.recursive_snapshots:
        for rsnap in snap.recursive_snapshots:
            if rsnap.moves:
                m = rsnap.moves[-1]
                print(f"  当前走势(L{rsnap.level_id}): {m.kind} {m.direction} "
                      f"seg[{m.seg_start}..{m.seg_end}] zs_count={m.zs_count}")

    if snap.lstar is not None:
        print(f"  L* (综合级别状态): {snap.lstar}")

    # 8. 最近买卖点（按 bar_idx 排序取最近10个）
    print("\n" + "=" * 60)
    print("最近买卖点 (按时间排序)")
    print("=" * 60)

    if bsps:
        sorted_bsps = sorted(bsps, key=lambda b: b.bar_idx, reverse=True)
        for bp in sorted_bsps[:10]:
            # 用 bar_idx 反查时间
            if 0 <= bp.bar_idx < len(bars):
                t = bars[bp.bar_idx].ts.strftime("%Y-%m-%d")
            else:
                t = f"bar#{bp.bar_idx}"
            print(f"  {t} | {bp.kind} {bp.side} | price={bp.price:.2f} "
                  f"| confirmed={bp.confirmed} settled={bp.settled}")

    # 9. 2024-2026 关键结构转折点
    print("\n" + "=" * 60)
    print("2024-2026 关键结构转折点")
    print("=" * 60)

    # 用 naive datetime 比较（数据中可能是 naive）
    sample_ts = bars[0].ts
    if sample_ts.tzinfo is not None:
        cutoff_2024 = datetime(2024, 1, 1, tzinfo=sample_ts.tzinfo)
    else:
        cutoff_2024 = datetime(2024, 1, 1)

    # 找 2024 之后 settled 的线段端点（结构转折）
    print("\n  -- 线段端点 (2024+) --")
    seg_count = 0
    for seg in segments:
        if seg.confirmed and seg.i1 < len(bars):
            seg_end_bar = bars[seg.i1] if seg.i1 < len(bars) else None
            if seg_end_bar and seg_end_bar.ts >= cutoff_2024:
                ep = seg.ep1_price if seg.ep1_price != 0 else (seg.high if seg.direction == "up" else seg.low)
                print(f"    {seg_end_bar.ts.strftime('%Y-%m-%d')} | "
                      f"seg[{seg.s0}..{seg.s1}] {seg.direction} | "
                      f"ep={ep:.2f} H={seg.high:.2f} L={seg.low:.2f}")
                seg_count += 1
    if seg_count == 0:
        print("    (无)")

    # 找 2024 之后的买卖点
    print("\n  -- 买卖点 (2024+) --")
    bsp_count = 0
    for bp in bsps:
        if 0 <= bp.bar_idx < len(bars):
            t = bars[bp.bar_idx].ts
            if t >= cutoff_2024:
                print(f"    {t.strftime('%Y-%m-%d')} | {bp.kind} {bp.side} | "
                      f"price={bp.price:.2f} confirmed={bp.confirmed}")
                bsp_count += 1
    if bsp_count == 0:
        print("    (无)")

    # 找 2024 之后的中枢突破
    print("\n  -- 中枢突破 (2024+) --")
    zs_break_count = 0
    for zs in zhongshus:
        if zs.settled and zs.break_seg >= 0 and zs.break_seg < len(segments):
            bs = segments[zs.break_seg]
            if bs.i0 < len(bars):
                t = bars[bs.i0].ts if bs.i0 < len(bars) else None
                if t and t >= cutoff_2024:
                    print(f"    {t.strftime('%Y-%m-%d')} | "
                          f"ZD={zs.zd:.2f} ZG={zs.zg:.2f} break={zs.break_direction} | "
                          f"seg[{zs.seg_start}..{zs.seg_end}]")
                    zs_break_count += 1
    if zs_break_count == 0:
        print("    (无)")

    # 10. 补充：最近 20 根笔 + 线段时间定位
    print("\n" + "=" * 60)
    print("最近 20 根笔 (时间定位)")
    print("=" * 60)
    for st in strokes[-20:]:
        t0 = bars[st.i0].ts.strftime("%Y-%m-%d") if st.i0 < len(bars) else "?"
        t1 = bars[st.i1].ts.strftime("%Y-%m-%d") if st.i1 < len(bars) else "?"
        print(f"  stroke[{st.i0}..{st.i1}] {t0}~{t1} "
              f"dir={'up' if st.direction == 'up' else 'down'} "
              f"H={st.high:.2f} L={st.low:.2f} confirmed={st.confirmed}")

    print("\n" + "=" * 60)
    print("全部线段 (时间定位)")
    print("=" * 60)
    for idx, seg in enumerate(segments):
        t0 = bars[seg.i0].ts.strftime("%Y-%m-%d") if seg.i0 < len(bars) else "?"
        t1 = bars[seg.i1].ts.strftime("%Y-%m-%d") if seg.i1 < len(bars) else "?"
        ep = seg.ep1_price if seg.ep1_price != 0 else (seg.high if seg.direction == "up" else seg.low)
        print(f"  seg[{idx}] s[{seg.s0}..{seg.s1}] {t0}~{t1} "
              f"{seg.direction} H={seg.high:.2f} L={seg.low:.2f} ep1={ep:.2f} "
              f"confirmed={seg.confirmed} kind={seg.kind}")

    # 11. 全部中枢时间定位
    print("\n" + "=" * 60)
    print("全部中枢 (时间定位)")
    print("=" * 60)
    for idx, zs in enumerate(zhongshus):
        # 中枢的时间范围：用第一段和最后段的 stroke 端点
        first_seg = segments[zs.seg_start] if zs.seg_start < len(segments) else None
        last_seg = segments[zs.seg_end] if zs.seg_end < len(segments) else None
        t0 = bars[first_seg.i0].ts.strftime("%Y-%m-%d") if first_seg and first_seg.i0 < len(bars) else "?"
        t1 = bars[last_seg.i1].ts.strftime("%Y-%m-%d") if last_seg and last_seg.i1 < len(bars) else "?"
        print(f"  zs[{idx}] {t0}~{t1} ZD={zs.zd:.2f} ZG={zs.zg:.2f} "
              f"DD={zs.dd:.2f} GG={zs.gg:.2f} seg[{zs.seg_start}..{zs.seg_end}] "
              f"settled={zs.settled} break={zs.break_direction}")

    # 12. 2024+ 笔端点结构转折
    print("\n" + "=" * 60)
    print("2024-2026 笔端点结构转折")
    print("=" * 60)
    recent_strokes = [s for s in strokes if s.confirmed and s.i1 < len(bars)
                      and bars[s.i1].ts >= cutoff_2024]
    for st in recent_strokes:
        t1 = bars[st.i1].ts.strftime("%Y-%m-%d")
        price = st.high if st.direction == "up" else st.low
        print(f"  {t1} | stroke[{st.i0}..{st.i1}] {st.direction} | "
              f"price={price:.2f}")

    # 13. 包含关系合并诊断
    print("\n" + "=" * 60)
    print("包含关系合并诊断")
    print("=" * 60)
    print(f"  原始K线: {len(bars)}")
    print(f"  合并后(merged): {snap.bi_snapshot.n_merged}")
    print(f"  分型数: {snap.bi_snapshot.n_fractals}")
    print(f"  压缩率: {snap.bi_snapshot.n_merged / len(bars) * 100:.1f}%")
    print(f"  最后一笔: merged[{strokes[-1].i0}..{strokes[-1].i1}] "
          f"confirmed={strokes[-1].confirmed}")
    print(f"  最后一笔之后的merged bars: {snap.bi_snapshot.n_merged - 1 - strokes[-1].i1}")
    print(f"  说明: 最后一笔(2021-05~2021-06)之后，分型间距不满足wide模式(gap>=4)")
    print(f"       这表示2021-06之后的价格波动被包含关系大量合并")

    print("\n=== 分析完成 ===")


if __name__ == "__main__":
    main()
