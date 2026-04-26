"""
宏观数据验证：L = M2 × ω(金油比) 与名义GDP的协整关系
独立验证，从头计算。
"""

import sys
import warnings
warnings.filterwarnings("ignore")

sys.path.insert(0, "/Users/silencehan/Projects/NewChanlun/src")

import pandas as pd
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.dates import DateFormatter
from scipy import stats

# ── 1. 获取 M2 和 GDP 数据 ──────────────────────────────────────

print("=" * 60)
print("步骤1: 获取 M2 和 GDP 数据（FRED）")
print("=" * 60)

def read_fred_csv(series_id, start="2000-01-01"):
    """从 FRED 下载 CSV，自动处理列名"""
    url = f"https://fred.stlouisfed.org/graph/fredgraph.csv?id={series_id}&cosd={start}"
    df = pd.read_csv(url)
    # FRED CSV 列名: observation_date + series_id
    date_col = df.columns[0]  # observation_date
    val_col = df.columns[1]   # series_id (e.g. M2SL, GDP)
    df[date_col] = pd.to_datetime(df[date_col])
    df = df.set_index(date_col)
    df[val_col] = pd.to_numeric(df[val_col], errors="coerce")
    df = df.dropna()
    return df

try:
    m2_raw = read_fred_csv("M2SL")
    m2_raw.columns = ["M2"]
    print(f"  M2: {len(m2_raw)} 条月度数据, {m2_raw.index[0].date()} ~ {m2_raw.index[-1].date()}")

    gdp_raw = read_fred_csv("GDP")
    gdp_raw.columns = ["GDP"]
    print(f"  GDP: {len(gdp_raw)} 条季度数据, {gdp_raw.index[0].date()} ~ {gdp_raw.index[-1].date()}")
except Exception as e:
    raise RuntimeError(f"无法获取 FRED 数据: {e}")


# ── 2. 计算金油比 ω(t) ──────────────────────────────────────

print("\n" + "=" * 60)
print("步骤2: 计算金油比 ω(t)")
print("=" * 60)

from newchan.cache import load_df

gc_df = load_df("GC_1day_raw")
bz_df = load_df("BZ_1day_raw")

gc_monthly = gc_df["close"].resample("ME").last()
bz_monthly = bz_df["close"].resample("ME").last()
omega = gc_monthly / bz_monthly
omega = omega.dropna()
omega.name = "omega"

print(f"  金油比(ω): {len(omega)} 个月度数据点")
print(f"  范围: {omega.index[0].date()} ~ {omega.index[-1].date()}")
print(f"  当前值: {omega.iloc[-1]:.2f}")
print(f"  均值: {omega.mean():.2f}, 标准差: {omega.std():.2f}")


# ── 3. 计算 L = M2 × ω ──────────────────────────────────────

print("\n" + "=" * 60)
print("步骤3: 计算 L = M2 × ω")
print("=" * 60)

# 对齐时间索引
# M2 是月初日期，omega 是月末日期 -> 统一到月末
m2_monthly = m2_raw.copy()
m2_monthly.index = m2_monthly.index + pd.offsets.MonthEnd(0)

combined = pd.DataFrame({
    "M2": m2_monthly["M2"],
    "omega": omega,
}).dropna()

combined["L"] = combined["M2"] * combined["omega"]

print(f"  对齐后数据点: {len(combined)}")
print(f"  范围: {combined.index[0].date()} ~ {combined.index[-1].date()}")
print(f"  L 当前值: {combined['L'].iloc[-1]:,.0f}")
print(f"  L 均值: {combined['L'].mean():,.0f}")


# ── 4. GDP 处理（季度→月度插值）──────────────────────────────

print("\n" + "=" * 60)
print("步骤4: GDP 插值到月度")
print("=" * 60)

# GDP 季度数据，线性插值到月度
gdp_quarterly = gdp_raw.copy()
# 对齐到月末
gdp_quarterly.index = gdp_quarterly.index + pd.offsets.MonthEnd(0)

# 创建月度索引并插值
gdp_monthly_idx = pd.date_range(
    start=gdp_quarterly.index[0],
    end=gdp_quarterly.index[-1],
    freq="ME"
)
gdp_monthly = gdp_quarterly.reindex(gdp_monthly_idx).interpolate(method="linear")
gdp_monthly.columns = ["GDP"]

print(f"  GDP 月度化: {len(gdp_monthly)} 个数据点")
print(f"  范围: {gdp_monthly.index[0].date()} ~ {gdp_monthly.index[-1].date()}")


# ── 5. 合并并验证 ──────────────────────────────────────────

