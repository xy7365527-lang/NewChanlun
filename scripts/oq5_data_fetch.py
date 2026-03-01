"""OQ5 历史数据获取——从 yfinance 拉取汇率/黄金/美元指数数据。

269号谱系验证需求：
  1. 2002-2007 反例验证：DXY 120→70，年线/季线 D 算子读数
  2. 1971 布雷顿森林崩溃：三重联立验证
  3. 多序列级别对齐：三个资产 D 算子产出联合状态

数据源：yfinance（免费，无需 API key）
数据范围：各品种最大可用范围

认识论等级：L2（真实数据验证的数据准备阶段）
"""

from __future__ import annotations

import json
import logging
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

import pandas as pd
import yfinance as yf

logger = logging.getLogger(__name__)

# ── 品种配置 ──────────────────────────────────────────────────

SYMBOLS: dict[str, dict] = {
    "DX-Y.NYB": {
        "name": "USD Index (DXY)",
        "role": "center",  # 中心货币
        "description": "美元指数——衡量美元相对一篮子货币的强弱",
    },
    "GC=F": {
        "name": "Gold Futures (XAU/USD proxy)",
        "role": "gold",  # 法币/黄金
        "description": "黄金期货——法币/黄金通道",
    },
    "USDJPY=X": {
        "name": "USD/JPY",
        "role": "periphery",  # 外围
        "description": "美元/日元——亚洲外围",
    },
    "GBPUSD=X": {
        "name": "GBP/USD",
        "role": "periphery",
        "description": "英镑/美元——欧洲外围",
    },
    "EURUSD=X": {
        "name": "EUR/USD",
        "role": "periphery",
        "description": "欧元/美元——欧洲外围（1999年起）",
    },
    "USDCHF=X": {
        "name": "USD/CHF",
        "role": "periphery",
        "description": "美元/瑞郎——避险外围",
    },
}

# ── 缓存路径 ──────────────────────────────────────────────────

CACHE_DIR = Path(__file__).resolve().parents[1] / ".cache" / "oq5"


@dataclass(frozen=True, slots=True)
class FetchResult:
    """单品种数据获取结果。"""

    symbol: str
    name: str
    daily_bars: int
    monthly_bars: int
    yearly_bars: int
    date_start: str
    date_end: str
    daily_path: str
    monthly_path: str
    yearly_path: str


def _aggregate_to_period(
    daily: pd.DataFrame,
    rule: str,
) -> pd.DataFrame:
    """从日线聚合到指定周期。

    Parameters
    ----------
    daily : pd.DataFrame
        日线数据（columns: Open, High, Low, Close, Volume）。
    rule : str
        pandas resample 规则（'ME' 月线, 'QE' 季线, 'YE' 年线）。

    Returns
    -------
    pd.DataFrame
        聚合后的 OHLCV 数据。
    """
    agg = daily.resample(rule).agg({
        "Open": "first",
        "High": "max",
        "Low": "min",
        "Close": "last",
        "Volume": "sum",
    })
    return agg.dropna(subset=["Open", "Close"])


