"""Databento 数据源 — 美股 + 期货历史 OHLCV

单一数据源覆盖：
- 期货（CME Globex）: CL, GC, ES, NQ, SI 等 — 1min 数据自 2010 年起
- 美股（NASDAQ）: AMD, NVDA 等 — 1min 数据自 2018 年起

API 输出格式（to_df）：
- index: ts_event (datetime64[ns, UTC])
- 列: open, high, low, close, volume, rtype, publisher_id, instrument_id, symbol
- 期货 stype_in="continuous", symbol 如 "ES.c.0"
- 美股 stype_in="raw_symbol", symbol 如 "AMD"
"""

from __future__ import annotations

import logging
from datetime import datetime, timedelta

import databento as db
import pandas as pd

from newchan.config import DATABENTO_API_KEY

logger = logging.getLogger(__name__)

# ====================================================================
# Symbol → Databento dataset/symbol 映射
# ====================================================================

# 期货：CME Globex MDP 3.0
# 连续合约格式: {ROOT}.{ROLL_RULE}.{RANK}
# c=日历展期, v=成交量展期, n=持仓量展期; 0=主力合约
_FUTURES_MAP: dict[str, str] = {
    "BZ": "BZ.c.0",    # 布伦特原油（CME 影子合约）
    "CL": "CL.c.0",    # WTI 原油
    "GC": "GC.c.0",    # 黄金
    "ES": "ES.c.0",    # 标普 E-mini
    "NQ": "NQ.c.0",    # 纳指 E-mini
    "SI": "SI.c.0",    # 白银
    "YM": "YM.c.0",    # 道指 E-mini
    "RTY": "RTY.c.0",  # 罗素 2000
    "NG": "NG.c.0",    # 天然气
    "HG": "HG.c.0",    # 铜
    "ZB": "ZB.c.0",    # 30年国债
    "ZN": "ZN.c.0",    # 10年国债
}

# 美股：直接用 ticker
_STOCK_TICKERS: set[str] = {
    "AMD", "NVDA", "AAPL", "MSFT", "GOOG", "GOOGL", "AMZN", "META",
    "TSLA", "TSM", "NFLX", "BABA", "SPY", "QQQ", "IWM",
}

# 品种搜索索引：从 core.symbol_catalog 导入（避免循环依赖 databento 包）
from newchan.core.symbol_catalog import SYMBOL_CATALOG


def search_symbols(query: str) -> list[dict]:
    """在品种目录中模糊搜索（匹配 symbol/name/cn/exchange）。"""
    q = query.upper().strip()
    if not q:
        return []
    results = []
    for item in SYMBOL_CATALOG:
        searchable = f"{item['symbol']} {item['name']} {item['cn']} {item['exchange']}".upper()
        if q in searchable:
            results.append(item)
    return results

# interval → Databento schema
_SCHEMA_MAP: dict[str, str] = {
    "1min": "ohlcv-1m",
    "5min": "ohlcv-1m",    # 拉 1min 后 resample
    "15min": "ohlcv-1m",
    "30min": "ohlcv-1m",
    "1hour": "ohlcv-1h",
    "1day": "ohlcv-1d",
}

# 默认标的列表
DEFAULT_SYMBOLS: list[str] = ["CL", "GC", "ES", "NQ", "SI", "AMD", "NVDA"]


# ====================================================================
# 内部工具
# ====================================================================

def _get_client() -> db.Historical:
    if not DATABENTO_API_KEY:
        raise RuntimeError("DATABENTO_API_KEY 未设置。请在 .env 中配置。")
    return db.Historical(key=DATABENTO_API_KEY)


def _is_futures(symbol: str) -> bool:
    return symbol.upper() in _FUTURES_MAP


def _resolve(symbol: str) -> tuple[str, str, str]:
    """返回 (dataset, db_symbol, stype_in)。"""
    sym = symbol.upper()
    if sym in _FUTURES_MAP:
        return "GLBX.MDP3", _FUTURES_MAP[sym], "continuous"
    return "XNAS.ITCH", sym, "raw_symbol"