print("\n" + "=" * 60)
print("步骤5: 统计验证")
print("=" * 60)

df = pd.DataFrame({
    "L": combined["L"],
    "GDP": gdp_monthly["GDP"],
    "M2": combined["M2"],
    "omega": combined["omega"],
}).dropna()

print(f"\n分析窗口: {df.index[0].date()} ~ {df.index[-1].date()}, 共 {len(df)} 个月度观测")

# --- 5a. 水平值相关 ---
pearson_r, pearson_p = stats.pearsonr(df["L"], df["GDP"])
spearman_r, spearman_p = stats.spearmanr(df["L"], df["GDP"])

print(f"\n--- 水平值相关 ---")
print(f"  Pearson  r = {pearson_r:.4f}, p = {pearson_p:.2e}")
print(f"  Spearman ρ = {spearman_r:.4f}, p = {spearman_p:.2e}")

# --- 5b. 一阶差分相关 ---
dL = df["L"].diff().dropna()
dGDP = df["GDP"].diff().dropna()
aligned = pd.DataFrame({"dL": dL, "dGDP": dGDP}).dropna()

pearson_diff_r, pearson_diff_p = stats.pearsonr(aligned["dL"], aligned["dGDP"])
spearman_diff_r, spearman_diff_p = stats.spearmanr(aligned["dL"], aligned["dGDP"])

print(f"\n--- 一阶差分相关 ---")
print(f"  Pearson  r = {pearson_diff_r:.4f}, p = {pearson_diff_p:.2e}")
print(f"  Spearman ρ = {spearman_diff_r:.4f}, p = {spearman_diff_p:.2e}")

# --- 5c. 协整检验 ---
from statsmodels.tsa.stattools import coint, adfuller, grangercausalitytests

print(f"\n--- 协整检验 (Engle-Granger) ---")
coint_stat, coint_p, coint_crit = coint(df["L"], df["GDP"])
print(f"  t-statistic = {coint_stat:.4f}")
print(f"  p-value     = {coint_p:.4f}")
print(f"  临界值: 1%={coint_crit[0]:.4f}, 5%={coint_crit[1]:.4f}, 10%={coint_crit[2]:.4f}")
if coint_p < 0.05:
    print(f"  ✓ 在5%水平拒绝无协整假设 → L 和 GDP 存在协整关系")
else:
    print(f"  ✗ 无法在5%水平拒绝无协整假设")

# ADF 检验 L/GDP 比值的平稳性
ratio = df["L"] / df["GDP"]
adf_stat, adf_p, _, _, adf_crit, _ = adfuller(ratio.dropna(), autolag="AIC")
print(f"\n--- ADF检验 (L/GDP 比值平稳性) ---")
print(f"  ADF statistic = {adf_stat:.4f}")
print(f"  p-value       = {adf_p:.4f}")
print(f"  临界值: 1%={adf_crit['1%']:.4f}, 5%={adf_crit['5%']:.4f}, 10%={adf_crit['10%']:.4f}")
if adf_p < 0.05:
    print(f"  ✓ L/GDP 比值是平稳的（支持协整）")
else:
    print(f"  ✗ L/GDP 比值非平稳")

# --- 5d. Granger 因果检验 ---
print(f"\n--- Granger 因果检验 (最大滞后=6个月) ---")
granger_data = pd.DataFrame({"GDP": df["GDP"], "L": df["L"]}).dropna()
try:
    gc_result = grangercausalitytests(granger_data[["GDP", "L"]], maxlag=6, verbose=False)
    print(f"  L → GDP 方向:")
    for lag in [1, 3, 6]:
        f_stat = gc_result[lag][0]["ssr_ftest"][0]
        f_p = gc_result[lag][0]["ssr_ftest"][1]
        print(f"    lag={lag}: F={f_stat:.2f}, p={f_p:.4f} {'✓' if f_p < 0.05 else ''}")

    granger_data2 = pd.DataFrame({"L": df["L"], "GDP": df["GDP"]}).dropna()
    gc_result2 = grangercausalitytests(granger_data2[["L", "GDP"]], maxlag=6, verbose=False)
    print(f"  GDP → L 方向:")
    for lag in [1, 3, 6]:
        f_stat = gc_result2[lag][0]["ssr_ftest"][0]
        f_p = gc_result2[lag][0]["ssr_ftest"][1]
        print(f"    lag={lag}: F={f_stat:.2f}, p={f_p:.4f} {'✓' if f_p < 0.05 else ''}")
except Exception as e:
    print(f"  Granger 检验失败: {e}")


# ── 5e. L/GDP 比值分析 ──────────────────────────────────────

