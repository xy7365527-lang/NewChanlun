#!/usr/bin/env python3
"""全市场 K4 日线长史抓取 — yfinance（免费全史，覆盖 10+ 年宏观周期）。

日线是"必须 10 年+"层，yfinance 日线无范围限制（多数标的可回溯 15-25 年），
且能覆盖 Databento 无 dataset 的中国本土品种（沪深300/在岸人民币等）。

输出格式与 analysis/data_cache/*_databento.json 一致：
    {symbol, dates, opens, highs, lows, closes, volumes}
dates 为 ISO 日期字符串。文件名 {key}_1d_yf.json。

宏观周期覆盖目标：2016牛 / 2018跌 / 2019-20 covid / 2021牛 / 2022加息 / 2023-26。

用法：
    python scripts/fetch_k4_daily_yf.py            # 抓全部
    python scripts/fetch_k4_daily_yf.py <key>...   # 抓指定
"""

from __future__ import annotations

import json
import sys
import warnings
from pathlib import Path

warnings.filterwarnings("ignore")

import yfinance as yf

_ROOT = Path(__file__).resolve().parents[1]
CACHE_DIR = _ROOT / "analysis" / "data_cache"

# key → (yfinance ticker, symbol 名, 中文标签)
TICKERS: dict[str, tuple[str, str, str]] = {
    # 中国
    "csi300":   ("000300.SS", "CSI300", "沪深300指数 (仅2021+)"),
    "ashr":     ("ASHR",      "ASHR",   "沪深300 US-ETF 代理 (长史)"),
    "fxi":      ("FXI",       "FXI",    "中国大盘 US-ETF"),
    "usdcny":   ("CNY=X",     "USDCNY", "在岸人民币 USD/CNY"),
    # 欧洲
    "stoxx50":  ("^STOXX50E", "STOXX50", "欧洲斯托克50"),
    "brent":    ("BZ=F",      "BRENT",  "Brent 原油期货"),
    "eurusd":   ("EURUSD=X",  "EURUSD", "欧元/美元"),
    # 日本
    "n225":     ("^N225",     "N225",   "日经225指数"),
    "usdjpy":   ("JPY=X",     "USDJPY", "美元/日元"),
    # 美国 / 全球锚
    "spx":      ("^GSPC",     "SPX",    "标普500指数"),
    "spy":      ("SPY",       "SPY",    "标普500 ETF"),
    "wti":      ("CL=F",      "WTI",    "WTI 原油期货 (SC原油代理)"),
    "gold":     ("GC=F",      "GOLD",   "COMEX 黄金 (AU9999/伦敦金/东京金 代理)"),
    "gld":      ("GLD",       "GLD",    "黄金 ETF"),
    "uso":      ("USO",       "USO",    "原油 ETF"),
    "uup":      ("UUP",       "UUP",    "美元指数 ETF"),
}


def fetch_one(key: str, ticker: str, symbol: str, label: str) -> bool:
    h = yf.Ticker(ticker).history(period="max", interval="1d", auto_adjust=False)
    if h is None or len(h) == 0:
        print(f"[{key}] ✗ {label} ({ticker}) 无数据", flush=True)
        return False

    dates = [str(d.date()) for d in h.index]
    payload = {
        "symbol": symbol,
        "dates": dates,
        "opens": h["Open"].astype(float).tolist(),
        "highs": h["High"].astype(float).tolist(),
        "lows": h["Low"].astype(float).tolist(),
        "closes": h["Close"].astype(float).tolist(),
        "volumes": h["Volume"].fillna(0).astype("int64").tolist(),
    }
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    out = CACHE_DIR / f"{key}_1d_yf.json"
    with open(out, "w") as f:
        json.dump(payload, f)
    print(f"[{key}] ✓ {label} ({ticker}) {len(dates)} 条 | "
          f"{dates[0]} → {dates[-1]} | {out.name}", flush=True)
    return True


def main() -> int:
    keys = sys.argv[1:] if len(sys.argv) > 1 else list(TICKERS)
    ok = 0
    for k in keys:
        spec = TICKERS.get(k)
        if not spec:
            print(f"[{k}] 未知 key。可选: {', '.join(TICKERS)}")
            continue
        try:
            ok += fetch_one(k, *spec)
        except Exception as e:
            print(f"[{k}] ✗ 失败: {e!r}", flush=True)
    print(f"=== 日线完成 {ok}/{len(keys)} ===", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
