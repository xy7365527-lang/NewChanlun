"""GC 黄金期货日线缠论管线分析"""

from __future__ import annotations

import logging
from datetime import datetime

import pandas as pd

from newchan.a_inclusion import merge_inclusion
from newchan.cache import load_df
from newchan.nested_pipeline import run_nested_search
from newchan.types import Bar

logging.basicConfig(level=logging.WARNING)


def df_to_bars(df: pd.DataFrame) -> list[Bar]:
    bars: list[Bar] = []
    for ts, row in df.iterrows():
        bars.append(Bar(
            ts=ts.to_pydatetime(),
            open=float(row["open"]),
            high=float(row["high"]),
            low=float(row["low"]),
            close=float(row["close"]),
            volume=float(row["volume"]) if pd.notna(row["volume"]) else None,
        ))
    return bars


def main() -> None:
    # 1. 加载数据
    df = load_df("GC_1day_raw")
    if df is None:
        print("ERROR: GC_1day_raw not found in cache")
        return
    print(f"=== GC 黄金期货日线缠论分析 ===")
    print(f"数据范围: {df.index[0]} ~ {df.index[-1]}, 共 {len(df)} 根K线")
    print(f"最新价格: {df['close'].iloc[-1]}")
    print()

    # 2. 转 Bar，建立 merged→raw 映射
    bars = df_to_bars(df)
    df_merged, merged_to_raw = merge_inclusion(df)
    print(f"包含处理后: {len(df_merged)} 根合并K线 (原始 {len(df)})")

    raw_dates = list(df.index)

    def merged_idx_to_date(midx: int) -> str:
        """merged bar index → 原始日期字符串"""
        if midx < 0 or midx >= len(merged_to_raw):
            return f"merged_idx={midx}"
        raw_start, raw_end = merged_to_raw[midx]
        if raw_start < len(raw_dates):
            return raw_dates[raw_start].strftime("%Y-%m-%d")
        return f"raw_idx={raw_start}"

    def merged_idx_to_raw_date(midx: int) -> datetime | None:
        if midx < 0 or midx >= len(merged_to_raw):
            return None
        raw_start, _ = merged_to_raw[midx]
        if raw_start < len(raw_dates):
            return raw_dates[raw_start].to_pydatetime()
        return None

    # 3. 跑管线
    results, snap = run_nested_search(
        bars=bars, stroke_mode="wide", max_levels=6,
    )

    if snap is None:
        print("ERROR: pipeline returned None snapshot")
        return

    # ====== 4. 各级别统计 ======
    print()
    print("=" * 60)
    print("一、各级别结构统计")
    print("=" * 60)

    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves
    bsps = snap.bsp_snapshot.buysellpoints

    confirmed_strokes = [s for s in strokes if s.confirmed]
    confirmed_segs = [s for s in segments if s.confirmed]
    settled_zs = [z for z in zhongshus if z.settled]
    settled_moves = [m for m in moves if m.settled]

    print(f"\n[Level 1] 笔(Stroke)")
    print(f"  总数: {len(strokes)}, 已确认: {len(confirmed_strokes)}")
    if strokes:
        last = strokes[-1]
        dt0 = merged_idx_to_date(last.i0)
        dt1 = merged_idx_to_date(last.i1)
        print(f"  最后一笔: {dt0} → {dt1}, 方向={last.direction}, "
              f"价格 {last.p0:.1f} → {last.p1:.1f}, "
              f"confirmed={last.confirmed}")

    print(f"\n[Level 1] 线段(Segment)")
    print(f"  总数: {len(segments)}, 已确认: {len(confirmed_segs)}")
    if segments:
        last = segments[-1]
        dt0 = merged_idx_to_date(last.i0)
        dt1 = merged_idx_to_date(last.i1)
        print(f"  最后线段: {dt0} → {dt1}, 方向={last.direction}, "
              f"s0={last.s0}~s1={last.s1} ({last.s1 - last.s0 + 1}笔), "
              f"ep: {last.ep0_price:.1f} → {last.ep1_price:.1f}, "
              f"kind={last.kind}")

    print(f"\n[Level 1] 中枢(Zhongshu)")
    print(f"  总数: {len(zhongshus)}, 已结算: {len(settled_zs)}")
    for i, zs in enumerate(zhongshus):
        # 中枢对应的线段索引 → 线段的 i0/i1 → merged → raw date
        seg_start_dt = "?"
        seg_end_dt = "?"
        if zs.seg_start < len(segments):
            seg_start_dt = merged_idx_to_date(segments[zs.seg_start].i0)
        if zs.seg_end < len(segments):
            seg_end_dt = merged_idx_to_date(segments[zs.seg_end].i1)
        print(f"  [{i}] {seg_start_dt} ~ {seg_end_dt}: "
              f"[{zs.zd:.1f}, {zs.zg:.1f}], 段数={zs.seg_count}, "
              f"DD={zs.dd:.1f}, GG={zs.gg:.1f}, settled={zs.settled}"
              + (f", 突破方向={zs.break_direction}" if zs.settled else ""))

    print(f"\n[Level 1] 走势(Move)")
    print(f"  总数: {len(moves)}, 已结算: {len(settled_moves)}")
    for i, m in enumerate(moves):
        seg_start_dt = "?"
        seg_end_dt = "?"
        if m.seg_start < len(segments):
            seg_start_dt = merged_idx_to_date(segments[m.seg_start].i0)
        if m.seg_end < len(segments):
            seg_end_dt = merged_idx_to_date(segments[m.seg_end].i1)
        print(f"  [{i}] {seg_start_dt} ~ {seg_end_dt}: "
              f"{m.kind}/{m.direction}, 中枢数={m.zs_count}, "
              f"[{m.low:.1f}, {m.high:.1f}], settled={m.settled}")

    print(f"\n[Level 1] 买卖点(BuySellPoint)")
    print(f"  总数: {len(bsps)}")
    for bp in bsps:
        bp_date = "?"
        if bp.seg_idx < len(segments):
            seg = segments[bp.seg_idx]
            bp_date = merged_idx_to_date(seg.i1)
        print(f"  {bp_date}: {bp.kind}/{bp.side}, price={bp.price:.1f}, "
              f"confirmed={bp.confirmed}, settled={bp.settled}"
              + (f", overlaps={bp.overlaps_with}" if bp.overlaps_with else ""))

    # 递归级别
    for rs in snap.recursive_snapshots:
        lvl = rs.level_id
        n_zs = len(rs.zhongshus)
        n_moves = len(rs.moves)
        settled_zs_r = sum(1 for z in rs.zhongshus if z.settled)
        settled_moves_r = sum(1 for m in rs.moves if m.settled)
        print(f"\n[Level {lvl}]")
        print(f"  中枢: {n_zs} (settled: {settled_zs_r})")
        print(f"  走势: {n_moves} (settled: {settled_moves_r})")
        for i, zs in enumerate(rs.zhongshus):
            print(f"  中枢[{i}]: [{zs.zd:.1f}, {zs.zg:.1f}], "
                  f"comp=[{zs.comp_start}, {zs.comp_end}], "
                  f"settled={zs.settled}")
        for i, m in enumerate(rs.moves):
            print(f"  走势[{i}]: {m.kind}/{m.direction}, "
                  f"中枢数={m.zs_count}, [{m.low:.1f}, {m.high:.1f}], "
                  f"settled={m.settled}")

    # ====== 5. 区间套背驰搜索结果 ======
    print()
    print("=" * 60)
    print("二、区间套背驰搜索结果")
    print("=" * 60)

    if not results:
        print("  无区间套背驰信号")
    else:
        for i, nd in enumerate(results):
            print(f"\n  背驰链 #{i}:")
            br = nd.bar_range
            print(f"    merged bar range: [{br[0]}, {br[1]}] "
                  f"({merged_idx_to_date(br[0])} ~ {merged_idx_to_date(br[1])})")
            for lvl_id, div in nd.chain:
                if div is None:
                    print(f"    Level {lvl_id}: 无背驰")
                else:
                    ratio_str = ""
                    if div.force_a > 0:
                        ratio_str = f", force_ratio={div.force_c / div.force_a:.3f}"
                    print(f"    Level {lvl_id}: {div.kind}/{div.direction}, "
                          f"center_idx={div.center_idx}{ratio_str}")

    # ====== 6. 当前走势状态 ======
    print()
    print("=" * 60)
    print("三、当前走势状态")
    print("=" * 60)

    if moves:
        curr = moves[-1]
        print(f"  当前 Level 1 走势: {curr.kind}/{curr.direction}")
        print(f"  中枢数: {curr.zs_count}, 已结算: {curr.settled}")
        print(f"  价格范围: [{curr.low:.1f}, {curr.high:.1f}]")

    if snap.lstar is not None:
        print(f"  LStar: {snap.lstar}")

    # 当前价格 vs 各中枢
    current_price = df["close"].iloc[-1]
    print(f"\n  当前价格: {current_price:.1f}")
    for i, zs in enumerate(zhongshus):
        if current_price > zs.zg:
            pos = f"上方 (+{current_price - zs.zg:.1f})"
        elif current_price < zs.zd:
            pos = f"下方 (-{zs.zd - current_price:.1f})"
        else:
            pos = "区间内"
        print(f"  vs 中枢[{i}] [{zs.zd:.1f}, {zs.zg:.1f}]: {pos}")

    # ====== 7. 2024-2026 关键转折 ======
    print()
    print("=" * 60)
    print("四、2024-2026 关键结构转折点")
    print("=" * 60)

    cutoff = datetime(2024, 1, 1)

    # 笔级别转折（只看最近的）
    print("\n  --- 近期笔转折（2024年后）---")
    recent_strokes = []
    for si, s in enumerate(strokes):
        dt = merged_idx_to_raw_date(s.i1)
        if dt and dt >= cutoff:
            recent_strokes.append((si, s, dt))
    print(f"  2024年后共 {len(recent_strokes)} 笔")
    for si, s, dt in recent_strokes[-15:]:
        print(f"  [{si}] {merged_idx_to_date(s.i0)} → {merged_idx_to_date(s.i1)}: "
              f"{s.direction}, {s.p0:.1f} → {s.p1:.1f}, confirmed={s.confirmed}")

    # 线段级别转折
    print("\n  --- 近期线段转折（2024年后）---")
    for seg in segments:
        dt = merged_idx_to_raw_date(seg.i1)
        if dt and dt >= cutoff:
            print(f"  {merged_idx_to_date(seg.i0)} → {merged_idx_to_date(seg.i1)}: "
                  f"方向={seg.direction}, "
                  f"ep: {seg.ep0_price:.1f} → {seg.ep1_price:.1f}, "
                  f"({seg.s1 - seg.s0 + 1}笔), kind={seg.kind}")

    # 走势级别转折
    print("\n  --- 走势类型（涉及2024后的）---")
    for i, m in enumerate(moves):
        seg_end_dt = merged_idx_to_raw_date(
            segments[m.seg_end].i1 if m.seg_end < len(segments) else -1
        )
        seg_start_dt = merged_idx_to_raw_date(
            segments[m.seg_start].i0 if m.seg_start < len(segments) else -1
        )
        if (seg_end_dt and seg_end_dt >= cutoff) or (seg_start_dt and seg_start_dt >= cutoff):
            sd = merged_idx_to_date(segments[m.seg_start].i0) if m.seg_start < len(segments) else "?"
            ed = merged_idx_to_date(segments[m.seg_end].i1) if m.seg_end < len(segments) else "?"
            print(f"  [{i}] {sd} ~ {ed}: {m.kind}/{m.direction}, "
                  f"中枢数={m.zs_count}, [{m.low:.1f}, {m.high:.1f}]")


if __name__ == "__main__":
    main()
