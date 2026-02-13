"""技术指标计算 + 指标注册表"""

from __future__ import annotations

from typing import Any

import pandas as pd

# =====================================================================
# 指标计算函数
# =====================================================================


def calc_macd(
    df: pd.DataFrame, fast: int = 12, slow: int = 26, signal: int = 9,
) -> pd.DataFrame:
    """MACD (DIF / DEA / Histogram)。"""
    ema_fast = df["close"].ewm(span=fast, adjust=False).mean()
    ema_slow = df["close"].ewm(span=slow, adjust=False).mean()
    macd_line = ema_fast - ema_slow
    signal_line = macd_line.ewm(span=signal, adjust=False).mean()
    histogram = macd_line - signal_line
    return pd.DataFrame(
        {"macd": macd_line, "signal": signal_line, "histogram": histogram},
        index=df.index,
    )


def calc_sma(df: pd.DataFrame, period: int = 20) -> pd.DataFrame:
    """简单移动平均线。"""
    return pd.DataFrame({"sma": df["close"].rolling(period).mean()}, index=df.index)


def calc_ema(df: pd.DataFrame, period: int = 20) -> pd.DataFrame:
    """指数移动平均线。"""
    return pd.DataFrame(
        {"ema": df["close"].ewm(span=period, adjust=False).mean()}, index=df.index,
    )


def calc_rsi(df: pd.DataFrame, period: int = 14) -> pd.DataFrame:
    """RSI 相对强弱指标。"""
    delta = df["close"].diff()
    gain = delta.clip(lower=0)
    loss = (-delta).clip(lower=0)
    avg_gain = gain.ewm(alpha=1 / period, min_periods=period, adjust=False).mean()
    avg_loss = loss.ewm(alpha=1 / period, min_periods=period, adjust=False).mean()
    rs = avg_gain / avg_loss
    rsi = 100 - 100 / (1 + rs)
    return pd.DataFrame({"rsi": rsi}, index=df.index)


def calc_bollinger(
    df: pd.DataFrame, period: int = 20, std: float = 2.0,
) -> pd.DataFrame:
    """布林带（中轨 / 上轨 / 下轨）。"""
    mid = df["close"].rolling(period).mean()
    band = df["close"].rolling(period).std() * std
    return pd.DataFrame(
        {"bb_mid": mid, "bb_upper": mid + band, "bb_lower": mid - band},
        index=df.index,
    )


# =====================================================================
# 指标注册表
# =====================================================================

# display: "overlay" = 叠加主图, "subchart" = 独立子图面板
# params: 参数定义 [{name, label, default, type}]
# series: 输出序列描述 [{key, color, type(line/histogram)}]

INDICATOR_REGISTRY: dict[str, dict[str, Any]] = {
    "MACD": {
        "func": calc_macd,
        "display": "subchart",
        "params": [
            {"name": "fast", "label": "Fast", "default": 12, "type": "int"},
            {"name": "slow", "label": "Slow", "default": 26, "type": "int"},
            {"name": "signal", "label": "Signal", "default": 9, "type": "int"},
        ],
        "series": [
            {"key": "histogram", "color": None, "type": "histogram"},
            {"key": "macd", "color": "#2962FF", "type": "line"},
            {"key": "signal", "color": "#FF6D00", "type": "line"},
        ],
    },
    "SMA": {
        "func": calc_sma,
        "display": "overlay",
        "params": [
            {"name": "period", "label": "Period", "default": 20, "type": "int"},
        ],
        "series": [
            {"key": "sma", "color": "#E91E63", "type": "line"},
        ],
    },
    "EMA": {
        "func": calc_ema,
        "display": "overlay",
        "params": [
            {"name": "period", "label": "Period", "default": 20, "type": "int"},
        ],
        "series": [
            {"key": "ema", "color": "#FF9800", "type": "line"},
        ],
    },
    "RSI": {
        "func": calc_rsi,
        "display": "subchart",
        "params": [
            {"name": "period", "label": "Period", "default": 14, "type": "int"},
        ],
        "series": [
            {"key": "rsi", "color": "#7B1FA2", "type": "line"},
        ],
    },
    "Bollinger": {
        "func": calc_bollinger,
        "display": "overlay",
        "params": [
            {"name": "period", "label": "Period", "default": 20, "type": "int"},
            {"name": "std", "label": "Std Dev", "default": 2.0, "type": "float"},
        ],
        "series": [
            {"key": "bb_upper", "color": "rgba(33,150,243,0.5)", "type": "line"},
            {"key": "bb_mid", "color": "rgba(33,150,243,0.8)", "type": "line"},
            {"key": "bb_lower", "color": "rgba(33,150,243,0.5)", "type": "line"},
        ],
    },
}


def compute_indicator(
    name: str, df: pd.DataFrame, params: dict | None = None,
) -> pd.DataFrame:
    """通用入口：按名称计算指标。"""
    reg = INDICATOR_REGISTRY.get(name)
    if reg is None:
        raise ValueError(f"未知指标 '{name}'，可用: {list(INDICATOR_REGISTRY.keys())}")
    p = {d["name"]: d["default"] for d in reg["params"]}
    if params:
        for k, v in params.items():
            if k in p:
                p[k] = type(p[k])(v)
    return reg["func"](df, **p)