print(f"\n--- L/GDP 比值分析 ---")
df["L_GDP_ratio"] = df["L"] / df["GDP"]

mean_ratio = df["L_GDP_ratio"].mean()
std_ratio = df["L_GDP_ratio"].std()
current_ratio = df["L_GDP_ratio"].iloc[-1]
zscore = (current_ratio - mean_ratio) / std_ratio

print(f"  长期均值: {mean_ratio:.2f}")
print(f"  标准差:   {std_ratio:.2f}")
print(f"  当前值:   {current_ratio:.2f}")
print(f"  当前偏离: {zscore:+.2f} 个标准差")

# 2020前后对比
pre_2020 = df.loc[:"2019-12", "L_GDP_ratio"]
post_2020 = df.loc["2020-01":, "L_GDP_ratio"]
print(f"\n  2020前 均值: {pre_2020.mean():.2f} (std={pre_2020.std():.2f})")
print(f"  2020后 均值: {post_2020.mean():.2f} (std={post_2020.std():.2f})")
print(f"  裂口: {post_2020.mean() - pre_2020.mean():.2f} ({(post_2020.mean()/pre_2020.mean() - 1)*100:+.1f}%)")

# Welch t-test
t_stat, t_p = stats.ttest_ind(pre_2020, post_2020, equal_var=False)
print(f"  Welch t-test: t={t_stat:.2f}, p={t_p:.2e}")


# ── 6. 画图 ──────────────────────────────────────────────────

print("\n" + "=" * 60)
print("步骤6: 生成图表")
print("=" * 60)

fig, axes = plt.subplots(2, 2, figsize=(16, 12))
fig.suptitle("L = M2 × ω(Gold/Oil) vs Nominal GDP — Cointegration Verification", fontsize=14, fontweight="bold")

# 图1: L(t) 和 GDP(t) 叠加（双轴）
ax1 = axes[0, 0]
color_l = "#1f77b4"
color_gdp = "#d62728"
ax1.plot(df.index, df["L"], color=color_l, linewidth=1.5, label="L = M2 × ω")
ax1.set_ylabel("L (M2 × Gold/Oil ratio)", color=color_l)
ax1.tick_params(axis="y", labelcolor=color_l)

ax1b = ax1.twinx()
ax1b.plot(df.index, df["GDP"], color=color_gdp, linewidth=1.5, label="Nominal GDP")
ax1b.set_ylabel("GDP (Billions USD)", color=color_gdp)
ax1b.tick_params(axis="y", labelcolor=color_gdp)

ax1.set_title("L(t) vs GDP(t)")
ax1.axvline(pd.Timestamp("2020-03-01"), color="gray", linestyle="--", alpha=0.5, label="COVID-19")
lines1 = ax1.get_lines() + ax1b.get_lines()
ax1.legend(lines1, [l.get_label() for l in lines1], loc="upper left")

# 图2: L/GDP 比值时间序列
ax2 = axes[0, 1]
ax2.plot(df.index, df["L_GDP_ratio"], color="#2ca02c", linewidth=1.5)
ax2.axhline(mean_ratio, color="gray", linestyle="--", alpha=0.7, label=f"Mean={mean_ratio:.1f}")
ax2.axhline(mean_ratio + 2*std_ratio, color="red", linestyle=":", alpha=0.5, label=f"±2σ")
ax2.axhline(mean_ratio - 2*std_ratio, color="red", linestyle=":", alpha=0.5)
ax2.axvline(pd.Timestamp("2020-03-01"), color="gray", linestyle="--", alpha=0.5)
ax2.fill_between(df.index, mean_ratio - 2*std_ratio, mean_ratio + 2*std_ratio, alpha=0.1, color="red")
ax2.set_title(f"L/GDP Ratio (current z-score: {zscore:+.2f}σ)")
ax2.set_ylabel("L/GDP")
ax2.legend()

# 图3: 散点图
ax3 = axes[1, 0]
pre_mask = df.index < "2020-01-01"
post_mask = df.index >= "2020-01-01"
ax3.scatter(df.loc[pre_mask, "L"], df.loc[pre_mask, "GDP"], alpha=0.5, s=15, label="Pre-2020", color="#1f77b4")
ax3.scatter(df.loc[post_mask, "L"], df.loc[post_mask, "GDP"], alpha=0.5, s=15, label="Post-2020", color="#d62728")
# 回归线
slope, intercept, r_val, _, _ = stats.linregress(df["L"], df["GDP"])
x_fit = np.linspace(df["L"].min(), df["L"].max(), 100)
ax3.plot(x_fit, slope * x_fit + intercept, "k--", alpha=0.5, label=f"OLS: R²={r_val**2:.3f}")
ax3.set_xlabel("L = M2 × ω")
ax3.set_ylabel("GDP")
ax3.set_title("Scatter: L vs GDP")
ax3.legend()

