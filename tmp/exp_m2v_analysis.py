"""
M2V（货币流通速度）vs 金油比(ω) 关系分析
假说：M2/GDP上升（纸面膨胀）对应 ω 上升（空转加剧）
"""
import os
import sys
import warnings
warnings.filterwarnings('ignore')

import pandas as pd
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from scipy import stats
from dotenv import load_dotenv

load_dotenv()

# ── 1. 拉 M2V 数据 ──
from fredapi import Fred
fred = Fred(api_key=os.environ['FRED_API_KEY'])

m2v = fred.get_series('M2V', observation_start='2000-01-01')
m2v.name = 'M2V'
print(f'M2V: {len(m2v)} obs, {m2v.index[0].date()} ~ {m2v.index[-1].date()}')

# 1/V = M2/GDP（总纸实比）
paper_real = (1 / m2v).rename('M2_over_GDP')
print(f'M2/GDP range: {paper_real.min():.3f} ~ {paper_real.max():.3f}')

# ── 2. 金油比 ω 月度序列 ──
macro = pd.read_csv('/Users/silencehan/Projects/NewChanlun/tmp/macro_full_window.csv',
                     index_col=0, parse_dates=True)
omega_monthly = macro['omega'].resample('MS').last().dropna()
omega_monthly.name = 'omega'
print(f'omega monthly: {len(omega_monthly)} obs, {omega_monthly.index[0].date()} ~ {omega_monthly.index[-1].date()}')

# ── 3. 对齐（M2V是季度，omega是月度，都resample到季度） ──
omega_q = macro['omega'].resample('QS').mean().dropna()
omega_q.name = 'omega'

# 对齐到共同时间窗口
df = pd.concat([m2v, paper_real, omega_q], axis=1).dropna()
print(f'\n对齐后: {len(df)} 季度观测, {df.index[0].date()} ~ {df.index[-1].date()}')

# ── 4. 相关性分析 ──
# 水平值
r_level, p_level = stats.pearsonr(df['M2_over_GDP'], df['omega'])
r_spearman, p_spearman = stats.spearmanr(df['M2_over_GDP'], df['omega'])

# 一阶差分
d_pr = df['M2_over_GDP'].diff().dropna()
d_om = df['omega'].diff().dropna()
idx_common = d_pr.index.intersection(d_om.index)
r_diff, p_diff = stats.pearsonr(d_pr[idx_common], d_om[idx_common])

print(f'\n=== 相关性 ===')
print(f'水平值 Pearson:  r={r_level:.4f}, p={p_level:.2e}')
print(f'水平值 Spearman: r={r_spearman:.4f}, p={p_spearman:.2e}')
print(f'一阶差分 Pearson: r={r_diff:.4f}, p={p_diff:.2e}')

# ── 5. 交叉相关（谁领先谁） ──
# 用月度数据做交叉相关，更精细
omega_m = macro['omega'].resample('MS').mean().dropna()
# M2V季度 → 插值到月度
m2v_m = m2v.resample('MS').interpolate(method='linear')
paper_real_m = (1 / m2v_m).dropna()

df_m = pd.concat([paper_real_m.rename('M2_over_GDP'), omega_m.rename('omega')], axis=1).dropna()

max_lag = 12  # 最多看12个月
ccf_results = []
for lag in range(-max_lag, max_lag + 1):
    if lag > 0:
        # M2/GDP领先omega lag个月
        x = df_m['M2_over_GDP'].iloc[:-lag].values
        y = df_m['omega'].iloc[lag:].values
    elif lag < 0:
        # omega领先M2/GDP |lag|个月
        x = df_m['M2_over_GDP'].iloc[-lag:].values
        y = df_m['omega'].iloc[:lag].values
    else:
        x = df_m['M2_over_GDP'].values
        y = df_m['omega'].values
    r, p = stats.pearsonr(x, y)
    ccf_results.append({'lag_months': lag, 'correlation': r, 'p_value': p})

ccf_df = pd.DataFrame(ccf_results)
best_lag = ccf_df.loc[ccf_df['correlation'].abs().idxmax()]
print(f'\n=== 交叉相关 ===')
print(f'最强相关在 lag={int(best_lag["lag_months"])} 个月: r={best_lag["correlation"]:.4f}')
if best_lag['lag_months'] > 0:
    print(f'  → M2/GDP 领先 omega {int(best_lag["lag_months"])} 个月')
elif best_lag['lag_months'] < 0:
    print(f'  → omega 领先 M2/GDP {int(-best_lag["lag_months"])} 个月')
else:
    print(f'  → 同步')

# 一阶差分的交叉相关
d_pr_m = df_m['M2_over_GDP'].diff().dropna()
d_om_m = df_m['omega'].diff().dropna()
df_d = pd.concat([d_pr_m, d_om_m], axis=1).dropna()

