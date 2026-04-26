"""BZ Brent 4小时缠论管线分析"""
import sys
sys.path.insert(0, "/Users/silencehan/Projects/NewChanlun/src")

import pandas as pd
from datetime import datetime

from newchan.data_databento import fetch_ohlcv
from newchan.cache import save_df, load_df
from newchan.nested_pipeline import run_nested_search
from newchan.a_inclusion import merge_inclusion
from newchan.types import Bar

# ============================================================
# 第一步：拉取数据
# ============================================================

# 先尝试从缓存加载
df_1h = load_df("BZ_1hour_raw")
if df_1h is None:
    print("从 Databento 拉取 BZ 1hour 数据...")
    df_1h = fetch_ohlcv(symbol="BZ", interval="1hour", start="2021-01-01")
    save_df("BZ_1hour_raw", df_1h)
    print(f"已缓存 BZ_1hour_raw")
else:
    print("从缓存加载 BZ_1hour_raw")

print(f"BZ 1hour: {len(df_1h)} rows, {df_1h.index[0]} ~ {df_1h.index[-1]}")

# resample 到 4h
df_4h = df_1h.resample("4h").agg({
    "open": "first", "high": "max", "low": "min", "close": "last", "volume": "sum"
}).dropna()
save_df("BZ_4hour_raw", df_4h)
print(f"BZ 4hour: {len(df_4h)} rows, {df_4h.index[0]} ~ {df_4h.index[-1]}")

# ============================================================
# 第二步：跑管线
# ============================================================

df = load_df("BZ_4hour_raw")
bars = [
    Bar(ts=ts, open=row["open"], high=row["high"], low=row["low"],
        close=row["close"], volume=row.get("volume"))
    for ts, row in df.iterrows()
]
print(f"\nK线数: {len(bars)}")

results, snap = run_nested_search(bars=bars, stroke_mode="wide", max_levels=6)

# ============================================================
# 第三步：输出分析
# ============================================================

if snap is None:
    print("管线无输出")
    sys.exit(1)

# 构建 merged bar 索引 -> 时间戳映射
df_raw = pd.DataFrame({
    "open": [b.open for b in bars],
    "high": [b.high for b in bars],
    "low": [b.low for b in bars],
    "close": [b.close for b in bars],
}, index=pd.DatetimeIndex([b.ts for b in bars]))
df_merged, merged_to_raw = merge_inclusion(df_raw)
# merged_to_raw[i] = (raw_start, raw_end) 表示第 i 根合并K线对应原始K线范围

def merged_idx_to_ts(idx):
    """将 merged bar index 转成时间戳"""
    if idx < 0 or idx >= len(merged_to_raw):
        return "N/A"
    raw_start, raw_end = merged_to_raw[idx]
    if raw_end < len(bars):
        return str(bars[raw_end].ts)
    return str(bars[-1].ts)

def merged_idx_to_ts_start(idx):
    """将 merged bar index 转成起始时间戳"""
    if idx < 0 or idx >= len(merged_to_raw):
        return "N/A"
    raw_start, _ = merged_to_raw[idx]
    if raw_start < len(bars):
        return str(bars[raw_start].ts)
    return str(bars[-1].ts)

print(f"\n合并K线数: {len(df_merged)}")

# --- Level 1 统计 ---
strokes = snap.bi_snapshot.strokes
segments = snap.seg_snapshot.segments
zhongshus = snap.zs_snapshot.zhongshus
moves = snap.move_snapshot.moves
bsps = snap.bsp_snapshot.buysellpoints

print(f"\n{'='*60}")
print(f"Level 1 统计")
print(f"{'='*60}")
print(f"  笔:     {len(strokes)}")
print(f"  线段:   {len(segments)}")
print(f"  中枢:   {len(zhongshus)}")
print(f"  走势:   {len(moves)}")
print(f"  买卖点: {len(bsps)}")

# --- 递归级别统计 ---
for rs in snap.recursive_snapshots:
    print(f"\nLevel {rs.level_id} 统计")
    print(f"  中枢:   {len(rs.zhongshus)}")
    print(f"  走势:   {len(rs.moves)}")

