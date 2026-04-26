"""
补 2000-2010 黄金/Brent 数据，重跑 L=M*omega 协整检验。

数据源：
  - 黄金月度价格: datahub.io/core/gold-prices (London Fix, USD/oz, 月度)
  - Brent 日线: FRED DCOILBRENTEU -> 月度化
  - M2: FRED M2SL
  - GDP: FRED GDP (季度->月度插值)
  - 2010-2026 黄金/Brent: Databento 缓存 (GC_1day_raw, BZ_1day_raw)
"""

import sys
import warnings

warnings.filterwarnings("ignore")

sys.path.insert(0, "/Users/silencehan/Projects/NewChanlun/src")

import matplotlib
import numpy as np
import pandas as pd

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from scipy import stats
from statsmodels.tsa.stattools import adfuller, coint, grangercausalitytests

OUT_DIR = "/Users/silencehan/Projects/NewChanlun/tmp"


# == 1. 获取 2000-2010 黄金月度数据 (datahub.io) ==

print("=" * 60)
print("步骤1: 获取黄金月度价格 (datahub.io)")
print("=" * 60)

try:
    gold_all = pd.read_csv("https://datahub.io/core/gold-prices/r/monthly.csv")
    gold_all["Date"] = pd.to_datetime(gold_all["Date"])
    gold_all = gold_all.set_index("Date").sort_index()
    gold_all.columns = ["close"]
    gold_old_monthly = gold_all.loc["2000-01":"2010-06", "close"]
    gold_old_monthly.index = gold_old_monthly.index + pd.offsets.MonthEnd(0)
    print(
        f"  黄金(datahub): {len(gold_old_monthly)} 月, "
        f"{gold_old_monthly.index[0].date()} ~ {gold_old_monthly.index[-1].date()}"
    )
    print(
        f"  样本: 2000-01={gold_old_monthly.iloc[0]:.2f}, "
        f"2010-06={gold_old_monthly.iloc[-1]:.2f}"
    )
except Exception as e:
    raise RuntimeError(f"无法获取 datahub 黄金数据: {e}")


# == 2. 获取 2000-2010 Brent 日线 (FRED) -> 月度化 ==

print("\n" + "=" * 60)
print("步骤2: 获取 Brent 日线 (FRED DCOILBRENTEU) -> 月度化")
print("=" * 60)

try:
    brent_fred = pd.read_csv(
        "https://fred.stlouisfed.org/graph/fredgraph.csv"
        "?id=DCOILBRENTEU&cosd=2000-01-01&coed=2010-07-01",
        parse_dates=["observation_date"],
        index_col="observation_date",
    )
    brent_fred.columns = ["close"]
    brent_fred["close"] = pd.to_numeric(brent_fred["close"], errors="coerce")
    brent_fred = brent_fred.dropna()
    print(
        f"  Brent(FRED daily): {len(brent_fred)} 条, "
        f"{brent_fred.index[0].date()} ~ {brent_fred.index[-1].date()}"
    )
    brent_old_monthly = brent_fred["close"].resample("ME").last().dropna()
    print(f"  Brent(月度化): {len(brent_old_monthly)} 月")
except Exception as e:
    raise RuntimeError(f"无法获取 FRED Brent 数据: {e}")


# == 3. 加载 Databento 2010-2026 缓存 -> 月度化 ==

print("\n" + "=" * 60)
print("步骤3: 加载 Databento 缓存 (2010-2026)")
print("=" * 60)

from newchan.cache import load_df

gc_new = load_df("GC_1day_raw")["close"]
bz_new = load_df("BZ_1day_raw")["close"]
print(f"  GC(Databento): {gc_new.index[0].date()} ~ {gc_new.index[-1].date()}")
print(f"  BZ(Databento): {bz_new.index[0].date()} ~ {bz_new.index[-1].date()}")

gc_new_monthly = gc_new.resample("ME").last().dropna()
bz_new_monthly = bz_new.resample("ME").last().dropna()


# == 4. 拼接完整月度序列 ==

print("\n" + "=" * 60)
print("步骤4: 拼接 2000-2026 月度序列")
print("=" * 60)

gc_monthly = pd.concat([gold_old_monthly, gc_new_monthly]).sort_index()
gc_monthly = gc_monthly[~gc_monthly.index.duplicated(keep="last")]

bz_monthly = pd.concat([brent_old_monthly, bz_new_monthly]).sort_index()
bz_monthly = bz_monthly[~bz_monthly.index.duplicated(keep="last")]

print(f"  黄金月度: {len(gc_monthly)} 月, {gc_monthly.index[0].date()} ~ {gc_monthly.index[-1].date()}")
print(f"  Brent月度: {len(bz_monthly)} 月, {bz_monthly.index[0].date()} ~ {bz_monthly.index[-1].date()}")