def fetch_symbol(symbol: str, cache_dir: Path) -> FetchResult | None:
    """获取单个品种的完整历史数据并缓存。

    Returns
    -------
    FetchResult | None
        成功返回结果，失败返回 None。
    """
    meta = SYMBOLS.get(symbol, {"name": symbol, "role": "unknown"})
    name = meta["name"]

    try:
        ticker = yf.Ticker(symbol)
        daily = ticker.history(period="max", interval="1d")
    except Exception as e:
        logger.error("Failed to fetch %s: %s", symbol, e)
        return None

    if daily.empty:
        logger.warning("No data for %s", symbol)
        return None

    # 去除时区信息以便统一处理
    if daily.index.tz is not None:
        daily.index = daily.index.tz_localize(None)

    # 聚合
    monthly = _aggregate_to_period(daily, "ME")
    quarterly = _aggregate_to_period(daily, "QE")
    yearly = _aggregate_to_period(daily, "YE")

    # 保存
    cache_dir.mkdir(parents=True, exist_ok=True)
    safe_name = symbol.replace("=", "_").replace(".", "_").replace("-", "_")

    daily_path = cache_dir / f"{safe_name}_daily.parquet"
    monthly_path = cache_dir / f"{safe_name}_monthly.parquet"
    quarterly_path = cache_dir / f"{safe_name}_quarterly.parquet"
    yearly_path = cache_dir / f"{safe_name}_yearly.parquet"

    daily.to_parquet(daily_path)
    monthly.to_parquet(monthly_path)
    quarterly.to_parquet(quarterly_path)
    yearly.to_parquet(yearly_path)

    logger.info(
        "%s: %d daily, %d monthly, %d yearly bars (%s to %s)",
        name, len(daily), len(monthly), len(yearly),
        daily.index[0].date(), daily.index[-1].date(),
    )

    return FetchResult(
        symbol=symbol,
        name=name,
        daily_bars=len(daily),
        monthly_bars=len(monthly),
        yearly_bars=len(yearly),
        date_start=str(daily.index[0].date()),
        date_end=str(daily.index[-1].date()),
        daily_path=str(daily_path),
        monthly_path=str(monthly_path),
        yearly_path=str(yearly_path),
    )


def fetch_all(cache_dir: Path = CACHE_DIR) -> list[FetchResult]:
    """获取所有品种数据。"""
    results = []
    for symbol in SYMBOLS:
        result = fetch_symbol(symbol, cache_dir)
        if result is not None:
            results.append(result)
    return results


def load_cached(
    symbol: str,
    timeframe: str = "monthly",
    cache_dir: Path = CACHE_DIR,
) -> pd.DataFrame | None:
    """加载缓存的数据。

    Parameters
    ----------
    symbol : str
        品种代码。
    timeframe : str
        'daily', 'monthly', 'quarterly', 'yearly'。
    cache_dir : Path
        缓存目录。

    Returns
    -------
    pd.DataFrame | None
    """
    safe_name = symbol.replace("=", "_").replace(".", "_").replace("-", "_")
    path = cache_dir / f"{safe_name}_{timeframe}.parquet"
    if not path.exists():
        return None
    return pd.read_parquet(path)


def main() -> None:
    """CLI 入口：获取所有数据并打印摘要。"""
    logging.basicConfig(level=logging.INFO, format="%(message)s")

    print("=" * 70)
    print("OQ5 历史数据获取")
    print("=" * 70)

    results = fetch_all()

    print("\n" + "=" * 70)
    print("数据摘要")
    print("=" * 70)

    for r in results:
        print(
            f"  {r.name:30s} | "
            f"daily={r.daily_bars:6d} | "
            f"monthly={r.monthly_bars:4d} | "
            f"yearly={r.yearly_bars:3d} | "
            f"{r.date_start} ~ {r.date_end}"
        )

    # 验证窗口覆盖情况
    print("\n" + "=" * 70)
    print("验证窗口覆盖")
    print("=" * 70)

    for r in results:
        start_year = int(r.date_start[:4])
        covers_1971 = start_year <= 1971
        covers_2002 = start_year <= 2002
        print(
            f"  {r.name:30s} | "
            f"1971 窗口: {'覆盖' if covers_1971 else '不覆盖':4s} | "
            f"2002-2007 窗口: {'覆盖' if covers_2002 else '不覆盖':4s}"
        )

    # 保存摘要
    summary = {
        "fetch_time": datetime.now().isoformat(),
        "results": [
            {
                "symbol": r.symbol,
                "name": r.name,
                "daily_bars": r.daily_bars,
                "monthly_bars": r.monthly_bars,
                "yearly_bars": r.yearly_bars,
                "date_start": r.date_start,
                "date_end": r.date_end,
            }
            for r in results
        ],
    }
    summary_path = CACHE_DIR / "fetch_summary.json"
    summary_path.write_text(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"\n摘要已保存: {summary_path}")


if __name__ == "__main__":
    main()
