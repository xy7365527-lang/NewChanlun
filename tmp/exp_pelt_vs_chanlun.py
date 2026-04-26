"""PELT 断点 vs 缠论线段转折点 领先关系验证。

假说：PELT 测加速度（regime change），缠论测位置（结构转折），加速度领先位置。
验证：历史上 PELT 断点是否稳定领先缠论线段端点 2-3 个月。
"""
import sys
from pathlib import Path

sys.path.insert(0, "src")

import numpy as np
import pandas as pd

# ── 安装 ruptures（如需要）──
try:
    import ruptures as rpt
except ImportError:
    import subprocess
    subprocess.run(
        ["/Users/silencehan/Projects/NewChanlun/.venv/bin/pip", "install", "ruptures"],
        check=True,
    )
    import ruptures as rpt

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

from newchan.cache import load_df
from newchan.nested_pipeline import run_nested_search
from newchan.types import Bar

# ================================================================
# 第一步：构造金油比 ω 日线
# ================================================================
gc = load_df("GC_1day_raw")
bz = load_df("BZ_1day_raw")
if gc is None or bz is None:
    print("ERROR: GC 或 BZ 缓存文件不存在")
    sys.exit(1)

idx = gc.index.intersection(bz.index)
gc, bz = gc.loc[idx], bz.loc[idx]
omega = gc["close"] / bz["close"]
print(f"金油比 ω: {len(omega)} 日, 范围 [{omega.min():.2f}, {omega.max():.2f}]")
print(f"时间范围: {omega.index[0].date()} ~ {omega.index[-1].date()}")
print()

# ================================================================
# 第二步：PELT 断点检测（多 pen 值扫描）
# ================================================================
signal = omega.values.reshape(-1, 1)

pen_results = {}
for pen in [3, 5, 10, 15, 20, 30, 50]:
    algo = rpt.Pelt(model="rbf", min_size=20).fit(signal)
    bps = algo.predict(pen=pen)
    # 最后一个断点是序列长度，去掉
    bp_indices = [bp - 1 for bp in bps if bp < len(omega)]
    bp_dates = [omega.index[i] for i in bp_indices]
    pen_results[pen] = bp_dates
    print(f"pen={pen:3d}: {len(bp_dates)} 断点")

# 选择目标 pen：5-15 个断点
best_pen = None
for pen in [10, 15, 20, 5, 30, 3, 50]:
    n = len(pen_results[pen])
    if 5 <= n <= 15:
        best_pen = pen
        break

if best_pen is None:
    # fallback: 选最接近 10 个断点的
    best_pen = min(pen_results, key=lambda p: abs(len(pen_results[p]) - 10))

bp_dates = pen_results[best_pen]
print(f"\n选用 pen={best_pen}, {len(bp_dates)} 个断点:")
for d in bp_dates:
    print(f"  {d.strftime('%Y-%m-%d')}  ω={omega.loc[d]:.2f}")
print()

# ================================================================
# 第三步：缠论管线 → 线段端点
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

# 提取线段端点（每段的起点和终点都是转折点）
turning_points = []
for seg in segments:
    if seg.s0 < len(strokes):
        bar_i0 = strokes[seg.s0].i0
        if bar_i0 < len(bars):
            tp_date = pd.Timestamp(bars[bar_i0].ts)
            tp_val = omega.iloc[bar_i0] if bar_i0 < len(omega) else float("nan")
            # 起点方向：up 段起点是 bottom，down 段起点是 top
            tp_dir = "bottom" if seg.direction == "up" else "top"
            turning_points.append((tp_date, tp_val, tp_dir, seg))

    if seg.s1 < len(strokes):
        bar_i1 = strokes[seg.s1].i1
        if bar_i1 < len(bars):
            tp_date = pd.Timestamp(bars[bar_i1].ts)
            tp_val = omega.iloc[bar_i1] if bar_i1 < len(omega) else float("nan")
            # 终点方向：up 段终点是 top，down 段终点是 bottom
            tp_dir = "top" if seg.direction == "up" else "bottom"
            turning_points.append((tp_date, tp_val, tp_dir, seg))

# 去重（相邻线段共享端点）
seen = set()
unique_tps = []
for tp_date, tp_val, tp_dir, seg in turning_points:
    key = (tp_date, tp_dir)
    if key not in seen:
        seen.add(key)
        unique_tps.append((tp_date, tp_val, tp_dir))

unique_tps.sort(key=lambda x: x[0])
tp_dates_list = [t[0] for t in unique_tps]

