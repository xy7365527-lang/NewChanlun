"""MOS 持续同调(PH)分析驱动 —— 调用 src/newchan 的 PH 引擎。

认识论等级：数据=真实(yfinance MOS 日线)，PH算法本身 L0，
级别↔周期映射 L2，趋势/中枢判定 L1~L2。所有结论标注等级。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import yfinance as yf  # noqa: E402

from newchan.a_persistence_barcode import (  # noqa: E402
    barcode_from_prices,
    atr_noise_threshold,
    active_bars,
)
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_level_detection import detect_levels  # noqa: E402
from newchan.a_ph_zhongshu import detect_zhongshu, ZhongshuPolicy  # noqa: E402


def pull_mos(period: str = "1y", interval: str = "1d") -> dict:
    df = yf.download("MOS", period=period, interval=interval,
                     auto_adjust=False, progress=False)
    if df.empty:
        raise SystemExit("yfinance 返回空数据")
    # 处理 yfinance 多级列
    def col(name: str):
        if (name,) in df.columns or name in df.columns:
            s = df[name]
        else:
            s = df.xs(name, axis=1, level=0)
        return [float(x) for x in s.to_numpy().ravel()]
    closes = col("Close")
    highs = col("High")
    lows = col("Low")
    dates = [d.strftime("%Y-%m-%d") for d in df.index]
    return {"closes": closes, "highs": highs, "lows": lows, "dates": dates}


def to_mergebars(closes):
    tree = OnlineMergeTree()
    for p in closes:
        tree.update(p)
    snap_live = tree.current_barcode()
    final = tree.finalize()
    return tree, snap_live, final


def main():
    data = pull_mos()
    closes, highs, lows, dates = (
        data["closes"], data["highs"], data["lows"], data["dates"])
    n = len(closes)

    # ---- ATR 噪声阈 τ ----
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)

    # ---- (b) 批量 barcode: H0 sublevel + H1 loop(中枢级) ----
    bc = barcode_from_prices(closes, maxdim=1, embedding_dim=3, embedding_delay=1)
    h0 = bc.by_dimension(0)
    h1 = bc.by_dimension(1)
    act_h0 = active_bars(bc, tau, dimension=0)
    act_h1 = active_bars(bc, tau, dimension=1)

    # ---- (e) 在线 merge tree: alive vs settled ----
    tree, snap_live, final = to_mergebars(closes)
    th = tree.trend_health(window=5)

    # MergeBar 列表(含时间) —— 喂级别/中枢检测
    mbars = list(final.all_bars)

    # ---- (c) 级别检测 (log-gap 递归) ----
    lvl = detect_levels(mbars, bars_per_day=1.0, noise_floor=tau)

    # ---- (d) 中枢计数 (merge tree ≥3 同级子节点) ----
    zs = detect_zhongshu(mbars, bars_per_day=1.0, noise_floor=tau,
                         policy=ZhongshuPolicy(min_members=3, same_level_ratio=3.0,
                                               require_overlap=True))

    out = {
        "meta": {
            "symbol": "MOS", "n_bars": n,
            "date_start": dates[0], "date_end": dates[-1],
            "price_start": round(closes[0], 2), "price_end": round(closes[-1], 2),
            "price_min": round(min(closes), 2), "price_max": round(max(closes), 2),
            "tau_atr": round(tau, 4),
        },
        "barcode_h0": {
            "n_total": len(h0), "n_active": len(act_h0),
            "max_persistence": round(bc.max_persistence(0), 4),
            "total_persistence": round(bc.total_persistence(0), 4),
            "top5": [{"birth": round(b.birth, 2), "death": round(b.death, 2),
                      "pers": round(b.persistence, 3)} for b in act_h0[:5]],
        },
        "barcode_h1_loop": {
            "n_total": len(h1), "n_active": len(act_h1),
            "max_persistence": round(bc.max_persistence(1), 4) if h1 else 0.0,
            "top5": [{"birth": round(b.birth, 3), "death": round(b.death, 3),
                      "pers": round(b.persistence, 3)} for b in act_h1[:5]],
        },
        "levels": {
            "n_levels": len(lvl.levels),
            "n_roots": len(lvl.roots), "n_leaves": len(lvl.leaves),
            "detail": [{"id": L.level_id, "label": L.period_label,
                        "pers_range": [round(L.persistence_range[0], 3),
                                       round(L.persistence_range[1], 3)],
                        "n_bars": L.n_bars, "avg_span": round(L.avg_span, 1),
                        "parent": L.parent_id, "children": list(L.child_ids),
                        "depth": L.depth} for L in lvl.levels],
        },
        "zhongshu": {
            "count": len(zs),
            "detail": [{"label": z.period_label, "n_members": z.n_members,
                        "zd": round(z.zd, 2), "zg": round(z.zg, 2),
                        "width": round(z.width, 2), "span": z.span,
                        "lo": z.lo, "hi": z.hi,
                        "date_lo": dates[z.lo] if z.lo < n else "?",
                        "date_hi": dates[z.hi] if z.hi < n else "?",
                        "level_pers": round(z.level_persistence, 3)} for z in zs],
        },
        "online": {
            "n_settled": len(final.settled_bars),
            "n_alive_live_snapshot": len(snap_live.alive_bars),
            "n_settled_live_snapshot": len(snap_live.settled_bars),
            "alive_bars": [{"birth_price": round(b.birth_price, 2),
                            "death_price_est": round(b.death_price, 2),
                            "pers_est": round(b.persistence, 3),
                            "span": b.span, "lo": b.lo, "hi": b.hi,
                            "is_global": b.is_global,
                            "birth_date": dates[b.birth_idx] if b.birth_idx < n else "?"}
                           for b in snap_live.alive_bars],
            "trend_health": None if th is None else {
                "dominant_persistence": round(th.dominant_persistence, 3),
                "dominant_span": th.dominant_span,
                "dominant_birth_date": dates[th.dominant_birth_idx] if th.dominant_birth_idx < n else "?",
                "persistence_growth_rate": round(th.persistence_growth_rate, 4),
                "suppression_ratio": round(th.suppression_ratio, 3),
                "healthy": th.healthy,
            },
        },
        "recent_closes": [{"date": dates[i], "close": round(closes[i], 2)}
                          for i in range(max(0, n - 12), n)],
    }
    print(json.dumps(out, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
