"""从 Databento 拉取 PINS（Pinterest, NYSE 上市）1min OHLCV 全历史。

dataset = XNAS.ITCH（Nasdaq 全市场，含 NYSE 上市股票的 Nasdaq 场内成交；
1min 套餐免费先例：纵向圈闭合实测 Mag7）。PINS 自 2019-04-18 IPO 至今
无拆股 ⇒ 原始未复权价可直接作回测基底（Mag7 拆股复权陷阱不适用）。

保存为 columnar JSON（fetch_k4_futures 同 schema，load_ohlc parallel-array
分支直接消费）。
"""

import json
import sys
from datetime import date
from pathlib import Path

import databento as db

import os as _os
_API_KEY = _os.environ.get("DATABENTO_API_KEY") or _os.environ.get("DATABENTO_KEY")
if not _API_KEY:
    raise RuntimeError("DATABENTO_API_KEY / DATABENTO_KEY 环境变量未设置，拒绝执行")
DATA_DIR = Path(__file__).resolve().parent / "data_cache"
OUT = DATA_DIR / "pins_1m_databento_full.json"

START = date(2019, 4, 18)  # IPO 日
END = date(2026, 6, 10)


def main() -> None:
    client = db.Historical(_API_KEY)
    print(f"Fetching PINS XNAS.ITCH ohlcv-1m [{START} → {END}] ...", flush=True)
    data = client.timeseries.get_range(
        dataset="XNAS.ITCH",
        symbols=["PINS"],
        schema="ohlcv-1m",
        start=START,
        end=END,
    )
    df = data.to_df()
    print(f"  Raw rows: {len(df)}", flush=True)
    df = df[df["volume"] > 0].copy()
    print(f"  After volume>0 filter: {len(df)}", flush=True)
    result = {
        "symbol": "PINS",
        "dates": [str(ts) for ts in df.index],
        "opens": [float(x) for x in df["open"]],
        "highs": [float(x) for x in df["high"]],
        "lows": [float(x) for x in df["low"]],
        "closes": [float(x) for x in df["close"]],
        "volumes": [int(x) for x in df["volume"]],
    }
    OUT.write_text(json.dumps(result))
    print(f"  Saved: {OUT.name} ({len(result['closes']):,} bars)")
    print(f"  Range: {result['dates'][0][:19]} → {result['dates'][-1][:19]}")
    sys.stdout.flush()


if __name__ == "__main__":
    main()
