"""Alpha Vantage 日线数据获取层（统一回测数据源）。

存在论位置
----------
替换 yfinance —— yfinance 日线与 TV 标注坐标系对齐失败（HSI 价格歧义）。
本模块用 Alpha Vantage TIME_SERIES_DAILY_ADJUSTED 拉真实日线 OHLCV，
缓存到 analysis/data_cache/av_<SYM>_daily.json，供递归引擎直接消费。

认识论等级：L1（数据管线，确定性获取 + 缓存）。

约束
----
- AV 免费版 25 次/天，每标的 1 次调用（缓存命中不重复调用）。
- AV 不支持港股/A 股指数、期货 → 用代理 ETF（EWH/FXI/USO），尺度与原标的不同。
"""
from __future__ import annotations

import json
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / "analysis" / "data_cache"
API_KEY = "WP4GIQ6VALD179P3"


def fetch_daily(symbol: str, *, force: bool = False) -> dict:
    """拉单标的 AV 日线（DAILY_ADJUSTED full），缓存优先。

    返回 dict: {symbol, source, dates, opens, highs, lows, closes, adj_closes, volumes}
    其中 dates 升序。
    """
    cache_path = CACHE / f"av_{symbol}_daily.json"
    if cache_path.exists() and not force:
        return json.loads(cache_path.read_text())

    url = (
        "https://www.alphavantage.co/query"
        f"?function=TIME_SERIES_DAILY_ADJUSTED&symbol={symbol}"
        f"&outputsize=full&apikey={API_KEY}"
    )
    with urllib.request.urlopen(url, timeout=60) as r:
        raw = json.load(r)
    ts = raw.get("Time Series (Daily)")
    if not ts:
        note = raw.get("Note") or raw.get("Information") or raw.get("Error Message")
        raise RuntimeError(f"AV 获取 {symbol} 失败: {note or json.dumps(raw)[:300]}")

    dates = sorted(ts.keys())
    out: dict = {
        "symbol": symbol,
        "source": "alphavantage_daily_adjusted",
        "dates": [],
        "opens": [],
        "highs": [],
        "lows": [],
        "closes": [],
        "adj_closes": [],
        "volumes": [],
    }
    for dte in dates:
        row = ts[dte]
        out["dates"].append(dte)
        out["opens"].append(float(row["1. open"]))
        out["highs"].append(float(row["2. high"]))
        out["lows"].append(float(row["3. low"]))
        out["closes"].append(float(row["4. close"]))
        out["adj_closes"].append(float(row["5. adjusted close"]))
        out["volumes"].append(float(row["6. volume"]))

    cache_path.write_text(json.dumps(out))
    return out


def adjusted_ohlc(data: dict) -> dict:
    """用复权因子（adj_close/close）等比缩放 OHLC，保持形态、消除 split/分红跳空。

    缠论结构依赖相对形态（分型/笔/线段），split 造成的跳空会污染包含处理，
    故用复权因子统一缩放 open/high/low，close 直接用 adj_close。
    """
    closes = data["closes"]
    adj = data["adj_closes"]
    n = len(closes)
    factor = [adj[i] / closes[i] if closes[i] else 1.0 for i in range(n)]
    return {
        "dates": data["dates"],
        "opens": [data["opens"][i] * factor[i] for i in range(n)],
        "highs": [data["highs"][i] * factor[i] for i in range(n)],
        "lows": [data["lows"][i] * factor[i] for i in range(n)],
        "closes": adj,
        "volumes": data["volumes"],
    }


if __name__ == "__main__":
    import sys

    syms = sys.argv[1:] or ["QQQ", "EWH", "FXI", "USO"]
    for s in syms:
        try:
            d = fetch_daily(s)
            print(
                f"{s}: n={len(d['dates'])} "
                f"{d['dates'][0]}->{d['dates'][-1]} "
                f"close[-1]={d['closes'][-1]} adj[-1]={d['adj_closes'][-1]}"
            )
        except Exception as e:  # noqa: BLE001
            print(f"{s}: ERROR {e}")
        time.sleep(1)
