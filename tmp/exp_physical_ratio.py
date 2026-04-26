"""
实物金油比 vs 金融金油比——度量 $ 体系的扭曲度

数据策略：
- 金融金油比：已有 GC/BZ 日线 parquet
- 实物代理：
  1. GLD ETF 持仓量（黄金实物需求代理）— yfinance
  2. 美国原油库存（EIA 周度）— FRED CSV endpoint（无需 API key）
  3. 如果 FRED 不可用，用 USO ETF volume 作为原油实物需求代理
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from pathlib import Path
import warnings
warnings.filterwarnings("ignore")

CACHE = Path("/Users/silencehan/Projects/NewChanlun/.cache")
OUT_DIR = Path("/Users/silencehan/Projects/NewChanlun/tmp")

# ── 1. 金融金油比（月度） ──────────────────────────────────────

gc = pd.read_parquet(CACHE / "GC_1day_raw.parquet")
bz = pd.read_parquet(CACHE / "BZ_1day_raw.parquet")

# 对齐日期
common_idx = gc.index.intersection(bz.index)
financial_ratio = gc.loc[common_idx, "close"] / bz.loc[common_idx, "close"]
financial_ratio.name = "financial_ratio"

# 月度重采样
fin_monthly = financial_ratio.resample("ME").mean()
print(f"金融金油比：{fin_monthly.index[0].date()} ~ {fin_monthly.index[-1].date()}, {len(fin_monthly)} 月")


# ── 2. 实物代理数据 ────────────────────────────────────────────

# 2a. GLD ETF 持仓量 — 用 yfinance 拉 GLD 的 volume 作为代理
#     （GLD 的 AUM/shares outstanding 更好，但 yfinance 只有 volume）
import yfinance as yf

def fetch_yf(ticker, name):
    """从 yfinance 拉月度数据"""
    try:
        data = yf.download(ticker, start="2010-01-01", progress=False, auto_adjust=True)
        if data.empty:
            print(f"  [WARN] {name} ({ticker}): 空数据")
            return None
        # 处理可能的 MultiIndex columns
        if isinstance(data.columns, pd.MultiIndex):
            data.columns = data.columns.get_level_values(0)
        monthly = data["Close"].resample("ME").mean()
        monthly.name = name
        print(f"  {name}: {monthly.index[0].date()} ~ {monthly.index[-1].date()}, {len(monthly)} 月")
        return monthly
    except Exception as e:
        print(f"  [ERR] {name} ({ticker}): {e}")
        return None

print("\n拉取代理数据...")
gld_close = fetch_yf("GLD", "GLD_close")       # 黄金 ETF 价格
uso_close = fetch_yf("USO", "USO_close")       # 原油 ETF 价格

# 2b. 尝试 FRED CSV（无需 API key）— 美国原油库存
def fetch_fred_csv(series_id, name):
    """从 FRED fredgraph CSV endpoint 拉数据（无需 key）"""
    import urllib.request
    url = f"https://fred.stlouisfed.org/graph/fredgraph.csv?id={series_id}"
    try:
        local_path = OUT_DIR / f"_fred_{series_id}.csv"
        urllib.request.urlretrieve(url, local_path)
        df = pd.read_csv(local_path, parse_dates=["DATE"], index_col="DATE")
        col = df.columns[0]
        # FRED 用 "." 表示缺失
        df[col] = pd.to_numeric(df[col], errors="coerce")
        df = df.dropna()
        monthly = df[col].resample("ME").mean()
        monthly.name = name
        print(f"  {name} (FRED {series_id}): {monthly.index[0].date()} ~ {monthly.index[-1].date()}, {len(monthly)} 月")
        return monthly
    except Exception as e:
        print(f"  [ERR] {name} (FRED {series_id}): {e}")
        return None

print("\n尝试 FRED 数据...")
# WTTSTUS1 = 美国原油库存（周度，千桶）
crude_stock = fetch_fred_csv("WTTSTUS1", "US_crude_stock")
# GVZCLS = CBOE Gold Volatility (备用)
# 尝试全球原油消费
world_oil = fetch_fred_csv("MCOILWTICLS", "WTI_price_fred")


# ── 3. 构建实物金油比代理 ──────────────────────────────────────

# 策略：GLD/USO 价格比 作为实物金油比的代理
# 理由：GLD 跟踪实物金价（含持仓成本），USO 跟踪油价（含展期成本）
#        两者的比值反映 ETF 市场中金油的相对定价，包含了实物溢价/折价信息

physical_proxy = None
if gld_close is not None and uso_close is not None:
    common = gld_close.index.intersection(uso_close.index)
    physical_proxy = gld_close.loc[common] / uso_close.loc[common]
    physical_proxy.name = "physical_proxy_ratio"
    print(f"\n实物代理金油比（GLD/USO）：{common[0].date()} ~ {common[-1].date()}, {len(common)} 月")

# 备用：如果有 FRED 原油库存，构建另一个代理
crude_proxy = None
if crude_stock is not None and gld_close is not None:
    common2 = crude_stock.index.intersection(gld_close.index)
    if len(common2) > 12:
        # 黄金价格 / 原油库存（归一化）——库存高=供过于求=油便宜
        crude_norm = crude_stock.loc[common2] / crude_stock.loc[common2].mean()
        gld_norm = gld_close.loc[common2] / gld_close.loc[common2].mean()
        crude_proxy = gld_norm / crude_norm
        crude_proxy.name = "crude_stock_proxy"
        print(f"库存代理金油比（GLD_norm/库存_norm）：{common2[0].date()} ~ {common2[-1].date()}")


# ── 4. 计算扭曲度 ─────────────────────────────────────────────

print("\n=== 扭曲度计算 ===")

results = {}

if physical_proxy is not None:
    # 对齐金融比和实物代理比
    common_all = fin_monthly.index.intersection(physical_proxy.index)
    fin_aligned = fin_monthly.loc[common_all]
    phys_aligned = physical_proxy.loc[common_all]

    # 归一化到同一尺度（除以各自均值）
    fin_norm = fin_aligned / fin_aligned.mean()
    phys_norm = phys_aligned / phys_aligned.mean()

    # 扭曲度 = 金融归一化 / 实物归一化
    distortion = fin_norm / phys_norm
    distortion.name = "distortion"

    # 扭曲度的对数差（更稳定）
    log_distortion = np.log(fin_norm) - np.log(phys_norm)
    log_distortion.name = "log_distortion"

    print(f"扭曲度统计：")
    print(f"  均值={distortion.mean():.4f}, 标准差={distortion.std():.4f}")
    print(f"  对数扭曲度均值={log_distortion.mean():.4f}, 标准差={log_distortion.std():.4f}")

    results["fin_norm"] = fin_norm
    results["phys_norm"] = phys_norm
    results["distortion"] = distortion
    results["log_distortion"] = log_distortion


# ── 5. L 守恒检验 ──────────────────────────────────────────────

print("\n=== L 守恒检验 ===")

# L = 金融金油比本身（之前研究中 L = GC/BZ）
# 检验：E = α × L + β × distortion 的 cv 最小化
if "distortion" in results:
    from scipy.optimize import minimize_scalar
    from itertools import product

    L = fin_aligned.values
    D = results["distortion"].values

    # 网格搜索 α, β
    best_cv = np.inf
    best_ab = (None, None)

    alphas = np.linspace(0.1, 2.0, 50)
    betas = np.linspace(-2.0, 2.0, 50)

    cv_grid = np.zeros((len(alphas), len(betas)))

    for i, a in enumerate(alphas):
        for j, b in enumerate(betas):
            E = a * L + b * D
            if E.std() > 0:
                cv = E.std() / abs(E.mean()) if abs(E.mean()) > 1e-10 else np.inf
            else:
                cv = 0
            cv_grid[i, j] = cv
            if cv < best_cv:
                best_cv = cv
                best_ab = (a, b)

    print(f"最优 α={best_ab[0]:.4f}, β={best_ab[1]:.4f}")
    print(f"最小 cv={best_cv:.6f}")
    print(f"β 符号：{'负（符合预期——L涨扭曲度降）' if best_ab[1] < 0 else '正（不符预期）'}")

    # 计算最优 E
    E_opt = best_ab[0] * L + best_ab[1] * D

    # 对比：纯 L 的 cv
    cv_L_only = L.std() / abs(L.mean())
    print(f"\n纯 L 的 cv={cv_L_only:.6f}")
    print(f"加入扭曲度后 cv 变化：{(best_cv - cv_L_only) / cv_L_only * 100:.2f}%")

    # 相关性分析
    corr_LD = np.corrcoef(L, D)[0, 1]
    print(f"\nL 与 distortion 相关系数：{corr_LD:.4f}")

    results["E_opt"] = pd.Series(E_opt, index=fin_aligned.index, name="E_optimal")
    results["best_alpha"] = best_ab[0]
    results["best_beta"] = best_ab[1]
    results["best_cv"] = best_cv
    results["cv_L_only"] = cv_L_only


# ── 6. 可视化 ─────────────────────────────────────────────────

print("\n=== 输出 ===")

fig, axes = plt.subplots(4, 1, figsize=(14, 16), sharex=True)
fig.suptitle("实物金油比 vs 金融金油比 — $ 体系扭曲度", fontsize=14, fontweight="bold")

# 6a. 金融金油比
ax = axes[0]
ax.plot(fin_monthly.index, fin_monthly.values, "b-", linewidth=1, label="Financial GC/BZ")
ax.set_ylabel("Financial Ratio")
ax.set_title("金融金油比（GC/BZ 月度均值）")
ax.legend(loc="upper left")
ax.grid(True, alpha=0.3)

# 6b. 归一化对比
if "fin_norm" in results:
    ax = axes[1]
    ax.plot(results["fin_norm"].index, results["fin_norm"].values, "b-", linewidth=1, label="Financial (norm)")
    ax.plot(results["phys_norm"].index, results["phys_norm"].values, "r-", linewidth=1, label="Physical proxy (GLD/USO, norm)")
    ax.set_ylabel("Normalized Ratio")
    ax.set_title("归一化金油比：金融 vs 实物代理")
    ax.legend(loc="upper left")
    ax.grid(True, alpha=0.3)

# 6c. 扭曲度
if "distortion" in results:
    ax = axes[2]
    ax.plot(results["log_distortion"].index, results["log_distortion"].values, "purple", linewidth=1)
    ax.axhline(y=0, color="gray", linestyle="--", alpha=0.5)
    ax.fill_between(results["log_distortion"].index, 0, results["log_distortion"].values,
                    where=results["log_distortion"].values > 0, alpha=0.3, color="red", label="金融>实物（$ 高估金）")
    ax.fill_between(results["log_distortion"].index, 0, results["log_distortion"].values,
                    where=results["log_distortion"].values < 0, alpha=0.3, color="green", label="金融<实物（$ 低估金）")
    ax.set_ylabel("Log Distortion")
    ax.set_title("对数扭曲度 = ln(金融归一化) - ln(实物归一化)")
    ax.legend(loc="upper left")
    ax.grid(True, alpha=0.3)

# 6d. 守恒量 E
if "E_opt" in results:
    ax = axes[3]
    ax.plot(results["E_opt"].index, results["E_opt"].values, "darkgreen", linewidth=1,
            label=f"E = {results['best_alpha']:.2f}×L + ({results['best_beta']:.2f})×D, cv={results['best_cv']:.4f}")
    ax.axhline(y=results["E_opt"].mean(), color="gray", linestyle="--", alpha=0.5)
    ax.set_ylabel("E")
    ax.set_title(f"守恒量 E（cv: 纯L={results['cv_L_only']:.4f} → 加扭曲度={results['best_cv']:.4f}）")
    ax.legend(loc="upper left")
    ax.grid(True, alpha=0.3)
else:
    axes[3].text(0.5, 0.5, "数据不足，无法计算守恒量", transform=axes[3].transAxes,
                 ha="center", va="center", fontsize=14)

plt.tight_layout()
plt.savefig(OUT_DIR / "physical_ratio.png", dpi=150, bbox_inches="tight")
print(f"图表已保存: {OUT_DIR / 'physical_ratio.png'}")

# 输出 CSV
csv_parts = {"financial_ratio_monthly": fin_monthly}
if physical_proxy is not None:
    csv_parts["physical_proxy_monthly"] = physical_proxy
if "distortion" in results:
    csv_parts["distortion"] = results["distortion"]
    csv_parts["log_distortion"] = results["log_distortion"]
if "E_opt" in results:
    csv_parts["E_optimal"] = results["E_opt"]

csv_df = pd.DataFrame(csv_parts)
csv_df.to_csv(OUT_DIR / "physical_ratio.csv")
print(f"数据已保存: {OUT_DIR / 'physical_ratio.csv'}")

# 额外：库存代理
if crude_proxy is not None:
    print(f"\n[额外] 库存代理金油比可用，但未纳入主分析。可进一步探索。")

print("\n=== 完成 ===")
