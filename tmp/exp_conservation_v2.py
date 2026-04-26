import sys; sys.path.insert(0, "src")
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from newchan.cache import load_df

# 加载
base = pd.read_csv('tmp/macro_full_window.csv', index_col=0, parse_dates=True)
gc = load_df('GC_1day_raw')
bz = load_df('BZ_1day_raw')

# 月度黄金和油价
gold_monthly = gc['close'].resample('ME').last()
oil_monthly = bz['close'].resample('ME').last()

# 对齐
common = base.index.intersection(gold_monthly.dropna().index).intersection(oil_monthly.dropna().index)
common = common.sort_values()
M2 = base.loc[common, 'M2']
GDP = base.loc[common, 'GDP']
Gold = gold_monthly.loc[common]
Oil = oil_monthly.loc[common]
L = base.loc[common, 'L']

# 构造量
financial_side = M2 * Gold  # 金融侧总量
real_side = Oil * GDP         # 实体侧总量

# 三个比率
ratio1 = L / GDP                         # 原来的（已知不守恒）
ratio2 = financial_side / real_side       # 新的候选守恒量
ratio3 = (M2 * Gold) / GDP               # 另一个组合

# 变异系数
print("=" * 60)
print("变异系数比较 (CV = std/mean, 越低越接近守恒)")
print("=" * 60)
for name, r in [('L/GDP', ratio1), ('(M2*Gold)/(Oil*GDP)', ratio2), ('(M2*Gold)/GDP', ratio3)]:
    cv = r.std() / r.mean()
    print(f'{name:>25}: cv={cv:.4f} ({cv*100:.1f}%)')

print()
print(f"样本量: {len(common)} 个月 ({common[0].strftime('%Y-%m')} ~ {common[-1].strftime('%Y-%m')})")

# 保存数据
df_out = pd.DataFrame({
    'L_over_GDP': ratio1,
    'M2Gold_over_OilGDP': ratio2,
    'M2Gold_over_GDP': ratio3,
    'M2': M2, 'GDP': GDP, 'Gold': Gold, 'Oil': Oil, 'L': L,
}, index=common)
df_out.to_csv('tmp/conservation_v2.csv')
print("\n数据已保存到 tmp/conservation_v2.csv")

# 画图：三个比率归一化到 z-score
fig, axes = plt.subplots(2, 1, figsize=(14, 10), gridspec_kw={'height_ratios': [3, 1]})

ax = axes[0]
for name, r, color in [
    ('L/GDP', ratio1, '#e74c3c'),
    ('(M2*Gold)/(Oil*GDP)', ratio2, '#2ecc71'),
    ('(M2*Gold)/GDP', ratio3, '#3498db'),
]:
    z = (r - r.mean()) / r.std()
    cv = r.std() / r.mean()
    ax.plot(z.index, z.values, label=f'{name} (CV={cv:.3f})', color=color, linewidth=1.5)

ax.axhline(0, color='gray', linewidth=0.5, linestyle='--')
ax.set_title('Conservation Candidates (z-score normalized)', fontsize=14)
ax.set_ylabel('z-score')
ax.legend(fontsize=11)
ax.grid(True, alpha=0.3)

# 下方：原始比率2的水平
ax2 = axes[1]
ax2.plot(ratio2.index, ratio2.values, color='#2ecc71', linewidth=1.5)
ax2.set_title('(M2 * Gold) / (Oil * GDP) — raw level', fontsize=12)
ax2.set_ylabel('ratio')
ax2.grid(True, alpha=0.3)

plt.tight_layout()
plt.savefig('tmp/conservation_v2.png', dpi=150)
print("图已保存到 tmp/conservation_v2.png")