print(f"去重后线段端点: {len(unique_tps)} 个")
for d, v, direction in unique_tps[-10:]:
    print(f"  {d.strftime('%Y-%m-%d')}  ω={v:.2f}  {direction}")
print()

# ================================================================
# 第四步：配对 PELT 断点 ↔ 缠论转折
# ================================================================
MAX_WINDOW = 120  # 最大配对窗口（天）

# 4a. 对每个 PELT 断点，向后找最近的缠论转折点
forward_pairs = []
for bp_d in bp_dates:
    future = [(d, v, dr) for d, v, dr in unique_tps if d > bp_d]
    if future:
        nearest = min(future, key=lambda t: (t[0] - bp_d).days)
        lead = (nearest[0] - bp_d).days
        if lead <= MAX_WINDOW:
            forward_pairs.append({
                "pelt_date": bp_d.strftime("%Y-%m-%d"),
                "pelt_omega": round(omega.loc[bp_d], 2),
                "chanlun_date": nearest[0].strftime("%Y-%m-%d"),
                "chanlun_omega": round(nearest[1], 2),
                "turn_dir": nearest[2],
                "lead_days": lead,
                "relation": "PELT领先",
            })

# 4b. 对每个 PELT 断点，向前找最近的缠论转折点（检查滞后情况）
backward_pairs = []
for bp_d in bp_dates:
    past = [(d, v, dr) for d, v, dr in unique_tps if d < bp_d]
    if past:
        nearest = max(past, key=lambda t: t[0])
        lag = (bp_d - nearest[0]).days
        if lag <= MAX_WINDOW:
            backward_pairs.append({
                "pelt_date": bp_d.strftime("%Y-%m-%d"),
                "pelt_omega": round(omega.loc[bp_d], 2),
                "chanlun_date": nearest[0].strftime("%Y-%m-%d"),
                "chanlun_omega": round(nearest[1], 2),
                "turn_dir": nearest[2],
                "lag_days": lag,
                "relation": "PELT滞后",
            })

# 4c. 综合配对：对每个 PELT 断点取时间最近的缠论转折
all_pairs = []
for bp_d in bp_dates:
    candidates = [(d, v, dr) for d, v, dr in unique_tps]
    if not candidates:
        continue
    nearest = min(candidates, key=lambda t: abs((t[0] - bp_d).days))
    delta = (nearest[0] - bp_d).days
    if abs(delta) <= MAX_WINDOW:
        all_pairs.append({
            "pelt_date": bp_d.strftime("%Y-%m-%d"),
            "pelt_omega": round(omega.loc[bp_d], 2),
            "chanlun_date": nearest[0].strftime("%Y-%m-%d"),
            "chanlun_omega": round(nearest[1], 2),
            "turn_dir": nearest[2],
            "delta_days": delta,
            "relation": "PELT领先" if delta > 0 else ("同步" if delta == 0 else "PELT滞后"),
        })

# ================================================================
# 第五步：统计分析
# ================================================================
print("=" * 70)
print("配对结果（PELT断点 → 最近缠论转折, 120天窗口）")
print("=" * 70)
print(f"{'PELT日期':>12} {'ω':>6} {'缠论日期':>12} {'ω':>6} {'方向':>6} {'Δ天':>6} {'关系'}")
print("-" * 70)
for p in all_pairs:
    print(f"{p['pelt_date']:>12} {p['pelt_omega']:>6.2f} "
          f"{p['chanlun_date']:>12} {p['chanlun_omega']:>6.2f} "
          f"{p['turn_dir']:>6} {p['delta_days']:>6d} {p['relation']}")

print()

# 统计
deltas = [p["delta_days"] for p in all_pairs]
leading = [d for d in deltas if d > 0]
lagging = [d for d in deltas if d < 0]

print("=" * 70)
print("统计摘要")
print("=" * 70)
print(f"PELT 断点总数: {len(bp_dates)}")
print(f"成功配对数 (120天内): {len(all_pairs)}")
print(f"  PELT 领先: {len(leading)} 次")
print(f"  PELT 滞后: {len(lagging)} 次")
print(f"  同步: {len([d for d in deltas if d == 0])} 次")
print()

if leading:
    print(f"领先天数统计 (PELT领先缠论):")
    print(f"  均值: {np.mean(leading):.1f} 天")
    print(f"  中位数: {np.median(leading):.1f} 天")
    print(f"  标准差: {np.std(leading):.1f} 天")
    print(f"  范围: [{min(leading)}, {max(leading)}] 天")

