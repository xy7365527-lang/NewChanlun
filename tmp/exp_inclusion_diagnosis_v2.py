"""诊断 v2：确认差异是数据范围导致还是算法导致

关键发现：我们 1072 个分型全部落在查缠 1488 中（重叠=1072，只在我们=0）。
假设：差异来自数据时间范围不同（查缠 5282 行 2005~ vs 我们 3482 行 2012~）。
验证：在相同时间范围上对比。
"""
import sys
sys.path.insert(0, "src")

import numpy as np
import pandas as pd
from newchan.cache import load_df
from newchan.a_fractal import fractals_from_merged, Fractal
from newchan.a_inclusion import merge_inclusion

# ── 加载数据 ──────────────────────────────────────────────
tv = load_df('TV_XAUUSD_BCOUSD_1day_full_raw')

csv_path = '/Users/silencehan/Downloads/OANDA_XAUUSD_OANDA_BCOUSD, 1D_9a7eb.csv'
df_chan = pd.read_csv(csv_path)
df_chan['datetime'] = pd.to_datetime(df_chan['time'], unit='s')
df_chan = df_chan.set_index('datetime').sort_index()
typing_col = df_chan['typing']
chan_fractals = typing_col.dropna()

print("=== 数据范围对比 ===")
print(f"我们: {tv.index[0]} ~ {tv.index[-1]} ({len(tv)} 行)")
print(f"查缠: {df_chan.index[0]} ~ {df_chan.index[-1]} ({len(df_chan)} 行)")

# ── 对齐时间范围 ─────────────────────────────────────────
# 找重叠时间段
common_start = max(tv.index[0], df_chan.index[0])
common_end = min(tv.index[-1], df_chan.index[-1])
print(f"\n重叠时间段: {common_start} ~ {common_end}")

# 在重叠时间段内对比查缠分型数
chan_in_range = chan_fractals[(chan_fractals.index >= common_start) & (chan_fractals.index <= common_end)]
print(f"查缠在重叠段内的分型数: {len(chan_in_range)}")

# 我们在重叠段内的分型数（就是全部，因为我们的数据就在这个范围内）
merged_std, _ = merge_inclusion(tv)
fractals_std = fractals_from_merged(merged_std)
print(f"我们在重叠段内的分型数: {len(fractals_std)}")
print(f"差值: {len(chan_in_range) - len(fractals_std)}")

# ── 查缠在我们数据范围之外有多少分型？─────────────────────
chan_before = chan_fractals[chan_fractals.index < common_start]
chan_after = chan_fractals[chan_fractals.index > common_end]
print(f"\n查缠在我们数据之前的分型: {len(chan_before)}")
print(f"查缠在我们数据之后的分型: {len(chan_after)}")
print(f"总差异 {len(chan_fractals) - len(fractals_std)} = 范围外 {len(chan_before) + len(chan_after)} + 范围内差异 {len(chan_in_range) - len(fractals_std)}")

# ── 范围内差异分析 ─────────────────────────────────────────
if len(chan_in_range) != len(fractals_std):
    print(f"\n=== 范围内差异分析 ===")
    our_dates = set()
    for f in fractals_std:
        our_dates.add(merged_std.index[f.idx])

    chan_dates_in_range = set(chan_in_range.index)

    overlap = our_dates & chan_dates_in_range
    only_ours = our_dates - chan_dates_in_range
    only_chan = chan_dates_in_range - our_dates

    print(f"重叠: {len(overlap)}")
    print(f"只在我们: {len(only_ours)}")
    print(f"只在查缠: {len(only_chan)}")

    if only_chan:
        only_chan_sorted = sorted(only_chan)
        print(f"\n只在查缠的分型日期（前20个）:")
        for d in only_chan_sorted[:20]:
            # 找到这个日期在原始数据中的位置和周边K线
            idx_in_tv = tv.index.get_indexer([d], method='nearest')[0]
            if 0 <= idx_in_tv < len(tv):
                start = max(0, idx_in_tv - 1)
                end = min(len(tv), idx_in_tv + 2)
                window = tv.iloc[start:end]
                print(f"  {d}: 我们数据 idx={idx_in_tv}")
                for j, (dt, row) in enumerate(window.iterrows()):
                    marker = " <<" if dt == d else ""
                    print(f"    [{dt}] O={row['open']:.4f} H={row['high']:.4f} L={row['low']:.4f} C={row['close']:.4f}{marker}")
            else:
                print(f"  {d}: 不在我们数据中")

    if only_ours:
        print(f"\n只在我们的分型日期（前10个）:")
        for d in sorted(only_ours)[:10]:
            print(f"  {d}")

# ── 查缠的分型判定逻辑推断 ─────────────────────────────────
print("\n=== 推断查缠的分型判定方式 ===")
# 查缠数据自带 OHLC，直接用查缠的数据跑我们的算法
df_chan_ohlc = df_chan[['open', 'high', 'low', 'close']].copy()

