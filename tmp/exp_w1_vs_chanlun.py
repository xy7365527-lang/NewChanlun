"""W1 (Wasserstein-1) 拓扑相变检测 vs 缠论线段转折点。

W1 = 相邻滑动窗口的持续同调 H1 barcode 之间的 Wasserstein-1 距离。
Takens 嵌入 → 滑动窗口 → ripser H1 → W1 距离序列 → 峰值检测 → 与缠论配对。

同时保留 PELT 结果做对比。
"""
import sys
import time
from pathlib import Path

sys.path.insert(0, "src")

import numpy as np
import pandas as pd

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

from ripser import ripser
from persim import wasserstein as persim_wasserstein
import ruptures as rpt

from newchan.cache import load_df
from newchan.nested_pipeline import run_nested_search
from newchan.types import Bar

# ================================================================
# 数据准备：金油比 ω
# ================================================================
gc = load_df("GC_1day_raw")
bz = load_df("BZ_1day_raw")
if gc is None or bz is None:
    print("ERROR: GC 或 BZ 缓存文件不存在")
    sys.exit(1)

idx = gc.index.intersection(bz.index)
gc, bz = gc.loc[idx], bz.loc[idx]
omega = gc["close"] / bz["close"]
print(f"金油比 ω: {len(omega)} 日, [{omega.min():.2f}, {omega.max():.2f}]")
print(f"时间范围: {omega.index[0].date()} ~ {omega.index[-1].date()}")
print()

# ================================================================
# 缠论管线 → 线段端点（复用上一脚本逻辑）
# ================================================================
bars = [
    Bar(ts=ts, open=v, high=v, low=v, close=v, volume=0)
    for ts, v in omega.items()
]

results, snap = run_nested_search(bars=bars, stroke_mode="wide", max_levels=6)
if snap is None:
    print("ERROR: 管线返回 None")
    sys.exit(1)

segments = snap.seg_snapshot.segments
strokes = snap.bi_snapshot.strokes
print(f"缠论管线: {len(strokes)} 笔, {len(segments)} 线段")

turning_points = []
for seg in segments:
    if seg.s0 < len(strokes):
        bar_i0 = strokes[seg.s0].i0
        if bar_i0 < len(bars):
            tp_date = pd.Timestamp(bars[bar_i0].ts)
            tp_val = omega.iloc[bar_i0] if bar_i0 < len(omega) else float("nan")
            tp_dir = "bottom" if seg.direction == "up" else "top"
            turning_points.append((tp_date, tp_val, tp_dir))
    if seg.s1 < len(strokes):
        bar_i1 = strokes[seg.s1].i1
        if bar_i1 < len(bars):
            tp_date = pd.Timestamp(bars[bar_i1].ts)
            tp_val = omega.iloc[bar_i1] if bar_i1 < len(omega) else float("nan")
            tp_dir = "top" if seg.direction == "up" else "bottom"
            turning_points.append((tp_date, tp_val, tp_dir))

seen = set()
unique_tps = []
for tp_date, tp_val, tp_dir in turning_points:
    key = (tp_date, tp_dir)
    if key not in seen:
        seen.add(key)
        unique_tps.append((tp_date, tp_val, tp_dir))
unique_tps.sort(key=lambda x: x[0])
print(f"缠论线段端点（去重）: {len(unique_tps)} 个")
print()

# ================================================================
# PELT 断点（保留对比）
# ================================================================
signal = omega.values.reshape(-1, 1)
algo = rpt.Pelt(model="rbf", min_size=20).fit(signal)
pelt_bps = algo.predict(pen=30)
pelt_bp_indices = [bp - 1 for bp in pelt_bps if bp < len(omega)]
pelt_bp_dates = [omega.index[i] for i in pelt_bp_indices]
print(f"PELT 断点 (pen=30): {len(pelt_bp_dates)} 个")

# ================================================================
# W1 计算：Takens 嵌入 + 滑动窗口 + H1 + Wasserstein-1
# ================================================================
def takens_embedding(x: np.ndarray, dim: int = 3, delay: int = 5) -> np.ndarray:
    """Takens 延迟嵌入。"""
    n = len(x) - (dim - 1) * delay
    return np.array([x[i:i + dim * delay:delay] for i in range(n)])


