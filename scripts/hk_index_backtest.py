#!/usr/bin/env python3
"""
多标的多周期缠论买卖点回测（港股 + 原油）

输入：analysis/data_cache/{sym}_{tf}_chanlun.json（由 hk_index_cdp_fetch.py 生成）
输出：
  analysis/hk_china_index_backtest_{sym}.md（每标的报告）
  analysis/hk_china_index_backtest.md（总报告）

两组对比：
  A 组：纯缠论买卖点（lag=1 因果口径）
  B 组：缠论买卖点 + PH settle 5bar 过滤

yfinance 周期映射：
  daily   → interval=1d, period=max
  30min   → interval=30m, period=60d
  5min    → interval=5m,  period=60d
  1min    → interval=1m,  period=7d

BRN 备用数据源：
  yfinance BZ=F（主要）；Alpha Vantage BRENT daily（仅日线 fallback）
  AV key: .env.local → ALPHA_VANTAGE_API_KEY

用法:
    .venv/bin/python scripts/hk_index_backtest.py
    .venv/bin/python scripts/hk_index_backtest.py --symbols brn
    .venv/bin/python scripts/hk_index_backtest.py --no-download
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))
from newchan.a_online_persistence import OnlineMergeTree

ROOT = Path(__file__).parent.parent
DATA_DIR = ROOT / "analysis" / "data_cache"
REPORT_DIR = ROOT / "analysis"

# ── Alpha Vantage 配置 ────────────────────────────────────────────────────────

def _load_av_key() -> str:
    """从 .env.local 读取 ALPHA_VANTAGE_API_KEY。"""
    for path in [ROOT / ".env.local", ROOT / ".env"]:
        if path.exists():
            for line in path.read_text().splitlines():
                if line.startswith("ALPHA_VANTAGE_API_KEY"):
                    return line.split("=", 1)[-1].strip()
    return os.environ.get("ALPHA_VANTAGE_API_KEY", "")

AV_KEY = _load_av_key()


# ── 配置 ─────────────────────────────────────────────────────────────────────

ALL_SYMBOLS = [
    {
        "name": "hsi",
        "label": "恒生指数",
        "yf_tickers": ["^HSI"],
        "currency": "HKD",
    },
    {
        "name": "shcomp",
        "label": "上证指数",
        "yf_tickers": ["000001.SS"],
        "currency": "CNY",
    },
    {
        "name": "hstech",
        "label": "恒生科技指数",
        "yf_tickers": ["^HSTECH", "3032.HK"],
        "currency": "HKD",
    },
    {
        "name": "brn",
        "label": "Brent 原油",
        "yf_tickers": ["BZ=F"],
        "av_daily_function": "BRENT",   # AV Energy endpoint 仅日线 fallback
        "currency": "USD",
        "asset_type": "futures",        # 标注期货属性（有 rollover gap）
    },
]

ALL_TIMEFRAMES = [
    {"name": "daily",  "tv_res": "D",  "yf_interval": "1d",  "yf_period": "max",
     "ep_level": "L2", "ep_note": "单标的全历史"},
    {"name": "30min",  "tv_res": "30", "yf_interval": "30m", "yf_period": "60d",
     "ep_level": "L2", "ep_note": "60日窗口"},
    {"name": "5min",   "tv_res": "5",  "yf_interval": "5m",  "yf_period": "60d",
     "ep_level": "L2", "ep_note": "60日窗口"},
    {"name": "1min",   "tv_res": "1",  "yf_interval": "1m",  "yf_period": "7d",
     "ep_level": "L2", "ep_note": "7日窗口（yfinance 限制）"},
]

BSP_KEYWORDS = ["1买", "2买", "3买", "1卖", "2卖", "3卖"]
SETTLE_WINDOW = 5   # PH settle 窗口（交易 bar 数）
MIN_SIGNAL_COUNT = 4  # 少于此数时跳过回测

# ── CLI ───────────────────────────────────────────────────────────────────────

_sym_filter: set = set()
_no_download = False
for _arg in sys.argv[1:]:
    if _arg.startswith("--symbols"):
        parts = _arg.split("=", 1)
        val = parts[1] if len(parts) == 2 else (
            sys.argv[sys.argv.index(_arg) + 1]
            if sys.argv.index(_arg) + 1 < len(sys.argv) else ""
        )
        _sym_filter = {s.strip() for s in val.split(",")}
    elif _arg == "--no-download":
        _no_download = True

SYMBOLS = [s for s in ALL_SYMBOLS if not _sym_filter or s["name"] in _sym_filter]


# ── 信号解析 ─────────────────────────────────────────────────────────────────

def _clean_type(text: str) -> str:
    for kw in BSP_KEYWORDS:
        if kw in text:
            suffix = ""
            if "盘整" in text:
                suffix = "(盘整)"
            elif "趋势" in text:
                suffix = "(趋势)"
            return kw + suffix
    return text.strip()


def load_signals(data: dict) -> list[dict]:
    signals = []
    for label in data.get("pine", {}).get("labels", []):
        text = label.get("text") or ""
        if not any(kw in text for kw in BSP_KEYWORDS):
            continue
        price = label.get("price")
        if price is None:
            continue
        raw_type = next(kw for kw in BSP_KEYWORDS if kw in text)
        signals.append({
            "bar_idx": label.get("time"),
            "price": float(price),
            "signal_type": _clean_type(text),
            "raw_type": raw_type,
            "is_buy": "买" in raw_type,
        })
    signals.sort(key=lambda x: (x["bar_idx"] or 0, x["price"]))
    return signals


def detect_time_mode(signals: list[dict]) -> str:
    """判断 bar_idx 是 Unix 时间戳还是 bar_index。"""
    times = [s["bar_idx"] for s in signals if s["bar_idx"] is not None]
    if not times:
        return "unknown"
    max_t = max(times)
    if max_t > 1e12:
        return "unix_ms"   # Unix 毫秒
    elif max_t > 1e9:
        return "unix_s"    # Unix 秒
    else:
        return "bar_index"


# ── OHLCV 下载 ────────────────────────────────────────────────────────────────

def download_ohlcv(yf_tickers: list[str], interval: str, period: str) -> pd.DataFrame:
    """尝试多个 ticker，返回第一个成功的 OHLCV。"""
    import yfinance as yf
    for ticker in yf_tickers:
        try:
            df = yf.download(ticker, period=period, interval=interval,
                             auto_adjust=True, progress=False)
            if df is None or df.empty:
                continue
            df.index = pd.to_datetime(df.index)
            if isinstance(df.columns, pd.MultiIndex):
                df.columns = [col[0] for col in df.columns]
            df = df.dropna(subset=["Close"]).sort_index()
            if len(df) > 0:
                return df
        except Exception:
            continue
    return pd.DataFrame()


def download_av_brent_daily(av_key: str) -> pd.DataFrame:
    """从 Alpha Vantage 下载 Brent 原油日线数据（仅日线，作为 yfinance 的 fallback）。

    AV endpoint: function=BRENT&interval=daily
    返回格式: {date, value} list。注意 AV 的价格单位是 USD/BBL，
    与 yfinance BZ=F 单位相同，但价格可能因数据源差异略有不同。
    """
    if not av_key:
        return pd.DataFrame()
    url = (
        f"https://www.alphavantage.co/query"
        f"?function=BRENT&interval=daily&apikey={av_key}"
    )
    try:
        raw = urllib.request.urlopen(url, timeout=15).read()
        data = json.loads(raw)
        if "data" not in data:
            return pd.DataFrame()
        rows = []
        for item in data["data"]:
            val = item.get("value", ".")
            if val == "." or not val:
                continue
            try:
                rows.append({"Date": item["date"], "Close": float(val)})
            except ValueError:
                continue
        if not rows:
            return pd.DataFrame()
        df = pd.DataFrame(rows)
        df["Date"] = pd.to_datetime(df["Date"])
        df = df.set_index("Date").sort_index()
        # AV 只有 Close；补充 Open/High/Low/Volume 为 NaN（不影响价格匹配）
        for col in ["Open", "High", "Low", "Volume"]:
            df[col] = float("nan")
        return df
    except Exception:
        return pd.DataFrame()


def download_ohlcv_with_fallback(sym: dict, interval: str, period: str) -> tuple[pd.DataFrame, str]:
    """下载 OHLCV，对 BRN 日线提供 AV fallback。返回 (df, source) tuple。"""
    # 主路径：yfinance
    df = download_ohlcv(sym.get("yf_tickers", []), interval, period)
    if not df.empty:
        return df, "yfinance"

    # BRN 日线 fallback：Alpha Vantage BRENT endpoint
    if sym.get("av_daily_function") and interval == "1d" and AV_KEY:
        time.sleep(1)  # AV rate limit: 5 calls/min on free tier
        df_av = download_av_brent_daily(AV_KEY)
        if not df_av.empty:
            return df_av, "alpha_vantage_brent"

    return pd.DataFrame(), "none"


# ── 信号→日期映射 ─────────────────────────────────────────────────────────────

def map_signals_unix(signals: list[dict], ohlcv: pd.DataFrame, mode: str) -> list[dict]:
    """直接用 Unix 时间戳映射到 OHLCV 行（asof 查找）。"""
    mapped = []
    for sig in signals:
        t = sig["bar_idx"]
        if t is None:
            continue
        if mode == "unix_ms":
            ts = pd.Timestamp(t, unit="ms", tz="UTC")
        else:
            ts = pd.Timestamp(t, unit="s", tz="UTC")

        # 处理时区：ohlcv 可能有或没有 tz
        idx = ohlcv.index
        if idx.tz is None:
            ts = ts.tz_localize(None)
        else:
            ts = ts.tz_convert(idx.tz)

        pos = ohlcv.index.searchsorted(ts, side="right") - 1
        if pos < 0:
            continue
        row_date = ohlcv.index[pos]
        close = float(ohlcv["Close"].iloc[pos])
        diff_pct = abs(close - sig["price"]) / sig["price"] * 100

        mapped.append({
            **sig,
            "date": row_date,
            "date_idx": pos,
            "matched_close": close,
            "price_diff_pct": diff_pct,
        })
    return mapped


def map_signals_price(signals: list[dict], ohlcv: pd.DataFrame) -> list[dict]:
    """价格锚点映射：每个信号找价格最接近的 OHLCV bar。

    对于 bar_index 模式，利用 bar_idx 的相对顺序约束局部搜索范围
    （避免同价位在不同时段重复匹配）。
    """
    if len(signals) == 0 or len(ohlcv) == 0:
        return []

    closes = ohlcv["Close"].values.astype(float)
    dates = ohlcv.index
    n = len(closes)

    # 锚点：用最后一个信号价格全局定位
    last = signals[-1]
    anchor_pos = int(np.argmin(np.abs(closes - last["price"])))
    anchor_bar = last["bar_idx"] or len(signals) - 1

    # 估算 bar_idx 到 ohlcv 行的比例（线性外推）
    if len(signals) >= 2:
        first = signals[0]
        first_pos_est = max(0, anchor_pos - (anchor_bar - (first["bar_idx"] or 0)))
        bar_per_ohlcv = (anchor_pos - first_pos_est) / max(1, anchor_bar - (first["bar_idx"] or 0))
    else:
        bar_per_ohlcv = 1.0

    window = max(50, int(n * 0.05))  # 局部搜索窗口

    mapped = []
    for sig in signals:
        bar_t = sig["bar_idx"] or 0
        target = sig["price"]

        estimated = int(anchor_pos - (anchor_bar - bar_t) * bar_per_ohlcv)
        lo = max(0, estimated - window)
        hi = min(n - 1, estimated + window)
        chunk = closes[lo: hi + 1]
        rel = int(np.argmin(np.abs(chunk - target)))
        best = lo + rel

        close_at = float(closes[best])
        diff_pct = abs(close_at - target) / target * 100

        mapped.append({
            **sig,
            "date": dates[best],
            "date_idx": best,
            "matched_close": close_at,
            "price_diff_pct": diff_pct,
        })

    mapped.sort(key=lambda x: x["date"])
    return mapped


# ── PH settle ────────────────────────────────────────────────────────────────

def build_settle_timeline(closes: np.ndarray) -> list[int]:
    tree = OnlineMergeTree()
    indices = []
    for i, price in enumerate(closes):
        if tree.update(float(price)):
            indices.append(i)
    return indices


def has_settle_near(date_idx: int, settle_indices: list[int], window: int = SETTLE_WINDOW) -> bool:
    lo = date_idx - window
    return any(lo <= s <= date_idx for s in settle_indices)


# ── 回测引擎 ─────────────────────────────────────────────────────────────────

@dataclass
class Trade:
    entry_date: pd.Timestamp
    entry_price: float
    entry_signal: str
    exit_date: Optional[pd.Timestamp] = None
    exit_price: Optional[float] = None
    exit_signal: Optional[str] = None

    @property
    def is_closed(self) -> bool:
        return self.exit_price is not None

    @property
    def ret(self) -> float:
        if not self.is_closed:
            return float("nan")
        return (self.exit_price - self.entry_price) / self.entry_price

    @property
    def hold_bars(self) -> int:
        if not self.is_closed:
            return -1
        return self.exit_date - self.entry_date


def run_backtest(
    signals: list[dict],
    settle_indices: list[int],
    use_settle_filter: bool = False,
) -> list[Trade]:
    trades: list[Trade] = []
    position: Optional[Trade] = None

    for sig in signals:
        if sig["is_buy"]:
            if position is not None:
                continue
            if use_settle_filter and not has_settle_near(
                sig["date_idx"], settle_indices
            ):
                continue
            position = Trade(
                entry_date=sig["date"],
                entry_price=sig["matched_close"],
                entry_signal=sig["signal_type"],
            )
        else:
            if position is None:
                continue
            position.exit_date = sig["date"]
            position.exit_price = sig["matched_close"]
            position.exit_signal = sig["signal_type"]
            trades.append(position)
            position = None

    if position is not None:
        trades.append(position)

    return trades


# ── 统计 ─────────────────────────────────────────────────────────────────────

@dataclass
class Stats:
    total_trades: int = 0
    closed_trades: int = 0
    win_rate: float = float("nan")
    total_return: float = float("nan")
    max_drawdown: float = float("nan")
    avg_hold: str = "N/A"
    profit_factor: float = float("nan")
    by_type: dict = field(default_factory=dict)


def compute_stats(trades: list[Trade]) -> Stats:
    closed = [t for t in trades if t.is_closed]
    if not closed:
        return Stats(total_trades=len(trades))

    rets = [t.ret for t in closed]
    wins = [r for r in rets if r > 0]
    losses = [r for r in rets if r <= 0]

    cum = 1.0
    equity = [1.0]
    for r in rets:
        cum *= 1 + r
        equity.append(cum)

    peak = 1.0
    max_dd = 0.0
    for eq in equity:
        if eq > peak:
            peak = eq
        dd = (peak - eq) / peak
        if dd > max_dd:
            max_dd = dd

    by_type: dict[str, dict] = {}
    for t in closed:
        k = t.entry_signal
        if k not in by_type:
            by_type[k] = {"trades": 0, "wins": 0, "rets": []}
        by_type[k]["trades"] += 1
        if t.ret > 0:
            by_type[k]["wins"] += 1
        by_type[k]["rets"].append(t.ret)

    by_type_s = {}
    for k, v in sorted(by_type.items()):
        n = v["trades"]
        w = v["wins"]
        by_type_s[k] = {
            "trades": n,
            "win_rate": w / n if n else float("nan"),
            "avg_ret": float(np.mean(v["rets"])) if v["rets"] else float("nan"),
        }

    avg_win = float(np.mean(wins)) if wins else 0.0
    avg_loss = float(np.mean([abs(l) for l in losses])) if losses else 0.0
    pf = avg_win / avg_loss if avg_loss > 0 else float("inf")

    # 平均持仓时长
    hold_deltas = [t.exit_date - t.entry_date for t in closed]
    if hold_deltas:
        total_sec = sum(
            d.total_seconds() if hasattr(d, "total_seconds") else d.days * 86400
            for d in hold_deltas
        )
        avg_sec = total_sec / len(hold_deltas)
        if avg_sec < 3600:
            avg_hold_str = f"{avg_sec/60:.1f}分钟"
        elif avg_sec < 86400:
            avg_hold_str = f"{avg_sec/3600:.1f}小时"
        else:
            avg_hold_str = f"{avg_sec/86400:.1f}天"
    else:
        avg_hold_str = "N/A"

    return Stats(
        total_trades=len(trades),
        closed_trades=len(closed),
        win_rate=len(wins) / len(closed),
        total_return=cum - 1,
        max_drawdown=max_dd,
        avg_hold=avg_hold_str,
        profit_factor=pf,
        by_type=by_type_s,
    )


# ── 格式化 ───────────────────────────────────────────────────────────────────

def fp(v: float, d: int = 2) -> str:
    return "N/A" if v != v else f"{v * 100:.{d}f}%"


def ff(v: float, d: int = 2) -> str:
    return "N/A" if v != v else f"{v:.{d}f}"


# ── 单标的单周期回测 ──────────────────────────────────────────────────────────

def run_one(sym: dict, tf: dict, no_download: bool = False) -> dict:
    """返回包含回测结果的字典，失败时包含 error 键。"""
    sym_name = sym["name"]
    tf_name = tf["name"]
    data_path = DATA_DIR / f"{sym_name}_{tf_name}_chanlun.json"

    if not data_path.exists():
        return {"error": f"数据文件不存在: {data_path}"}

    with open(data_path) as f:
        data = json.load(f)

    signals = load_signals(data)
    if not signals:
        return {"error": "无买卖点信号", "label_count": data.get("summary", {}).get("label_count", 0)}

    time_mode = detect_time_mode(signals)

    if no_download:
        return {
            "signal_count": len(signals),
            "buy_count": sum(1 for s in signals if s["is_buy"]),
            "sell_count": sum(1 for s in signals if not s["is_buy"]),
            "time_mode": time_mode,
            "skipped": "no_download",
        }

    # 下载 OHLCV（BRN 提供 AV fallback）
    ohlcv, ohlcv_source = download_ohlcv_with_fallback(sym, tf["yf_interval"], tf["yf_period"])
    if ohlcv.empty:
        return {
            "error": f"OHLCV 数据为空 (tickers={sym.get('yf_tickers')}, interval={tf['yf_interval']})",
            "signal_count": len(signals),
        }

    # 信号映射
    if time_mode in ("unix_ms", "unix_s"):
        mapped = map_signals_unix(signals, ohlcv, time_mode)
    else:
        mapped = map_signals_price(signals, ohlcv)

    mapped = [s for s in mapped if "date" in s]
    if len(mapped) < MIN_SIGNAL_COUNT:
        return {
            "error": f"有效映射信号不足 ({len(mapped)}<{MIN_SIGNAL_COUNT})",
            "signal_count": len(signals),
        }

    # PH settle 时间线
    closes = ohlcv["Close"].values.astype(float)
    settle_indices = build_settle_timeline(closes)

    # 两组回测
    trades_a = run_backtest(mapped, settle_indices, use_settle_filter=False)
    trades_b = run_backtest(mapped, settle_indices, use_settle_filter=True)
    stats_a = compute_stats(trades_a)
    stats_b = compute_stats(trades_b)

    # 价格匹配质量
    diff_pcts = [s.get("price_diff_pct", 0) for s in mapped]
    bad_matches = [s for s in mapped if s.get("price_diff_pct", 0) > 5]

    return {
        "signal_count": len(signals),
        "mapped_count": len(mapped),
        "time_mode": time_mode,
        "ohlcv_source": ohlcv_source,
        "ohlcv_range": f"{ohlcv.index[0].date() if len(ohlcv) > 0 else '?'} → {ohlcv.index[-1].date() if len(ohlcv) > 0 else '?'}",
        "ohlcv_bars": len(ohlcv),
        "settle_events": len(settle_indices),
        "stats_a": stats_a,
        "stats_b": stats_b,
        "trades_a": trades_a,
        "trades_b": trades_b,
        "mapped_signals": mapped,
        "diff_pcts": diff_pcts,
        "bad_matches": bad_matches,
        "ep_level": tf["ep_level"],
        "ep_note": tf["ep_note"],
        "asset_type": sym.get("asset_type", "index"),
    }


# ── 单标的报告 ────────────────────────────────────────────────────────────────

def build_sym_report(sym: dict, tf_results: dict) -> str:
    lines: list[str] = []
    a = lines.append

    a(f"# {sym['label']} ({sym['name'].upper()}) 缠论多周期回测报告")
    a("")
    a(f"**标的**：{sym['label']}  |  **货币**：{sym['currency']}  |  **类型**：{sym.get('asset_type', 'index')}")
    a(f"**yfinance tickers**：{sym.get('yf_tickers', [])}  |  **AV fallback**：{sym.get('av_daily_function', '无')}")
    a("")
    a("---")
    a("")

    # 汇总表
    a("## 多周期汇总")
    a("")
    a("| 周期 | 信号数 | 成交笔(A) | 胜率(A) | 总收益(A) | 成交笔(B) | 胜率(B) | 总收益(B) | 认识论等级 |")
    a("|------|-------|---------|---------|---------|---------|---------|---------|---------|")

    for tf in ALL_TIMEFRAMES:
        tf_name = tf["name"]
        r = tf_results.get(tf_name, {})
        if "error" in r:
            a(f"| {tf_name} | — | — | — | — | — | — | — | ERROR: {r['error'][:40]} |")
            continue
        if "skipped" in r:
            a(f"| {tf_name} | {r.get('signal_count', '?')} | — | — | — | — | — | — | {r.get('skipped')} |")
            continue
        sa = r.get("stats_a", Stats())
        sb = r.get("stats_b", Stats())
        a(f"| {tf_name} | {r.get('signal_count', '?')} "
          f"| {sa.closed_trades} | {fp(sa.win_rate)} | {fp(sa.total_return)} "
          f"| {sb.closed_trades} | {fp(sb.win_rate)} | {fp(sb.total_return)} "
          f"| {r.get('ep_level')} ({r.get('ep_note')}) |")

    a("")
    a("---")
    a("")

    # 每个周期详情
    for tf in ALL_TIMEFRAMES:
        tf_name = tf["name"]
        r = tf_results.get(tf_name, {})
        a(f"## {tf_name} 详情")
        a("")

        if "error" in r:
            a(f"> **跳过**：{r['error']}")
            if r.get("signal_count"):
                a(f"> 原始信号数：{r['signal_count']}")
            a("")
            continue

        if "skipped" in r:
            a(f"> **跳过**（no_download）：信号数={r.get('signal_count', '?')}")
            a("")
            continue

        sa = r["stats_a"]
        sb = r["stats_b"]

        a(f"**数据范围**：{r.get('ohlcv_range')} ({r.get('ohlcv_bars')} bars) | **来源**：{r.get('ohlcv_source', '?')}")
        a(f"**信号总数**：{r.get('signal_count')} → 有效映射 {r.get('mapped_count')} 个")
        a(f"**时间模式**：{r.get('time_mode')} | **settle 事件**：{r.get('settle_events')}")
        a(f"**认识论等级**：{r.get('ep_level')} — {r.get('ep_note')}")
        a("")

        # 数据源偏差警告：中位偏差 > 10% 时加红色警示
        diff_pcts = r.get("diff_pcts", [])
        if diff_pcts:
            import numpy as _np
            median_dev = float(_np.median(diff_pcts))
            if median_dev > 10:
                a(f"> ⚠️ **数据源价格偏差警告**：中位偏差 {median_dev:.1f}% — "
                  f"TV（{sym.get('name','?').upper()}）与 yfinance 使用不同的连续合约价格序列（滚动方法差异），"
                  f"导致历史价格系统性不对齐。此周期回测结果**不可靠**，仅供参考。")
                a(f"> 可靠性排名：5min/1min（近期价格对齐）> 30min > daily（历史序列最不对齐）。")
                a("")

        a("### A/B 对比")
        a("")
        a("| 指标 | A 组（纯缠论） | B 组（+PH settle 5bar 过滤） |")
        a("|------|-------------|---------------------------|")
        a(f"| 成交笔数 | {sa.closed_trades} | {sb.closed_trades} |")
        a(f"| 胜率 | {fp(sa.win_rate)} | {fp(sb.win_rate)} |")
        a(f"| 总收益（复利） | {fp(sa.total_return)} | {fp(sb.total_return)} |")
        a(f"| 最大回撤 | {fp(sa.max_drawdown)} | {fp(sb.max_drawdown)} |")
        a(f"| 平均持仓 | {sa.avg_hold} | {sb.avg_hold} |")
        a(f"| 盈亏比 | {ff(sa.profit_factor)} | {ff(sb.profit_factor)} |")
        a("")

        # A 组按类型
        if sa.by_type:
            a("### A 组按信号类型")
            a("")
            a("| 类型 | 笔数 | 胜率 | 平均收益 |")
            a("|------|------|------|---------|")
            for k, v in sa.by_type.items():
                a(f"| {k} | {v['trades']} | {fp(v['win_rate'])} | {fp(v['avg_ret'])} |")
            a("")

        # 价格匹配质量
        diff_pcts = r.get("diff_pcts", [])
        bad = r.get("bad_matches", [])
        if diff_pcts:
            median_d = float(np.median(diff_pcts))
            max_d = float(np.max(diff_pcts))
            good = sum(1 for d in diff_pcts if d < 2)
            ok_d = sum(1 for d in diff_pcts if 2 <= d < 5)
            bad_count = sum(1 for d in diff_pcts if d >= 5)
            a("### 价格匹配质量")
            a("")
            a(f"中位偏差：{median_d:.2f}%，最大：{max_d:.2f}%  |  "
              f"<2%({good}) / 2-5%({ok_d}) / >5%({bad_count})")
            a("")

        a("")

    # 边界条件
    a("## 边界条件与有效域")
    a("")
    a("1. **信号来源**：TV 缠论指标实时标注，非历史回放——标注存在未来重绘风险（replay check 未执行）。")
    a("2. **价格映射**：TV bar_index → yfinance OHLCV 基于价格锚点匹配，复权口径可能不一致，偏差 >5% 的信号结论可靠性下降。")
    a("3. **intraday 数据限制**：yfinance 1分钟 ≤7天 / 5/30分钟 ≤60天——覆盖的历史极短，backtest 统计量置信度低（bar 数不足 → 可能翻转）。")
    a("4. **PH settle 过滤**：settle 是「右侧确认」；极端底部的右侧确认滞后于价格底部，B 组可能在关键买点处漏单（QQQ 回测已有 L2 否证）。")
    a(f"5. **翻转条件**：若信号数 <{MIN_SIGNAL_COUNT}×2 则统计无效；若市场处于单边大趋势，胜率可能因基准漂移而虚高。")
    a(f"6. **认识论等级**：daily=L2（全历史）；30min/5min=L2（短窗口）；1min=L2（极短窗口，可信度最低）。L3 需多标的/多时段交叉验证。")

    return "\n".join(lines)


# ── 总报告 ───────────────────────────────────────────────────────────────────

def build_master_report(all_results: dict) -> str:
    lines: list[str] = []
    a = lines.append

    a("# 港股三大指数 × 四周期 缠论回测总报告")
    a("")
    a("**标的**：恒生指数(HSI) / 上证指数(SHCOMP) / 恒生科技指数(HSTECH)")
    a("**周期**：日线 / 30分钟 / 5分钟 / 1分钟")
    a("**方法**：A 组=纯缠论买卖点；B 组=缠论+PH settle 5bar 过滤")
    a("**价格执行假设**：信号 bar 收盘价成交，无滑点无手续费")
    a("")
    a("---")
    a("")
    a("## 全局汇总（A 组）")
    a("")
    a("| 标的 | 周期 | 信号数 | 成交笔 | 胜率 | 总收益 | MDD | 认识论等级 |")
    a("|------|------|-------|-------|------|-------|-----|---------|")

    for sym in ALL_SYMBOLS:
        sym_name = sym["name"]
        sym_results = all_results.get(sym_name, {})
        for tf in ALL_TIMEFRAMES:
            tf_name = tf["name"]
            r = sym_results.get(tf_name, {})
            if "error" in r:
                a(f"| {sym['label']} | {tf_name} | — | — | — | — | — | ERROR |")
                continue
            if "skipped" in r:
                a(f"| {sym['label']} | {tf_name} | {r.get('signal_count', '?')} | — | — | — | — | — |")
                continue
            sa = r.get("stats_a", Stats())
            a(f"| {sym['label']} | {tf_name} | {r.get('signal_count', '?')} "
              f"| {sa.closed_trades} | {fp(sa.win_rate)} | {fp(sa.total_return)} "
              f"| {fp(sa.max_drawdown)} | {r.get('ep_level')} |")

    a("")
    a("---")
    a("")
    a("## PH settle 过滤效果（B 组 vs A 组）")
    a("")
    a("| 标的 | 周期 | A 总收益 | B 总收益 | 过滤信号数 | 过滤效果 |")
    a("|------|------|---------|---------|---------|---------|")

    for sym in ALL_SYMBOLS:
        sym_name = sym["name"]
        sym_results = all_results.get(sym_name, {})
        for tf in ALL_TIMEFRAMES:
            tf_name = tf["name"]
            r = sym_results.get(tf_name, {})
            if "error" in r or "skipped" in r:
                continue
            sa = r.get("stats_a", Stats())
            sb = r.get("stats_b", Stats())
            filtered = sa.closed_trades - sb.closed_trades
            if sa.total_return == sa.total_return and sb.total_return == sb.total_return:
                effect = "正贡献 ↑" if sb.total_return > sa.total_return else "负贡献 ↓"
            else:
                effect = "N/A"
            a(f"| {sym['label']} | {tf_name} | {fp(sa.total_return)} "
              f"| {fp(sb.total_return)} | {filtered} | {effect} |")

    a("")
    a("---")
    a("")
    a("## 边界条件与方法论声明")
    a("")
    a("1. **L2 等级限制**：所有结论均为单标的单时段假设检验，否定性结果（B<A）有效；正确性不外推到未测标的/时段。")
    a("2. **intraday 数据极短**：1分钟7天/5+30分钟60天——统计量依赖于此窗口，不代表长期有效性。")
    a("3. **信号重绘风险**：TV 实时指标的历史标注可能在 bar close 前重绘，当前数据仅代表最终收盘后状态。")
    a("4. **港股/A股 yfinance 限制**：000001.SS 的 intraday 数据质量差；^HSTECH 可能回退至 3032.HK ETF。")
    a("5. **L3 需求**：若要得出稳健结论，需要跨标的 + 跨时段 + 跨时间段的交叉验证（本报告为 L2 起点）。")

    return "\n".join(lines)


# ── 主流程 ───────────────────────────────────────────────────────────────────

def main():
    print(f"[*] 港股指数多周期回测")
    print(f"    标的: {[s['name'] for s in SYMBOLS]}")
    print(f"    no_download={_no_download}")
    print()

    all_results: dict = {}

    for sym in SYMBOLS:
        sym_name = sym["name"]
        all_results[sym_name] = {}
        print(f"\n{'='*60}")
        print(f"[★] {sym['label']} ({sym_name})")

        tf_results: dict = {}
        for tf in ALL_TIMEFRAMES:
            tf_name = tf["name"]
            print(f"\n  [{tf_name}] ", end="", flush=True)
            r = run_one(sym, tf, no_download=_no_download)
            tf_results[tf_name] = r
            all_results[sym_name][tf_name] = r

            if "error" in r:
                print(f"ERROR: {r['error'][:60]}")
            elif "skipped" in r:
                print(f"SKIPPED (signals={r.get('signal_count', 0)})")
            else:
                sa = r.get("stats_a", Stats())
                print(f"OK: signals={r.get('signal_count')} "
                      f"closed={sa.closed_trades} "
                      f"win={fp(sa.win_rate)} "
                      f"ret={fp(sa.total_return)}")

        # 单标的报告
        sym_report = build_sym_report(sym, tf_results)
        sym_path = REPORT_DIR / f"hk_china_index_backtest_{sym_name}.md"
        sym_path.write_text(sym_report, encoding="utf-8")
        print(f"\n  [✓] 报告: {sym_path}")

    # 总报告
    master = build_master_report(all_results)
    master_path = REPORT_DIR / "hk_china_index_backtest.md"
    master_path.write_text(master, encoding="utf-8")
    print(f"\n[✓] 总报告: {master_path}")

    print("\n[*] 完成")


if __name__ == "__main__":
    main()