ccf_diff = []
for lag in range(-max_lag, max_lag + 1):
    if lag > 0:
        x = df_d['M2_over_GDP'].iloc[:-lag].values
        y = df_d['omega'].iloc[lag:].values
    elif lag < 0:
        x = df_d['M2_over_GDP'].iloc[-lag:].values
        y = df_d['omega'].iloc[:lag].values
    else:
        x = df_d['M2_over_GDP'].values
        y = df_d['omega'].values
    n = min(len(x), len(y))
    x, y = x[:n], y[:n]
    if n > 2:
        r, p = stats.pearsonr(x, y)
        ccf_diff.append({'lag_months': lag, 'correlation': r, 'p_value': p})

ccf_diff_df = pd.DataFrame(ccf_diff)
best_lag_d = ccf_diff_df.loc[ccf_diff_df['correlation'].abs().idxmax()]
print(f'\n一阶差分交叉相关:')
print(f'最强相关在 lag={int(best_lag_d["lag_months"])} 个月: r={best_lag_d["correlation"]:.4f}')

# ── 6. 变化率关系 ──
pct_pr = df['M2_over_GDP'].pct_change().dropna()
pct_om = df['omega'].pct_change().dropna()
idx_pct = pct_pr.index.intersection(pct_om.index)
r_pct, p_pct = stats.pearsonr(pct_pr[idx_pct], pct_om[idx_pct])
print(f'\n变化率 Pearson: r={r_pct:.4f}, p={p_pct:.2e}')

# ── 7. 关键事件 ──
events = {
    '2008-09-15': '雷曼倒闭',
    '2014-06-01': '油价崩盘开始',
    '2020-03-15': 'COVID+QE',
    '2021-01-28': '白银轧空',
    '2022-02-24': '俄乌开战',
    '2024-04-14': '伊朗袭击以色列',
    '2025-02-20': 'Regime断点',
}

# ── 8. 画图 ──
fig, axes = plt.subplots(3, 2, figsize=(18, 16))
fig.suptitle('M2V (Monetary Velocity) vs Gold/Oil Ratio (ω)', fontsize=16, fontweight='bold')

# (0,0) 双Y轴时间序列
ax1 = axes[0, 0]
ax2 = ax1.twinx()
ln1 = ax1.plot(df_m.index, df_m['omega'], 'b-', alpha=0.7, linewidth=1.2, label='ω (Gold/Oil)')
ln2 = ax2.plot(df_m.index, df_m['M2_over_GDP'], 'r-', alpha=0.7, linewidth=1.2, label='M2/GDP (1/V)')
ax1.set_ylabel('ω (Gold/Oil Ratio)', color='b')
ax2.set_ylabel('M2/GDP = 1/V', color='r')
# 标注事件
for date_str, label in events.items():
    dt = pd.Timestamp(date_str)
    if dt >= df_m.index[0] and dt <= df_m.index[-1]:
        ax1.axvline(dt, color='gray', alpha=0.4, linestyle='--', linewidth=0.8)
        ax1.text(dt, ax1.get_ylim()[1]*0.95, label, rotation=45, fontsize=7,
                 ha='right', va='top')
lns = ln1 + ln2
labs = [l.get_label() for l in lns]
ax1.legend(lns, labs, loc='upper left', fontsize=8)
ax1.set_title('Time Series: ω vs M2/GDP')

# (0,1) 散点图（水平值）
ax = axes[0, 1]
ax.scatter(df['M2_over_GDP'], df['omega'], alpha=0.6, s=30, c='steelblue')
z = np.polyfit(df['M2_over_GDP'], df['omega'], 1)
p_line = np.poly1d(z)
x_range = np.linspace(df['M2_over_GDP'].min(), df['M2_over_GDP'].max(), 100)
ax.plot(x_range, p_line(x_range), 'r--', linewidth=1.5)
ax.set_xlabel('M2/GDP (1/V)')
ax.set_ylabel('ω (Gold/Oil)')
ax.set_title(f'Scatter: r={r_level:.3f} (p={p_level:.2e})')

# (1,0) 一阶差分散点
ax = axes[1, 0]
ax.scatter(d_pr[idx_common], d_om[idx_common], alpha=0.5, s=25, c='darkgreen')
ax.axhline(0, color='gray', linewidth=0.5)
ax.axvline(0, color='gray', linewidth=0.5)
ax.set_xlabel('Δ(M2/GDP)')
ax.set_ylabel('Δω')
ax.set_title(f'First Difference: r={r_diff:.3f} (p={p_diff:.2e})')

# (1,1) 交叉相关图
ax = axes[1, 1]
ax.bar(ccf_df['lag_months'], ccf_df['correlation'], color='steelblue', alpha=0.7)
ax.axhline(0, color='black', linewidth=0.5)
ax.axhline(2/np.sqrt(len(df_m)), color='red', linestyle='--', linewidth=0.8, alpha=0.5, label='95% CI')
ax.axhline(-2/np.sqrt(len(df_m)), color='red', linestyle='--', linewidth=0.8, alpha=0.5)
ax.set_xlabel('Lag (months, +ve = M2/GDP leads)')
ax.set_ylabel('Cross-correlation')
ax.set_title('Cross-Correlation (Level)')
ax.legend(fontsize=8)