omega = (gc_monthly / bz_monthly).dropna()
omega.name = "omega"
print(f"  金油比(omega): {len(omega)} 月, {omega.index[0].date()} ~ {omega.index[-1].date()}")
print(f"  omega 均值={omega.mean():.2f}, 当前={omega.iloc[-1]:.2f}")


# == 5. 获取 M2 和 GDP ==

print("\n" + "=" * 60)
print("步骤5: 获取 M2 和 GDP (FRED)")
print("=" * 60)


def read_fred(series_id: str, start: str = "2000-01-01") -> pd.Series:
    url = f"https://fred.stlouisfed.org/graph/fredgraph.csv?id={series_id}&cosd={start}"
    raw = pd.read_csv(url)
    date_col = raw.columns[0]
    val_col = raw.columns[1]
    raw[date_col] = pd.to_datetime(raw[date_col])
    raw = raw.set_index(date_col)
    raw[val_col] = pd.to_numeric(raw[val_col], errors="coerce")
    return raw[val_col].dropna()


m2_raw = read_fred("M2SL")
m2_raw.name = "M2"
m2_monthly = m2_raw.copy()
m2_monthly.index = m2_monthly.index + pd.offsets.MonthEnd(0)
print(f"  M2: {len(m2_monthly)} 月, {m2_monthly.index[0].date()} ~ {m2_monthly.index[-1].date()}")

gdp_raw = read_fred("GDP")
gdp_raw.name = "GDP"
gdp_q = gdp_raw.copy()
gdp_q.index = gdp_q.index + pd.offsets.MonthEnd(0)
gdp_monthly_idx = pd.date_range(start=gdp_q.index[0], end=gdp_q.index[-1], freq="ME")
gdp_monthly = gdp_q.reindex(gdp_monthly_idx).interpolate(method="linear")
gdp_monthly.name = "GDP"
print(
    f"  GDP(月度插值): {len(gdp_monthly)} 月, "
    f"{gdp_monthly.index[0].date()} ~ {gdp_monthly.index[-1].date()}"
)


# == 6. 构建 L = M2 * omega, 合并 ==

print("\n" + "=" * 60)
print("步骤6: 构建完整数据集")
print("=" * 60)

df = pd.DataFrame({"M2": m2_monthly, "omega": omega}).dropna()
df["L"] = df["M2"] * df["omega"]
df["GDP"] = gdp_monthly
df = df.dropna()

print(f"  分析窗口: {df.index[0].date()} ~ {df.index[-1].date()}, N={len(df)} 月")


# == 7. 统计验证 ==

print("\n" + "=" * 60)
print("步骤7: 统计验证")
print("=" * 60)

pearson_r, pearson_p = stats.pearsonr(df["L"], df["GDP"])
spearman_r, spearman_p = stats.spearmanr(df["L"], df["GDP"])
print("\n--- 水平值相关 ---")
print(f"  Pearson  r = {pearson_r:.4f}, p = {pearson_p:.2e}")
print(f"  Spearman rho = {spearman_r:.4f}, p = {spearman_p:.2e}")

dL = df["L"].diff().dropna()
dGDP = df["GDP"].diff().dropna()
aligned = pd.DataFrame({"dL": dL, "dGDP": dGDP}).dropna()
pearson_diff_r, pearson_diff_p = stats.pearsonr(aligned["dL"], aligned["dGDP"])
spearman_diff_r, spearman_diff_p = stats.spearmanr(aligned["dL"], aligned["dGDP"])
print("\n--- 一阶差分相关 ---")
print(f"  Pearson  r = {pearson_diff_r:.4f}, p = {pearson_diff_p:.2e}")
print(f"  Spearman rho = {spearman_diff_r:.4f}, p = {spearman_diff_p:.2e}")

print("\n--- 协整检验 (Engle-Granger) ---")
coint_stat, coint_p, coint_crit = coint(df["L"], df["GDP"])
print(f"  t-statistic = {coint_stat:.4f}")
print(f"  p-value     = {coint_p:.4f}")
print(f"  临界值: 1%={coint_crit[0]:.4f}, 5%={coint_crit[1]:.4f}, 10%={coint_crit[2]:.4f}")
if coint_p < 0.05:
    print("  >>> 5%水平显著: L 和 GDP 存在协整关系")
elif coint_p < 0.10:
    print("  >>> 10%水平显著, 5%水平未达 (边际)")
else:
    print("  >>> 不显著 (p > 0.10)")

