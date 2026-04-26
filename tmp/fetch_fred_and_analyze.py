"""
用 FRED API 拉取实物经济数据，计算实物金油比指标，重跑守恒检验。
FRED 的伦敦金价和 EIA 原油库存/消费序列已停用，
因此金融金油比从 macro_full_window.csv 的 L 列获取。
实物指标用 FRED 可用序列：工业生产、制造业就业、M2、Fed资产、铜价等。
"""
import os
import sys
import warnings
warnings.filterwarnings('ignore')

import numpy as np
import pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

sys.path.insert(0, '/Users/silencehan/Projects/NewChanlun/src')
os.chdir('/Users/silencehan/Projects/NewChanlun')

from dotenv import load_dotenv
load_dotenv()

from fredapi import Fred

FRED_API_KEY = os.environ['FRED_API_KEY']
fred = Fred(api_key=FRED_API_KEY)

# ============================================================
# 1. 拉取 FRED 数据
# ============================================================
series_map = {
    # 原油价格
    'DCOILBRENTEU': 'Brent Crude Oil Price ($/bbl)',
    'DCOILWTICO':   'WTI Crude Oil Price ($/bbl)',
    # 原油生产（工业生产子项）
    'IPG21112S':    'Industrial Production: Crude Oil Mining',
    # 货币 / 宏观
    'M2SL':         'M2 Money Stock (billions)',
    'WALCL':        'Fed Total Assets (millions)',
    'GDP':          'US GDP (billions)',
    'INDPRO':       'Industrial Production Index',
    'MANEMP':       'Manufacturing Employment (thousands)',
    # 大宗商品
    'PCOPPUSDM':    'Copper Price ($/mt)',
    # 利率
    'DFF':          'Fed Funds Rate',
    'DGS10':        '10-Year Treasury Yield',
    # 通胀
    'CPIAUCSL':     'CPI All Items',
    # 贸易
    'BOPGSTB':      'Trade Balance (billions)',
    # 金融条件
    'TEDRATE':      'TED Spread',
    'T10Y2Y':       '10Y-2Y Spread',
    # 能源消费代理
    'TOTALSA':      'Total Vehicle Sales (millions)',
    'GASREGCOVW':   'Regular Gas Price ($/gallon)',
}

results = {}
for sid, desc in series_map.items():
    try:
        s = fred.get_series(sid, observation_start='1990-01-01')
        s.name = sid
        results[sid] = s
        print(f"  OK: {sid:20s} | {desc:45s} | {len(s):6d} obs | {s.index[0].date()} ~ {s.index[-1].date()}")
        s.to_csv(f'tmp/fred_{sid}.csv', header=True)
    except Exception as e:
        print(f"  FAIL: {sid:20s} | {desc:45s} | {e}")

print(f"\n成功拉取 {len(results)}/{len(series_map)} 个序列\n")

# ============================================================
# 2. 构建月度面板
# ============================================================
def to_monthly(s, method='last'):
    s = s.dropna()
    if method == 'last':
        return s.resample('ME').last().dropna()
    elif method == 'mean':
        return s.resample('ME').mean().dropna()

panel = pd.DataFrame()

# 油价
for sid in ['DCOILBRENTEU', 'DCOILWTICO']:
    if sid in results:
        panel[sid] = to_monthly(results[sid], 'mean')

# M2
if 'M2SL' in results:
    panel['M2'] = to_monthly(results['M2SL'], 'last')

# Fed 资产
if 'WALCL' in results:
    panel['fed_assets'] = to_monthly(results['WALCL'], 'last')

# GDP
if 'GDP' in results:
    gdp = to_monthly(results['GDP'], 'last')
    panel['GDP'] = gdp
    panel['GDP'] = panel['GDP'].ffill()

# 工业生产
if 'INDPRO' in results:
    panel['indpro'] = to_monthly(results['INDPRO'], 'last')

# 原油采掘生产
if 'IPG21112S' in results:
    panel['oil_mining'] = to_monthly(results['IPG21112S'], 'last')

# 制造业就业
if 'MANEMP' in results:
    panel['mfg_emp'] = to_monthly(results['MANEMP'], 'last')

# 铜价
if 'PCOPPUSDM' in results:
    panel['copper'] = to_monthly(results['PCOPPUSDM'], 'last')

# CPI
if 'CPIAUCSL' in results:
    panel['CPI'] = to_monthly(results['CPIAUCSL'], 'last')

# 贸易差额
if 'BOPGSTB' in results:
    panel['trade_bal'] = to_monthly(results['BOPGSTB'], 'last')

# 利率
if 'DFF' in results:
    panel['fed_funds'] = to_monthly(results['DFF'], 'mean')
if 'DGS10' in results:
    panel['treasury_10y'] = to_monthly(results['DGS10'], 'mean')