# --- 当前走势状态 ---
print(f"\n{'='*60}")
print(f"当前走势状态")
print(f"{'='*60}")

if moves:
    m = moves[-1]
    print(f"  Level 1 当前走势: {m.kind} {m.direction}")
    print(f"    seg_start={m.seg_start} seg_end={m.seg_end} 中枢数={m.zs_count} settled={m.settled}")
    if m.seg_start < len(segments) and m.seg_end < len(segments):
        seg_s = segments[m.seg_start]
        seg_e = segments[m.seg_end]
        print(f"    起点时间: {merged_idx_to_ts_start(seg_s.i0)}")
        print(f"    当前位置: {merged_idx_to_ts(seg_e.i1)}")

for rs in snap.recursive_snapshots:
    if rs.moves:
        m = rs.moves[-1]
        print(f"  Level {rs.level_id} 当前走势: {m.kind} {m.direction} 中枢数={m.zs_count} settled={m.settled}")

# --- 买卖点详情 ---
print(f"\n{'='*60}")
print(f"买卖点详情 (Level 1)")
print(f"{'='*60}")

for bp in bsps:
    ts = merged_idx_to_ts(bp.bar_idx)
    print(f"  [{bp.kind}] {bp.side} @ price={bp.price:.2f} bar_idx={bp.bar_idx} ts={ts} "
          f"confirmed={bp.confirmed} settled={bp.settled}"
          f"{' overlaps=' + bp.overlaps_with if bp.overlaps_with else ''}")

# --- 2025-02-20 附近结构 ---
print(f"\n{'='*60}")
print(f"2025-02-20 附近结构分析")
print(f"{'='*60}")

target_date = pd.Timestamp("2025-02-20")
tolerance = pd.Timedelta(days=15)  # +/- 15天窗口

# 找附近的线段端点
print("\n线段端点 (2025-02-05 ~ 2025-03-07):")
for i, seg in enumerate(segments):
    ts0 = merged_idx_to_ts_start(seg.i0)
    ts1 = merged_idx_to_ts(seg.i1)
    try:
        t0 = pd.Timestamp(ts0)
        t1 = pd.Timestamp(ts1)
    except Exception:
        continue
    if (abs(t0 - target_date) < tolerance) or (abs(t1 - target_date) < tolerance):
        print(f"  seg[{i}] {seg.direction} s0={seg.s0} s1={seg.s1} "
              f"high={seg.high:.2f} low={seg.low:.2f} confirmed={seg.confirmed}")
        print(f"    时间: {ts0} ~ {ts1}")

# 找附近的中枢
print("\n中枢 (2025-02-05 ~ 2025-03-07):")
for i, zs in enumerate(zhongshus):
    if zs.seg_start < len(segments) and zs.seg_end < len(segments):
        seg_s = segments[zs.seg_start]
        seg_e = segments[zs.seg_end]
        ts0 = merged_idx_to_ts_start(seg_s.i0)
        ts1 = merged_idx_to_ts(seg_e.i1)
        try:
            t0 = pd.Timestamp(ts0)
            t1 = pd.Timestamp(ts1)
        except Exception:
            continue
        if (t0 < target_date + tolerance) and (t1 > target_date - tolerance):
            print(f"  zs[{i}] zd={zs.zd:.2f} zg={zs.zg:.2f} "
                  f"seg[{zs.seg_start}:{zs.seg_end}] settled={zs.settled} "
                  f"break={zs.break_direction}")
            print(f"    时间: {ts0} ~ {ts1}")

# 找附近的买卖点
print("\n买卖点 (2025-02-05 ~ 2025-03-07):")
for bp in bsps:
    ts = merged_idx_to_ts(bp.bar_idx)
    try:
        t = pd.Timestamp(ts)
    except Exception:
        continue
    if abs(t - target_date) < tolerance:
        print(f"  [{bp.kind}] {bp.side} @ {bp.price:.2f} ts={ts} "
              f"confirmed={bp.confirmed} settled={bp.settled}")

# --- 最近3个月关键转折点 ---
print(f"\n{'='*60}")
print(f"最近3个月关键转折点 (2026-01 ~ 2026-04)")
print(f"{'='*60}")