df["L_GDP_ratio"] = df["L"] / df["GDP"]
adf_stat, adf_p, _, _, adf_crit, _ = adfuller(df["L_GDP_ratio"].dropna(), autolag="AIC")
print("\n--- ADF检验 (L/GDP 比值平稳性) ---")
print(f"  ADF stat = {adf_stat:.4f}, p = {adf_p:.4f}")
print(f"  临界值: 1%={adf_crit['1%']:.4f}, 5%={adf_crit['5%']:.4f}, 10%={adf_crit['10%']:.4f}")
if adf_p < 0.05:
    print("  >>> L/GDP 比值平稳 (支持协整)")
else:
    print("  >>> L/GDP 比值非平稳")

print("\n--- Granger 因果检验 (maxlag=6) ---")
granger_df = pd.DataFrame({"GDP": df["GDP"], "L": df["L"]}).dropna()
try:
    gc_result = grangercausalitytests(granger_df[["GDP", "L"]], maxlag=6, verbose=False)
    print("  L -> GDP:")
    for lag in [1, 3, 6]:
        f_p = gc_result[lag][0]["ssr_ftest"][1]
        sig = " *" if f_p < 0.05 else ""
        print(f"    lag={lag}: p={f_p:.4f}{sig}")

    gc_result2 = grangercausalitytests(granger_df[["L", "GDP"]], maxlag=6, verbose=False)
    print("  GDP -> L:")
    for lag in [1, 3, 6]:
        f_p = gc_result2[lag][0]["ssr_ftest"][1]
        sig = " *" if f_p < 0.05 else ""
        print(f"    lag={lag}: p={f_p:.4f}{sig}")
except Exception as e:
    print(f"  Granger 失败: {e}")


# == 8. L/GDP 比值分析 ==

print("\n--- L/GDP 比值分析 ---")
mean_ratio = df["L_GDP_ratio"].mean()
std_ratio = df["L_GDP_ratio"].std()
current_ratio = df["L_GDP_ratio"].iloc[-1]
zscore = (current_ratio - mean_ratio) / std_ratio

print(f"  长期均值: {mean_ratio:.2f}")
print(f"  标准差:   {std_ratio:.2f}")
print(f"  当前值:   {current_ratio:.2f}")
print(f"  偏离:     {zscore:+.2f} sigma")

pre_2020 = df.loc[:"2019-12", "L_GDP_ratio"]
post_2020 = df.loc["2020-01":, "L_GDP_ratio"]
print(f"\n  2020前 均值: {pre_2020.mean():.2f} (std={pre_2020.std():.2f})")
print(f"  2020后 均值: {post_2020.mean():.2f} (std={post_2020.std():.2f})")
gap_pct = (post_2020.mean() / pre_2020.mean() - 1) * 100
print(f"  裂口: {gap_pct:+.1f}%")

t_welch, p_welch = stats.ttest_ind(pre_2020, post_2020, equal_var=False)
print(f"  Welch t-test: t={t_welch:.2f}, p={p_welch:.2e}")


# == 9. 子窗口对比 ==

print("\n" + "=" * 60)
print("步骤8: 子窗口对比")
print("=" * 60)

windows = {
    "旧(2010-06~now)": df.loc["2010-06":],
    "全量(2000~now)": df,
    "Pre-COVID(2000~2019)": df.loc[:"2019-12"],
}

results = {}
for name, sub in windows.items():
    if len(sub) < 20:
        continue
    c_stat, c_p, _ = coint(sub["L"], sub["GDP"])
    dL_s = sub["L"].diff().dropna()
    dG_s = sub["GDP"].diff().dropna()
    al = pd.DataFrame({"a": dL_s, "b": dG_s}).dropna()
    r_s, p_s = stats.pearsonr(al["a"], al["b"])
    results[name] = {"N": len(sub), "coint_p": c_p, "diff_r": r_s, "diff_p": p_s}
    print(f"\n  {name} (N={len(sub)}):")
    print(f"    协整 p = {c_p:.4f}")
    print(f"    差分 r = {r_s:.4f}, p = {p_s:.4e}")


# == 10. 图表 ==

print("\n" + "=" * 60)
print("步骤9: 生成图表")
print("=" * 60)

fig, axes = plt.subplots(2, 2, figsize=(16, 12))
fig.suptitle(
    f"L = M2 x omega(Gold/Oil) vs GDP — Extended Window\n"
    f"{df.index[0].date()} ~ {df.index[-1].date()} (N={len(df)})",
    fontsize=14,
    fontweight="bold",
)

ax1 = axes[0, 0]
c_l, c_g = "#1f77b4", "#d62728"
ax1.plot(df.index, df["L"], color=c_l, linewidth=1.5, label="L = M2 x omega")
ax1.set_ylabel("L", color=c_l)
ax1b = ax1.twinx()
ax1b.plot(df.index, df["GDP"], color=c_g, linewidth=1.5, label="GDP")
ax1b.set_ylabel("GDP (B USD)", color=c_g)
for ts in ["2008-09-30", "2020-03-31"]:
    ax1.axvline(pd.Timestamp(ts), color="gray", linestyle="--", alpha=0.5)