# (2,0) 差分交叉相关
ax = axes[2, 0]
ax.bar(ccf_diff_df['lag_months'], ccf_diff_df['correlation'], color='darkgreen', alpha=0.7)
ax.axhline(0, color='black', linewidth=0.5)
ax.axhline(2/np.sqrt(len(df_d)), color='red', linestyle='--', linewidth=0.8, alpha=0.5)
ax.axhline(-2/np.sqrt(len(df_d)), color='red', linestyle='--', linewidth=0.8, alpha=0.5)
ax.set_xlabel('Lag (months, +ve = M2/GDP leads)')
ax.set_ylabel('Cross-correlation (diff)')
ax.set_title('Cross-Correlation (First Difference)')

# (2,1) M2V 本身 + omega 标准化对比
ax = axes[2, 1]
# 标准化到 0-1
norm_m2v = (m2v - m2v.min()) / (m2v.max() - m2v.min())
norm_omega = (omega_monthly - omega_monthly.min()) / (omega_monthly.max() - omega_monthly.min())
ax.plot(norm_m2v.index, norm_m2v, 'r-', alpha=0.7, linewidth=1.2, label='M2V (normalized)')
ax.plot(norm_omega.index, norm_omega, 'b-', alpha=0.7, linewidth=1.2, label='ω (normalized)')
for date_str, label in events.items():
    dt = pd.Timestamp(date_str)
    if dt >= norm_m2v.index[0]:
        ax.axvline(dt, color='gray', alpha=0.3, linestyle='--', linewidth=0.8)
ax.set_title('Normalized: M2V vs ω (inverse expected)')
ax.legend(fontsize=8)
ax.set_ylabel('Normalized [0,1]')

plt.tight_layout()
plt.savefig('/Users/silencehan/Projects/NewChanlun/tmp/m2v_vs_omega.png', dpi=150, bbox_inches='tight')
print('\n图表已保存: tmp/m2v_vs_omega.png')

# ── 9. 保存数据 ──
out = df_m.copy()
out.to_csv('/Users/silencehan/Projects/NewChanlun/tmp/m2v_vs_omega.csv')
print(f'数据已保存: tmp/m2v_vs_omega.csv ({len(out)} rows)')

# ── 10. 分段分析 ──
print('\n=== 分段相关性 ===')
periods = [
    ('2000-01', '2007-12', 'Pre-GFC'),
    ('2008-01', '2014-12', 'GFC+Recovery'),
    ('2015-01', '2019-12', 'Pre-COVID'),
    ('2020-01', '2022-12', 'COVID+QE'),
    ('2023-01', '2026-12', 'Post-QE'),
]
for start, end, name in periods:
    sub = df.loc[start:end]
    if len(sub) > 3:
        r, p = stats.pearsonr(sub['M2_over_GDP'], sub['omega'])
        print(f'  {name} ({start}~{end}): n={len(sub)}, r={r:.3f}, p={p:.3e}')

# ── 11. 总结 ──
print('\n' + '='*60)
print('=== 结论摘要 ===')
print('='*60)
print(f'1. 水平值相关: r={r_level:.3f} ({"显著" if p_level<0.05 else "不显著"})')
print(f'   M2/GDP↑ {"对应" if r_level>0 else "不对应"} ω↑')
print(f'2. 一阶差分相关: r={r_diff:.3f} ({"显著" if p_diff<0.05 else "不显著"})')
print(f'3. 变化率相关: r={r_pct:.3f} ({"显著" if p_pct<0.05 else "不显著"})')
print(f'4. 交叉相关最强lag: {int(best_lag["lag_months"])}个月 (r={best_lag["correlation"]:.3f})')
print(f'5. 差分交叉相关最强lag: {int(best_lag_d["lag_months"])}个月 (r={best_lag_d["correlation"]:.3f})')

if r_level > 0.5 and p_level < 0.05:
    print('\n→ 假说支持: M2/GDP(纸面膨胀)与ω(金油比/空转)正相关')
    print('  两者可能是同一过程（金融化/空转）的两种度量')
elif r_level < -0.5 and p_level < 0.05:
    print('\n→ 反向关系: M2/GDP↑ 对应 ω↓')
    print('  纸面膨胀期间油的涨幅大于金——可能因为流动性先推实体商品')
elif abs(r_level) < 0.3:
    print('\n→ 关系较弱: 两者不是同一过程的简单映射')
    print('  可能存在非线性关系或 regime-dependent 结构')
else:
    print(f'\n→ 中等相关: 关系存在但不强，可能受 regime 切换影响')
