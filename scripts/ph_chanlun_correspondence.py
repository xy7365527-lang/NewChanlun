"""PH ↔ 缠论 概念对应关系的 L2 实证验证（QQQ 日线）。

存在论位置
----------
本脚本验证一个总假说：**缠论的核心概念是某些 PH（持续同调）拓扑量的影子**。
任务给出 5 个子假说（级别/中枢/背驰/力度/量价），每个在 QQQ 日线真实数据上独立做
L2 检验。一个失败不影响其他（每个 hypothesis_* 函数 try/except 隔离，返回结构化判定）。

理论先验（必读——决定结果的认识论解读）
----------------------------------------
docs/persistence_theory.md §5/§13/§14 已**先验地**证明一个有效域边界：
  persistence ≈ **幅度** ∈ ker(D)（时间盲、动量盲），而"背驰/力度" = **MACD 动量**，
  落在 PH 框架之外（§14「纯拓扑动量不可能」定理）。
故对假说 3（背驰）、假说 4（力度）：
  - 先验预期是 PH 量只能捕捉「幅度」这一**必要条件**，捕捉不到「力度衰竭」这一**充分条件**。
  - 因此**部分否证 / 弱相关是预期内的、有价值的 L2 结果**（formalization-validity-domain
    规则：否定性结果缩小有效域边界 > 确认性结果）。
本脚本的职责是**诚实地测量**这些量的实际相关性，不预设成立也不预设否证。

认识论标注（每个判定都带）
--------------------------
- L0：纯算法/定义（PH 计算本身）。
- L2：QQQ 单标的真实数据假设检验（本脚本的产出等级）。
- ground truth：analysis/data_cache/qqq_chanlun_labels.json（TV 缠论指标标注，第三方 proxy）。

用法
----
    .venv/bin/python scripts/ph_chanlun_correspondence.py
输出：stdout 逐假说判定 + analysis/data_cache/ph_chanlun_correspondence_result.json

概念溯源标签
-----------
- 缠论概念 = PH 拓扑量 [新缠论:候选——本脚本的总假说]
- 有效域 ≠ 定义域 [新缠论:231号 formalization-validity-domain]
"""

from __future__ import annotations

import json
import math
import sys
from dataclasses import dataclass, asdict
from pathlib import Path

import numpy as np

# ---- 项目内 PH 引擎 --------------------------------------------------------
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
QQQ_CACHE = CACHE / "QQQ_1d_max.json"
GT_PATH = CACHE / "qqq_chanlun_labels.json"
OUT_PATH = CACHE / "ph_chanlun_correspondence_result.json"


# ===========================================================================
# 数据加载
# ===========================================================================


@dataclass(frozen=True, slots=True)
class Series:
    """QQQ 日线 OHLCV + 日期（immutable）。"""

    dates: tuple[str, ...]
    opens: np.ndarray
    highs: np.ndarray
    lows: np.ndarray
    closes: np.ndarray
    volumes: np.ndarray | None  # 可能为 None（缓存无 volume 且 yfinance 不可用）

    @property
    def n(self) -> int:
        return len(self.closes)


def load_qqq() -> Series:
    """加载 QQQ 日线。OHLC 来自缓存（离线、确定性），volume 尝试 yfinance 对齐。

    缓存 json 无 volume 列；假说 5（量价）需要成交量，故对齐 yfinance 的 volume。
    若 yfinance 不可用（网络/未装），volumes=None，假说 5 自行降级为 data-unavailable。
    """
    d = json.loads(QQQ_CACHE.read_text())
    dates = tuple(d["dates"])
    series = Series(
        dates=dates,
        opens=np.asarray(d["opens"], dtype=float),
        highs=np.asarray(d["highs"], dtype=float),
        lows=np.asarray(d["lows"], dtype=float),
        closes=np.asarray(d["closes"], dtype=float),
        volumes=_try_fetch_volume(dates),
    )
    return series


VOL_CACHE = CACHE / "QQQ_volume_aligned.json"


def _try_fetch_volume(dates: tuple[str, ...]) -> np.ndarray | None:
    """尝试用 yfinance 取 volume 并按缓存日期对齐。失败返回 None（诚实降级）。

    本地缓存对齐结果（QQQ_volume_aligned.json），避免每次运行重复下载。
    """
    if VOL_CACHE.exists():
        try:
            arr = np.asarray(json.loads(VOL_CACHE.read_text()), dtype=float)
            if len(arr) == len(dates):
                return arr
        except Exception:
            pass
    try:
        import yfinance as yf

        df = yf.download(
            "QQQ", period="max", interval="1d", progress=False, auto_adjust=False
        )
        if df is None or df.empty:
            return None
        vol = df["Volume"]
        # 处理 yfinance 可能的 MultiIndex 列
        if hasattr(vol, "columns"):
            vol = vol.iloc[:, 0]
        idx = {str(ts.date()): float(v) for ts, v in vol.items()}
        out = np.asarray([idx.get(dt, np.nan) for dt in dates], dtype=float)
        if np.isnan(out).all():
            return None
        try:
            VOL_CACHE.write_text(json.dumps(out.tolist()))
        except Exception:
            pass
        return out
    except Exception as exc:  # noqa: BLE001 — 数据源不可用是合法降级，不是概念矛盾
        print(f"  [volume] yfinance 不可用，假说5将降级: {type(exc).__name__}: {exc}")
        return None