ax1.axvline(
    pd.Timestamp("2010-06-30"), color="green", linestyle=":", alpha=0.7, label="Old window start"
)
lines1 = ax1.get_lines() + ax1b.get_lines()
ax1.legend(lines1, [l.get_label() for l in lines1], loc="upper left", fontsize=9)
ax1.set_title("L(t) vs GDP(t)")

ax2 = axes[0, 1]
ax2.plot(df.index, df["L_GDP_ratio"], color="#2ca02c", linewidth=1.5)
ax2.axhline(mean_ratio, color="gray", linestyle="--", alpha=0.7, label=f"mean={mean_ratio:.1f}")
ax2.axhline(mean_ratio + 2 * std_ratio, color="red", linestyle=":", alpha=0.5, label="+-2sigma")
ax2.axhline(mean_ratio - 2 * std_ratio, color="red", linestyle=":", alpha=0.5)
ax2.fill_between(
    df.index, mean_ratio - 2 * std_ratio, mean_ratio + 2 * std_ratio, alpha=0.1, color="red"
)
for ts in ["2008-09-30", "2020-03-31"]:
    ax2.axvline(pd.Timestamp(ts), color="gray", linestyle="--", alpha=0.5)
ax2.set_title(f"L/GDP Ratio (z={zscore:+.2f}sigma)")
ax2.set_ylabel("L/GDP")
ax2.legend(fontsize=9)

ax3 = axes[1, 0]
pre_mask = df.index < "2020-01-01"
post_mask = df.index >= "2020-01-01"
ax3.scatter(
    df.loc[pre_mask, "L"], df.loc[pre_mask, "GDP"], alpha=0.5, s=15, label="Pre-2020", color="#1f77b4"
)
ax3.scatter(
    df.loc[post_mask, "L"],
    df.loc[post_mask, "GDP"],
    alpha=0.5,
    s=15,
    label="Post-2020",
    color="#d62728",
)
slope, intercept, r_val, _, _ = stats.linregress(df["L"], df["GDP"])
x_fit = np.linspace(df["L"].min(), df["L"].max(), 100)
ax3.plot(x_fit, slope * x_fit + intercept, "k--", alpha=0.5, label=f"OLS R2={r_val**2:.3f}")
ax3.set_xlabel("L = M2 x omega")
ax3.set_ylabel("GDP")
ax3.set_title("Scatter: L vs GDP")
ax3.legend(fontsize=9)

ax4 = axes[1, 1]
ax4.axis("off")
rows = [["Window", "N", "Coint p", "Diff r"]]
for name, r in results.items():
    rows.append([name, str(r["N"]), f"{r['coint_p']:.4f}", f"{r['diff_r']:.4f}"])
table = ax4.table(cellText=rows, loc="center", cellLoc="center")
table.auto_set_font_size(False)
table.set_fontsize(11)
table.scale(1.2, 1.8)
ax4.set_title("Window Comparison", fontsize=12, fontweight="bold")

plt.tight_layout()
fig_path = f"{OUT_DIR}/macro_cointegration_extended.png"
plt.savefig(fig_path, dpi=150, bbox_inches="tight")
print(f"  图表已保存: {fig_path}")

csv_path = f"{OUT_DIR}/macro_L_GDP_timeseries_extended.csv"
df.to_csv(csv_path)
print(f"  数据已保存: {csv_path}")


# == 汇总 ==

print("\n" + "=" * 60)
print("汇总对比")
print("=" * 60)
print(
    f"""
  数据源:
    黄金 2000-2010: datahub.io London Gold Fix (月度)
    Brent 2000-2010: FRED DCOILBRENTEU (日线->月度)
    黄金/Brent 2010-2026: Databento GC/BZ (日线->月度)
    M2: FRED M2SL, GDP: FRED GDP

  窗口对比:
"""
)

for name, r in results.items():
    coint_sig = (
        "< 0.05 显著"
        if r["coint_p"] < 0.05
        else ("< 0.10 边际" if r["coint_p"] < 0.10 else "> 0.10 不显著")
    )
    print(f"    {name}: 协整 p={r['coint_p']:.4f} ({coint_sig}), 差分 r={r['diff_r']:.4f}")

print(
    f"""
  L/GDP 比值:
    长期均值 = {mean_ratio:.2f}, 当前 = {current_ratio:.2f}, z = {zscore:+.2f}sigma
    2020裂口 = {gap_pct:+.1f}%
"""
)