if 'T10Y2Y' in results:
    panel['yield_spread'] = to_monthly(results['T10Y2Y'], 'mean')

# 汽油价格（能源消费代理）
if 'GASREGCOVW' in results:
    panel['gas_price'] = to_monthly(results['GASREGCOVW'], 'mean')

# 车辆销售
if 'TOTALSA' in results:
    panel['vehicle_sales'] = to_monthly(results['TOTALSA'], 'last')

print(f"面板: {len(panel)} 行, {panel.columns.tolist()}")
print(f"时间范围: {panel.index[0].date()} ~ {panel.index[-1].date()}")
print()

# ============================================================
# 3. 加载已有的 L、GDP 数据并合并
# ============================================================
macro = pd.read_csv('tmp/macro_full_window.csv', index_col=0, parse_dates=True)
print(f"已有 macro: {len(macro)} 行, cols={macro.columns.tolist()}")

merged = panel.join(macro[['L', 'L_over_GDP', 'omega']], how='inner')
print(f"合并后: {len(merged)} 行\n")

# ============================================================
# 4. 派生指标
# ============================================================

# M2/GDP（金融化深度）
if 'M2' in merged.columns and 'GDP' in merged.columns:
    merged['M2_GDP'] = merged['M2'] / merged['GDP']
    print("M2/GDP 计算完成")

# Fed资产/GDP
if 'fed_assets' in merged.columns and 'GDP' in merged.columns:
    merged['fed_GDP'] = merged['fed_assets'] / (merged['GDP'] * 1000)
    print("Fed资产/GDP 计算完成")

# 实际油价（油价/CPI）
if 'DCOILBRENTEU' in merged.columns and 'CPI' in merged.columns:
    merged['real_oil'] = merged['DCOILBRENTEU'] / merged['CPI'] * 100
    print("实际油价 计算完成")

# 铜油比（实物需求代理）
if 'copper' in merged.columns and 'DCOILBRENTEU' in merged.columns:
    merged['copper_oil'] = merged['copper'] / merged['DCOILBRENTEU']
    print("铜油比 计算完成")

# 工业生产/M2（实物/金融比）
if 'indpro' in merged.columns and 'M2' in merged.columns:
    merged['indpro_M2'] = merged['indpro'] / merged['M2']
    print("工业生产/M2 计算完成")

# 原油采掘/工业总产出
if 'oil_mining' in merged.columns and 'indpro' in merged.columns:
    merged['oil_share'] = merged['oil_mining'] / merged['indpro']
    print("原油采掘占比 计算完成")

# omega（金油比）已在 macro 中
# L（金融化指标）已在 macro 中

print()

# ============================================================
# 5. 守恒检验：E = α×L + β×(实物指标)，搜索最小 cv
# ============================================================
def conservation_search(L, phys, name, n_grid=200):
    valid = L.notna() & phys.notna()
    L_v = L[valid].values.astype(float)
    P_v = phys[valid].values.astype(float)

    if len(L_v) < 20:
        print(f"  {name:35s}: 数据不足 ({len(L_v)} obs), 跳过")
        return None

    L_n = L_v / np.nanmean(L_v)
    P_n = P_v / np.nanmean(P_v)

    cv_L = np.std(L_n) / np.abs(np.mean(L_n))

    best_cv = 999
    best_ab = (1.0, 0.0)

    alphas = np.linspace(0.01, 2.0, n_grid)
    betas = np.linspace(-2.0, 2.0, n_grid)

    for a in alphas:
        for b in betas:
            E = a * L_n + b * P_n
            m = np.mean(E)
            if abs(m) < 1e-10:
                continue
            cv = np.std(E) / abs(m)
            if cv < best_cv:
                best_cv = cv
                best_ab = (a, b)

    a_opt, b_opt = best_ab
    improvement = 1 - best_cv / cv_L if cv_L > 0 else 0

    print(f"  {name:35s}: α={a_opt:.3f}, β={b_opt:.3f}, cv(E)={best_cv:.4f}, "
          f"cv(L)={cv_L:.4f}, Δcv={improvement:.1%}, N={len(L_v)}")

    return {
        'name': name,
        'alpha': a_opt, 'beta': b_opt,
        'cv_E': best_cv, 'cv_L': cv_L,
        'improvement': improvement,
        'beta_negative': b_opt < 0,
        'n_obs': len(L_v),
    }

print("=" * 90)
print("守恒检验: E = α×L + β×(实物指标), 搜索使 cv(E) 最小的 α, β")
print("=" * 90)

conservation_results = []

