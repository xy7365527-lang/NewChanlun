"""拉取 GC（黄金）前月合约 1min OHLCV 并拼接为连续序列。

Databento GC.c.0 连续合约 OHLCV-1m 数据稀疏，改为逐合约月拉取。
"""

import json
import sys
from datetime import date
from pathlib import Path

import databento as db
import pandas as pd

API_KEY = "db-4Cxk3Q35QPXqFFj8snSbqENcwqr4G"
DATA_DIR = Path(__file__).resolve().parent / "data_cache"

CONTRACTS = [
    ("GCG4", date(2024, 1, 1),  date(2024, 1, 29)),
    ("GCJ4", date(2024, 1, 29), date(2024, 3, 26)),
    ("GCM4", date(2024, 3, 26), date(2024, 5, 29)),
    ("GCQ4", date(2024, 5, 29), date(2024, 7, 29)),
    ("GCV4", date(2024, 7, 29), date(2024, 9, 26)),
    ("GCZ4", date(2024, 9, 26), date(2024, 11, 25)),
    ("GCG5", date(2024, 11, 25), date(2025, 1, 29)),
    ("GCJ5", date(2025, 1, 29), date(2025, 3, 27)),
    ("GCM5", date(2025, 3, 27), date(2025, 5, 28)),
    ("GCQ5", date(2025, 5, 28), date(2025, 7, 29)),
    ("GCV5", date(2025, 7, 29), date(2025, 9, 26)),
    ("GCZ5", date(2025, 9, 26), date(2025, 11, 25)),
    ("GCG6", date(2025, 11, 25), date(2026, 1, 28)),
    ("GCJ6", date(2026, 1, 28), date(2026, 3, 27)),
    ("GCM6", date(2026, 3, 27), date(2026, 6, 5)),
]


def main() -> None:
    print("=" * 50)
    print("  GC 前月合约逐月拉取 + 拼接")
    print("=" * 50)
    sys.stdout.flush()

    client = db.Historical(API_KEY)
    frames: list[pd.DataFrame] = []

    for sym, start, end in CONTRACTS:
        print(f"  {sym} ({start} → {end}) ...", end=" ")
        sys.stdout.flush()
        data = client.timeseries.get_range(
            dataset="GLBX.MDP3",
            symbols=[sym],
            schema="ohlcv-1m",
            start=start,
            end=end,
            stype_in="raw_symbol",
        )
        df = data.to_df()
        df = df[df["volume"] > 0].copy()
        print(f"{len(df)} bars")
        sys.stdout.flush()
        frames.append(df)

    combined = pd.concat(frames).sort_index()
    combined = combined[~combined.index.duplicated(keep="first")]
    print(f"\n  拼接后: {len(combined):,} bars")
    print(f"  Range: {combined.index[0]} → {combined.index[-1]}")

    result = {
        "symbol": "GC",
        "dates": [str(ts) for ts in combined.index],
        "opens": [float(x) for x in combined["open"]],
        "highs": [float(x) for x in combined["high"]],
        "lows": [float(x) for x in combined["low"]],
        "closes": [float(x) for x in combined["close"]],
        "volumes": [int(x) for x in combined["volume"]],
    }

    out = DATA_DIR / "gc_1m_databento.json"
    out.write_text(json.dumps(result))
    print(f"  Saved: {out.name}")
    print(f"  Price: {result['closes'][0]:.2f} → {result['closes'][-1]:.2f}")


if __name__ == "__main__":
    main()
