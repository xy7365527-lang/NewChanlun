"""从 Databento 拉取 K4 三条独立边的连续期货 1min OHLCV 数据。

ES.c.0 (E-mini S&P 500) → E/$
GC.c.0 (Gold)            → Au/$
CL.c.0 (Crude Oil)       → Oil/$

保存为 columnar JSON 到 analysis/data_cache/。
"""

import json
import sys
from datetime import date
from pathlib import Path

import databento as db

API_KEY = "db-4Cxk3Q35QPXqFFj8snSbqENcwqr4G"
DATA_DIR = Path(__file__).resolve().parent / "data_cache"

SYMBOLS = [
    ("ES.c.0", "es_1m_databento.json", "ES"),
    ("GC.c.0", "gc_1m_databento.json", "GC"),
    ("CL.c.0", "cl_1m_databento.json", "CL"),
]

START = date(2024, 1, 1)
END = date(2026, 6, 5)


def fetch_and_save(client: db.Historical, symbol: str, filename: str, label: str) -> None:
    print(f"  Fetching {label} ({symbol}) ...")
    sys.stdout.flush()

    data = client.timeseries.get_range(
        dataset="GLBX.MDP3",
        symbols=[symbol],
        schema="ohlcv-1m",
        start=START,
        end=END,
        stype_in="continuous",
    )

    df = data.to_df()
    print(f"    Raw rows: {len(df)}")

    df = df[df["volume"] > 0].copy()
    print(f"    After volume>0 filter: {len(df)}")

    result = {
        "symbol": label,
        "dates": [str(ts) for ts in df.index],
        "opens": [float(x) for x in df["open"]],
        "highs": [float(x) for x in df["high"]],
        "lows": [float(x) for x in df["low"]],
        "closes": [float(x) for x in df["close"]],
        "volumes": [int(x) for x in df["volume"]],
    }

    out_path = DATA_DIR / filename
    out_path.write_text(json.dumps(result))
    print(f"    Saved: {out_path.name} ({len(result['closes']):,} bars)")
    print(f"    Range: {result['dates'][0][:19]} → {result['dates'][-1][:19]}")
    sys.stdout.flush()


def main() -> None:
    print("=" * 50)
    print("  Databento K4 期货数据拉取")
    print("=" * 50)

    client = db.Historical(API_KEY)

    for symbol, filename, label in SYMBOLS:
        fetch_and_save(client, symbol, filename, label)
        print()

    print("完成。")


if __name__ == "__main__":
    main()
