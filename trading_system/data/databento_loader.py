"""历史数据加载（设计 §2.2 两条路，均已实装）。

路 A（首选）：Databento API → DBN 文件 → DatabentoDataLoader → ParquetDataCatalog。
    实装：databento_catalog.py（拉取+缓存+写入+验证 CLI）；本文件
    load_dbn_to_catalog 为薄委托。仓库旧有 databento 10y 数据是 opens 数组
    格式（非 DBN），不走此路。
路 B（fallback / BTC 恒走）：自有数组/parquet → Bar 构造器。
    风险：精度/字段口径自己负责，与路 A 产物**不可混表**。
"""

from __future__ import annotations

from pathlib import Path

import pandas as pd
from nautilus_trader.model.data import Bar, BarType
from nautilus_trader.model.objects import Price, Quantity


def bars_from_dataframe(
    df: pd.DataFrame,
    bar_type: BarType,
    price_precision: int,
    size_precision: int = 0,
) -> list[Bar]:
    """路 B：OHLCV DataFrame（DatetimeIndex）→ list[Bar]。

    要求：index 为 ts_event（DatetimeIndex），列含 open/high/low/close/volume。
    ts_init = ts_event（历史数据无接收延迟概念）。
    NaN 行必删（在册纪律：BRN 0.68%/DX 0.24% nan 必删）。
    """
    df = df.dropna(subset=["open", "high", "low", "close"])
    ts_ns = df.index.view("int64")
    opens = df["open"].to_numpy()
    highs = df["high"].to_numpy()
    lows = df["low"].to_numpy()
    closes = df["close"].to_numpy()
    volumes = (
        df["volume"].to_numpy() if "volume" in df.columns else [0.0] * len(df)
    )

    bars: list[Bar] = []
    for i in range(len(df)):
        ts = int(ts_ns[i])
        bars.append(
            Bar(
                bar_type=bar_type,
                open=Price(float(opens[i]), price_precision),
                high=Price(float(highs[i]), price_precision),
                low=Price(float(lows[i]), price_precision),
                close=Price(float(closes[i]), price_precision),
                volume=Quantity(max(0.0, float(volumes[i])), size_precision),
                ts_event=ts,
                ts_init=ts,
            )
        )
    return bars


def load_parquet_bars(
    path: str | Path,
    bar_type: BarType,
    price_precision: int,
    max_bars: int | None = None,
    tz_assume_utc: bool = True,
) -> list[Bar]:
    """从仓库现有 parquet（如 .cache/BZ_1min_2024_raw.parquet）构造 Bars。"""
    df = pd.read_parquet(path)
    if tz_assume_utc and df.index.tz is None:
        df.index = df.index.tz_localize("UTC")
    if max_bars is not None:
        df = df.iloc[:max_bars]
    return bars_from_dataframe(df, bar_type, price_precision)


def load_dbn_to_catalog(
    definition_dbn_path: str | Path,
    data_dbn_paths: list[str | Path],
    catalog_path: str | Path,
) -> dict:
    """路 A：DBN 文件 → ParquetDataCatalog（实装在 databento_catalog.write_catalog）。

    definition 文件必须先解码——它同时填充 loader 的价格精度缓存，
    bars 解码依赖该缓存（DBN bar 记录本身不带精度）。

    Databento 时间戳口径（已确认）：DBN 原始 ts_event 在 open 时刻，
    Nautilus decoder 归一化到 close 时刻——零前视，无需额外处理。

    端到端管线（API 拉取 + 缓存 + 写入 + 验证）用 CLI：
        .venv/bin/python trading_system/data/databento_catalog.py --help
    """
    from trading_system.data.databento_catalog import write_catalog

    return write_catalog(
        Path(definition_dbn_path),
        [Path(p) for p in data_dbn_paths],
        Path(catalog_path),
    )
