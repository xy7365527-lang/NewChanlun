"""QQQ 日线数据加载器 — streaming 验证与回测共用。

优先 yfinance 实时拉取最近 N 根日线；失败时 fallback 到本地缓存
analysis/data_cache/QQQ_1d_max.json 的末尾 N 根。

加载结果同时落盘 analysis/data_cache/QQQ_1d_1000.json，保证验证脚本与回测
脚本消费同一份数据（可复现）。

认识论等级：L2（真实 QQQ 日线，结论可否证）。
"""
from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / "analysis" / "data_cache"


def _from_yfinance(n: int) -> dict | None:
    """yfinance 拉取 QQQ 日线，返回末尾 n 根；失败返回 None。"""
    try:
        import yfinance as yf
    except ImportError:
        return None
    try:
        df = yf.download(
            "QQQ", period="max", interval="1d",
            auto_adjust=False, progress=False,
        )
        if df is None or df.empty:
            return None
        df = df.tail(n)
        # yfinance MultiindexColumns 兼容
        def col(name: str) -> list[float]:
            s = df[name]
            if hasattr(s, "columns"):
                s = s.iloc[:, 0]
            return [float(x) for x in s.to_numpy().ravel()]
        return {
            "symbol": "QQQ",
            "source": "yfinance",
            "dates": [d.strftime("%Y-%m-%d") for d in df.index],
            "opens": col("Open"),
            "highs": col("High"),
            "lows": col("Low"),
            "closes": col("Close"),
        }
    except Exception:
        return None


def _from_cache(n: int) -> dict:
    """本地缓存 fallback：QQQ_1d_max.json 末尾 n 根。"""
    d = json.load(open(CACHE / "QQQ_1d_max.json"))
    out = {"symbol": "QQQ", "source": "cache:QQQ_1d_max.json"}
    out["dates"] = d["dates"][-n:]
    for k in ("opens", "highs", "lows", "closes"):
        out[k] = [float(x) for x in d[k][-n:]]
    return out


def load_qqq_last_n(n: int = 1000, *, save: bool = True) -> dict:
    """加载 QQQ 日线最近 n 根（yfinance 优先，cache fallback）。"""
    data = _from_yfinance(n)
    if data is None:
        data = _from_cache(n)
    if save:
        (CACHE / "QQQ_1d_1000.json").write_text(
            json.dumps(data), encoding="utf-8",
        )
    return data


def to_bars(data: dict) -> list:
    """data dict → list[Bar]（按时间升序）。"""
    from newchan.types import Bar
    bars: list = []
    for i, d in enumerate(data["dates"]):
        ts = datetime.strptime(d[:10], "%Y-%m-%d").replace(tzinfo=timezone.utc)
        bars.append(Bar(
            ts=ts,
            open=data["opens"][i],
            high=data["highs"][i],
            low=data["lows"][i],
            close=data["closes"][i],
        ))
    return bars


def load_tv_bsp() -> list[dict]:
    """TV CZSC 137 个买卖点 labels（含 price 与 买/卖 文本）。"""
    d = json.load(open(CACHE / "qqq_chanlun_labels.json"))
    labels = d["pine"]["labels"]
    return [
        lbl for lbl in labels
        if lbl.get("price")
        and ("买" in str(lbl.get("text", "")) or "卖" in str(lbl.get("text", "")))
    ]
