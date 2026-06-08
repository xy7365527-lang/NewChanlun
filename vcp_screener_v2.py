# -*- coding: utf-8 -*-
"""
VCP 式「资金放量介入 → 三日超紧缩量蓄势」港美股日线选股器
================================================================

形态定义（全部基于前复权日K线，结算价=收盘价）：

阶段一  资金动能爆发期（锚点设为 T-3）
  情况 A 单日暴风拉升：T-3 为一根放量大阳线，且
          当日 Range(高-低) > atr_burst_mult * ATR14
  情况 B 连续小幅推高：以 T-3 结尾、长度 2~4 天的连续上涨，
          收盘价逐日走高，且每日成交量站上量能基准线（不设 ATR 限制）
  （A 与 B 满足其一即可，A 优先判定）

阶段二  缩量紧密收缩期（固定 3 天：T-2, T-1, T）
  1. 梯级缩量（含锚点）：  V[T-3] > V[T-2] > V[T-1] > V[T]
  2. 极度紧密振幅：       每日 (高-低)/前收 < max_daily_amp（默认 1.5%）
  3. 微幅回撤：           以 T-3 收盘为基准，阶段二最低价回撤 <= max_pullback
                          （默认 3%；微涨/走平则回撤为负，最佳为走平）

设计说明
  - 把成交量梯降条件 V_{T-3}>V_{T-2}>... 落在 T-3 上，所以 T-3 既是阶段一的
    爆发/收尾日，又是缩量阶梯的最高点，二者天然衔接。
  - 量能基准线 vol_ma_base 用 shift(1) 的滚动均量，判定「放量」时不把当日算进去。
  - ATR 默认用 Wilder 平滑；Range 按你给的定义取「高-低」。
  - 核心函数与数据源解耦：只吃标准 OHLCV 的 DataFrame，便于换 yfinance / akshare / 自有数据。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional

import numpy as np
import pandas as pd


# ----------------------------------------------------------------------
# 参数配置
# ----------------------------------------------------------------------
@dataclass
class VCPParams:
    # ATR
    atr_period: int = 14
    atr_method: str = "wilder"          # "wilder" 或 "sma"
    atr_burst_mult: float = 1.5         # 情况A：Range > 此倍数 * ATR

    # 量能
    vol_ma_period: int = 20
    vol_burst_mult: float = 1.5         # 情况A：T-3 量 > 此倍数 * 基准量
    vol_above_mult: float = 1.0         # 情况B：每日量 > 此倍数 * 基准量

    # 阶段一-情况B
    caseB_min_days: int = 2
    caseB_max_days: int = 4

    # 阶段二
    contraction_days: int = 3           # 规则固定3天（保留为参数便于实验）
    max_daily_amp: float = 0.015        # 单日振幅上限 1.5%
    max_pullback: float = 0.03          # 阶段二最大回撤 3%

    # 其它
    yang_only: bool = True              # 情况A是否强制大阳线(收>开)
    require_stepdown: bool = True        # 是否强制梯级缩量


@dataclass
class VCPResult:
    symbol: str
    matched: bool
    case: str = ""                       # "A" / "B(3d)" / ""
    pivot_pos: Optional[int] = None      # T-3 在 df 中的整数位置
    pivot_date: Optional[object] = None  # T-3 日期
    end_date: Optional[object] = None    # T 日期
    pivot_close: float = np.nan
    contraction_amps: List[float] = field(default_factory=list)
    contraction_vols: List[float] = field(default_factory=list)
    pivot_vol: float = np.nan
    pullback: float = np.nan
    notes: str = ""


# ----------------------------------------------------------------------
# 数据预处理
# ----------------------------------------------------------------------
def prepare(df: pd.DataFrame, p: VCPParams) -> pd.DataFrame:
    """
    标准化列名并计算 prev_close / TR / ATR / 单日振幅 / 量能基准线。
    输入 df 需含 open/high/low/close/volume（大小写不限），按时间升序。
    """
    rename = {}
    for c in df.columns:
        lc = str(c).strip().lower()
        if lc in ("open", "high", "low", "close", "volume"):
            rename[c] = lc
        elif lc in ("vol", "turnovervolume"):
            rename[c] = "volume"
        elif lc in ("adj close", "adjclose", "adj_close"):
            rename[c] = "close"   # 若只给了复权收盘
    out = df.rename(columns=rename).copy()

    need = {"open", "high", "low", "close", "volume"}
    missing = need - set(out.columns)
    if missing:
        raise ValueError(f"缺少必要列: {missing}")

    out = out.sort_index()  # 假定 index 为日期；如不是请先 set_index
    for c in ("open", "high", "low", "close", "volume"):
        out[c] = pd.to_numeric(out[c], errors="coerce")

    prev_close = out["close"].shift(1)
    out["prev_close"] = prev_close

    # 真实波幅 TR 与 ATR
    tr = pd.concat([
        out["high"] - out["low"],
        (out["high"] - prev_close).abs(),
        (out["low"] - prev_close).abs(),
    ], axis=1).max(axis=1)
    out["tr"] = tr
    if p.atr_method == "sma":
        out["atr"] = tr.rolling(p.atr_period).mean()
    else:  # Wilder
        out["atr"] = tr.ewm(alpha=1.0 / p.atr_period, adjust=False).mean()
        out.loc[out.index[: p.atr_period - 1], "atr"] = np.nan  # 前期数据不足置空

    # 单日振幅 = (高-低)/前收
    out["amp"] = (out["high"] - out["low"]) / prev_close

    # 量能基准线：滚动均量并 shift(1)，判定放量时不含当日
    out["vol_ma_base"] = out["volume"].rolling(p.vol_ma_period).mean().shift(1)

    return out


# ----------------------------------------------------------------------
# 单点形态判定：以整数位置 iT 作为 T 日
# ----------------------------------------------------------------------
def evaluate_at(prep: pd.DataFrame, iT: int, p: VCPParams,
                symbol: str = "") -> Optional[VCPResult]:
    n = len(prep)
    if iT >= n or iT < 0:
        return None
    iT3 = iT - 3                      # 锚点 T-3
    iT2, iT1, iT0 = iT - 2, iT - 1, iT
    # 需要 T-3 之前还有一根做 prev_close，且各指标非空
    if iT3 - 1 < 0:
        return None

    close = prep["close"].values
    openp = prep["open"].values
    high = prep["high"].values
    low = prep["low"].values
    vol = prep["volume"].values
    atr = prep["atr"].values
    amp = prep["amp"].values
    vbase = prep["vol_ma_base"].values

    # ---- 阶段二：缩量梯降 ----
    vT3, vT2, vT1, vT0 = vol[iT3], vol[iT2], vol[iT1], vol[iT0]
    if np.isnan([vT3, vT2, vT1, vT0]).any():
        return None
    stepdown = (vT3 > vT2 > vT1 > vT0)
    if p.require_stepdown and not stepdown:
        return None

    # ---- 阶段二：紧密振幅 ----
    amps = [amp[iT2], amp[iT1], amp[iT0]]
    if np.isnan(amps).any():
        return None
    tight = all(a < p.max_daily_amp for a in amps)
    if not tight:
        return None

    # ---- 阶段二：微幅回撤（基准 = T-3 收盘）----
    ref = close[iT3]
    if np.isnan(ref) or ref <= 0:
        return None
    min_low = min(low[iT2], low[iT1], low[iT0])
    pullback = (ref - min_low) / ref          # 走平/微涨 -> <=0
    if pullback > p.max_pullback:
        return None

    # ---- 阶段一：情况A 单日暴风拉升 ----
    case = ""
    if not np.isnan(atr[iT3]) and not np.isnan(vbase[iT3]) and vbase[iT3] > 0:
        rngT3 = high[iT3] - low[iT3]
        a_yang = (close[iT3] > openp[iT3]) if p.yang_only else True
        a_range = rngT3 > p.atr_burst_mult * atr[iT3]
        a_vol = vol[iT3] > p.vol_burst_mult * vbase[iT3]
        if a_yang and a_range and a_vol:
            case = "A"

    # ---- 阶段一：情况B 连续小幅推高（A 未命中才判 B）----
    if not case:
        for L in range(p.caseB_max_days, p.caseB_min_days - 1, -1):
            start = iT3 - L + 1
            if start - 1 < 0:
                continue
            seg_close = close[start:iT3 + 1]
            seg_vol = vol[start:iT3 + 1]
            seg_base = vbase[start:iT3 + 1]
            if np.isnan(seg_base).any() or (seg_base <= 0).any():
                continue
            inc = all(seg_close[k] > seg_close[k - 1] for k in range(1, L))
            vol_ok = all(seg_vol[k] > p.vol_above_mult * seg_base[k] for k in range(L))
            if inc and vol_ok:
                case = f"B({L}d)"
                break

    if not case:
        return None

    return VCPResult(
        symbol=symbol,
        matched=True,
        case=case,
        pivot_pos=iT3,
        pivot_date=prep.index[iT3],
        end_date=prep.index[iT0],
        pivot_close=float(ref),
        contraction_amps=[round(float(a), 5) for a in amps],
        contraction_vols=[float(vT2), float(vT1), float(vT0)],
        pivot_vol=float(vT3),
        pullback=round(float(pullback), 5),
        notes="走平最佳" if abs(pullback) < 1e-9 else "",
    )


# ----------------------------------------------------------------------
# 对单只标的：检查最新一根 / 或扫描全部历史位置
# ----------------------------------------------------------------------
def check_symbol(df: pd.DataFrame, p: VCPParams, symbol: str = "",
                 scan_history: bool = False) -> List[VCPResult]:
    prep = prepare(df, p)
    n = len(prep)
    min_need = max(p.atr_period, p.vol_ma_period) + p.caseB_max_days + 3
    if n < min_need:
        return []

    results: List[VCPResult] = []
    if scan_history:
        for iT in range(min_need, n):
            r = evaluate_at(prep, iT, p, symbol)
            if r:
                results.append(r)
    else:
        r = evaluate_at(prep, n - 1, p, symbol)   # T = 最新一根
        if r:
            results.append(r)
    return results


# ----------------------------------------------------------------------
# 批量选股
# ----------------------------------------------------------------------
def screen(data: Dict[str, pd.DataFrame], p: Optional[VCPParams] = None,
           scan_history: bool = False) -> pd.DataFrame:
    """
    data: {symbol: DataFrame(日期index, OHLCV)}
    返回命中明细 DataFrame。
    """
    p = p or VCPParams()
    rows = []
    for sym, df in data.items():
        try:
            for r in check_symbol(df, p, sym, scan_history):
                rows.append({
                    "symbol": r.symbol,
                    "case": r.case,
                    "pivot_date(T-3)": r.pivot_date,
                    "end_date(T)": r.end_date,
                    "pivot_close": r.pivot_close,
                    "amps(T-2,T-1,T)": r.contraction_amps,
                    "pivot_vol": r.pivot_vol,
                    "vols(T-2,T-1,T)": r.contraction_vols,
                    "pullback": r.pullback,
                    "note": r.notes,
                })
        except Exception as e:
            print(f"[跳过] {sym}: {e}")
    cols = ["symbol", "case", "pivot_date(T-3)", "end_date(T)", "pivot_close",
            "amps(T-2,T-1,T)", "pivot_vol", "vols(T-2,T-1,T)", "pullback", "note"]
    return pd.DataFrame(rows, columns=cols)


# ----------------------------------------------------------------------
# 数据源示例：yfinance（港美股通用，auto_adjust=True 近似前复权）
# 港股代码用 4 位 + .HK，例如 0700.HK；美股直接 AAPL
# ----------------------------------------------------------------------
def fetch_yf(symbols: List[str], period: str = "6mo",
             interval: str = "1d") -> Dict[str, pd.DataFrame]:
    import yfinance as yf  # pip install yfinance
    out: Dict[str, pd.DataFrame] = {}
    for s in symbols:
        df = yf.download(s, period=period, interval=interval,
                         auto_adjust=True, progress=False)
        if df is None or df.empty:
            continue
        if isinstance(df.columns, pd.MultiIndex):       # 单票时压平多层列
            df.columns = df.columns.get_level_values(0)
        out[s] = df
    return out


# ======================================================================
# v2：相对波动率收缩检测器（用 YPF 5/18-22、吉利0175.HK 3/19-30 校准）
# ----------------------------------------------------------------------
# v1→v2 的修订史（每条都被真实样本证伪后才放宽）：
#   1. v1「单日振幅<1.5%」误杀全部样本(实测2.7%~6.6%)
#      → v2 改为：收盘价盘整带 coil_max + 每日range<爆发日range(波动率收缩)
#   2. v1「整理恰好3天」误杀(YPF=4天, 吉利=5天)
#      → v2 改为 2~5天可变，长整理优先
#   3. v1「成交量严格逐日梯降」误杀(吉利末日回升0.5%)
#      → v2 改为：缩量到量峰的比例(末日≤40%、均量≤55%)+近单调(允许≤1次小回升)
#   4. v1阶段一「单日大阳/连续小幅」漏掉多日放量主升(吉利212M/231M climax)
#      → v2 改为：枢轴前近3日涨幅达标 + 枢轴为量峰(climax)
#   回撤≤3% 两样本都通过(2.17%/0.90%)，保留。
# ======================================================================
@dataclass
class VCP2Params:
    atr_period: int = 14
    cmin: int = 2                 # 整理段最短天数
    cmax: int = 5                 # 整理段最长天数
    mk_look: int = 3              # 主升涨幅回看天数
    mk_gain: float = 0.05         # 枢轴近mk_look日涨幅下限
    base_win: int = 20            # 量能基准回看
    base_min: int = 5
    vol_mult: float = 1.5         # 量峰≥基准量*此倍数
    dry_last: float = 0.40        # 整理末日量≤量峰*此比例
    dry_mean: float = 0.55        # 整理均量≤量峰*此比例
    up_tol: int = 1               # 整理段量能允许的回升次数
    up_max: float = 0.15          # 单次回升幅度上限
    coil_max: float = 0.04        # 收盘盘整带上限(max-min收盘/枢轴收盘)
    atr_cap: float = 1.7          # 整理日 range/ATR 上限(挡极端宽幅)
    pull_max: float = 0.03        # 回撤上限(枢轴收盘→整理最低)


def _prep2(df: pd.DataFrame, p: VCP2Params) -> pd.DataFrame:
    # 直接复刻必要列，避免与v1的VCPParams耦合
    rename = {c: str(c).strip().lower() for c in df.columns}
    o = df.rename(columns=rename).sort_index().copy()
    for c in ("open", "high", "low", "close", "volume"):
        o[c] = pd.to_numeric(o[c], errors="coerce")
    pc = o["close"].shift(1)
    tr = pd.concat([o["high"] - o["low"], (o["high"] - pc).abs(),
                    (o["low"] - pc).abs()], axis=1).max(axis=1)
    o["atr"] = tr.ewm(alpha=1.0 / p.atr_period, adjust=False).mean()
    o["rng"] = o["high"] - o["low"]
    o["amp"] = (o["high"] - o["low"]) / pc
    return o


def evaluate_v2(o: pd.DataFrame, iT: int, p: VCP2Params,
                symbol: str = "") -> Optional[VCPResult]:
    c = o["close"].values; h = o["high"].values; l = o["low"].values
    v = o["volume"].values; rng = o["rng"].values; atr = o["atr"].values
    amp = o["amp"].values
    for C in range(p.cmax, p.cmin - 1, -1):              # 长整理优先
        ip = iT - C                                       # 整理前最后一根=突破/量峰枢轴
        if ip - p.mk_look - p.base_min < 0:
            continue
        if np.isnan([atr[ip], v[ip], c[ip]]).any():
            continue
        # 1) 主升leg
        if c[ip] / np.min(c[ip - p.mk_look:ip + 1]) - 1 < p.mk_gain:
            continue
        # 2) 量峰
        base = np.nanmean(v[max(0, ip - p.base_win - p.mk_look):ip - p.mk_look])
        if not base or np.isnan(base) or v[ip] < p.vol_mult * base:
            continue
        if v[ip] != np.max(v[ip - 2:iT + 1]):
            continue
        # 3) 缩量 + 近单调
        cv = v[ip + 1:iT + 1]
        if not np.all(cv < v[ip]):
            continue
        if cv[-1] > p.dry_last * v[ip] or cv.mean() > p.dry_mean * v[ip]:
            continue
        ups = [(cv[k] - cv[k - 1]) / cv[k - 1]
               for k in range(1, len(cv)) if cv[k] > cv[k - 1]]
        if len(ups) > p.up_tol or any(u > p.up_max for u in ups):
            continue
        # 4) 收盘盘整带
        cc = c[ip + 1:iT + 1]
        if (cc.max() - cc.min()) / c[ip] > p.coil_max:
            continue
        # 5) 波动率收缩：每日range<爆发日range，且range/ATR有上限
        cr = rng[ip + 1:iT + 1]; ca = atr[ip + 1:iT + 1]
        if not np.all(cr < rng[ip]):
            continue
        if np.any(cr / ca > p.atr_cap):
            continue
        # 6) 回撤
        pull = (c[ip] - np.min(l[ip + 1:iT + 1])) / c[ip]
        if pull > p.pull_max:
            continue
        coil = (cc.max() - cc.min()) / c[ip]
        return VCPResult(
            symbol=symbol, matched=True, case=f"V2(C={C})",
            pivot_pos=ip, pivot_date=o.index[ip], end_date=o.index[iT],
            pivot_close=float(c[ip]),
            contraction_amps=[round(float(a), 5) for a in amp[ip + 1:iT + 1]],
            contraction_vols=[float(x) for x in cv],
            pivot_vol=float(v[ip]), pullback=round(float(pull), 5),
            notes=f"coil{coil*100:.2f}% dry_last{cv[-1]/v[ip]*100:.0f}%",
        )
    return None


def check_symbol2(df: pd.DataFrame, p: VCP2Params, symbol: str = "",
                  scan_history: bool = False) -> List[VCPResult]:
    o = _prep2(df, p)
    n = len(o)
    min_need = p.cmax + p.mk_look + p.base_min      # 评估函数对短前置历史已兜底
    if n < min_need:
        return []
    out: List[VCPResult] = []
    rng_iter = range(min_need, n) if scan_history else [n - 1]
    for iT in rng_iter:
        r = evaluate_v2(o, iT, p, symbol)
        if r:
            out.append(r)
    return out


# ----------------------------------------------------------------------
# 演示：合成一只满足形态的数据，验证脚本可直接运行
# ----------------------------------------------------------------------
def _demo():
    rng = np.random.default_rng(42)
    n = 60
    dates = pd.bdate_range("2025-01-01", periods=n)
    close = 100 + np.cumsum(rng.normal(0, 0.3, n))      # 平缓底部
    high = close + rng.uniform(0.4, 0.8, n)
    low = close - rng.uniform(0.4, 0.8, n)
    openp = close - rng.normal(0, 0.2, n)
    vol = rng.uniform(0.9e6, 1.1e6, n)

    # 人工植入形态：T-3=爆发大阳放量，T-2/T-1/T 缩量+紧密+走平
    iT = n - 1
    iT3 = iT - 3
    base_c = close[iT3 - 1]
    # T-3 大阳放量
    close[iT3] = base_c * 1.05
    openp[iT3] = base_c * 1.005
    low[iT3] = base_c * 1.00
    high[iT3] = base_c * 1.06          # Range≈6%>1.5*ATR
    vol[iT3] = 3.0e6
    # 三日缩量蓄势（价格基本走平，振幅<1.5%）
    for k, pos in enumerate((iT - 2, iT - 1, iT)):
        pc = close[pos - 1]
        close[pos] = pc * (1.000 - 0.001 * k)          # 微跌走平
        high[pos] = close[pos] * 1.006
        low[pos] = close[pos] * 0.995
        openp[pos] = close[pos]
    vol[iT - 2], vol[iT - 1], vol[iT] = 1.8e6, 1.2e6, 0.7e6   # 梯降

    df = pd.DataFrame({"open": openp, "high": high, "low": low,
                       "close": close, "volume": vol}, index=dates)
    res = screen({"DEMO": df}, VCPParams())
    print("=== 演示批量选股结果 ===")
    print(res.to_string(index=False) if not res.empty else "无命中")


if __name__ == "__main__":
    _demo()
    # 实盘用法示例：
    # pool = ["AAPL", "NVDA", "0700.HK", "9988.HK"]
    # data = fetch_yf(pool, period="6mo")
    # print(screen(data, VCPParams()))
    # 回测全历史命中：print(screen(data, VCPParams(), scan_history=True))