def compute_w1_series(
    values: np.ndarray,
    window: int = 120,
    dim: int = 3,
    delay: int = 5,
    subsample: int = 1,
) -> np.ndarray:
    """计算相邻滑动窗口 H1 barcode 的 W1 距离序列。

    subsample: 每隔 N 个窗口计算一次（加速用，1=不跳过）
    """
    embedded = takens_embedding(values, dim=dim, delay=delay)
    n_windows = len(embedded) - window
    print(f"Takens 嵌入: {len(embedded)} 点 (dim={dim}, delay={delay})")
    print(f"滑动窗口数: {n_windows} (window={window})")

    w1_values = np.full(n_windows, np.nan)
    prev_dgm = None

    t0 = time.time()
    computed = 0
    for i in range(n_windows):
        if i % subsample != 0 and i > 0:
            continue

        cloud = embedded[i:i + window]
        result = ripser(cloud, maxdim=1)
        dgm_h1 = result['dgms'][1]

        # 清理无穷值
        if dgm_h1.shape[0] > 0:
            finite_mask = np.isfinite(dgm_h1).all(axis=1)
            dgm_h1 = dgm_h1[finite_mask]

        if prev_dgm is not None:
            if dgm_h1.shape[0] == 0 and prev_dgm.shape[0] == 0:
                w1_values[i] = 0.0
            else:
                w1_values[i] = persim_wasserstein(prev_dgm, dgm_h1)

        prev_dgm = dgm_h1
        computed += 1

        if computed % 200 == 0:
            elapsed = time.time() - t0
            rate = computed / elapsed
            remaining = (n_windows // subsample - computed) / rate
            print(f"  进度: {computed}/{n_windows // subsample} "
                  f"({elapsed:.0f}s, 剩余约 {remaining:.0f}s)")

    elapsed = time.time() - t0
    print(f"W1 计算完成: {computed} 窗口, {elapsed:.1f}s")

    # 插值填充跳过的位置
    if subsample > 1:
        valid = ~np.isnan(w1_values)
        if valid.any():
            indices = np.arange(len(w1_values))
            w1_values = np.interp(indices, indices[valid], w1_values[valid])

    return w1_values


print()
print("=" * 70)
print("计算 W1 距离序列（Takens dim=3, delay=5, window=120）")
print("=" * 70)

omega_values = omega.values.astype(np.float64)
# 标准化以稳定 ripser 数值
omega_norm = (omega_values - omega_values.mean()) / omega_values.std()

w1_series = compute_w1_series(omega_norm, window=120, dim=3, delay=5, subsample=1)

# W1 序列对应的日期索引
# Takens 嵌入后长度 = len(omega) - (3-1)*5 = len(omega) - 10
# 滑动窗口后起始位置 = window = 120
# 所以 w1_series[i] 对应 omega.index[i + 120 + 10 - 1] 附近
# 更精确：embedded[i:i+window] 对应原始序列 [i, i + window + (dim-1)*delay - 1]
# w1_series[i] 对应 embedded 窗口结束位置 i+window-1
# 对应原始序列位置 i + window - 1 + (dim-1)*delay = i + 119 + 10 = i + 129
# 用窗口中心更合理：i + window//2 + (dim-1)*delay//2
offset = 120 + (3 - 1) * 5  # = 130，W1[0] 对应 omega.index[130] 开始
w1_dates = omega.index[offset:offset + len(w1_series)]

# 截断到实际长度（可能因边界少几个）
min_len = min(len(w1_dates), len(w1_series))
w1_dates = w1_dates[:min_len]
w1_series = w1_series[:min_len]

# 去掉 NaN
valid_mask = ~np.isnan(w1_series)
w1_dates_valid = w1_dates[valid_mask]
w1_valid = w1_series[valid_mask]

print(f"\nW1 序列: {len(w1_valid)} 有效值")
print(f"W1 范围: [{w1_valid.min():.4f}, {w1_valid.max():.4f}]")
print(f"W1 均值: {w1_valid.mean():.4f}, std: {w1_valid.std():.4f}")

# ================================================================
# W1 峰值检测
# ================================================================
w1_mean = w1_valid.mean()
w1_std = w1_valid.std()

# 多阈值扫描
for sigma in [1.5, 2.0, 2.5, 3.0]:
    threshold = w1_mean + sigma * w1_std
    peaks = w1_valid > threshold
    n_peaks = peaks.sum()
    print(f"  σ={sigma}: 阈值={threshold:.4f}, 超过阈值点数={n_peaks}")

# 用 2σ 做主要检测，然后对连续超阈值区间取最大值
threshold_2sigma = w1_mean + 2.0 * w1_std
above = w1_valid > threshold_2sigma

# 找连续超阈值区间的峰值
raw_peak_dates = []
raw_peak_values = []
i = 0
while i < len(above):
    if above[i]:
        j = i
        while j < len(above) and above[j]:
            j += 1
        segment = w1_valid[i:j]
        max_pos = i + np.argmax(segment)
        raw_peak_dates.append(w1_dates_valid[max_pos])
        raw_peak_values.append(w1_valid[max_pos])
        i = j
    else:
        i += 1

# 合并 30 天内的峰值簇：每个簇取 W1 最大的点
MERGE_GAP = 30  # 天
peak_dates = []
peak_values = []
if raw_peak_dates:
    cluster_dates = [raw_peak_dates[0]]
    cluster_vals = [raw_peak_values[0]]
    for d, v in zip(raw_peak_dates[1:], raw_peak_values[1:]):
        if (d - cluster_dates[-1]).days <= MERGE_GAP:
            cluster_dates.append(d)
            cluster_vals.append(v)
        else:
            # 结算当前簇
            best = int(np.argmax(cluster_vals))
            peak_dates.append(cluster_dates[best])
            peak_values.append(cluster_vals[best])
            cluster_dates = [d]
            cluster_vals = [v]
    # 最后一个簇
    best = int(np.argmax(cluster_vals))
    peak_dates.append(cluster_dates[best])
    peak_values.append(cluster_vals[best])

print(f"\nW1 峰值 (2σ, 30天合并): {len(peak_dates)} 个 (合并前 {len(raw_peak_dates)} 个)")
for d, v in zip(peak_dates, peak_values):
    omega_val = omega.loc[d] if d in omega.index else float("nan")
    print(f"  {d.strftime('%Y-%m-%d')}  W1={v:.4f}  ω={omega_val:.2f}")

# ================================================================
# 配对分析：W1 峰值 ↔ 缠论转折
# ================================================================
MAX_WINDOW = 120

print()
print("=" * 70)
print("W1 峰值 → 缠论转折点配对 (120天窗口)")
print("=" * 70)

w1_pairs = []
for bp_d, bp_w1 in zip(peak_dates, peak_values):
    candidates = [(d, v, dr) for d, v, dr in unique_tps]
    if not candidates:
        continue
    nearest = min(candidates, key=lambda t: abs((t[0] - bp_d).days))
    delta = (nearest[0] - bp_d).days
    if abs(delta) <= MAX_WINDOW:
        omega_at_bp = omega.loc[bp_d] if bp_d in omega.index else float("nan")
        w1_pairs.append({
            "w1_date": bp_d.strftime("%Y-%m-%d"),
            "w1_value": round(bp_w1, 4),
            "w1_omega": round(omega_at_bp, 2),
            "chanlun_date": nearest[0].strftime("%Y-%m-%d"),
            "chanlun_omega": round(nearest[1], 2),
            "turn_dir": nearest[2],
            "delta_days": delta,
            "relation": "W1领先" if delta > 0 else ("同步" if delta == 0 else "W1滞后"),
        })

print(f"{'W1日期':>12} {'W1值':>8} {'ω':>6} {'缠论日期':>12} {'ω':>6} {'方向':>6} {'Δ天':>6} {'关系'}")
print("-" * 75)
for p in w1_pairs:
    print(f"{p['w1_date']:>12} {p['w1_value']:>8.4f} {p['w1_omega']:>6.2f} "
          f"{p['chanlun_date']:>12} {p['chanlun_omega']:>6.2f} "
          f"{p['turn_dir']:>6} {p['delta_days']:>6d} {p['relation']}")

# ================================================================
# W1 vs PELT 对比
# ================================================================
print()
print("=" * 70)
print("W1 峰值 vs PELT 断点对比")
print("=" * 70)

print(f"{'W1 峰值日期':>14} {'W1值':>8} {'最近PELT日期':>14} {'Δ天':>6} {'一致性'}")
print("-" * 55)
for bp_d, bp_w1 in zip(peak_dates, peak_values):
    if pelt_bp_dates:
        nearest_pelt = min(pelt_bp_dates, key=lambda t: abs((t - bp_d).days))
        delta_pelt = (nearest_pelt - bp_d).days
        consistent = "一致" if abs(delta_pelt) <= 30 else "不一致"
    else:
        nearest_pelt = None
        delta_pelt = float("nan")
        consistent = "N/A"
    print(f"{bp_d.strftime('%Y-%m-%d'):>14} {bp_w1:>8.4f} "
          f"{nearest_pelt.strftime('%Y-%m-%d') if nearest_pelt else 'N/A':>14} "
          f"{delta_pelt:>6d} {consistent}")

# ================================================================
# 统计摘要
# ================================================================
print()
print("=" * 70)
print("统计摘要")
print("=" * 70)

deltas_w1 = [p["delta_days"] for p in w1_pairs]
leading_w1 = [d for d in deltas_w1 if d > 0]
lagging_w1 = [d for d in deltas_w1 if d < 0]

print(f"W1 峰值总数: {len(peak_dates)}")
print(f"成功配对数 (120天内): {len(w1_pairs)}")
print(f"  W1 领先: {len(leading_w1)} 次")
print(f"  W1 滞后: {len(lagging_w1)} 次")
print(f"  同步: {len([d for d in deltas_w1 if d == 0])} 次")

if leading_w1:
    print(f"\n领先天数统计:")
    print(f"  均值: {np.mean(leading_w1):.1f} 天")
    print(f"  中位数: {np.median(leading_w1):.1f} 天")
    print(f"  标准差: {np.std(leading_w1):.1f} 天")
    print(f"  范围: [{min(leading_w1)}, {max(leading_w1)}] 天")

if lagging_w1:
    print(f"\n滞后天数统计:")
    lag_abs = [-d for d in lagging_w1]
    print(f"  均值: {np.mean(lag_abs):.1f} 天")
    print(f"  中位数: {np.median(lag_abs):.1f} 天")
    print(f"  范围: [{min(lag_abs)}, {max(lag_abs)}] 天")

if deltas_w1:
    print(f"\n全部 delta 统计:")
    print(f"  均值: {np.mean(deltas_w1):.1f} 天 (正=领先)")
    print(f"  中位数: {np.median(deltas_w1):.1f} 天")

    stable_40_90 = [d for d in leading_w1 if 40 <= d <= 90]
    print(f"\n领先 40-90 天的配对数: {len(stable_40_90)} / {len(leading_w1) if leading_w1 else 0}")

# W1 vs PELT 一致性统计
consistent_count = 0
for bp_d in peak_dates:
    if pelt_bp_dates:
        nearest_pelt = min(pelt_bp_dates, key=lambda t: abs((t - bp_d).days))
        if abs((nearest_pelt - bp_d).days) <= 30:
            consistent_count += 1
print(f"\nW1 与 PELT 一致 (30天内): {consistent_count}/{len(peak_dates)}")

# ================================================================
# 可视化
# ================================================================
fig, axes = plt.subplots(3, 1, figsize=(18, 14), sharex=True,
                          gridspec_kw={"height_ratios": [3, 2, 1]})

# --- 子图1: 金油比 + 转折点 + W1峰值 + PELT断点 ---
ax1 = axes[0]
ax1.plot(omega.index, omega.values, color="steelblue", linewidth=0.8,
         label="ω (gold/oil ratio)", zorder=1)

# PELT 断点
for i, d in enumerate(pelt_bp_dates):
    label = "PELT breakpoint" if i == 0 else None
    ax1.axvline(d, color="red", linewidth=1.0, alpha=0.4, linestyle="--",
                label=label, zorder=2)

# W1 峰值
for i, (d, v) in enumerate(zip(peak_dates, peak_values)):
    label = "W1 peak" if i == 0 else None
    omega_val = omega.loc[d] if d in omega.index else omega.iloc[omega.index.get_indexer([d], method="nearest")[0]]
    ax1.axvline(d, color="purple", linewidth=1.5, alpha=0.7, linestyle="-.",
                label=label, zorder=3)

# 缠论转折
tops = [(d, v) for d, v, dr in unique_tps if dr == "top"]
bottoms = [(d, v) for d, v, dr in unique_tps if dr == "bottom"]
if tops:
    ax1.scatter([t[0] for t in tops], [t[1] for t in tops],
                color="darkred", marker="v", s=60, zorder=4, label="Chanlun top")
if bottoms:
    ax1.scatter([t[0] for t in bottoms], [t[1] for t in bottoms],
                color="darkgreen", marker="^", s=60, zorder=4, label="Chanlun bottom")

ax1.set_ylabel("ω (gold/oil ratio)")
ax1.set_title("W1 peaks vs PELT breakpoints vs Chanlun turning points", fontsize=14)
ax1.legend(loc="upper left", fontsize=9)
ax1.grid(True, alpha=0.3)

# --- 子图2: W1 距离序列 ---
ax2 = axes[1]
ax2.plot(w1_dates_valid, w1_valid, color="purple", linewidth=0.6, label="W1 distance")
ax2.axhline(threshold_2sigma, color="orange", linewidth=1, linestyle="--",
            label=f"2σ threshold ({threshold_2sigma:.4f})")
ax2.axhline(w1_mean, color="gray", linewidth=0.5, linestyle=":",
            label=f"mean ({w1_mean:.4f})")

# 标注峰值
ax2.scatter(peak_dates, peak_values, color="red", s=50, zorder=5, label="W1 peaks")

# 缠论转折点垂直线
for i, (d, v, dr) in enumerate(unique_tps):
    label = "Chanlun turning point" if i == 0 else None
    ax2.axvline(d, color="green", linewidth=0.5, alpha=0.3, label=label)

ax2.set_ylabel("W1 distance")
ax2.legend(loc="upper left", fontsize=9)
ax2.grid(True, alpha=0.3)

# --- 子图3: W1 峰值 → 缠论配对的 delta 天数 ---
ax3 = axes[2]
if w1_pairs:
    pair_dates = [pd.Timestamp(p["w1_date"]) for p in w1_pairs]
    pair_deltas = [p["delta_days"] for p in w1_pairs]
    colors = ["green" if d > 0 else "orange" for d in pair_deltas]
    ax3.bar(pair_dates, pair_deltas, color=colors, width=20, alpha=0.7)
    ax3.axhline(0, color="black", linewidth=0.5)
    ax3.set_ylabel("Lead days (+) / Lag days (-)")
    ax3.set_xlabel("Date")
    ax3.grid(True, alpha=0.3)

ax3.xaxis.set_major_formatter(mdates.DateFormatter("%Y-%m"))
ax3.xaxis.set_major_locator(mdates.MonthLocator(interval=12))
plt.xticks(rotation=45)
plt.tight_layout()

out_png = Path("tmp/w1_vs_chanlun.png")
fig.savefig(out_png, dpi=150)
print(f"\n图表已保存: {out_png}")

# ================================================================
# 保存 CSV
# ================================================================
df_out = pd.DataFrame(w1_pairs)
out_csv = Path("tmp/w1_vs_chanlun.csv")
df_out.to_csv(out_csv, index=False)
print(f"配对数据已保存: {out_csv}")

# W1 时间序列也保存
w1_df = pd.DataFrame({"date": w1_dates_valid, "w1": w1_valid})
w1_df.to_csv("tmp/w1_series.csv", index=False)
print(f"W1 序列已保存: tmp/w1_series.csv")