if lagging:
    print(f"滞后天数统计 (PELT滞后于缠论):")
    print(f"  均值: {np.mean([-d for d in lagging]):.1f} 天")
    print(f"  中位数: {np.median([-d for d in lagging]):.1f} 天")
    print(f"  范围: [{min(-d for d in lagging)}, {max(-d for d in lagging)}] 天")

if deltas:
    print(f"\n全部 delta 统计:")
    print(f"  均值: {np.mean(deltas):.1f} 天 (正=领先)")
    print(f"  中位数: {np.median(deltas):.1f} 天")

# 领先关系是否稳定（40-90天内）
stable_range = [d for d in leading if 40 <= d <= 90]
print(f"\n领先 40-90 天的配对数: {len(stable_range)} / {len(leading)}")
if leading:
    print(f"占领先总数比例: {len(stable_range)/len(leading)*100:.0f}%")

print()

# ================================================================
# 第六步：多 pen 值对比
# ================================================================
print("=" * 70)
print("不同 pen 值下的断点分布")
print("=" * 70)
for pen in sorted(pen_results.keys()):
    dates = pen_results[pen]
    n = len(dates)
    date_str = ", ".join(d.strftime("%Y-%m-%d") for d in dates[:5])
    suffix = f"... (+{n-5})" if n > 5 else ""
    print(f"  pen={pen:3d}: {n:2d} 断点  [{date_str}{suffix}]")

# ================================================================
# 第七步：可视化
# ================================================================
fig, ax = plt.subplots(figsize=(18, 8))

# 金油比时间序列
ax.plot(omega.index, omega.values, color="steelblue", linewidth=0.8, label="ω (gold/oil ratio)", zorder=1)

# PELT 断点 (垂直线)
for i, d in enumerate(bp_dates):
    label = "PELT breakpoint" if i == 0 else None
    ax.axvline(d, color="red", linewidth=1.5, alpha=0.7, linestyle="--", label=label, zorder=2)

# 缠论转折点
tops = [(d, v) for d, v, dr in unique_tps if dr == "top"]
bottoms = [(d, v) for d, v, dr in unique_tps if dr == "bottom"]

if tops:
    ax.scatter([t[0] for t in tops], [t[1] for t in tops],
               color="darkred", marker="v", s=80, zorder=3, label="Chanlun top")
if bottoms:
    ax.scatter([t[0] for t in bottoms], [t[1] for t in bottoms],
               color="darkgreen", marker="^", s=80, zorder=3, label="Chanlun bottom")

# 配对线（PELT → 缠论）
for p in all_pairs:
    pelt_d = pd.Timestamp(p["pelt_date"])
    chan_d = pd.Timestamp(p["chanlun_date"])
    color = "green" if p["delta_days"] > 0 else "orange"
    ax.annotate("", xy=(chan_d, p["chanlun_omega"]),
                xytext=(pelt_d, p["pelt_omega"]),
                arrowprops=dict(arrowstyle="->", color=color, alpha=0.4, lw=1))

ax.set_title(f"PELT breakpoints vs Chanlun segment turning points (pen={best_pen})", fontsize=14)
ax.set_xlabel("Date")
ax.set_ylabel("ω (gold/oil ratio)")
ax.legend(loc="upper left", fontsize=10)
ax.xaxis.set_major_formatter(mdates.DateFormatter("%Y-%m"))
ax.xaxis.set_major_locator(mdates.MonthLocator(interval=6))
plt.xticks(rotation=45)
ax.grid(True, alpha=0.3)
plt.tight_layout()

out_png = Path("tmp/pelt_vs_chanlun.png")
fig.savefig(out_png, dpi=150)
print(f"\n图表已保存: {out_png}")

# ================================================================
# 第八步：保存 CSV
# ================================================================
df_out = pd.DataFrame(all_pairs)
out_csv = Path("tmp/pelt_vs_chanlun.csv")
df_out.to_csv(out_csv, index=False)
print(f"数据已保存: {out_csv}")

# 额外保存断点和转折点原始数据
bp_df = pd.DataFrame([
    {"date": d.strftime("%Y-%m-%d"), "omega": round(omega.loc[d], 2), "type": "pelt_breakpoint"}
    for d in bp_dates
])
tp_df = pd.DataFrame([
    {"date": d.strftime("%Y-%m-%d"), "omega": round(v, 2), "type": f"chanlun_{dr}"}
    for d, v, dr in unique_tps
])
raw_df = pd.concat([bp_df, tp_df], ignore_index=True).sort_values("date")
raw_df.to_csv("tmp/pelt_vs_chanlun_raw.csv", index=False)
print(f"原始数据已保存: tmp/pelt_vs_chanlun_raw.csv")