phys_candidates = {
    'indpro':        '工业生产指数',
    'oil_mining':    '原油采掘生产指数',
    'mfg_emp':       '制造业就业',
    'copper':        '铜价',
    'M2_GDP':        'M2/GDP',
    'fed_GDP':       'Fed资产/GDP',
    'real_oil':      '实际油价(油价/CPI)',
    'copper_oil':    '铜油比',
    'indpro_M2':     '工业生产/M2',
    'oil_share':     '原油采掘占比',
    'CPI':           'CPI',
    'trade_bal':     '贸易差额',
    'fed_funds':     '联邦基金利率',
    'treasury_10y':  '10年国债收益率',
    'yield_spread':  '10Y-2Y利差',
    'gas_price':     '汽油价格',
    'vehicle_sales': '车辆销售',
    'DCOILBRENTEU':  '布伦特原油价格',
}

for col, desc in phys_candidates.items():
    if col in merged.columns:
        r = conservation_search(merged['L'], merged[col], desc)
        if r:
            conservation_results.append(r)

# 排序
conservation_results.sort(key=lambda x: x['improvement'], reverse=True)

print("\n--- 按 cv 改善排序 ---")
for r in conservation_results:
    marker = " ***" if r['improvement'] > 0.05 else ""
    print(f"  {r['name']:35s}: Δcv={r['improvement']:+.1%}, β={'<0' if r['beta_negative'] else '≥0'}{marker}")

# 多因子组合
print("\n" + "=" * 90)
print("多因子守恒检验: E = α×L + β1×P1 + β2×P2")
print("=" * 90)

combos = [
    ('indpro', 'M2_GDP', '工业生产 + M2/GDP'),
    ('copper', 'fed_GDP', '铜价 + Fed资产/GDP'),
    ('real_oil', 'indpro_M2', '实际油价 + 工业/M2'),
    ('mfg_emp', 'M2_GDP', '制造业就业 + M2/GDP'),
]

multi_results = []
for c1, c2, name in combos:
    if c1 not in merged.columns or c2 not in merged.columns:
        continue
    valid = merged[['L', c1, c2]].dropna()
    if len(valid) < 20:
        continue

    L_n = (valid['L'] / valid['L'].mean()).values
    P1_n = (valid[c1] / valid[c1].mean()).values
    P2_n = (valid[c2] / valid[c2].mean()).values
    cv_L = np.std(L_n) / np.abs(np.mean(L_n))

    best_cv = 999
    best_params = (1, 0, 0)
    for a in np.linspace(0.01, 2, 80):
        for b1 in np.linspace(-2, 2, 80):
            for b2 in np.linspace(-2, 2, 80):
                E = a * L_n + b1 * P1_n + b2 * P2_n
                m = np.mean(E)
                if abs(m) < 1e-10:
                    continue
                cv = np.std(E) / abs(m)
                if cv < best_cv:
                    best_cv = cv
                    best_params = (a, b1, b2)

    a, b1, b2 = best_params
    impr = 1 - best_cv / cv_L
    print(f"  {name:35s}: α={a:.3f}, β1={b1:.3f}, β2={b2:.3f}, "
          f"cv(E)={best_cv:.4f}, cv(L)={cv_L:.4f}, Δcv={impr:.1%}")
    multi_results.append({'name': name, 'alpha': a, 'beta1': b1, 'beta2': b2,
                          'cv_E': best_cv, 'cv_L': cv_L, 'improvement': impr})

# ============================================================
# 6. 相关性分析
# ============================================================
print("\n" + "=" * 90)
print("相关性: L vs 各指标")
print("=" * 90)

corr_targets = ['L', 'omega', 'indpro', 'M2_GDP', 'fed_GDP', 'real_oil',
                'copper_oil', 'indpro_M2', 'oil_share', 'copper', 'mfg_emp',
                'CPI', 'fed_funds', 'yield_spread']
corr_cols = [c for c in corr_targets if c in merged.columns]
corr = merged[corr_cols].corr()
print("\n与 L 的相关系数:")
for c in corr_cols:
    if c != 'L':
        r = corr.loc['L', c]
        print(f"  {c:20s}: {r:+.3f}")

# ============================================================
# 7. 画图
# ============================================================
fig, axes = plt.subplots(3, 2, figsize=(16, 18))
fig.suptitle('Physical vs Financial Indicators - Conservation Test (FRED Data)',
             fontsize=14, fontweight='bold')

# 7a. L (金融化指标) + 工业生产
ax = axes[0, 0]
ax.plot(merged.index, merged['L'] / merged['L'].mean(), 'b-', label='L (norm)', alpha=0.8)
if 'indpro' in merged.columns:
    ax2 = ax.twinx()
    ax2.plot(merged.index, merged['indpro'], 'r-', label='Industrial Production', alpha=0.7)
    ax2.set_ylabel('INDPRO', color='r')
    ax2.legend(loc='upper right')
ax.set_ylabel('L (normalized)', color='b')
ax.legend(loc='upper left')
ax.set_title('L vs Industrial Production')
ax.grid(True, alpha=0.3)

