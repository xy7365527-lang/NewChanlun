#!/usr/bin/env python3
"""
中国市场 1min 数据拉取（IF / AU / SC）——第四次尝试，专注执行。

数据源：akshare → 新浪财经期货分钟接口 futures_zh_minute_sina。

品种与交易所：
  IF  股指期货（沪深300）   CFFEX   无夜盘
  AU  黄金期货              SHFE    有夜盘（21:00–次日02:30）
  SC  原油期货              INE     有夜盘（21:00–次日02:30）

夜盘要求：AU/SC 的夜盘 bar 必须包含。新浪分钟接口返回连续 datetime 序列，
天然包含夜盘时段——脚本统计落在 [21:00, 03:00) 的 bar 数作为夜盘存在的证据。

接口已知限制：新浪分钟数据只保留近期若干根（通常几千根），
脚本如实报告每个符号实际拉到多少根、覆盖的时间跨度。

符号探测策略（两条路径都试，取根数多者）：
  路径A 连续主力符号：IF0 / AU0 / SC0（新浪连续主力代码）
  路径B 当前主力合约：经 ak.match_main_contract(交易所) 解析出的具体合约（如 IF2412）

输出：列式 JSON 到 analysis/data_cache/，格式：
  {symbol, source, period, exchange, has_night_session, night_session_bars,
   n_bars, time_start, time_end, times[], opens[], highs[], lows[], closes[], volumes[]}

认识论等级：纯技术性数据拉取（数据获取，不涉及概念定义）→ 简化版结果包。
"""

import json
import sys
from pathlib import Path

import akshare as ak

CACHE_DIR = Path(__file__).resolve().parent / "data_cache"
CACHE_DIR.mkdir(parents=True, exist_ok=True)

# 品种 → (交易所代码 for match_main_contract, 是否有夜盘)
VARIETIES = {
    "IF": {"exchange": "cffex", "night": False},
    "AU": {"exchange": "shfe", "night": True},
    "SC": {"exchange": "ine", "night": True},
}

# 夜盘时间窗口（小时，24h 制）：21:00 起至次日 03:00 前
NIGHT_START_HOUR = 21
NIGHT_END_HOUR = 3  # 次日凌晨，落在 [0,3) 也算夜盘


def resolve_main_contract(variety: str, exchange: str) -> str | None:
    """路径B：从 match_main_contract 解析当前主力合约具体代码。"""
    try:
        raw = ak.match_main_contract(symbol=exchange)
    except Exception as exc:  # noqa: BLE001 — 接口异常如实报告，不吞掉
        print(f"  [{variety}] match_main_contract({exchange}) 失败: {exc}")
        return None
    # 返回形如 "IF2412,IC2412,IH2412,..." 的逗号分隔串
    contracts = [c.strip() for c in str(raw).split(",") if c.strip()]
    for c in contracts:
        # 合约代码前缀匹配品种（去掉数字部分比较）
        prefix = "".join(ch for ch in c if not ch.isdigit())
        if prefix.upper() == variety.upper():
            return c
    print(f"  [{variety}] 在 {exchange} 主力列表中未找到匹配前缀，列表={contracts}")
    return None


def fetch_minute(symbol: str) -> "list[dict] | None":
    """调用新浪分钟接口，返回原始 records；失败返回 None。"""
    try:
        df = ak.futures_zh_minute_sina(symbol=symbol, period="1")
    except Exception as exc:  # noqa: BLE001
        print(f"    fetch {symbol}: 异常 {exc}")
        return None
    if df is None or df.empty:
        print(f"    fetch {symbol}: 空数据")
        return None
    return df


def count_night_bars(times: "list[str]") -> int:
    """统计落在夜盘窗口 [21:00, 03:00) 的 bar 数。time 形如 'YYYY-MM-DD HH:MM:SS'。"""
    n = 0
    for t in times:
        try:
            hh = int(t[11:13])
        except (ValueError, IndexError):
            continue
        if hh >= NIGHT_START_HOUR or hh < NIGHT_END_HOUR:
            n += 1
    return n


def df_to_columnar(df, symbol: str, exchange: str, has_night: bool) -> dict:
    """新浪分钟 DataFrame → 列式 dict。列名：datetime, open, high, low, close, volume[, hold]。"""
    cols = {c.lower(): c for c in df.columns}
    dt_col = cols.get("datetime") or list(df.columns)[0]

    times = [str(v) for v in df[dt_col].tolist()]

    def col(name: str) -> list:
        real = cols.get(name)
        return [float(v) for v in df[real].tolist()] if real else []

    night_bars = count_night_bars(times)
    return {
        "symbol": symbol,
        "source": "akshare/futures_zh_minute_sina",
        "period": "1min",
        "exchange": exchange,
        "has_night_session_expected": has_night,
        "night_session_bars": night_bars,
        "n_bars": len(times),
        "time_start": times[0] if times else None,
        "time_end": times[-1] if times else None,
        "times": times,
        "opens": col("open"),
        "highs": col("high"),
        "lows": col("low"),
        "closes": col("close"),
        "volumes": col("volume"),
    }


def pull_variety(variety: str, meta: dict) -> dict:
    exchange = meta["exchange"]
    has_night = meta["night"]
    print(f"\n=== {variety} ({exchange}, 夜盘={'有' if has_night else '无'}) ===")

    candidates = []  # (label, symbol)
    candidates.append(("continuous", f"{variety}0"))  # 路径A 连续主力
    main = resolve_main_contract(variety, exchange)
    if main:
        candidates.append(("main_contract", main))
        print(f"  当前主力合约: {main}")

    best = None
    for label, symbol in candidates:
        print(f"  尝试 [{label}] symbol={symbol}")
        df = fetch_minute(symbol)
        if df is None:
            continue
        rec = df_to_columnar(df, symbol, exchange, has_night)
        print(
            f"    → {rec['n_bars']} 根, {rec['time_start']} ~ {rec['time_end']}, "
            f"夜盘 bar={rec['night_session_bars']}"
        )
        if best is None or rec["n_bars"] > best["n_bars"]:
            best = rec
            best["_picked_label"] = label

    if best is None:
        print(f"  [{variety}] 两条路径均失败，无数据")
        return {"variety": variety, "status": "no_data"}

    out = CACHE_DIR / f"china_{variety}_1min.json"
    with out.open("w") as f:
        json.dump(best, f, ensure_ascii=False)
    print(f"  ✓ 已写入 {out.name} (来自 {best['_picked_label']})")

    # 夜盘断言：有夜盘的品种必须真的含夜盘 bar，否则告警
    if has_night and best["night_session_bars"] == 0:
        print(f"  ⚠️  {variety} 应有夜盘但夜盘 bar=0 —— 数据可能不完整！")

    return {
        "variety": variety,
        "status": "ok",
        "symbol": best["symbol"],
        "n_bars": best["n_bars"],
        "time_start": best["time_start"],
        "time_end": best["time_end"],
        "night_session_bars": best["night_session_bars"],
        "file": out.name,
    }


def main() -> int:
    print(f"akshare 版本: {ak.__version__}")
    print(f"输出目录: {CACHE_DIR}")
    summary = [pull_variety(v, m) for v, m in VARIETIES.items()]

    print("\n" + "=" * 60)
    print("汇总")
    print("=" * 60)
    for s in summary:
        if s["status"] == "ok":
            print(
                f"  {s['variety']:3s} {s['symbol']:10s} {s['n_bars']:6d} 根  "
                f"夜盘={s['night_session_bars']:5d}  {s['time_start']} ~ {s['time_end']}"
            )
        else:
            print(f"  {s['variety']:3s} 失败: {s['status']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