def _resample_ohlcv(df: pd.DataFrame, target: str) -> pd.DataFrame:
    freq_map = {"5min": "5min", "15min": "15min", "30min": "30min"}
    freq = freq_map.get(target)
    if freq is None:
        return df
    agg = {"open": "first", "high": "max", "low": "min", "close": "last"}
    if "volume" in df.columns:
        agg["volume"] = "sum"
    return df.resample(freq).agg(agg).dropna(subset=["close"])


# ====================================================================
# 公开 API
# ====================================================================

def fetch_ohlcv(
    symbol: str,
    interval: str = "1min",
    start: str = "2020-01-01",
    end: str | None = None,
) -> pd.DataFrame:
    """从 Databento 拉取 OHLCV 数据。

    Parameters
    ----------
    symbol : str
        品种代码，如 "CL", "AMD", "GC" 等。
    interval : str
        K线周期: "1min", "5min", "15min", "30min", "1hour", "1day"
    start : str
        起始日期 "YYYY-MM-DD"
    end : str | None
        结束日期，None 表示到 T-1（免费层不能访问当天数据）

    Returns
    -------
    pd.DataFrame
        标准 OHLCV DataFrame（tz-naive DateTimeIndex + open/high/low/close/volume）。
    """
    dataset, db_symbol, stype_in = _resolve(symbol)
    schema = _SCHEMA_MAP.get(interval, "ohlcv-1m")
    need_resample = interval in ("5min", "15min", "30min")
    if need_resample:
        schema = "ohlcv-1m"

    end_str = _resolve_end_date(end)

    logger.info(
        "Databento: %s (%s/%s stype=%s) schema=%s [%s → %s]",
        symbol, dataset, db_symbol, stype_in, schema, start, end_str,
    )

    df = _fetch_raw_df(dataset, db_symbol, stype_in, schema, start, end_str)
    if df.empty:
        logger.warning("Databento 返回空数据: %s %s", symbol, interval)
        return pd.DataFrame(columns=["open", "high", "low", "close", "volume"])

    df = _normalize_ohlcv(df)

    if need_resample:
        df = _resample_ohlcv(df, interval)

    logger.info("Databento 返回 %d 条 %s %s", len(df), symbol, interval)
    return df


def _resolve_end_date(end: str | None) -> str:
    """免费/历史层不能访问当天数据，默认 T-1。"""
    if end is None:
        return (datetime.utcnow() - timedelta(days=1)).strftime("%Y-%m-%d")
    return end


def _fetch_raw_df(
    dataset: str, db_symbol: str, stype_in: str, schema: str,
    start: str, end_str: str,
) -> pd.DataFrame:
    """调用 Databento API 获取原始 DataFrame。"""
    client = _get_client()
    data = client.timeseries.get_range(
        dataset=dataset,
        symbols=[db_symbol],
        stype_in=stype_in,
        schema=schema,
        start=start,
        end=end_str,
    )
    return data.to_df()


def _normalize_ohlcv(df: pd.DataFrame) -> pd.DataFrame:
    """只保留 OHLCV 列，去时区，去重，排序。"""
    keep = [c for c in ["open", "high", "low", "close", "volume"] if c in df.columns]
    df = df[keep]
    if hasattr(df.index, "tz") and df.index.tz is not None:
        df.index = df.index.tz_localize(None)
    df = df.sort_index()
    df = df[~df.index.duplicated(keep="last")]
    return df


def fetch_and_cache(
    symbol: str,
    interval: str = "1min",
    start: str = "2020-01-01",
    end: str | None = None,
) -> tuple[str, int]:
    """拉取数据并增量追加到缓存。返回 (cache_name, total_rows)。"""
    from newchan.cache import append_df, load_df

    df = fetch_ohlcv(symbol, interval, start, end)
    if df.empty:
        return f"{symbol}_{interval}_raw", 0

    cache_name = f"{symbol}_{interval}_raw"
    append_df(cache_name, df)

    df_cached = load_df(cache_name)
    count = len(df_cached) if df_cached is not None else len(df)
    return cache_name, count
