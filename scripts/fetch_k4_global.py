#!/usr/bin/env python3
"""全市场 K4 数据抓取 — Databento 多交易所 1min OHLCV。

覆盖美国之外的 K4 标的（欧洲 / 日本 / 跨国汇率），数据源 Databento。
输出格式与 analysis/data_cache/es_1m_databento.json 完全一致：
    {symbol, dates, opens, highs, lows, closes, volumes}
dates 为 UTC tz-aware 字符串（与现有美国 K4 文件对齐）。

数据源边界（Databento 账户可访问 dataset，2026-06 实测）：
- GLBX.MDP3  (CME Globex)   → NKD / 6E / 6J / CNH
- IFEU.IMPACT(ICE Europe)   → BRN (真·ICE 布伦特)
- XEUR.EOBI  (Eurex)        → FESX (STOXX50)
不可访问（无对应 dataset）：CFFEX(沪深300) / SGE(AU9999) / INE(SC原油) /
现货伦敦金 / TOCOM(东京金) / 在岸 USDCNY。

多分辨率（10年宏观周期覆盖）：
- 1m  : CME 自 2010、IFEU(BRN) 自 2018-12（用户预期 2-3 年即可）
- 1h  : 同上，10年盘中结构
- 1d  : 同上（更长日线优先用 yfinance，见 fetch_k4_daily_yf.py）

用法：
    PYTHONPATH=src python scripts/fetch_k4_global.py <key> [interval] [start]
    PYTHONPATH=src python scripts/fetch_k4_global.py all [interval] [start]
    PYTHONPATH=src python scripts/fetch_k4_global.py --list
  interval ∈ {1m,1h,1d}（默认 1m）；start 默认 2024-01-01（1m）。
  例：fetch_k4_global.py nkd 1h 2016-01-01
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_ROOT / "src"))

from dotenv import load_dotenv

load_dotenv(_ROOT / ".env")

import os

import databento as db

CACHE_DIR = _ROOT / "analysis" / "data_cache"
DEFAULT_START = "2024-01-01"  # 与现有 es/gc/cl 美国 K4 对齐


@dataclass(frozen=True)
class K4Spec:
    key: str          # CLI 选择键 / 输出文件前缀
    symbol: str       # 写入 JSON 的逻辑符号名
    dataset: str
    db_symbol: str    # Databento 连续合约符号
    label: str        # 人类可读标签
    earliest: str     # dataset 该 schema 可用的最早日期（避免 422）
    invert: bool = False  # True → 对 OHLC 取倒数（6J JPY/USD → USDJPY）


# ⚠️ 连续合约用 v（成交量展期）非 c（日历展期）：实测 GC.c.0 仅 631 bars/月，
#    GC.v.0 27485 bars/月；6E/6J.c.0 仅 .v.0 的一半。c 对实物商品会选中低流动性
#    临近月（活跃月非连续）。v 始终跟踪流动性主力合约。
_GLBX = "2010-06-06"   # CME GLBX.MDP3 全 schema 起点
_IFEU = "2018-12-23"   # ICE IFEU.IMPACT 起点
_XEUR = "2025-03-10"   # Eurex XEUR.EOBI 起点

SPECS: dict[str, K4Spec] = {
    # 全球 K4（美国之外）
    "brn":    K4Spec("brn",    "BRN",    "IFEU.IMPACT", "BRN.v.0",  "Brent 原油 (ICE)",        _IFEU),
    "fesx":   K4Spec("fesx",   "FESX",   "XEUR.EOBI",   "FESX.v.0", "STOXX50 (Eurex)",         _XEUR),
    "nkd":    K4Spec("nkd",    "NKD",    "GLBX.MDP3",   "NKD.v.0",  "日经225 (CME)",            _GLBX),
    "eurusd": K4Spec("eurusd", "EURUSD", "GLBX.MDP3",   "6E.v.0",   "EUR/USD (CME 6E)",        _GLBX),
    "usdjpy": K4Spec("usdjpy", "USDJPY", "GLBX.MDP3",   "6J.v.0",   "USD/JPY (CME 6J 倒数)",   _GLBX, invert=True),
    "usdcnh": K4Spec("usdcnh", "USDCNH", "GLBX.MDP3",   "CNH.v.0",  "USD/CNH 离岸人民币 (CME)", _GLBX),
    # 美国 K4（补长史 1h/1d 用；1m 已有 *_1m_databento.json）
    "es":     K4Spec("es",     "ES",     "GLBX.MDP3",   "ES.v.0",   "标普 E-mini (CME)",        _GLBX),
    "gc":     K4Spec("gc",     "GC",     "GLBX.MDP3",   "GC.v.0",   "黄金 (CME)",               _GLBX),
    "cl":     K4Spec("cl",     "CL",     "GLBX.MDP3",   "CL.v.0",   "WTI 原油 (CME)",           _GLBX),
}

_SCHEMA = {"1m": "ohlcv-1m", "1h": "ohlcv-1h", "1d": "ohlcv-1d"}


def _client() -> db.Historical:
    key = os.environ.get("DATABENTO_API_KEY")
    if not key:
        raise RuntimeError("DATABENTO_API_KEY 未设置（.env）")
    return db.Historical(key=key)


def _end_date() -> str:
    return (datetime.utcnow() - timedelta(days=1)).strftime("%Y-%m-%d")


def fetch_one(spec: K4Spec, interval: str = "1m", start: str = DEFAULT_START) -> Path:
    schema = _SCHEMA[interval]
    end = _end_date()
    # 不早于 dataset 该 schema 可用起点，避免 422
    if start < spec.earliest:
        start = spec.earliest
    print(f"[{spec.key}] {spec.label} | {spec.dataset} {spec.db_symbol} "
          f"continuous {interval} [{start} → {end}]", flush=True)

    client = _client()
    data = client.timeseries.get_range(
        dataset=spec.dataset,
        symbols=[spec.db_symbol],
        stype_in="continuous",
        schema=schema,
        start=start,
        end=end,
    )
    df = data.to_df()
    if df.empty:
        raise RuntimeError(f"[{spec.key}] Databento 返回空数据")

    # 规整：排序、去重、保留 OHLCV
    df = df.sort_index()
    df = df[~df.index.duplicated(keep="last")]

    opens = df["open"].astype(float).tolist()
    highs = df["high"].astype(float).tolist()
    lows = df["low"].astype(float).tolist()
    closes = df["close"].astype(float).tolist()
    volumes = df["volume"].astype("int64").tolist()

    if spec.invert:
        # 精确 OHLC 倒数变换（1/x 单调递减 → high/low 互换），volume 不变。
        opens = [1.0 / v for v in opens]
        closes = [1.0 / v for v in closes]
        new_highs = [1.0 / v for v in lows]   # 原 low → 新 high
        new_lows = [1.0 / v for v in highs]   # 原 high → 新 low
        highs, lows = new_highs, new_lows

    dates = [str(ts) for ts in df.index]

    payload = {
        "symbol": spec.symbol,
        "dates": dates,
        "opens": opens,
        "highs": highs,
        "lows": lows,
        "closes": closes,
        "volumes": volumes,
    }

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    out = CACHE_DIR / f"{spec.key}_{interval}_databento.json"
    with open(out, "w") as f:
        json.dump(payload, f)

    span = f"{dates[0]} → {dates[-1]}" if dates else "—"
    size_mb = out.stat().st_size / 1024 / 1024
    print(f"[{spec.key}] ✓ {len(dates)} 条 | {span} | "
          f"{out.name} ({size_mb:.1f}MB)"
          + ("  [已倒数为 USDJPY]" if spec.invert else ""), flush=True)
    return out


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] in ("-h", "--help"):
        print(__doc__)
        return 0
    arg = sys.argv[1].lower()
    interval = sys.argv[2] if len(sys.argv) > 2 else "1m"
    if interval not in _SCHEMA:
        print(f"未知 interval: {interval}。可选: {', '.join(_SCHEMA)}")
        return 1
    start = sys.argv[3] if len(sys.argv) > 3 else DEFAULT_START

    if arg == "--list":
        for s in SPECS.values():
            print(f"  {s.key:8s} {s.label:28s} {s.dataset:12s} {s.db_symbol}  (≥{s.earliest})")
        return 0

    if arg == "all":
        for spec in SPECS.values():
            try:
                fetch_one(spec, interval, start)
            except Exception as e:
                print(f"[{spec.key}] ✗ 失败: {e!r}", flush=True)
        return 0

    spec = SPECS.get(arg)
    if not spec:
        print(f"未知标的: {arg}。可选: {', '.join(SPECS)} 或 all/--list")
        return 1
    fetch_one(spec, interval, start)
    return 0


if __name__ == "__main__":
    sys.exit(main())