# 7b. L vs M2/GDP 散点
ax = axes[0, 1]
if 'M2_GDP' in merged.columns:
    ax.scatter(merged['M2_GDP'], merged['L'], alpha=0.4, s=10, c=range(len(merged)), cmap='viridis')
    r = merged['L'].corr(merged['M2_GDP'])
    ax.set_xlabel('M2/GDP')
    ax.set_ylabel('L')
    ax.set_title(f'L vs M2/GDP (corr={r:.3f})')
    ax.grid(True, alpha=0.3)

# 7c. 实际油价 + Fed资产/GDP
ax = axes[1, 0]
if 'real_oil' in merged.columns:
    ax.plot(merged.index, merged['real_oil'], 'g-', label='Real Oil Price', alpha=0.8)
    ax.set_ylabel('Real Oil Price', color='g')
    ax.legend(loc='upper left')
if 'fed_GDP' in merged.columns:
    ax2 = ax.twinx()
    ax2.plot(merged.index, merged['fed_GDP'], 'purple', label='Fed Assets/GDP', alpha=0.7)
    ax2.set_ylabel('Fed Assets/GDP', color='purple')
    ax2.legend(loc='upper right')
ax.set_title('Real Oil Price vs Fed Balance Sheet / GDP')
ax.grid(True, alpha=0.3)

# 7d. 铜油比
ax = axes[1, 1]
if 'copper_oil' in merged.columns:
    ax.plot(merged.index, merged['copper_oil'], 'brown', alpha=0.8, label='Copper/Oil Ratio')
    ax.set_title('Copper/Oil Ratio (Physical Demand Proxy)')
    ax.set_ylabel('Copper/Oil')
    ax.legend()
    ax.grid(True, alpha=0.3)

# 7e. 守恒检验柱状图
ax = axes[2, 0]
if conservation_results:
    names = [r['name'] for r in conservation_results]
    improvements = [r['improvement'] * 100 for r in conservation_results]
    colors = ['green' if imp > 5 else 'steelblue' if imp > 0 else 'salmon'
              for imp in improvements]
    y_pos = range(len(names))
    ax.barh(y_pos, improvements, color=colors, alpha=0.7)
    ax.set_yticks(list(y_pos))
    ax.set_yticklabels(names, fontsize=7)
    ax.set_xlabel('CV Improvement over L alone (%)')
    ax.set_title('Conservation Test: Which physical indicator reduces CV?')
    ax.axvline(0, color='k', linewidth=0.5)
    ax.grid(True, alpha=0.3, axis='x')

# 7f. omega（金油比）时序 + Fed资产
ax = axes[2, 1]
ax.plot(merged.index, merged['omega'], 'b-', label='Gold/Oil Ratio (omega)', alpha=0.8)
ax.set_ylabel('omega', color='b')
ax.legend(loc='upper left')
if 'fed_assets' in merged.columns:
    ax2 = ax.twinx()
    ax2.plot(merged.index, merged['fed_assets'] / 1e6, 'r-', label='Fed Assets ($T)', alpha=0.7)
    ax2.set_ylabel('Fed Assets ($T)', color='r')
    ax2.legend(loc='upper right')
ax.set_title('Gold/Oil Ratio vs Fed Balance Sheet')
ax.grid(True, alpha=0.3)

plt.tight_layout()
plt.savefig('tmp/physical_ratio_fred.png', dpi=150, bbox_inches='tight')
print(f"\n图表已保存: tmp/physical_ratio_fred.png")

# ============================================================
# 8. 保存数据
# ============================================================
merged.to_csv('tmp/physical_ratio_fred.csv')
print(f"数据已保存: tmp/physical_ratio_fred.csv ({len(merged)} 行, {len(merged.columns)} 列)")

cr_df = pd.DataFrame(conservation_results)
cr_df.to_csv('tmp/conservation_test_fred.csv', index=False)
print(f"守恒检验结果: tmp/conservation_test_fred.csv")

if multi_results:
    mr_df = pd.DataFrame(multi_results)
    mr_df.to_csv('tmp/conservation_test_multi_fred.csv', index=False)
    print(f"多因子守恒检验: tmp/conservation_test_multi_fred.csv")

# 摘要
print("\n" + "=" * 90)
print("摘要")
print("=" * 90)
print(f"FRED 拉取: {len(results)} 个序列")
print(f"面板范围: {merged.index[0].date()} ~ {merged.index[-1].date()}, {len(merged)} 个月")
print(f"单因子守恒检验: {len(conservation_results)} 项")
if conservation_results:
    top = conservation_results[0]
    print(f"最佳单因子: {top['name']} (Δcv={top['improvement']:.1%})")
if multi_results:
    best_multi = max(multi_results, key=lambda x: x['improvement'])
    print(f"最佳多因子: {best_multi['name']} (Δcv={best_multi['improvement']:.1%})")

print("\n完成。")
