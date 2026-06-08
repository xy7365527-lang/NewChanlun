"""资本流转四维度假说验证（缠论框架内）。

编排者假说（2026-06-08）：跨结算尺残差向量在缠论框架内有四个可观测维度：

  1. 方向    = 残差符号（哪个结算尺被高估 → 资本从高估流向低估）
  2. 加速度  = 残差时间序列的 MACD（DIF=短期vs长期，柱=流转方向变化加速度，
               残差背驰=流转力度衰减=方向即将反转）。MACD 与单标的用法同构。
  3. 流量    = 残差方向 × min(Vol_A, Vol_B)（两标的成交量取瓶颈端）。量价关系
               在比价关系上的推广。
  4. 流速/持续性 = 残差在缠论递归中涌现的趋势级别（bi级别=快但不确定；
               走势级别=慢但持续性强；级别越高流转越确定）。

认识论等级（formalization-validity-domain 规则）：
  - 维度1（方向）：L0/L2——残差符号是定义，符号序列结构是 L2。
  - 维度2（加速度/MACD背驰）：L2——背驰时点 vs 已知 regime 切换是可证伪比对。
  - 维度3（流量领先）：**L2 唯一强可证伪维度**——累积流量是否领先残差水平。
    若不领先 → "流量 proxy"假说被否证（否定性结果，缩小有效域）。
  - 维度4（持续性/级别）：L2——级别 vs 持续性，复用 §8.5b 已验证的残差结构。

主残差对象：r = log(DX) − 0.576·log(USD6E)（跨结算尺，占 DX 方差 33.8%，
verify_cross_national_closure.py Part B）。对照：ω = GC/CL（空转/剥削率，482号）。

数据：analysis/data_cache/{sym}_1m_databento_10y.json（含 volumes）。
用法：PYTHONPATH=src python analysis/verify_capital_flow_4d.py [hourly_window]
"""

from __future__ import annotations

import json
import math
from datetime import datetime
from pathlib import Path

import numpy as np
import pandas as pd

from newchan.a_macd import compute_macd, macd_area_for_range
from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

_DATA = Path(__file__).resolve().parent / "data_cache"
_EUR_WEIGHT = 0.576

# 已知宏观 regime 切换日（用于维度2背驰时点比对）。来源：宏观流动性转折公认时点
# + auto-memory user_trading_direction（2025-02-20 用户标注 regime 断点）。
_KNOWN_REGIMES: list[tuple[str, str]] = [
    ("2020-03-23", "COVID 崩盘底 / 美联储无限 QE 启动"),
    ("2021-11-08", "流动性见顶 / Fed taper 信号"),
    ("2022-03-16", "首次加息"),
    ("2022-09-28", "美元/利率见顶区（CPI 见顶前）"),
    ("2024-09-18", "美联储首次降息"),
    ("2025-02-20", "用户标注 regime 断点（金油比）"),
]


def _load(sym: str) -> dict:
    with open(_DATA / f"{sym}_1m_databento_10y.json") as f:
        return json.load(f)


def _parse(s: str) -> datetime:
    core = s.split("+")[0].strip()
    return datetime.strptime(core[:19], "%Y-%m-%d %H:%M:%S")


def _aligned_residual_with_volume(
    sym_a: str, sym_b: str, w_a: float, w_b: float
) -> tuple[list[datetime], np.ndarray, np.ndarray, np.ndarray]:
    """逐分钟对齐 a、b，返回 (ts, 残差 r=w_a·log a + w_b·log b, vol_a, vol_b)。"""
    da, db = _load(sym_a), _load(sym_b)
    ia = {t: (c, v) for t, c, v in zip(da["dates"], da["closes"], da["volumes"])}
    ib = {t: (c, v) for t, c, v in zip(db["dates"], db["closes"], db["volumes"])}
    common = sorted(ia.keys() & ib.keys())
    ts_out: list[datetime] = []
    r, va, vb = [], [], []
    for t in common:
        ca, vaa = ia[t]
        cb, vbb = ib[t]
        if ca > 0 and cb > 0:
            ts_out.append(_parse(t))
            r.append(w_a * math.log(ca) + w_b * math.log(cb))
            va.append(float(vaa))
            vb.append(float(vbb))
    return ts_out, np.asarray(r), np.asarray(va), np.asarray(vb)


# ════════════════════════════════════════════════════════════
# 维度 1 — 方向
# ════════════════════════════════════════════════════════════