# 图4: 一阶差分散点图
ax4 = axes[1, 1]
pre_mask_d = aligned.index < "2020-01-01"
post_mask_d = aligned.index >= "2020-01-01"
ax4.scatter(aligned.loc[pre_mask_d, "dL"], aligned.loc[pre_mask_d, "dGDP"], alpha=0.5, s=15, label="Pre-2020", color="#1f77b4")
ax4.scatter(aligned.loc[post_mask_d, "dL"], aligned.loc[post_mask_d, "dGDP"], alpha=0.5, s=15, label="Post-2020", color="#d62728")
ax4.axhline(0, color="gray", linewidth=0.5)
ax4.axvline(0, color="gray", linewidth=0.5)
ax4.set_xlabel("ΔL")
ax4.set_ylabel("ΔGDP")
ax4.set_title(f"First Differences (Pearson r={pearson_diff_r:.3f})")
ax4.legend()

plt.tight_layout()
plt.savefig("/Users/silencehan/Projects/NewChanlun/tmp/macro_verification.png", dpi=150, bbox_inches="tight")
print("  图表已保存: tmp/macro_verification.png")

# 额外图：组件分解
fig2, axes2 = plt.subplots(3, 1, figsize=(16, 10), sharex=True)
fig2.suptitle("Component Decomposition: M2, ω(Gold/Oil), L", fontsize=14, fontweight="bold")

axes2[0].plot(df.index, df["M2"], color="#1f77b4", linewidth=1.5)
axes2[0].set_ylabel("M2 (Billions USD)")
axes2[0].set_title("M2 Money Supply")
axes2[0].axvline(pd.Timestamp("2020-03-01"), color="gray", linestyle="--", alpha=0.5)

axes2[1].plot(df.index, df["omega"], color="#ff7f0e", linewidth=1.5)
axes2[1].set_ylabel("Gold/Oil Ratio")
axes2[1].set_title("ω = Gold Price / Oil Price")
axes2[1].axvline(pd.Timestamp("2020-03-01"), color="gray", linestyle="--", alpha=0.5)

axes2[2].plot(df.index, df["L"], color="#2ca02c", linewidth=1.5)
axes2[2].set_ylabel("L = M2 × ω")
axes2[2].set_title("Capital Rotation Momentum L")
axes2[2].axvline(pd.Timestamp("2020-03-01"), color="gray", linestyle="--", alpha=0.5)

plt.tight_layout()
plt.savefig("/Users/silencehan/Projects/NewChanlun/tmp/macro_components.png", dpi=150, bbox_inches="tight")
print("  图表已保存: tmp/macro_components.png")


# ── 7. 数据输出 ──────────────────────────────────────────────

# 保存 L/GDP 比值时间序列
output_df = df[["L", "GDP", "L_GDP_ratio", "M2", "omega"]].copy()
output_df.to_csv("/Users/silencehan/Projects/NewChanlun/tmp/macro_L_GDP_timeseries.csv")
print(f"  时间序列已保存: tmp/macro_L_GDP_timeseries.csv")


# ── 汇总 ──────────────────────────────────────────────────────

print("\n" + "=" * 60)
print("汇总")
print("=" * 60)
print(f"""
  数据窗口:     {df.index[0].date()} ~ {df.index[-1].date()} ({len(df)} 个月)

  水平值:
    Pearson r  = {pearson_r:.4f} (p={pearson_p:.2e})
    Spearman ρ = {spearman_r:.4f} (p={spearman_p:.2e})

  一阶差分:
    Pearson r  = {pearson_diff_r:.4f} (p={pearson_diff_p:.2e})
    Spearman ρ = {spearman_diff_r:.4f} (p={spearman_diff_p:.2e})

  协整 (Engle-Granger):
    p-value    = {coint_p:.4f} {'✓ 协整' if coint_p < 0.05 else '✗ 无协整'}

  ADF (L/GDP平稳性):
    p-value    = {adf_p:.4f} {'✓ 平稳' if adf_p < 0.05 else '✗ 非平稳'}

  L/GDP 比值:
    长期均值   = {mean_ratio:.2f}
    当前值     = {current_ratio:.2f}
    偏离       = {zscore:+.2f}σ
    2020裂口   = {(post_2020.mean()/pre_2020.mean() - 1)*100:+.1f}%
""")
