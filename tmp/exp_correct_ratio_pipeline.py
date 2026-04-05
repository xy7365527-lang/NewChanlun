"""用完整1分钟数据构造正确的金油比日线K线，跑缠论管线。

修正版：stroke 的 i0/i1 是合并K线索引，需要用 df_merged 来读取时间戳。
"""

import sys
sys.path.insert(0, "/Users/silencehan/Projects/NewChanlun/src")

import numpy as np
import pandas as pd

from newchan.cache import load_df, save_df
from newchan.nested_pipeline import run_nested_search
from newchan.types import Bar
from newchan.a_inclusion import merge_inclusion
from newchan.a_fractal import fractals_from_merged

# ── 1. 拼接4段1分钟数据 ──

print("=== 拼接1分钟数据 ===")

gc_parts = [load_df(f'GC_1min_{s}_raw') for s in ['2010_2014', '2015_2018', '2019_2023', '2024']]
gc_1m = pd.concat([p for p in gc_parts if p is not None]).sort_index()
gc_1m = gc_1m[~gc_1m.index.duplicated(keep='last')]
print(f"GC 1min: {len(gc_1m)} rows, {gc_1m.index[0]} ~ {gc_1m.index[-1]}")

bz_parts = [load_df(f'BZ_1min_{s}_raw') for s in ['2010_2014', '2015_2018', '2019_2023', '2024']]
bz_1m = pd.concat([p for p in bz_parts if p is not None]).sort_index()
bz_1m = bz_1m[~bz_1m.index.duplicated(keep='last')]
print(f"BZ 1min: {len(bz_1m)} rows, {bz_1m.index[0]} ~ {bz_1m.index[-1]}")

# ── 2. 按TradingView方式构造日线比价K线 ──

print("\n=== 构造日线比价K线 ===")

idx = gc_1m.index.intersection(bz_1m.index)
print(f"交集分钟数: {len(idx)}")

ratio_1m = (gc_1m.loc[idx, 'close'] / bz_1m.loc[idx, 'close']).replace([np.inf, -np.inf], np.nan).dropna()
print(f"有效 ratio 1min 数: {len(ratio_1m)}")

ratio_daily = ratio_1m.resample('D').agg(['first', 'max', 'min', 'last']).dropna()
ratio_daily.columns = ['open', 'high', 'low', 'close']

if 'volume' in gc_1m.columns:
    gc_vol = gc_1m.loc[idx, 'volume'].resample('D').sum()
    ratio_daily['volume'] = gc_vol.reindex(ratio_daily.index).fillna(0)
else:
    ratio_daily['volume'] = 0

print(f"日线K线: {len(ratio_daily)} rows, {ratio_daily.index[0].strftime('%Y-%m-%d')} ~ {ratio_daily.index[-1].strftime('%Y-%m-%d')}")

save_df("GC_BZ_ratio_1min_agg_1day_raw", ratio_daily)
print("已保存到缓存: GC_BZ_ratio_1min_agg_1day_raw")

# ── 准备合并K线索引（用于正确的时间戳映射） ──

df_merged, _ = merge_inclusion(ratio_daily, reset_dir_on_fractal=False)


def merged_ts(i: int) -> str:
    """合并K线索引 -> 日期字符串"""
    if 0 <= i < len(df_merged):
        return df_merged.index[i].strftime('%Y-%m-%d')
    return f'?({i})'


def merged_price(i: int, col: str) -> float:
    if 0 <= i < len(df_merged):
        return df_merged.iloc[i][col]
    return float('nan')


# ── 3. 跑管线（多种模式对比） ──

print("\n=== 跑管线 ===")

bars = [
    Bar(ts=ts, open=r['open'], high=r['high'], low=r['low'], close=r['close'], volume=r.get('volume', 0))
    for ts, r in ratio_daily.iterrows()
]

configs = [
    ('new', False),
    ('new', True),
    ('wide', False),
]

best_config = None
best_total = 0
best_results = None
best_snap = None