def dim1_direction(ts: list[datetime], r: np.ndarray) -> None:
    print("\n" + "=" * 72)
    print("维度 1 [方向] — 残差符号（中心化后）= 哪个结算尺被高估")
    print("=" * 72)
    rc = r - r.mean()  # 中心化，符号才有"相对高估"含义
    pos = int((rc > 0).sum())
    neg = int((rc < 0).sum())
    # 符号翻转次数（方向切换频率）
    sign = np.sign(rc)
    flips = int((np.diff(sign) != 0).sum())
    print(f"  样本 = {len(r):,} 分钟bar | 残差>0(DX 端高估): {pos:,} | <0: {neg:,}")
    print(f"  符号翻转次数 = {flips:,}（方向切换频率 = {flips/len(r)*100:.3f}%/bar）")
    print(f"  残差均值={r.mean():.5f} 标准差={r.std():.5f} | 中心化极差=[{rc.min():.4f},{rc.max():.4f}]")
    print("  → 方向是 L0 定义（符号）；其结构性见维度4（符号序列是否涌现趋势）。")


# ════════════════════════════════════════════════════════════
# 维度 2 — 加速度（残差 MACD + 背驰 vs 已知 regime）
# ════════════════════════════════════════════════════════════


def _hourly_residual_df(ts: list[datetime], r: np.ndarray) -> pd.DataFrame:
    """残差重采样为小时序列（close=末值，high/low=极值）。残差本身是 log 量，
    MACD 直接作用于 r（不再取 log）——维度2"MACD 直接适用"。"""
    idx = pd.DatetimeIndex(ts)
    s = pd.Series(r, index=idx)
    g = s.resample("1h")
    df = pd.DataFrame({
        "close": g.last(),
        "high": g.max(),
        "low": g.min(),
    }).dropna()
    return df


def _find_swings(vals: np.ndarray, win: int) -> tuple[list[int], list[int]]:
    """简单摆动点检测：局部极大/极小（窗口 win 内）。返回 (高点idx, 低点idx)。"""
    highs, lows = [], []
    n = len(vals)
    for i in range(win, n - win):
        seg = vals[i - win : i + win + 1]
        if vals[i] == seg.max() and vals[i] > vals[i - 1]:
            highs.append(i)
        if vals[i] == seg.min() and vals[i] < vals[i - 1]:
            lows.append(i)
    return highs, lows


def _nearest_regime(d: datetime) -> tuple[str, int]:
    """返回最近的已知 regime 日及天数差。"""
    best, bestdays = "", 10**9
    for ds, _name in _KNOWN_REGIMES:
        rd = datetime.strptime(ds, "%Y-%m-%d")
        days = abs((d - rd).days)
        if days < bestdays:
            bestdays, best = days, ds
    return best, bestdays


def dim2_macd_divergence(ts: list[datetime], r: np.ndarray, name: str) -> None:
    print("\n" + "=" * 72)
    print(f"维度 2 [加速度] — 残差 MACD 背驰 vs 已知 regime 切换  [{name}]")
    print("=" * 72)
    df = _hourly_residual_df(ts, r)
    macd = compute_macd(df)  # 直接在残差上算 MACD（残差已是 log 量）
    closes = df["close"].to_numpy()
    print(f"  残差小时bar = {len(df):,} | MACD(12,26,9) 直接作用于残差序列")

    # 摆动点（窗口=120 小时 ≈ 5 交易日，捕捉中级别转折）
    win = 120
    highs, lows = _find_swings(closes, win)

    def scan(extrema: list[int], kind: str) -> list[tuple[datetime, int]]:
        """相邻同向极值：价新极值但 MACD 面积衰减 = 背驰。"""
        hits = []
        for k in range(1, len(extrema)):
            i0, i1 = extrema[k - 1], extrema[k]
            a0 = macd_area_for_range(macd, max(0, i0 - win), i0)
            a1 = macd_area_for_range(macd, max(0, i1 - win), i1)
            p0, p1 = closes[i0], closes[i1]
            if kind == "top" and p1 > p0 and abs(a1["area_total"]) < abs(a0["area_total"]):
                hits.append((df.index[i1].to_pydatetime(), 1))
            if kind == "bot" and p1 < p0 and abs(a1["area_total"]) < abs(a0["area_total"]):
                hits.append((df.index[i1].to_pydatetime(), -1))
        return hits

    div = sorted(scan(highs, "top") + scan(lows, "bot"))
    print(f"  检测到背驰点 = {len(div)} 个（顶背驰=方向将反转向下，底背驰=反转向上）")
    # 比对已知 regime：背驰点落在已知 regime ±N 天内算"命中"
    TOL = 45
    hit, total = 0, 0
    for d, sgn in div:
        rg, days = _nearest_regime(d)
        total += 1
        mark = ""
        if days <= TOL:
            hit += 1
            mark = f"  ✓命中 regime {rg}(±{days}d)"
        # 只打印命中的，避免刷屏
        if mark:
            print(f"    {d.date()} {'顶' if sgn>0 else '底'}背驰{mark}")
    rate = hit / total * 100 if total else 0.0
    print(f"  背驰点命中已知 regime（±{TOL}天）：{hit}/{total} = {rate:.1f}%")
    print("  诚实标注：命中率高=背驰领先/同步 regime（L2 支持）；低=残差背驰非宏观信号"
          "（L2 否定）。背驰点稀疏时统计力弱。")


