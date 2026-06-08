"""A 系统 — MACD 力度指标

MACD 是"指标力度"，不参与结构断言；只作为输出与买卖点/显示依据。
使用 pandas ewm 实现，不依赖 TA-Lib。

OnlineMacdState：增量 EMA 状态机（O(1)/bar），消除全量重算路径。
认识论：L0——EMA 递推关系 EMA_t = α·x_t + (1-α)·EMA_{t-1} 是数学恒等式。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime

import numpy as np
import pandas as pd


def compute_macd(
    df_raw: pd.DataFrame,
    fast: int = 12,
    slow: int = 26,
    signal: int = 9,
) -> pd.DataFrame:
    """计算 MACD 指标。

    Parameters
    ----------
    df_raw : pd.DataFrame
        必须含 ``close`` 列。
    fast, slow, signal : int
        EMA 周期参数。

    Returns
    -------
    pd.DataFrame
        列: ``macd``, ``signal``, ``hist``（hist = macd - signal）。
        index 与 df_raw 相同。
    """
    close = df_raw["close"]
    ema_fast = close.ewm(span=fast, adjust=False).mean()
    ema_slow = close.ewm(span=slow, adjust=False).mean()
    macd_line = ema_fast - ema_slow
    signal_line = macd_line.ewm(span=signal, adjust=False).mean()
    hist = macd_line - signal_line

    return pd.DataFrame({
        "macd": macd_line,
        "signal": signal_line,
        "hist": hist,
    }, index=df_raw.index)


def compute_log_macd(
    df_raw: pd.DataFrame,
    fast: int = 12,
    slow: int = 26,
    signal: int = 9,
) -> pd.DataFrame:
    """在 log(close) 上计算 MACD，保证比价可加性。

    性质：log(A/B) + log(B/C) = log(A/C)，EMA 是线性算子，
    因此 log_macd(A/B).hist + log_macd(B/C).hist = log_macd(A/C).hist。

    Parameters
    ----------
    df_raw : pd.DataFrame
        必须含 ``close`` 列，且 close > 0。
    fast, slow, signal : int
        EMA 周期参数。

    Returns
    -------
    pd.DataFrame
        列: ``macd``, ``signal``, ``hist``。index 与 df_raw 相同。
    """
    log_close = np.log(df_raw["close"])
    ema_fast = log_close.ewm(span=fast, adjust=False).mean()
    ema_slow = log_close.ewm(span=slow, adjust=False).mean()
    macd_line = ema_fast - ema_slow
    signal_line = macd_line.ewm(span=signal, adjust=False).mean()
    hist = macd_line - signal_line

    return pd.DataFrame({
        "macd": macd_line,
        "signal": signal_line,
        "hist": hist,
    }, index=df_raw.index)


class OnlineMacdState:
    """增量 EMA 状态机——每 bar O(1) 更新，消除全量 pandas ewm 重算。

    保留完整历史列表，`to_dataframe()` 返回与 `compute_macd` 等价的 DataFrame，
    供 `macd_area_for_range` 按位置索引做范围查询。

    使用方式：
        state = OnlineMacdState()
        for close, ts in stream:
            state.update(close, ts)
        df = state.to_dataframe()   # 数值等价于 compute_macd(df_raw)，见下

    认识论：算法层 L0（EMA 递推的代数形式恒定）。但与 `compute_macd`
    （pandas ewm adjust=False）**并非 bit-exact**：本类用朴素递推
    `α·x+(1−α)·prev`，而 pandas 编译的 Cython 对 `old_wt*weighted+new_wt*cur`
    做了浮点收缩（FMA），末位舍入不同 → 两路径数值偏差 ~1e-13（实测 macd
    max|Δ|≈8.5e-14, signal≈6.3e-14, hist≈4.1e-14，n=5000）。
    `test_a_macd.TestOnlineMacdState` 用 atol=1e-10 容差断言（非 exact），
    系统既有立场即"容差等价"。背驰力度比较的阈值远粗于 1e-14，故此偏差不影响
    背驰判定的路径不变性。Rust 移植（newchan_rust）对两路径各自忠实复刻其算术
    （compute_macd 用 f64::mul_add 对齐 pandas FMA，OnlineMacdState 用朴素递推），
    各自 bit-exact，但同样保留两路径间的 ~1e-14 偏差。
    """

    def __init__(
        self,
        fast: int = 12,
        slow: int = 26,
        signal: int = 9,
    ) -> None:
        self._alpha_fast = 2.0 / (fast + 1)
        self._alpha_slow = 2.0 / (slow + 1)
        self._alpha_sig = 2.0 / (signal + 1)
        self._ema_fast: float | None = None
        self._ema_slow: float | None = None
        self._signal_val: float | None = None
        # 历史列表，用于 to_dataframe() 的 iloc 范围查询
        self._macd_hist: list[float] = []
        self._signal_hist: list[float] = []
        self._hist_hist: list[float] = []
        self._ts_hist: list[datetime] = []

    def update(self, close: float, ts: datetime) -> tuple[float, float, float]:
        """摄入一根 bar 的 close，返回 (macd, signal, hist)。O(1)。"""
        a_f = self._alpha_fast
        a_s = self._alpha_slow
        a_g = self._alpha_sig

        if self._ema_fast is None:
            self._ema_fast = close
            self._ema_slow = close
        else:
            self._ema_fast = a_f * close + (1 - a_f) * self._ema_fast
            self._ema_slow = a_s * close + (1 - a_s) * self._ema_slow

        macd = self._ema_fast - self._ema_slow

        if self._signal_val is None:
            self._signal_val = macd
        else:
            self._signal_val = a_g * macd + (1 - a_g) * self._signal_val

        hist = macd - self._signal_val

        self._macd_hist.append(macd)
        self._signal_hist.append(self._signal_val)
        self._hist_hist.append(hist)
        self._ts_hist.append(ts)

        return macd, self._signal_val, hist

    def to_dataframe(self) -> pd.DataFrame:
        """返回历史 DataFrame（与 compute_macd 格式完全一致），供 macd_area_for_range 使用。"""
        return pd.DataFrame(
            {
                "macd": self._macd_hist,
                "signal": self._signal_hist,
                "hist": self._hist_hist,
            },
            index=pd.DatetimeIndex(self._ts_hist),
        )

    def reset(self) -> None:
        self._ema_fast = None
        self._ema_slow = None
        self._signal_val = None
        self._macd_hist.clear()
        self._signal_hist.clear()
        self._hist_hist.clear()
        self._ts_hist.clear()

    @property
    def n_bars(self) -> int:
        return len(self._macd_hist)


def macd_area_for_range(
    df_macd: pd.DataFrame,
    raw_i0: int,
    raw_i1: int,
) -> dict:
    """计算指定原始 bar 范围内的 MACD 面积。

    Parameters
    ----------
    df_macd : pd.DataFrame
        由 ``compute_macd`` 返回。
    raw_i0, raw_i1 : int
        原始 df 的位置索引范围 [raw_i0, raw_i1]（闭区间）。

    Returns
    -------
    dict
        ``area_total``, ``area_pos``, ``area_neg``, ``n_bars``
    """
    if raw_i0 < 0:
        raw_i0 = 0
    if raw_i1 >= len(df_macd):
        raw_i1 = len(df_macd) - 1
    if raw_i0 > raw_i1:
        return {"area_total": 0.0, "area_pos": 0.0, "area_neg": 0.0, "n_bars": 0}

    hist = df_macd["hist"].iloc[raw_i0 : raw_i1 + 1]
    area_total = float(hist.sum())
    area_pos = float(hist.clip(lower=0).sum())
    area_neg = float(hist.clip(upper=0).sum())

    return {
        "area_total": round(area_total, 6),
        "area_pos": round(area_pos, 6),
        "area_neg": round(area_neg, 6),
        "n_bars": len(hist),
    }
