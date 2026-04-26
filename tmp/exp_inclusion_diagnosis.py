"""包含处理 & 分型密度差异诊断实验

目标：找到为什么查缠 PRO 产出 ~1488 分型，而我们只产出 ~300 的根因。
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
assert tv is not None, "缓存数据不存在"
print(f"原始 K 线数: {len(tv)}")
print(f"时间范围: {tv.index[0]} ~ {tv.index[-1]}")
print()

# ── 加载查缠分型数据 ──────────────────────────────────────
csv_path = '/Users/silencehan/Downloads/OANDA_XAUUSD_OANDA_BCOUSD, 1D_9a7eb.csv'
try:
    df_chan = pd.read_csv(csv_path)
    df_chan['datetime'] = pd.to_datetime(df_chan['time'], unit='s')
    df_chan = df_chan.set_index('datetime').sort_index()
    # 查缠 typing 列 — 找非空的 typing 列
    # 先看列名
    print(f"查缠 CSV 列名: {list(df_chan.columns)}")
    # typing 列通常是指标输出，找含 typing 的列或第10列之后
    typing_cols = [c for c in df_chan.columns if 'typing' in str(c).lower()]
    if typing_cols:
        typing_col = df_chan[typing_cols[0]]
    else:
        # 尝试第10列（0-based index 10）
        typing_col = df_chan.iloc[:, 10]
    chan_fractals = typing_col.dropna()
    print(f"查缠分型数: {len(chan_fractals)}")
    print(f"查缠数据时间范围: {df_chan.index[0]} ~ {df_chan.index[-1]}")
    print(f"查缠总行数: {len(df_chan)}")
    HAS_CHAN = True
except Exception as e:
    print(f"加载查缠数据失败: {e}")
    HAS_CHAN = False

print("\n" + "="*70)

# ── 实验0：当前实现（基准） ────────────────────────────────
print("\n## 实验0：当前实现（merge_inclusion + fractals_from_merged）")
merged_std, m2r_std = merge_inclusion(tv)
fractals_std = fractals_from_merged(merged_std)
print(f"  merged bars: {len(merged_std)}")
print(f"  分型数: {len(fractals_std)}")
print(f"  压缩率: {len(tv)} → {len(merged_std)} ({len(merged_std)/len(tv)*100:.1f}%)")

# ── 实验0b：reset_dir_on_fractal=True ──────────────────────
print("\n## 实验0b：当前实现 + reset_dir_on_fractal=True")
merged_reset, m2r_reset = merge_inclusion(tv, reset_dir_on_fractal=True)
fractals_reset = fractals_from_merged(merged_reset)
print(f"  merged bars: {len(merged_reset)}")
print(f"  分型数: {len(fractals_reset)}")

# ── 实验1：完全不做包含处理 ────────────────────────────────
print("\n## 实验1：完全不做包含处理（原始K线直接分型）")
fractals_raw = fractals_from_merged(tv)
print(f"  merged bars: {len(tv)}（无合并）")
print(f"  分型数: {len(fractals_raw)}")

# ── 实验1b：单条件分型（只看 high 或只看 low） ──────────────
print("\n## 实验1b：单条件分型（不做包含，只看 high/low 其一）")

def fractals_single_condition(df):
    """只要 high 形成顶 或 low 形成底就算分型（不要求双条件）"""
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

fractals_single_raw = fractals_single_condition(tv)
print(f"  分型数: {len(fractals_single_raw)}")

# ── 实验1c：单条件 + 允许非严格（>=/<= 代替 >/<） ─────────
print("\n## 实验1c：非严格单条件分型（>=/<= ，不做包含）")

def fractals_nonstrict_single(df):
    """非严格单条件：high >= 或 low <="""
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

fractals_nonstrict = fractals_nonstrict_single(tv)
print(f"  分型数: {len(fractals_nonstrict)}")

# ── 实验2：用 close 做包含判断 ─────────────────────────────
print("\n## 实验2：用 max(O,C)/min(O,C) 做包含判断")

def merge_by_close(df):
    """用 close/open 的实体部分代替 high/low 做包含判断"""
    closes = df['close'].values.astype(np.float64)
    opens = df['open'].values.astype(np.float64)
    highs = df['high'].values.astype(np.float64)
    lows = df['low'].values.astype(np.float64)
    n = len(df)

    buf = [[opens[0], highs[0], lows[0], closes[0], 0, 0]]

    for i in range(1, n):
        last = buf[-1]
        last_upper = max(last[0], last[3])
        last_lower = min(last[0], last[3])
        curr_upper = max(opens[i], closes[i])
        curr_lower = min(opens[i], closes[i])

        has_inclusion = (last_upper >= curr_upper and last_lower <= curr_lower) or \
                       (curr_upper >= last_upper and curr_lower <= last_lower)

        if has_inclusion:
            last[1] = max(last[1], highs[i])
            last[2] = min(last[2], lows[i])
            last[3] = closes[i]
            last[5] = i
        else:
            buf.append([opens[i], highs[i], lows[i], closes[i], i, i])

    out_index = [df.index[int(row[5])] for row in buf]
    arr = np.array([[row[0], row[1], row[2], row[3]] for row in buf], dtype=np.float64)
    merged = pd.DataFrame(arr, columns=['open','high','low','close'], index=out_index)
    return merged

merged_close = merge_by_close(tv)
fractals_close = fractals_from_merged(merged_close)
print(f"  merged bars: {len(merged_close)}")
print(f"  分型数: {len(fractals_close)}")

# ── 实验3：严格不等式包含（> 和 <，不含等号） ──────────────
print("\n## 实验3：严格不等式包含（h1 > h2 AND l1 < l2）")

def merge_strict(df):
    """严格包含：只有 h1 > h2 && l1 < l2 才合并（不含等号）"""
    highs = df['high'].values.astype(np.float64)
    lows = df['low'].values.astype(np.float64)
    opens = df['open'].values.astype(np.float64)
    closes = df['close'].values.astype(np.float64)
    n = len(df)

    buf = [[opens[0], highs[0], lows[0], closes[0], 0, 0]]
    dir_state = None

    for i in range(1, n):
        curr_h, curr_l = highs[i], lows[i]
        last = buf[-1]
        last_h, last_l = last[1], last[2]

        # 严格不等式：> 和 < 代替 >= 和 <=
        has_inclusion = (last_h > curr_h and last_l < curr_l) or \
                       (curr_h > last_h and curr_l < last_l)

        if has_inclusion:
            if dir_state is not None:
                effective_up = dir_state == "UP"
            else:
                effective_up = last[3] >= last[0]
            if effective_up:
                last[1] = max(last_h, curr_h)
                last[2] = max(last_l, curr_l)
            else:
                last[1] = min(last_h, curr_h)
                last[2] = min(last_l, curr_l)
            last[3] = closes[i]
            last[5] = i
        else:
            if curr_h > last_h and curr_l > last_l:
                dir_state = "UP"
            elif curr_h < last_h and curr_l < last_l:
                dir_state = "DOWN"
            buf.append([opens[i], curr_h, curr_l, closes[i], i, i])

    out_index = [df.index[int(row[5])] for row in buf]
    arr = np.array([[row[0], row[1], row[2], row[3]] for row in buf], dtype=np.float64)
    merged = pd.DataFrame(arr, columns=['open','high','low','close'], index=out_index)
    return merged

merged_strict = merge_strict(tv)
fractals_strict = fractals_from_merged(merged_strict)
print(f"  merged bars: {len(merged_strict)}")
print(f"  分型数: {len(fractals_strict)}")

# ── 实验3b：严格不等式包含 + 单条件分型 ──────────────────
print("\n## 实验3b：严格不等式包含 + 单条件分型")
fractals_strict_single = fractals_single_condition(merged_strict)
print(f"  分型数: {len(fractals_strict_single)}")

# ── 实验4：不包含 + 单条件分型 + 最小间距 ──────────────────
print("\n## 实验4：不做包含 + 单条件分型 + 相邻分型间距>=1")

def fractals_with_min_gap(fractals, min_gap=1):
    """过滤掉间距不够的分型"""
    if not fractals:
        return fractals
    result = [fractals[0]]
    for f in fractals[1:]:
        if f.idx - result[-1].idx > min_gap:
            result.append(f)
    return result

fractals_gap1 = fractals_with_min_gap(fractals_single_raw, min_gap=1)
print(f"  分型数(间距>=1): {len(fractals_gap1)}")
fractals_gap0 = fractals_single_raw  # 间距>=0 即无过滤
print(f"  分型数(无间距): {len(fractals_gap0)}")

# ── 实验5：压缩率统计 ──────────────────────────────────────
print("\n## 实验5：包含处理压缩率统计")
raw_per_merged = [end - start + 1 for start, end in m2r_std]
print(f"  merged bars 中包含 1 根 raw 的: {sum(1 for x in raw_per_merged if x == 1)} ({sum(1 for x in raw_per_merged if x == 1)/len(raw_per_merged)*100:.1f}%)")
print(f"  merged bars 中包含 2 根 raw 的: {sum(1 for x in raw_per_merged if x == 2)} ({sum(1 for x in raw_per_merged if x == 2)/len(raw_per_merged)*100:.1f}%)")
print(f"  merged bars 中包含 3+ 根 raw 的: {sum(1 for x in raw_per_merged if x >= 3)} ({sum(1 for x in raw_per_merged if x >= 3)/len(raw_per_merged)*100:.1f}%)")
print(f"  最大合并: {max(raw_per_merged)} 根")
print(f"  平均合并: {np.mean(raw_per_merged):.2f} 根")

# ── 实验6：查缠的分型到底是什么？─────────────────────────
if HAS_CHAN:
    print("\n## 实验6：查缠分型分析")
    print(f"  查缠分型值样例: {chan_fractals.head(10).tolist()}")
    print(f"  查缠分型值分布:")
    val_counts = chan_fractals.value_counts()
    for val, cnt in val_counts.items():
        print(f"    {val}: {cnt}")

# ── 实验7：查缠可能在每个 typing 列都有数据 ───────────────
if HAS_CHAN:
    print("\n## 实验7：查缠所有可能的分型列")
    for i, col in enumerate(df_chan.columns):
        non_null = df_chan[col].dropna()
        if len(non_null) > 100 and len(non_null) < len(df_chan) * 0.8:
            print(f"  col[{i}] '{col}': {len(non_null)} 非空值, 样例: {non_null.head(3).tolist()}")

# ── 汇总表 ────────────────────────────────────────────────
print("\n" + "="*70)
print("\n## 汇总对比表")
print(f"{'方法':<45} {'merged':>7} {'分型数':>6} {'与查缠差距':>10}")
print("-" * 75)

chan_count = len(chan_fractals) if HAS_CHAN else 1488
results = [
    ("0: 当前实现(双条件+宽松包含)", len(merged_std), len(fractals_std)),
    ("0b: 当前+reset_dir_on_fractal", len(merged_reset), len(fractals_reset)),
    ("1: 不包含+双条件", len(tv), len(fractals_raw)),
    ("1b: 不包含+单条件(严格>/<)", len(tv), len(fractals_single_raw)),
    ("1c: 不包含+单条件(非严格>=)", len(tv), len(fractals_nonstrict)),
    ("2: close做包含+双条件", len(merged_close), len(fractals_close)),
    ("3: 严格包含+双条件", len(merged_strict), len(fractals_strict)),
    ("3b: 严格包含+单条件", len(merged_strict), len(fractals_strict_single)),
    ("4: 不包含+单条件+间距>=1", len(tv), len(fractals_gap1)),
]

for name, merged, fcount in results:
    ratio = fcount / chan_count
    print(f"{name:<45} {merged:>7} {fcount:>6} {ratio:>9.2f}x")

print(f"\n查缠基准: {chan_count} 分型")

# ── 如果有查缠数据，做位置重叠分析 ─────────────────────────
if HAS_CHAN:
    print("\n## 位置重叠分析（我们 vs 查缠）")
    # 把我们的分型位置转成日期
    our_dates_std = set()
    for f in fractals_std:
        our_dates_std.add(merged_std.index[f.idx])

    chan_dates = set(chan_fractals.index)

    overlap = our_dates_std & chan_dates
    only_ours = our_dates_std - chan_dates
    only_chan = chan_dates - our_dates_std
    print(f"  我们的分型日期数: {len(our_dates_std)}")
    print(f"  查缠的分型日期数: {len(chan_dates)}")
    print(f"  重叠: {len(overlap)}")
    print(f"  只在我们: {len(only_ours)}")
    print(f"  只在查缠: {len(only_chan)}")