for mode, reset in configs:
    # 每种配置需要对应的 merged
    df_m_cfg, _ = merge_inclusion(ratio_daily, reset_dir_on_fractal=reset)
    results, snap = run_nested_search(bars=bars, stroke_mode=mode, max_levels=6, reset_dir_on_fractal=reset)
    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves

    total = len(strokes) + len(segments) + len(zhongshus) + len(moves)
    print(f'\n--- mode={mode}, reset={reset} ---')
    print(f'笔: {len(strokes)}, 线段: {len(segments)}, 中枢: {len(zhongshus)}, 走势: {len(moves)}')

    if strokes:
        last = strokes[-1]
        d = 'UP' if last.direction == 'up' else 'DN'
        ts0 = df_m_cfg.index[last.i0].strftime('%Y-%m-%d') if last.i0 < len(df_m_cfg) else '?'
        ts1 = df_m_cfg.index[last.i1].strftime('%Y-%m-%d') if last.i1 < len(df_m_cfg) else '?'
        print(f'最后一笔: {ts0}({last.p0:.2f}) -> {ts1}({last.p1:.2f}) {d}')
        print(f'最后一笔之后还有 {len(df_m_cfg)-1-last.i1} 根合并K线')

    if total > best_total:
        best_total = total
        best_config = (mode, reset)
        best_results = results
        best_snap = snap
        best_merged = df_m_cfg

# ── 4. 选择结构最丰富的配置，输出详细结果 ──

print(f'\n\n{"="*60}')
print(f'最优配置: mode={best_config[0]}, reset={best_config[1]}')
print(f'{"="*60}')

# 重新赋值 merged_ts 使用 best_merged
df_merged = best_merged


def mt(i: int) -> str:
    if 0 <= i < len(df_merged):
        return df_merged.index[i].strftime('%Y-%m-%d')
    return f'?({i})'


strokes = best_snap.bi_snapshot.strokes
segments = best_snap.seg_snapshot.segments
zhongshus = best_snap.zs_snapshot.zhongshus
moves = best_snap.move_snapshot.moves

# 全部走势
print(f'\n--- 全部走势 ({len(moves)}) ---')
for i, m in enumerate(moves):
    seg_s = segments[m.seg_start] if m.seg_start < len(segments) else None
    seg_e = segments[m.seg_end] if m.seg_end < len(segments) else None
    ts_s = mt(seg_s.i0) if seg_s else '?'
    ts_e = mt(seg_e.i1) if seg_e else '?'
    print(f'  [{i}] {m.kind} {m.direction} | 段{m.seg_start}-{m.seg_end} | {ts_s} ~ {ts_e}')

# 全部中枢
print(f'\n--- 全部中枢 ({len(zhongshus)}) ---')
for i, zs in enumerate(zhongshus):
    seg_s = segments[zs.seg_start] if zs.seg_start < len(segments) else None
    seg_e = segments[zs.seg_end] if zs.seg_end < len(segments) else None
    ts_s = mt(seg_s.i0) if seg_s else '?'
    ts_e = mt(seg_e.i1) if seg_e else '?'
    print(f'  [{i}] ZD={zs.zd:.2f} ZG={zs.zg:.2f} | 段{zs.seg_start}-{zs.seg_end}({zs.seg_count}段) | {ts_s} ~ {ts_e} | settled={zs.settled}')

# 最后15笔
n_strokes = min(15, len(strokes))
print(f'\n--- 最后 {n_strokes} 笔 ---')
for s in strokes[-n_strokes:]:
    d = 'UP' if s.direction == 'up' else 'DN'
    print(f'  {mt(s.i0)}({s.p0:.2f}) -> {mt(s.i1)}({s.p1:.2f}) {d} | high={s.high:.2f} low={s.low:.2f}')

# 最后8段线段
n_segs = min(8, len(segments))
print(f'\n--- 最后 {n_segs} 段线段 ---')
for seg in segments[-n_segs:]:
    d = 'UP' if seg.direction == 'up' else 'DN'
    print(f'  笔{seg.s0}-{seg.s1} | {mt(seg.i0)} -> {mt(seg.i1)} {d} | high={seg.high:.2f} low={seg.low:.2f}')