recent_cutoff = pd.Timestamp("2026-01-01")

print("\n最近线段:")
for i, seg in enumerate(segments):
    ts1 = merged_idx_to_ts(seg.i1)
    try:
        t1 = pd.Timestamp(ts1)
    except Exception:
        continue
    if t1 >= recent_cutoff:
        ts0 = merged_idx_to_ts_start(seg.i0)
        print(f"  seg[{i}] {seg.direction} high={seg.high:.2f} low={seg.low:.2f} "
              f"confirmed={seg.confirmed}")
        print(f"    时间: {ts0} ~ {ts1}")

print("\n最近中枢:")
for i, zs in enumerate(zhongshus):
    if zs.seg_start < len(segments) and zs.seg_end < len(segments):
        seg_e = segments[zs.seg_end]
        ts1 = merged_idx_to_ts(seg_e.i1)
        try:
            t1 = pd.Timestamp(ts1)
        except Exception:
            continue
        if t1 >= recent_cutoff:
            seg_s = segments[zs.seg_start]
            ts0 = merged_idx_to_ts_start(seg_s.i0)
            print(f"  zs[{i}] zd={zs.zd:.2f} zg={zs.zg:.2f} "
                  f"seg[{zs.seg_start}:{zs.seg_end}] settled={zs.settled} "
                  f"break={zs.break_direction}")
            print(f"    时间: {ts0} ~ {ts1}")

print("\n最近买卖点:")
for bp in bsps:
    ts = merged_idx_to_ts(bp.bar_idx)
    try:
        t = pd.Timestamp(ts)
    except Exception:
        continue
    if t >= recent_cutoff:
        print(f"  [{bp.kind}] {bp.side} @ {bp.price:.2f} ts={ts} "
              f"confirmed={bp.confirmed} settled={bp.settled}")

# --- 最近走势详情 ---
print(f"\n{'='*60}")
print(f"最近走势详情 (Level 1 最后5个走势)")
print(f"{'='*60}")

for m in moves[-5:]:
    if m.seg_start < len(segments) and m.seg_end < len(segments):
        seg_s = segments[m.seg_start]
        seg_e = segments[m.seg_end]
        ts0 = merged_idx_to_ts_start(seg_s.i0)
        ts1 = merged_idx_to_ts(seg_e.i1)
        print(f"  {m.kind} {m.direction} | 中枢数={m.zs_count} settled={m.settled}")
        print(f"    seg[{m.seg_start}:{m.seg_end}] | 时间: {ts0} ~ {ts1}")

# --- 背驰信号 ---
print(f"\n{'='*60}")
print(f"区间套背驰搜索结果")
print(f"{'='*60}")

if results:
    for i, nd in enumerate(results):
        print(f"\n背驰链 #{i+1}:")
        print(f"  bar_range: {nd.bar_range}")
        if nd.bar_range[0] < len(bars) and nd.bar_range[1] < len(bars):
            print(f"  时间范围: {bars[nd.bar_range[0]].ts} ~ {bars[nd.bar_range[1]].ts}")
        for level_id, div in nd.chain:
            if div is not None:
                print(f"  Level {level_id}: {div.kind} {div.direction} "
                      f"force_a={div.force_a:.4f} force_c={div.force_c:.4f}")
            else:
                print(f"  Level {level_id}: (无背驰)")
else:
    print("  未检测到区间套背驰信号")

# --- 所有买卖点汇总（跨级别）---
print(f"\n{'='*60}")
print(f"全部三类买卖点汇总")
print(f"{'='*60}")

print(f"\nLevel 1 买卖点 ({len(bsps)} 个):")
for bp in bsps:
    ts = merged_idx_to_ts(bp.bar_idx)
    print(f"  [{bp.kind}] {bp.side} @ {bp.price:.2f} ts={ts} settled={bp.settled}")

print(f"\n{'='*60}")
print(f"按类型统计:")
print(f"{'='*60}")
from collections import Counter
type_counter = Counter((bp.kind, bp.side) for bp in bsps)
for (kind, side), count in sorted(type_counter.items()):
    print(f"  {kind} {side}: {count}")