# 在查缠的完整数据上跑我们的管线
merged_chan, _ = merge_inclusion(df_chan_ohlc)
fractals_chan = fractals_from_merged(merged_chan)
print(f"用查缠完整数据跑我们的管线: merged={len(merged_chan)}, 分型={len(fractals_chan)}")
print(f"查缠自己的分型数: {len(chan_fractals)}")
print(f"差值: {len(chan_fractals) - len(fractals_chan)}")

# 不做包含 + 单条件
def fractals_single_condition(df):
    n = len(df)
    highs = df['high'].values.astype(np.float64)
    lows = df['low'].values.astype(np.float64)
    result = []
    for i in range(1, n - 1):
        if highs[i] > highs[i-1] and highs[i] > highs[i+1]:
            result.append(Fractal(idx=i, kind="top", price=float(highs[i])))
        elif lows[i] < lows[i-1] and lows[i] < lows[i+1]:
            result.append(Fractal(idx=i, kind="bottom", price=float(lows[i])))
    return result

fractals_chan_single_raw = fractals_single_condition(df_chan_ohlc)
print(f"查缠数据+不包含+单条件: {len(fractals_chan_single_raw)}")

# 不做包含 + 单条件（非严格）
def fractals_nonstrict_single(df):
    n = len(df)
    highs = df['high'].values.astype(np.float64)
    lows = df['low'].values.astype(np.float64)
    result = []
    for i in range(1, n - 1):
        if highs[i] >= highs[i-1] and highs[i] >= highs[i+1] and \
           not (highs[i] == highs[i-1] and highs[i] == highs[i+1]):
            result.append(Fractal(idx=i, kind="top", price=float(highs[i])))
        elif lows[i] <= lows[i-1] and lows[i] <= lows[i+1] and \
             not (lows[i] == lows[i-1] and lows[i] == lows[i+1]):
            result.append(Fractal(idx=i, kind="bottom", price=float(lows[i])))
    return result

fractals_chan_nonstrict = fractals_nonstrict_single(df_chan_ohlc)
print(f"查缠数据+不包含+非严格单条件: {len(fractals_chan_nonstrict)}")

# 我们的管线（宽松包含+双条件）在查缠数据上
merged_full, _ = merge_inclusion(df_chan_ohlc)
fractals_full = fractals_from_merged(merged_full)
our_dates_full = set(merged_full.index[f.idx] for f in fractals_full)
chan_dates_full = set(chan_fractals.index)
overlap_full = our_dates_full & chan_dates_full
print(f"\n在查缠完整数据上: 我们={len(fractals_full)}, 查缠={len(chan_fractals)}, 重叠={len(overlap_full)}")
print(f"只在查缠: {len(chan_dates_full - our_dates_full)}")
print(f"只在我们: {len(our_dates_full - chan_dates_full)}")

# ── 查缠多出来的分型，看它们是否满足单条件 ──────────────────
only_in_chan = chan_dates_full - our_dates_full
print(f"\n=== 查缠多出来的 {len(only_in_chan)} 个分型分析 ===")

if only_in_chan:
    # 查这些位置在原始数据上是否满足单条件
    single_match = 0
    double_but_inclusion_merged = 0
    neither = 0

    for d in sorted(only_in_chan)[:50]:
        # 在 df_chan_ohlc 找最近位置
        try:
            iloc_pos = df_chan_ohlc.index.get_loc(d)
        except KeyError:
            continue

        if isinstance(iloc_pos, slice):
            iloc_pos = iloc_pos.start

        if iloc_pos <= 0 or iloc_pos >= len(df_chan_ohlc) - 1:
            continue

        h_prev = df_chan_ohlc['high'].iloc[iloc_pos - 1]
        h_curr = df_chan_ohlc['high'].iloc[iloc_pos]
        h_next = df_chan_ohlc['high'].iloc[iloc_pos + 1]
        l_prev = df_chan_ohlc['low'].iloc[iloc_pos - 1]
        l_curr = df_chan_ohlc['low'].iloc[iloc_pos]
        l_next = df_chan_ohlc['low'].iloc[iloc_pos + 1]

        is_top_single = h_curr > h_prev and h_curr > h_next
        is_bottom_single = l_curr < l_prev and l_curr < l_next
        is_top_double = is_top_single and l_curr > l_prev and l_curr > l_next
        is_bottom_double = is_bottom_single and h_curr < h_prev and h_curr < h_next

        if is_top_double or is_bottom_double:
            double_but_inclusion_merged += 1
        elif is_top_single or is_bottom_single:
            single_match += 1
        else:
            neither += 1

    sampled = min(50, len(only_in_chan))
    print(f"  抽样 {sampled} 个查缠独有分型:")
    print(f"    满足双条件但被包含合并: {double_but_inclusion_merged}")
    print(f"    只满足单条件: {single_match}")
    print(f"    两者都不满足（可能涉及包含处理后的判定）: {neither}")