# 区间套背驰
print(f'\n--- 区间套背驰 ({len(best_results)}) ---')
for nd in best_results:
    print(f'  level={nd.level} | 笔idx {nd.stroke_idx} | 方向={nd.direction} | macd_area_ratio={nd.macd_area_ratio:.4f}')

# ── 盘整背驰手动检查 ──

if zhongshus:
    last_zs = zhongshus[-1]
    print(f'\n--- 最后一个中枢的盘整背驰检查 ---')
    seg_s = segments[last_zs.seg_start] if last_zs.seg_start < len(segments) else None
    seg_e = segments[last_zs.seg_end] if last_zs.seg_end < len(segments) else None
    print(f'中枢: ZD={last_zs.zd:.2f} ZG={last_zs.zg:.2f} | 段{last_zs.seg_start}-{last_zs.seg_end} | {mt(seg_s.i0) if seg_s else "?"} ~ {mt(seg_e.i1) if seg_e else "?"}')

    a_idx = last_zs.seg_start - 1
    c_idx = last_zs.seg_end + 1
    if a_idx >= 0 and a_idx < len(segments):
        a_seg = segments[a_idx]
        a_range = abs(a_seg.high - a_seg.low)
        print(f'  A段(进入段): 段idx={a_idx} | {mt(a_seg.i0)} ~ {mt(a_seg.i1)} | 幅度={a_range:.2f}')
    else:
        print(f'  A段: 不存在 (seg_start={last_zs.seg_start})')
        a_range = None

    if c_idx < len(segments):
        c_seg = segments[c_idx]
        c_range = abs(c_seg.high - c_seg.low)
        print(f'  C段(离开段): 段idx={c_idx} | {mt(c_seg.i0)} ~ {mt(c_seg.i1)} | 幅度={c_range:.2f}')
    else:
        print(f'  C段: 尚未形成 (seg_end={last_zs.seg_end}, total_segs={len(segments)})')
        c_range = None

    if a_range is not None and c_range is not None:
        if c_range < a_range:
            print(f'  ** 盘整背驰信号: C段幅度({c_range:.2f}) < A段幅度({a_range:.2f}) **')
        else:
            print(f'  无盘整背驰: C段幅度({c_range:.2f}) >= A段幅度({a_range:.2f})')

# ── 5. 关键验证 ──

print(f'\n--- 关键验证 ---')

strokes_after_2022 = [s for s in strokes if df_merged.index[s.i0].year >= 2022]
print(f'2022年之后的笔: {len(strokes_after_2022)}')
if strokes_after_2022:
    first = strokes_after_2022[0]
    last_s = strokes_after_2022[-1]
    print(f'  最早: {mt(first.i0)}')
    print(f'  最晚: {mt(last_s.i1)}')

strokes_2025 = [s for s in strokes if df_merged.index[s.i0].year >= 2025]
print(f'2025年之后的笔: {len(strokes_2025)}')
if strokes_2025:
    for s in strokes_2025:
        d = 'UP' if s.direction == 'up' else 'DN'
        print(f'  {mt(s.i0)}({s.p0:.2f}) -> {mt(s.i1)}({s.p1:.2f}) {d}')

segs_after_2022 = [seg for seg in segments if df_merged.index[seg.i0].year >= 2022]
print(f'2022年之后的线段: {len(segs_after_2022)}')
if segs_after_2022:
    for seg in segs_after_2022:
        d = 'UP' if seg.direction == 'up' else 'DN'
        print(f'  笔{seg.s0}-{seg.s1} | {mt(seg.i0)} -> {mt(seg.i1)} {d} | high={seg.high:.2f} low={seg.low:.2f}')

# ── 保存CSV ──

ratio_daily.to_csv('/Users/silencehan/Projects/NewChanlun/tmp/correct_ratio_pipeline.csv')
print(f'\n已保存 CSV: tmp/correct_ratio_pipeline.csv')
print(f'已保存缓存: GC_BZ_ratio_1min_agg_1day_raw')