# ════════════════════════════════════════════════════════════
# 维度 3 — 流量（残差方向 × 成交量瓶颈，是否领先价格）
# ════════════════════════════════════════════════════════════


def dim3_flow_leads(
    ts: list[datetime], r: np.ndarray, va: np.ndarray, vb: np.ndarray, name: str
) -> None:
    print("\n" + "=" * 72)
    print(f"维度 3 [流量] — flow = sign(Δr)×min(Vol_A,Vol_B)，是否领先残差  [{name}]")
    print("=" * 72)
    dr = np.diff(r)                       # 残差变化
    vbottle = np.minimum(va, vb)[1:]      # 成交量瓶颈（对齐 dr）
    flow = np.sign(dr) * vbottle          # 瞬时流量 proxy
    cumflow = np.cumsum(flow)             # 累积流量
    level = (r - r.mean())[1:]            # 残差水平（中心化）

    # 标准化
    def z(x: np.ndarray) -> np.ndarray:
        s = x.std()
        return (x - x.mean()) / s if s > 0 else x - x.mean()

    zc, zl = z(cumflow), z(level)
    # 互相关：cumflow[t] vs level[t+lag]。lag>0 命中 = 流量领先水平。
    lags = [-2880, -1440, -480, -120, -30, 0, 30, 120, 480, 1440, 2880]  # 分钟
    print(f"  样本 = {len(flow):,} 分钟 | 互相关 corr(累积流量[t], 残差水平[t+lag])")
    print(f"  {'lag(min)':>10} {'corr':>9}   (lag>0=流量领先)")
    best_lag, best_c = 0, -2.0
    for lag in lags:
        if lag == 0:
            c = float(np.corrcoef(zc, zl)[0, 1])
        elif lag > 0:
            c = float(np.corrcoef(zc[:-lag], zl[lag:])[0, 1])
        else:
            c = float(np.corrcoef(zc[-lag:], zl[:lag])[0, 1])
        star = ""
        if c > best_c:
            best_c, best_lag = c, lag
        print(f"  {lag:>10} {c:>9.4f}{star}")
    # 严格裁决：真领先需"剖面非平坦"——正滞后峰值显著高于零滞后 + 负滞后。
    # 仅靠 best_lag>0 不够（cumsum(sign Δr·vol) 近同义反复地跟踪 r，corr 高是构造性的）。
    c0 = float(np.corrcoef(zc, zl)[0, 1])
    c_neg = float(np.corrcoef(zc[-(-(-2880)):], zl[:-2880])[0, 1]) if len(zc) > 2880 else c0
    spread = best_c - min(c0, c_neg)
    print(f"  峰值相关 lag={best_lag}min corr={best_c:.4f} | lag0={c0:.4f} | "
          f"剖面跨度(峰−min)={spread:.4f}")
    if best_lag > 0 and spread > 0.02:
        verdict = "流量领先残差水平（剖面非平坦，L2 支持假设）"
    else:
        verdict = (f"**剖面平坦/无领先**（跨度{spread:.4f}<0.02）→ 流量 proxy 不领先价格 "
                   "= 累积流量与残差水平近同义反复（cumsum 构造性高相关），无独立领先信息"
                   "（L2 否定，缩小有效域）")
    print(f"  → 裁决：{verdict}")

    # 增量瞬时检验：flow[t] 是否预测 future Δr[t+k]（方向预测力）
    print("\n  瞬时流量 → 未来残差变化方向预测力（signed-volume momentum）：")
    for k in (1, 5, 30, 120):
        fut = r[1 + k :] - r[1 : len(r) - k]   # Δr over next k
        f0 = flow[: len(fut)]
        c = float(np.corrcoef(z(f0), z(fut))[0, 1]) if len(fut) > 10 else float("nan")
        print(f"    k={k:>4}min  corr(flow[t], Δr[t→t+k]) = {c:.4f}")


