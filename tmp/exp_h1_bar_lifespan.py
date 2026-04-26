"""
H1 bar 寿命分布分析：金油比-GDP 双变量联合 Takens 嵌入 + 滑动窗口持续同调
"""
import numpy as np
import pandas as pd
from ripser import ripser
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

# ── 第一步：加载 & 嵌入 ──
df = pd.read_csv('tmp/macro_full_window.csv', index_col=0, parse_dates=True)
df = df.dropna(subset=['omega', 'GDP'])
omega = df['omega'].values
gdp = df['GDP'].values

n = len(omega)
print(f'数据点数: {n}, 时间范围: {df.index[0]} ~ {df.index[-1]}')

# z-score 归一化
omega_z = (omega - omega.mean()) / omega.std()
gdp_z = (gdp - gdp.mean()) / gdp.std()


def takens_2d(x, y, dim=3, delay=1):
    """双变量 Takens 嵌入"""
    m = len(x) - (dim - 1) * delay
    points = []
    for i in range(m):
        point = []
        for j in range(dim):
            idx = i + j * delay
            point.extend([x[idx], y[idx]])
        points.append(point)
    return np.array(points)


embedded = takens_2d(omega_z, gdp_z, dim=3, delay=1)
print(f'嵌入维度: {embedded.shape}')

dates = df.index[:n]

# ── 第二步：滑动窗口 H1 持续同调 ──
window = 24
step = 1

all_bars = []
max_bars = []

for i in range(window, len(embedded), step):
    cloud = embedded[i - window:i]
    result = ripser(cloud, maxdim=1)
    h1 = result['dgms'][1]

    center_idx = i - window // 2
    center_date = dates[center_idx] if center_idx < len(dates) else dates[-1]

    if len(h1) > 0:
        finite = h1[np.isfinite(h1[:, 1])]
        if len(finite) > 0:
            lifespans = finite[:, 1] - finite[:, 0]
            max_bar_life = lifespans.max()
            max_bars.append({
                'date': center_date,
                'max_lifespan': max_bar_life,
                'n_bars': len(finite),
                'total_persistence': lifespans.sum(),
            })
            for j, (b, d) in enumerate(finite):
                all_bars.append({
                    'birth': b,
                    'death': d,
                    'lifespan': d - b,
                    'window_date': center_date,
                })
        else:
            max_bars.append({'date': center_date, 'max_lifespan': 0, 'n_bars': 0, 'total_persistence': 0})
    else:
        max_bars.append({'date': center_date, 'max_lifespan': 0, 'n_bars': 0, 'total_persistence': 0})

all_bars_df = pd.DataFrame(all_bars)
max_bars_df = pd.DataFrame(max_bars)

# ── 第三步：寿命分布分析 ──
lifespans = all_bars_df['lifespan'].values
print(f'\n=== H1 Bar 寿命分布 ===')
print(f'总 bar 数: {len(lifespans)}')
print(f'均值: {lifespans.mean():.4f}')
print(f'中位数: {np.median(lifespans):.4f}')
print(f'std: {lifespans.std():.4f}')
print(f'最大: {lifespans.max():.4f}')
for pct in [50, 75, 90, 95, 99]:
    print(f'  P{pct}: {np.percentile(lifespans, pct):.4f}')

latest = max_bars_df.iloc[-1]
print(f'\n=== 当前最新窗口 ===')
print(f'日期: {latest["date"]}')
print(f'最长 bar 寿命: {latest["max_lifespan"]:.4f}')
print(f'bar 数量: {latest["n_bars"]}')

percentile = (lifespans < latest['max_lifespan']).sum() / len(lifespans) * 100
print(f'历史百分位: {percentile:.1f}%')

if percentile > 90:
    print('WARNING: 超过历史90%的bar寿命——塌缩临近')
elif percentile > 75:
    print('当前 bar 较长但未到极端')
else:
    print('当前 bar 在正常范围内')

# ── 第四步：画图 ──
fig, axes = plt.subplots(2, 1, figsize=(14, 10))

# 上图：max bar 寿命时间序列
ax1 = axes[0]
ax1.plot(max_bars_df['date'], max_bars_df['max_lifespan'], 'b-', linewidth=0.8, label='Max H1 bar lifespan')
p90 = np.percentile(lifespans, 90)
p95 = np.percentile(lifespans, 95)
ax1.axhline(p90, color='orange', linestyle='--', label=f'P90 = {p90:.4f}')
ax1.axhline(p95, color='red', linestyle='--', label=f'P95 = {p95:.4f}')
# 标注当前值
ax1.scatter([latest['date']], [latest['max_lifespan']], color='red', s=100, zorder=5,
            label=f'Current = {latest["max_lifespan"]:.4f} (P{percentile:.0f})')
ax1.set_title('Max H1 Bar Lifespan over Time (omega-GDP Joint Embedding)')
ax1.set_ylabel('Lifespan (persistence)')
ax1.legend(loc='upper left')
ax1.grid(True, alpha=0.3)

# 下图：寿命分布直方图
ax2 = axes[1]
ax2.hist(lifespans, bins=80, density=True, alpha=0.7, color='steelblue', edgecolor='white')
ax2.axvline(latest['max_lifespan'], color='red', linewidth=2,
            label=f'Current max bar = {latest["max_lifespan"]:.4f}')
ax2.axvline(p90, color='orange', linestyle='--', label=f'P90 = {p90:.4f}')
ax2.axvline(p95, color='red', linestyle='--', label=f'P95 = {p95:.4f}')
ax2.set_title('H1 Bar Lifespan Distribution (All Windows)')
ax2.set_xlabel('Lifespan')
ax2.set_ylabel('Density')
ax2.legend()
ax2.grid(True, alpha=0.3)

plt.tight_layout()
plt.savefig('tmp/h1_bar_lifespan.png', dpi=150)
print(f'\n图表已保存: tmp/h1_bar_lifespan.png')

# ── 第五步：保存数据 ──
max_bars_df.to_csv('tmp/h1_bar_lifespan.csv', index=False)
print(f'数据已保存: tmp/h1_bar_lifespan.csv')

# 输出 top10 最长 bar 的窗口
print(f'\n=== 历史 Top 10 最长 max bar 窗口 ===')
top10 = max_bars_df.nlargest(10, 'max_lifespan')
for _, row in top10.iterrows():
    pct = (lifespans < row['max_lifespan']).sum() / len(lifespans) * 100
    print(f'  {row["date"].strftime("%Y-%m") if hasattr(row["date"], "strftime") else row["date"]}  '
          f'lifespan={row["max_lifespan"]:.4f}  bars={row["n_bars"]:.0f}  P{pct:.0f}')
