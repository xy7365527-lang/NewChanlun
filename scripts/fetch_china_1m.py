#!/usr/bin/env python3
"""中国市场 1min OHLCV 抓取（累积式）。

目标标的：
  - IF（沪深300股指期货）  → 新浪主连 'IF0'
  - SC（上海原油期货）      → 新浪主连 'SC0'
  - AU9999（黄金现货, SGE） → 1min 无免费源；用 SHFE 沪金主连 'AU0' 作为 1min 代理，
                              另存 SGE Au99.99 现货日线（该标的唯一可得的真实历史）。

数据源边界（硬约束，非可绕过的矛盾）：
  - 新浪期货分时（futures_zh_minute_sina）对**所有周期**只返回最近 1023 根 K 线
    （≈4 个交易日）。这是免费源的客观上限，无翻页参数。
  - 东方财富 1min（*_hist_min_em）本身只存最近 ~5 天，且本机经 Clash 代理 / 直连
    均无法连通 push2.eastmoney.com（连接被重置）。
  - AU9999 是上海黄金交易所现货，无任何免费 1min 接口（spot_hist_sge 仅日线）。

因此"尽可能长"通过**滚动累积**实现：每次运行把新拉的 1023 根与已有缓存按时间戳
去重合并。脚本可挂 cron 定期跑，历史随时间真实增长。

输出（analysis/data_cache/，列式格式与 *_1m_databento_10y.json 一致）：
  - if_1m_sina.json      {symbol,dates,opens,highs,lows,closes,volumes}
  - sc_1m_sina.json
  - au_1m_sina.json      （AU0 沪金期货，作为 AU9999 的 1min 代理）
  - au9999_1d_sge.json   （SGE Au99.99 现货日线，真实标的全历史）

时间戳：保留新浪原始的中国本地时间，标注真实时区 +08:00（不伪造 UTC）。

用法：
  PYTHONPATH=src .venv/bin/python scripts/fetch_china_1m.py
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

import akshare as ak

_ROOT = Path(__file__).resolve().parents[1]
CACHE_DIR = _ROOT / "analysis" / "data_cache"

# 新浪主连 1min 标的 → 输出文件名
MIN_TARGETS = {
    "IF0": ("if_1m_sina.json", "IF"),    # 沪深300股指期货
    "SC0": ("sc_1m_sina.json", "SC"),    # 上海原油期货
    "AU0": ("au_1m_sina.json", "AU"),    # 沪金期货（AU9999 现货的 1min 代理）
}

_TZ = "+08:00"  # Asia/Shanghai，新浪期货时间戳的真实时区


def _columnar_load(path: Path) -> dict[str, list]:
    """读取已有列式缓存；不存在则返回空骨架。"""
    if not path.exists():
        return {"dates": [], "opens": [], "highs": [], "lows": [], "closes": [], "volumes": []}
    with path.open() as fh:
        return json.load(fh)


def _columnar_merge(existing: dict, df, symbol: str) -> tuple[dict, int]:
    """把新 DataFrame 按时间戳去重合并进已有列式数据。

    返回 (合并后的列式 dict, 新增条数)。不可变：构造新 dict，不原地改 existing。
    """
    # 已有时间戳集合（去重键）
    seen = dict(zip(existing["dates"], range(len(existing["dates"]))))

    # 新数据：datetime → 行
    new_rows: dict[str, dict] = {}
    for rec in df.to_dict("records"):
        ts = f"{rec['datetime']}{_TZ}"
        new_rows[ts] = {
            "open": float(rec["open"]),
            "high": float(rec["high"]),
            "low": float(rec["low"]),
            "close": float(rec["close"]),
            "volume": int(rec["volume"]),
        }

    # 合并：已有行 + 新行（新行覆盖同戳已有，时间戳为唯一键），再按时间排序
    merged: dict[str, dict] = {}
    for i, ts in enumerate(existing["dates"]):
        merged[ts] = {
            "open": existing["opens"][i],
            "high": existing["highs"][i],
            "low": existing["lows"][i],
            "close": existing["closes"][i],
            "volume": existing["volumes"][i],
        }
    added = 0
    for ts, row in new_rows.items():
        if ts not in seen:
            added += 1
        merged[ts] = row

    ordered = sorted(merged.keys())
    out = {
        "symbol": symbol,
        "dates": ordered,
        "opens": [merged[t]["open"] for t in ordered],
        "highs": [merged[t]["high"] for t in ordered],
        "lows": [merged[t]["low"] for t in ordered],
        "closes": [merged[t]["close"] for t in ordered],
        "volumes": [merged[t]["volume"] for t in ordered],
    }
    return out, added


def fetch_minute(sina_symbol: str, fname: str, out_symbol: str) -> None:
    path = CACHE_DIR / fname
    try:
        df = ak.futures_zh_minute_sina(symbol=sina_symbol, period="1")
    except Exception as e:  # noqa: BLE001 — 数据源错误需显式上报，不静默吞掉
        print(f"  [{sina_symbol}] 抓取失败: {type(e).__name__}: {e}", file=sys.stderr)
        return
    if df is None or len(df) == 0:
        print(f"  [{sina_symbol}] 返回空数据", file=sys.stderr)
        return

    existing = _columnar_load(path)
    merged, added = _columnar_merge(existing, df, out_symbol)
    with path.open("w") as fh:
        json.dump(merged, fh, ensure_ascii=False)

    n = len(merged["dates"])
    rng = f"{merged['dates'][0]} → {merged['dates'][-1]}" if n else "(空)"
    print(f"  [{sina_symbol}] 本次拉取 {len(df)} 根，新增 {added}，"
          f"累积 {n} 根 [{rng}] → {fname}")


def fetch_au9999_daily() -> None:
    """SGE Au99.99 现货日线（AU9999 真实标的的唯一可得历史）。"""
    path = CACHE_DIR / "au9999_1d_sge.json"
    try:
        df = ak.spot_hist_sge(symbol="Au99.99")
    except Exception as e:  # noqa: BLE001
        print(f"  [Au99.99 日线] 抓取失败: {type(e).__name__}: {e}", file=sys.stderr)
        return
    # SGE 日线列：date, open, close, low, high（无成交量）
    df = df.dropna(subset=["open", "high", "low", "close"])
    out = {
        "symbol": "AU9999",
        "dates": [f"{d}" for d in df["date"].astype(str).tolist()],
        "opens": [float(x) for x in df["open"].tolist()],
        "highs": [float(x) for x in df["high"].tolist()],
        "lows": [float(x) for x in df["low"].tolist()],
        "closes": [float(x) for x in df["close"].tolist()],
        "volumes": [0 for _ in range(len(df))],  # SGE 日线无成交量，占位 0
    }
    with path.open("w") as fh:
        json.dump(out, fh, ensure_ascii=False)
    n = len(out["dates"])
    rng = f"{out['dates'][0]} → {out['dates'][-1]}" if n else "(空)"
    print(f"  [Au99.99 日线] {n} 根 [{rng}] → au9999_1d_sge.json")


def main() -> None:
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    print("== 中国市场 1min 抓取（新浪主连，1023 根滚动累积）==")
    for sina_symbol, (fname, out_symbol) in MIN_TARGETS.items():
        fetch_minute(sina_symbol, fname, out_symbol)
        time.sleep(1)  # 礼貌限速
    print("== AU9999 现货日线（SGE，真实标的全历史）==")
    fetch_au9999_daily()


if __name__ == "__main__":
    main()