# ════════════════════════════════════════════════════════════
# 维度 4 — 流速/持续性（缠论递归级别）
# ════════════════════════════════════════════════════════════


def _resample_to_hourly_bars(ts: list[datetime], r: np.ndarray) -> list[Bar]:
    """残差 exp 归一为正值价格序列后重采样小时 OHLC（缠论引擎需正值）。"""
    idx = pd.DatetimeIndex(ts)
    s = pd.Series(np.exp(r - r.mean()), index=idx)  # 中心化后 exp，量级合理
    g = s.resample("1h")
    o, h, l, c = g.first(), g.max(), g.min(), g.last()
    bars: list[Bar] = []
    for t in o.dropna().index:
        bars.append(Bar(ts=t.to_pydatetime(), open=float(o[t]), high=float(h[t]),
                        low=float(l[t]), close=float(c[t])))
    return bars


def dim4_persistence(ts: list[datetime], r: np.ndarray, name: str, window: int | None) -> None:
    print("\n" + "=" * 72)
    print(f"维度 4 [流速/持续性] — 残差缠论递归级别（级别越高=流转越确定）  [{name}]")
    print("=" * 72)
    bars = _resample_to_hourly_bars(ts, r)
    if window:
        bars = bars[-window:]
    print(f"  残差小时bar = {len(bars):,}")
    orch = RecursiveOrchestrator(stream_id=name, max_levels=6, stroke_mode="wide")
    snap = None
    for b in bars:
        snap = orch.process_bar(b)
    assert snap is not None
    moves = snap.move_snapshot.moves
    top = moves[-1] if moves else None
    max_level = max((rs.level_id for rs in snap.recursive_snapshots), default=1)
    zs_total = len(snap.zs_snapshot.zhongshus) + sum(
        len(rs.zhongshus) for rs in snap.recursive_snapshots)
    bsps = snap.bsp_snapshot.buysellpoints
    diverg = [p for p in bsps if getattr(p, "divergence_key", None) is not None]
    print(f"  最高涌现级别 L{max_level} | 全级别中枢 = {zs_total} | 走势段 = {len(moves)}")
    if top:
        print(f"  顶层走势：kind={top.kind} dir={top.direction} settled={top.settled} "
              f"zs_count={top.zs_count}")
    print(f"  买卖点 = {len(bsps)} 个，背驰驱动 = {len(diverg)} 个")
    print("  → 级别解读：bi/低级别趋势=快但不确定的流转；高级别走势=慢但持续的资本流转。")
    print("  → '走势终完美'(缠论)=资本流转不可能无限持续——顶层 settled 趋势接近完美=流转将转向。")


if __name__ == "__main__":
    import sys

    window = int(sys.argv[1]) if len(sys.argv) > 1 else None
    print("资本流转四维度假说验证")
    print(f"小时递归窗口 = {'全量' if window is None else f'最近 {window} 小时bar'}")

    # ── 主残差：跨结算尺 DX·EURUSD^0.576（EURUSD=1/USD6E → −0.576·log USD6E）──
    print("\n" + "#" * 72)
    print("# 主残差 r = log(DX) − 0.576·log(USD6E)  [跨结算尺，33.8% DX方差]")
    print("#" * 72)
    ts, r, va, vb = _aligned_residual_with_volume("dx", "usd6e", 1.0, -_EUR_WEIGHT)
    dim1_direction(ts, r)
    dim2_macd_divergence(ts, r, "DX-residual")
    dim3_flow_leads(ts, r, va, vb, "DX-residual")
    dim4_persistence(ts, r, "DX-residual", window)

    # ── 对照残差：ω = GC/CL（空转/剥削率，§8.5b 已验证有结构）──
    print("\n" + "#" * 72)
    print("# 对照残差 ω = log(GC) − log(CL)  [金油比/空转剥削率，482号]")
    print("#" * 72)
    ts2, r2, va2, vb2 = _aligned_residual_with_volume("gc", "cl", 1.0, -1.0)
    dim1_direction(ts2, r2)
    dim2_macd_divergence(ts2, r2, "omega-GC/CL")
    dim3_flow_leads(ts2, r2, va2, vb2, "omega-GC/CL")
    dim4_persistence(ts2, r2, "omega-GC/CL", window)

    print("\n" + "=" * 72)
    print("四维度验证完成。")
    print("=" * 72)