def load_ground_truth() -> dict:
    """加载 TV 缠论标注。返回原始 dict（schema 在 main 中自描述打印）。"""
    if not GT_PATH.exists():
        return {}
    return json.loads(GT_PATH.read_text())


# ===========================================================================
# PH 基元
# ===========================================================================


def sublevel_h0_diagram(prices: np.ndarray) -> np.ndarray:
    """价格序列的 sublevel H0 persistence diagram。

    复用项目 OnlineMergeTree（因果在线 merge tree，finalize 后 = 批量等价，L0 已证）。
    返回 (k, 2) 数组 [(birth_price, death_price), ...]，含全局分量。
    """
    if len(prices) < 2:
        return np.empty((0, 2), dtype=float)
    bc = OnlineMergeTree.from_prices([float(p) for p in prices])
    pts = [(b.birth_price, b.death_price) for b in bc.settled_bars]
    return np.asarray(pts, dtype=float) if pts else np.empty((0, 2), dtype=float)


def macd(
    closes: np.ndarray, fast: int = 12, slow: int = 26, signal: int = 9
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """标准 MACD（12/26/9 EMA）。返回 (dif, dea, hist)。

    动力学层基线（§5：PH 不能平替 MACD）。内联标准定义——MACD 是稳定的标准指标，
    内联是自包含的正确实现，非补丁（避免与 a_macd 的脆弱耦合）。
    """

    def ema(x: np.ndarray, span: int) -> np.ndarray:
        alpha = 2.0 / (span + 1.0)
        out = np.empty_like(x)
        out[0] = x[0]
        for i in range(1, len(x)):
            out[i] = alpha * x[i] + (1 - alpha) * out[i - 1]
        return out

    dif = ema(closes, fast) - ema(closes, slow)
    dea = ema(dif, signal)
    hist = dif - dea
    return dif, dea, hist


def rips_h1_max_persistence(cloud: np.ndarray, maxdim: int = 1) -> float:
    """点云的 Vietoris-Rips H1 最大 persistence（无 H1 时返回 0.0）。

    L0：纯算法（ripser 确定性）。点云需已归一化（调用方负责量纲）。
    """
    from ripser import ripser

    if len(cloud) < 4:
        return 0.0
    dgms = ripser(np.asarray(cloud, dtype=float), maxdim=maxdim)["dgms"]
    if len(dgms) < 2 or len(dgms[1]) == 0:
        return 0.0
    h1 = dgms[1]
    h1 = h1[np.isfinite(h1[:, 1])]
    if len(h1) == 0:
        return 0.0
    return float(np.max(h1[:, 1] - h1[:, 0]))


def diagram_wasserstein(dgm_a: np.ndarray, dgm_b: np.ndarray, p: int = 1) -> float:
    """两个 persistence diagram 之间的 Wasserstein-p 距离（含对角线匹配）。

    严格定义：在两图的点 ∪ 各自到对角线的投影上做最优匹配（Hungarian）。
    L0：纯算法。用于假说 3（背驰 = 后段 W 距离 < 前段？）。
    """
    from scipy.optimize import linear_sum_assignment

    a = np.asarray(dgm_a, dtype=float).reshape(-1, 2)
    b = np.asarray(dgm_b, dtype=float).reshape(-1, 2)
    a = a[np.isfinite(a).all(axis=1)]
    b = b[np.isfinite(b).all(axis=1)]
    na, nb = len(a), len(b)
    if na == 0 and nb == 0:
        return 0.0

    def diag_proj(pt: np.ndarray) -> np.ndarray:
        m = (pt[0] + pt[1]) / 2.0
        return np.array([m, m])

    # 代价矩阵 (na+nb) x (na+nb)：左上 a-b，右上/左下 到对角线，右下 对角线-对角线=0
    size = na + nb
    cost = np.zeros((size, size), dtype=float)
    INF = 1e12
    for i in range(na):
        for j in range(nb):
            cost[i, j] = np.linalg.norm(a[i] - b[j]) ** p
        # a[i] 匹配对角线（在 b 侧虚拟点 nb+i）
        da = np.linalg.norm(a[i] - diag_proj(a[i])) ** p
        for j in range(nb, size):
            cost[i, j] = da if j - nb == i else INF
    for j in range(nb):
        db = np.linalg.norm(b[j] - diag_proj(b[j])) ** p
        for i in range(na, size):
            cost[i, j] = db if i - na == j else INF
    # 右下角对角线-对角线匹配，代价 0
    row, col = linear_sum_assignment(cost)
    total = cost[row, col].sum()
    return float(total ** (1.0 / p))


# ===========================================================================
# 假说 1：级别 = persistence 聚类
# ===========================================================================


def atr(highs: np.ndarray, lows: np.ndarray, closes: np.ndarray, n: int = 14) -> float:
    """全样本平均真实波幅（噪声阈 τ 的估计，与 repo settle 阶梯口径一致）。"""
    pc = np.concatenate([[closes[0]], closes[:-1]])
    tr = np.maximum.reduce([highs - lows, np.abs(highs - pc), np.abs(lows - pc)])
    return float(np.mean(tr))


def cluster_levels(persistences: np.ndarray, tau: float) -> dict:
    """检测 settled persistence 是否形成**离散级别带**，还是**连续谱**。

    首轮教训（formalization-validity-domain，必读）
    ----------------------------------------------
    朴素 log-gap 在大量近似重复的微小 persistence 上**退化**：median_gap→0 使任何
    gap 的相对倍数虚高（首轮 31 万倍假阳性），且簇大小 [20,1680,2,1] 是「主簇+离群」
    而非级别带。本函数修复三点：
      1. **τ 过滤**：剔除 < τ（ATR 噪声阈）的 persistence——它们是噪声不是级别。
      2. **去重**：在排序后的 **唯一** log 值上算 gap，消除重复值导致的 median=0。
      3. **稳健判据 + 多峰检验**：gap 显著性用 IQR 而非 median；并用 Hartigan dip 检验
         判定分布是「多峰（离散带）」还是「单峰/连续谱」。

    诚实立场：**连续谱（单峰）是对「级别=离散 persistence 簇」假说的否证**，但它本身
    与「缠论级别是相对/递归的，无绝对尺度」相容——这是有价值的负结果，不强行凑成立。

    Returns: dict（含 n_above_tau / n_modes / dip_pvalue / boundaries / band_sizes / shape）
    """
    p = np.sort(persistences[persistences > tau])
    if len(p) < 5:
        return {"n_above_tau": int(len(p)), "shape": "数据不足", "n_modes": 0,
                "boundaries": [], "band_sizes": [int(len(p))]}
    lp = np.log(p)
    lp_u = np.unique(np.round(lp, 6))          # 去重（消除 median_gap→0）
    gaps = np.diff(lp_u)
    q1, q3 = np.percentile(gaps, [25, 75])
    iqr = q3 - q1
    thresh = q3 + 1.5 * iqr                      # 稳健离群 gap 阈（Tukey）
    sig_pos = np.where(gaps > max(thresh, 1e-9))[0]
    boundaries = [float(np.exp((lp_u[i] + lp_u[i + 1]) / 2.0)) for i in sig_pos]
    # 各带计数（用原始 p）
    edges = [0.0] + sorted(boundaries) + [np.inf]
    band_sizes = [int(np.sum((p >= lo) & (p < hi))) for lo, hi in zip(edges[:-1], edges[1:])]

    # 多峰性：Hartigan dip 检验（diptest 若可用），否则用「显著带数 + 各带质量」近似
    dip_p = None
    try:
        from diptest import diptest as _dip
        _, dip_p = _dip(lp)
    except Exception:
        dip_p = None

    n_bands = len(boundaries) + 1
    # 「离散级别带」需：≥2 带 且 每带都有实质质量（≥总数 5%），否则视为「主簇+离群」
    substantial = [s for s in band_sizes if s >= 0.05 * len(p)]
    if dip_p is not None:
        shape = "多峰(离散带)" if dip_p < 0.05 else "单峰(连续谱)"
    else:
        shape = "多带(离散候选)" if len(substantial) >= 2 else "主簇+离群(非离散级别)"
    return {
        "n_above_tau": int(len(p)),
        "tau": round(float(tau), 4),
        "n_bands": n_bands,
        "n_substantial_bands": len(substantial),
        "boundaries": sorted(boundaries),
        "band_sizes": band_sizes,
        "dip_pvalue": dip_p,
        "gap_iqr_thresh": float(thresh),
        "shape": shape,
    }


def _settled_persistences(prices: np.ndarray) -> np.ndarray:
    tree = OnlineMergeTree()
    for c in prices:
        tree.update(float(c))
    final = tree.finalize()
    return np.asarray([b.persistence for b in final.settled_bars], dtype=float)


def hypothesis_1_levels(series: Series, gt: dict) -> dict:
    """假说1：settled persistence 是否形成离散级别带（多峰），还是连续谱（单峰）？

    **混淆变量控制（必读）**：persistence 用绝对价格差，而 QQQ 价格 20→738，绝对幅度
    在不同 regime 差几十倍 → 绝对价格 persistence 的连续谱可能是**非平稳尺度的伪影**，
    不是"级别本就连续"。故同时测**对数价格**（收益率尺度 = 级别的尺度不变量）：
      - 对数尺度下 dip 多峰 → 级别是离散带（绝对版的否证是伪影）。
      - 对数尺度下仍单峰 → 否证更稳健（级别确为连续/相对，无绝对断层）。

    判定以**对数价格版**为准（尺度不变量）；绝对价格版作对照。dip p<0.05=多峰=成立。
    """
    tau_abs = atr(series.highs, series.lows, series.closes)
    pers_abs = _settled_persistences(series.closes)
    clust_abs = cluster_levels(pers_abs, tau_abs)

    log_c = np.log(series.closes)
    log_h = np.log(series.highs)
    log_l = np.log(series.lows)
    tau_log = atr(log_h, log_l, log_c)
    pers_log = _settled_persistences(log_c)
    clust_log = cluster_levels(pers_log, tau_log)

    dip_log = clust_log.get("dip_pvalue")
    dip_abs = clust_abs.get("dip_pvalue")
    if dip_log is not None:
        verdict = "成立" if dip_log < 0.05 else "否证"
    elif clust_log.get("shape") in ("多峰(离散带)", "多带(离散候选)"):
        verdict = "成立"
    else:
        verdict = "否证"
    return {
        "hypothesis": "级别 = persistence 聚类",
        "level": "L2",
        "n_settled_total": int(len(pers_abs)),
        "abs_price": {"tau": clust_abs.get("tau"), "dip_pvalue": dip_abs,
                      "shape": clust_abs.get("shape"), "n_above_tau": clust_abs.get("n_above_tau")},
        "log_price": {"tau": clust_log.get("tau"), "dip_pvalue": dip_log,
                      "shape": clust_log.get("shape"), "n_above_tau": clust_log.get("n_above_tau"),
                      "n_substantial_bands": clust_log.get("n_substantial_bands")},
        "verdict": verdict,
        "note": "对数价格(尺度不变)dip多峰检验为准,绝对价格作对照。dip p<0.05=多峰=离散级别带=成立;"
                "单峰=连续谱=否证。控制了价格非平稳尺度混淆",
    }


# ===========================================================================
# 假说 2：中枢 = H1 persistent cycle
# ===========================================================================


def _norm_cloud(t: np.ndarray, p: np.ndarray) -> np.ndarray:
    """(t,p) 归一化点云：各轴除以 std（σ 自由参数，§8 已知风险，此处固定 std）。"""
    t = t.astype(float)
    p = p.astype(float)
    ts = t.std() or 1.0
    ps = p.std() or 1.0
    return np.column_stack([(t - t.mean()) / ts, (p - p.mean()) / ps])


def takens_h1(prices: np.ndarray, dim: int = 3, delays=(1, 2, 3, 5)) -> float:
    """Takens 延迟嵌入(相空间)的 H1 最大 persistence，扫多个 delay 取最大。

    为什么不是 (t,p) 点云（§8/§2 理论根据）
    ----------------------------------------
    (t,price) 点云的时间轴**单调递增** → 点云沿时间排成开放曲线，永不闭环 → H1 恒空
    （§8「单调路径 H1 恒空」）。中枢的"震荡环"只在**相空间**显现：延迟嵌入
    X_i = (p_i, p_{i+d}, ..., p_{i+(dim-1)d}) 把"价格回到先前水平"的震荡变成闭轨（极限环
    → H1≠0），趋势(单调)是开放轨迹(H1≈0)。这是 §2「H1 Takens 嵌入 + Rips」的方法。

    delay 未知(中枢震荡周期未知) → 扫 {1,2,3,5} 取最大 H1（对 zhongshu/trend 同一程序，
    无偏）。归一化用段内 std。L0：纯算法。
    """
    p = np.asarray(prices, dtype=float)
    s = p.std() or 1.0
    p = (p - p.mean()) / s
    best = 0.0
    for d in delays:
        n = len(p) - (dim - 1) * d
        if n < 5:
            continue
        cloud = np.column_stack([p[j * d: j * d + n] for j in range(dim)])
        best = max(best, rips_h1_max_persistence(cloud))
    return best


def detect_zhongshu_from_legs(legs: list) -> list:
    """从 GT 笔线用缠论定义构造中枢（≥3 笔价格区间重叠），独立于 PH。

    避免循环论证：中枢来自 TV 缠论指标的**笔**（独立 ground truth），不来自 PH
    自己算的中枢。中枢 = 连续 ≥3 笔的重叠区间 [ZD, ZG]，ZG=min(各笔高), ZD=max(各笔低)，
    ZD < ZG 时存在重叠。贪心延伸：重叠成立则吸收下一笔，破坏则封口。非重叠输出。

    Returns: [{lo, hi, zd, zg, n_legs}, ...]（lo/hi 为 bar 索引）。
    """
    zs = []
    n = len(legs)
    i = 0
    while i < n - 2:
        if legs[i].get("plo") is None or legs[i].get("phi") is None:
            i += 1
            continue
        cur_zd, cur_zg = legs[i]["plo"], legs[i]["phi"]
        last = i
        k = i + 1
        while k < n and legs[k].get("plo") is not None and legs[k].get("phi") is not None:
            new_zd = max(cur_zd, legs[k]["plo"])
            new_zg = min(cur_zg, legs[k]["phi"])
            if new_zd < new_zg:
                cur_zd, cur_zg, last = new_zd, new_zg, k
                k += 1
            else:
                break
        if last - i + 1 >= 3:
            zs.append({"lo": legs[i]["lo"], "hi": legs[last]["hi"],
                       "zd": float(cur_zd), "zg": float(cur_zg), "n_legs": last - i + 1})
            i = last + 1
        else:
            i += 1
    return zs


def hypothesis_2_zhongshu(series: Series, gt: dict, zhongshu: list) -> dict:
    """假说2：中枢区间的 (t,p) 点云 H1 max-persistence 是否显著高于趋势区间？

    中枢 = 价格在区间内反复震荡 = (t,p) 平面上的环 → H1 cycle。
    趋势 = 单调 → H1 恒空（§8 预测3）。
    判定：中枢窗口 H1 中位数 显著 > 趋势窗口 → 成立。
    """
    if not zhongshu:
        return {"hypothesis": "中枢 = H1", "level": "L2", "verdict": "data-unavailable",
                "note": "GT 未解析出中枢区间，无法定位测试窗口"}

    closes = series.closes
    n = len(closes)
    zs_h1, zs_spans = [], []
    for z in zhongshu:
        lo, hi = z.get("lo"), z.get("hi")
        if lo is None or hi is None or not (0 <= lo < hi < n) or hi - lo < 6:
            continue
        seg = closes[lo:hi + 1]
        zs_h1.append(takens_h1(seg))
        zs_spans.append(hi - lo + 1)
    if not zs_h1:
        return {"hypothesis": "中枢 = H1", "level": "L2", "verdict": "data-unavailable",
                "note": "中枢区间均过短(<6 bar)或越界"}

    # 对照组：等长度的随机趋势窗口（避开中枢位置）
    rng = np.random.default_rng(42)
    zs_mask = np.zeros(n, dtype=bool)
    for z in zhongshu:
        lo, hi = z.get("lo"), z.get("hi")
        if lo is not None and hi is not None and 0 <= lo <= hi < n:
            zs_mask[lo:hi + 1] = True
    trend_h1 = []
    for span in zs_spans:
        for _ in range(20):
            start = int(rng.integers(0, max(1, n - span)))
            if not zs_mask[start:start + span].any():
                seg = closes[start:start + span]
                trend_h1.append(takens_h1(seg))
                break

    zs_med = float(np.median(zs_h1))
    tr_med = float(np.median(trend_h1)) if trend_h1 else 0.0
    # Mann-Whitney U（非参，单标的稳健）
    try:
        from scipy.stats import mannwhitneyu
        if trend_h1 and zs_h1:
            _, pval = mannwhitneyu(zs_h1, trend_h1, alternative="greater")
        else:
            pval = float("nan")
    except Exception:
        pval = float("nan")

    verdict = "成立" if (zs_med > tr_med and pval == pval and pval < 0.05) else (
        "否证" if (zs_med <= tr_med) else "待定")
    return {
        "hypothesis": "中枢 = H1 persistent cycle",
        "level": "L2",
        "n_zhongshu_tested": len(zs_h1),
        "zhongshu_h1_median": zs_med,
        "trend_h1_median": tr_med,
        "mannwhitney_p_greater": None if pval != pval else float(pval),
        "verdict": verdict,
        "note": "中枢窗口 Takens相空间嵌入(dim3,delay扫1/2/3/5) H1 vs 等长趋势窗口；"
                "单边MWU p<0.05 且中位更高=成立。中枢来源=GT笔线缠论构造(独立于PH)",
    }


# ===========================================================================
# 假说 3：背驰 = Wasserstein 距离递减
# ===========================================================================


def hypothesis_3_divergence(series: Series, gt: dict, legs: list) -> dict:
    """假说3：连续同向腿的 sublevel diagram，后段间 W 距离 < 前段 → 背驰？

    背驰 = 后一段走势"力度"弱于前一段。本假说测试 W-距离能否作为代理。
    理论先验(§13/§14)：W 距离度量幅度差异，非力度 → 预期只捕捉幅度衰减(必要条件)。
    判定：用 GT 标注的背驰点处 W 距离变化方向是否一致 → 计算一致率。
    """
    if len(legs) < 3:
        return {"hypothesis": "背驰 = Wasserstein 递减", "level": "L2",
                "verdict": "data-unavailable", "note": f"可用腿段不足(n={len(legs)})"}

    closes = series.closes
    n = len(closes)
    # 对每相邻同向腿对计算 W 距离序列
    diags = []
    for lg in legs:
        lo, hi = lg["lo"], lg["hi"]
        if 0 <= lo < hi < n and hi - lo >= 2:
            diags.append((lg, sublevel_h0_diagram(closes[lo:hi + 1])))
    if len(diags) < 3:
        return {"hypothesis": "背驰 = Wasserstein 递减", "level": "L2",
                "verdict": "data-unavailable", "note": "有效 diagram 不足"}

    # 同向连续三腿 A→B→C（同 direction）：比较 d(A,B) vs d(B,C)
    decreasing = 0
    total = 0
    samples = []
    for i in range(len(diags) - 2):
        (lA, dA), (lB, dB), (lC, dC) = diags[i], diags[i + 1], diags[i + 2]
        if lA.get("direction") == lC.get("direction"):  # 同向（B 为反向回调）
            w1 = diagram_wasserstein(dA, dB)
            w2 = diagram_wasserstein(dB, dC)
            total += 1
            if w2 < w1:
                decreasing += 1
            samples.append({"i": i, "w_AB": round(w1, 3), "w_BC": round(w2, 3),
                            "decreasing": bool(w2 < w1)})
    if total == 0:
        return {"hypothesis": "背驰 = Wasserstein 递减", "level": "L2",
                "verdict": "data-unavailable", "note": "无同向三腿序列"}

    rate = decreasing / total
    # 二项检验：递减率是否显著偏离 0.5（随机）。
    try:
        from scipy.stats import binomtest
        binom_p = binomtest(decreasing, total, 0.5, alternative="two-sided").pvalue
    except Exception:
        binom_p = float("nan")
    # 显著偏离 0.5 且方向为递减(>0.5) → 成立；显著但反向 → 否证；不显著 → 待定(随机)
    if binom_p == binom_p and binom_p < 0.05:
        verdict = "成立" if rate > 0.5 else "否证(反向)"
    else:
        verdict = "待定(≈随机,不显著)"
    return {
        "hypothesis": "背驰 = Wasserstein 距离递减",
        "level": "L2",
        "n_triples": total,
        "decreasing_rate": round(rate, 3),
        "binomial_p_vs_0.5": None if binom_p != binom_p else float(binom_p),
        "samples": samples[:6],
        "verdict": verdict,
        "note": "同向三腿(GT笔级) d(A,B)vsd(B,C)递减率,二项检验vs0.5。边界:笔级≠段级,"
                "背驰本属段/走势级(级别错配);理论先验W捕幅度非力度(§13/14)",
    }


# ===========================================================================
# 假说 4：力度 = alive 分量数量
# ===========================================================================


def hypothesis_4_strength(series: Series, gt: dict) -> dict:
    """假说4：settle 事件发生时的 alive 分量数 是否与 MACD 柱(力度) 相关？

    alive 多 = 走势内部分化剧烈；MACD hist = 传统力度。
    理论先验(§14): alive count 是结构量非动量量 → 预期弱相关/不相关(否定性有价值)。
    判定：|spearman(alive_count, |MACD_hist|)| ≥ 0.3 且 p<0.05 → 成立；否则否证。
    """
    closes = series.closes
    _, _, hist = macd(closes)

    tree = OnlineMergeTree()
    alive_at_settle = []
    hist_at_settle = []
    for i, c in enumerate(closes):
        newly = tree.update(float(c))
        if newly:  # 本根有 settle 事件
            snap = tree.current_barcode()
            alive_at_settle.append(len(snap.alive_bars))
            hist_at_settle.append(abs(float(hist[i])))

    if len(alive_at_settle) < 10:
        return {"hypothesis": "力度 = alive 分量数", "level": "L2",
                "verdict": "data-unavailable", "note": f"settle 事件过少(n={len(alive_at_settle)})"}

    a = np.asarray(alive_at_settle, dtype=float)
    h = np.asarray(hist_at_settle, dtype=float)
    try:
        from scipy.stats import spearmanr
        rho, pval = spearmanr(a, h)
    except Exception:
        rho, pval = float("nan"), float("nan")

    # 原假说是**正相关**（"alive 多 = 内部分化剧烈 = 力度强"）。判定按方向区分：
    #   - 显著正相关(rho>=0.3) → 成立（支持 alive=力度）。
    #   - 显著负相关(rho<=-0.3) → **否证原方向**，但揭示 alive≈盘整/震荡分化度
    #     （多个未完成下跌分量并存=盘整=MACD 柱小），与 §14「alive 是结构量非动量量」一致。
    #   - |rho|<0.3 → 否证（无关，符合 §14 先验）。
    if rho != rho or pval != pval:
        verdict = "待定"
    elif pval < 0.05 and rho >= 0.3:
        verdict = "成立"
    elif pval < 0.05 and rho <= -0.3:
        verdict = "否证(原方向相反)——揭示 alive≈盘整度,与力度负相关(印证§14)"
    else:
        verdict = "否证(无关,符合§14先验)"
    return {
        "hypothesis": "力度 = alive 分量数量",
        "level": "L2",
        "n_settle_events": int(len(a)),
        "alive_count_range": [int(a.min()), int(a.max())],
        "spearman_rho": None if rho != rho else round(float(rho), 4),
        "spearman_p": None if pval != pval else float(pval),
        "verdict": verdict,
        "note": "alive数 vs |MACD hist| Spearman；按方向判定。原假说=正相关(alive=力度)；"
                "负相关则否证原方向但揭示 alive=盘整度(§14)",
    }


# ===========================================================================
# 假说 5：量价联合拓扑（探索性）
# ===========================================================================


def hypothesis_5_volume_price(series: Series, gt: dict, window: int = 30) -> dict:
    """假说5(探索性)：close+volume 2D 滑窗点云的 H1，是否在量价背离处结构变化？

    量价背离 = 价创新高但量未跟上(或反之)。本假说测试联合拓扑 H1 是否对此敏感。
    判定：背离窗口 H1 与非背离窗口 H1 是否可分(MWU p<0.05)。
    """
    if series.volumes is None:
        return {"hypothesis": "量价联合拓扑", "level": "L2",
                "verdict": "data-unavailable", "note": "volume 不可用(yfinance 未取到)"}

    closes = series.closes
    vols = series.volumes
    mask = np.isfinite(vols)
    closes = closes[mask]
    vols = vols[mask]
    n = len(closes)
    if n < window * 4:
        return {"hypothesis": "量价联合拓扑", "level": "L2",
                "verdict": "data-unavailable", "note": "对齐后数据不足"}

    # 量价背离标记：窗口内 价格斜率 与 量斜率 反号
    div_h1, ndiv_h1 = [], []
    for s in range(0, n - window, window // 2):
        cw = closes[s:s + window]
        vw = vols[s:s + window]
        cloud = _norm_cloud(cw, vw)  # (close, volume) 2D 归一
        h1 = rips_h1_max_persistence(cloud)
        # 斜率符号
        tt = np.arange(window)
        cs = np.polyfit(tt, cw, 1)[0]
        vs = np.polyfit(tt, vw, 1)[0]
        if cs * vs < 0:  # 反号 = 量价背离
            div_h1.append(h1)
        else:
            ndiv_h1.append(h1)

    if len(div_h1) < 5 or len(ndiv_h1) < 5:
        return {"hypothesis": "量价联合拓扑", "level": "L2", "verdict": "待定",
                "note": f"背离/非背离窗口样本不足 ({len(div_h1)}/{len(ndiv_h1)})"}

    try:
        from scipy.stats import mannwhitneyu
        _, pval = mannwhitneyu(div_h1, ndiv_h1, alternative="two-sided")
    except Exception:
        pval = float("nan")

    div_med, ndiv_med = float(np.median(div_h1)), float(np.median(ndiv_h1))
    sig = (pval == pval) and pval < 0.05
    verdict = "成立(可分)" if sig else "否证(不可分)" if (pval == pval) else "待定"
    return {
        "hypothesis": "量价联合拓扑 (close+volume H1)",
        "level": "L2",
        "n_divergence_windows": len(div_h1),
        "n_normal_windows": len(ndiv_h1),
        "divergence_h1_median": round(div_med, 4),
        "normal_h1_median": round(ndiv_med, 4),
        "mannwhitney_p": None if pval != pval else float(pval),
        "verdict": verdict,
        "note": f"window={window}; 量价斜率反号=背离; 联合H1可分性 MWU p<0.05=成立",
    }


# ===========================================================================
# Ground truth 解析（自适应——schema 未知，运行时探测）
# ===========================================================================


def _time_mapper(dates: tuple[str, ...]):
    """构造一个把多种时间编码 → 交易日索引 的函数（自适应 TV 坐标）。

    支持：bar 索引(int<n) / unix 秒(>1e9) / unix 毫秒(>1e12) / 'YYYY-MM-DD' 字符串。
    unix 时间映射到**最近的交易日索引**（bisect）——TV box/line 端点可能落在
    非交易日或盘中，取最近交易日是忠实近似。
    """
    import bisect
    from datetime import datetime, timezone

    date_to_idx = {d: i for i, d in enumerate(dates)}
    epochs = []
    for d in dates:
        try:
            dt = datetime.strptime(d[:10], "%Y-%m-%d").replace(tzinfo=timezone.utc)
            epochs.append(dt.timestamp())
        except Exception:
            epochs.append(float("nan"))
    n = len(dates)

    def to_idx(v):
        if v is None:
            return None
        if isinstance(v, bool):
            return None
        if isinstance(v, str):
            if v[:10] in date_to_idx:
                return date_to_idx[v[:10]]
            try:
                v = float(v)
            except ValueError:
                return None
        if isinstance(v, (int, float)):
            x = float(v)
            if 0 <= x < n and x == int(x):          # bar 索引
                return int(x)
            if x > 1e12:                              # unix 毫秒
                x /= 1000.0
            if x > 1e8:                               # unix 秒 → 最近交易日
                j = bisect.bisect_left(epochs, x)
                cands = [c for c in (j - 1, j, j + 1) if 0 <= c < n]
                if not cands:
                    return None
                return min(cands, key=lambda c: abs(epochs[c] - x))
        return None

    return to_idx


def parse_ground_truth(gt: dict, dates: tuple[str, ...]) -> dict:
    """从 TV 缠论 GT 解析 中枢(boxes) / 腿段(lines) / 买卖点(labels)。

    真实 schema：gt['pine'] = {labels, lines, boxes, tables}（TV pine 导出）。
    自诊断：返回 _diag 记录每类的总数/成功解析数/样本字段，便于一次运行即定位问题。
    """
    to_idx = _time_mapper(dates)
    n = len(dates)
    src = gt.get("pine", gt) if isinstance(gt, dict) else {}
    boxes = src.get("boxes") or []
    lines = src.get("lines") or []
    labels = src.get("labels") or []

    def pick(d: dict, *names):
        for nm in names:
            if nm in d and d[nm] is not None:
                return d[nm]
        return None

    # ---- 中枢 = box（left/right 时间, top/bottom 价格）----
    zhongshu = []
    for z in boxes:
        if not isinstance(z, dict):
            continue
        lo = to_idx(pick(z, "left", "x1", "start", "t1", "from", "left_time", "lo", "bar1"))
        hi = to_idx(pick(z, "right", "x2", "end", "t2", "to", "right_time", "hi", "bar2"))
        if lo is None or hi is None:
            continue
        top = pick(z, "top", "high", "y2", "zg", "upper")
        bot = pick(z, "bottom", "low", "y1", "zd", "lower")
        zhongshu.append({"lo": min(lo, hi), "hi": max(lo, hi),
                         "zg": top, "zd": bot})

    # ---- 腿段 = line（x1,y1 → x2,y2）----
    legs = []
    for s in lines:
        if not isinstance(s, dict):
            continue
        lo = to_idx(pick(s, "x1", "left", "start", "t1", "from", "bar1"))
        hi = to_idx(pick(s, "x2", "right", "end", "t2", "to", "bar2"))
        if lo is None or hi is None or lo == hi:
            continue
        y1 = pick(s, "y1", "price1", "start_price", "p1")
        y2 = pick(s, "y2", "price2", "end_price", "p2")
        direction = None
        if y1 is not None and y2 is not None:
            direction = 1 if y2 > y1 else -1
        a, b = (lo, hi) if lo < hi else (hi, lo)
        plo = min(y1, y2) if (y1 is not None and y2 is not None) else None
        phi = max(y1, y2) if (y1 is not None and y2 is not None) else None
        legs.append({"lo": a, "hi": b, "direction": direction, "plo": plo, "phi": phi})
    legs.sort(key=lambda x: x["lo"])

    # ---- 买卖点 = label（含价位/文本）----
    buysell = []
    for lb in labels:
        if not isinstance(lb, dict):
            continue
        idx = to_idx(pick(lb, "x", "time", "bar", "t"))
        txt = pick(lb, "text", "label", "tooltip") or ""
        buysell.append({"idx": idx, "y": pick(lb, "y", "price"), "text": str(txt)[:40]})

    diag = {
        "boxes_total": len(boxes), "zhongshu_parsed": len(zhongshu),
        "lines_total": len(lines), "legs_parsed": len(legs),
        "labels_total": len(labels), "buysell_parsed": len(buysell),
        "box_fields": sorted(boxes[0].keys()) if boxes and isinstance(boxes[0], dict) else [],
        "line_fields": sorted(lines[0].keys()) if lines and isinstance(lines[0], dict) else [],
        "label_fields": sorted(labels[0].keys()) if labels and isinstance(labels[0], dict) else [],
    }
    schema = {k: (type(v).__name__) for k, v in src.items()} if isinstance(src, dict) else {}
    return {"zhongshu": zhongshu, "legs": legs, "buysell": buysell,
            "_schema": schema, "_diag": diag}


# ===========================================================================
# main
# ===========================================================================


def main() -> None:
    print("=" * 78)
    print("PH ↔ 缠论 概念对应关系 L2 验证（QQQ 日线）")
    print("=" * 78)

    series = load_qqq()
    print(f"\n[数据] QQQ {series.dates[0]} ~ {series.dates[-1]}, n={series.n} bars, "
          f"价格[{series.closes.min():.2f}, {series.closes.max():.2f}], "
          f"volume={'有' if series.volumes is not None else '无(假说5降级)'}")

    gt_raw = load_ground_truth()
    gt = parse_ground_truth(gt_raw, series.dates)
    print(f"\n[GT schema] {json.dumps(gt['_schema'], ensure_ascii=False)}")
    print(f"[GT 自诊断] {json.dumps(gt['_diag'], ensure_ascii=False)}")
    # GT box(中枢) 坐标在 TV 导出层丢失(price1/price2=null,无bar坐标) → 用 GT 笔线
    # 按缠论定义(连续≥3笔重叠)独立构造中枢，避免用 PH 自算中枢的循环论证。
    gt["_zhongshu_source"] = "GT_box"
    if not gt["zhongshu"]:
        gt["zhongshu"] = detect_zhongshu_from_legs(gt["legs"])
        gt["_zhongshu_source"] = "GT_笔线构造(box坐标丢失)"
    print(f"[GT 中枢来源] {gt['_zhongshu_source']}")
    print(f"[GT 解析] 中枢={len(gt['zhongshu'])} 腿段={len(gt['legs'])} 买卖点={len(gt['buysell'])}")
    if gt["legs"][:2]:
        print(f"[GT 腿段样例] {gt['legs'][:2]}")
    if gt["zhongshu"][:2]:
        print(f"[GT 中枢样例] {gt['zhongshu'][:2]}")
    if gt["buysell"][:2]:
        print(f"[GT 买卖点样例] {gt['buysell'][:2]}")

    results = {}
    for name, fn in [
        ("h1_levels", lambda: hypothesis_1_levels(series, gt)),
        ("h2_zhongshu", lambda: hypothesis_2_zhongshu(series, gt, gt["zhongshu"])),
        ("h3_divergence", lambda: hypothesis_3_divergence(series, gt, gt["legs"])),
        ("h4_strength", lambda: hypothesis_4_strength(series, gt)),
        ("h5_volume_price", lambda: hypothesis_5_volume_price(series, gt)),
    ]:
        print(f"\n{'─' * 78}\n▶ {name}")
        try:
            res = fn()
        except Exception as exc:  # noqa: BLE001 — 单假说失败隔离，不影响其他
            import traceback
            res = {"verdict": "ERROR", "error": f"{type(exc).__name__}: {exc}",
                   "trace": traceback.format_exc().splitlines()[-3:]}
        results[name] = res
        print(json.dumps(res, ensure_ascii=False, indent=2))

    OUT_PATH.write_text(json.dumps(
        {"meta": {"symbol": "QQQ", "n_bars": series.n,
                  "date_range": [series.dates[0], series.dates[-1]],
                  "gt_schema": gt["_schema"]},
         "results": results}, ensure_ascii=False, indent=2))
    print(f"\n{'=' * 78}\n[输出] {OUT_PATH}")
    print("[判定汇总]")
    for k, v in results.items():
        print(f"  {k:18s} → {v.get('verdict', '?')}")


if __name__ == "__main__":
    main()
