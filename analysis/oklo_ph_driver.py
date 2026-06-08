"""OKLO（Oklo Inc.）持续同调(PH)分析驱动 —— 调用 src/newchan 的 PH 引擎。

与 MOS 用**同一套引擎、同一套方法学**（mos_ph_driver.py 的镜像 + settle/buypoint 串联），
以便逐项对比"OKLO 递归得很好 vs MOS 不行"是否成立。

七模块管线
----------
1. a_persistence_barcode.sublevel_h0_bars / rips_h1_bars —— H0/H1 barcode
2. a_level_detection.detect_levels(_adaptive) —— log-gap 级别识别
3. a_ph_zhongshu.detect_zhongshu —— 中枢计数（merge tree ≥3 同级重叠）
4. a_online_persistence.OnlineMergeTree —— 在线 merge tree（alive vs settled）
5. a_settle_trigger.settle_triggers —— alive→settled 触发阈值（近端 + 全局）
6. a_buypoint_score.score_buypoint —— 五维买点质量评分（几何乘积 + 加权）

认识论等级：数据=真实(yfinance OKLO 日线，L2 底材)；PH 算法 L0；
级别↔周期映射 L2；中枢/趋势判定 L1~L2；跨标的 L3 未做。
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
from newchan.a_level_detection import (  # noqa: E402
    detect_levels,
    detect_levels_adaptive,
)
from newchan.a_ph_zhongshu import detect_zhongshu, ZhongshuPolicy  # noqa: E402
from newchan.a_settle_trigger import (  # noqa: E402
    settle_triggers,
    nearest_rebound_settle,
    dominant_trigger,
)
from newchan.a_buypoint_score import score_buypoint, ScorePolicy  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"


def pull(symbol: str = "OKLO", period: str = "max", interval: str = "1d") -> dict:
    """拉 yfinance 日线（默认全部可用历史）。"""
    df = yf.download(symbol, period=period, interval=interval,
                     auto_adjust=False, progress=False)
    if df.empty:
        raise SystemExit("yfinance 返回空数据")

    def col(name: str):
        if (name,) in df.columns or name in df.columns:
            s = df[name]
        else:
            s = df.xs(name, axis=1, level=0)
        return [float(x) for x in s.to_numpy().ravel()]

    return {
        "symbol": symbol,
        "closes": col("Close"),
        "highs": col("High"),
        "lows": col("Low"),
        "opens": col("Open"),
        "dates": [d.strftime("%Y-%m-%d") for d in df.index],
    }


def analyze(data: dict) -> dict:
    closes, highs, lows, dates = (
        data["closes"], data["highs"], data["lows"], data["dates"])
    n = len(closes)
    symbol = data["symbol"]

    # ---- ATR 噪声阈 τ ----
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)

    # ---- (1) 批量 barcode：H0 sublevel + H1 loop ----
    bc = barcode_from_prices(closes, maxdim=1, embedding_dim=3, embedding_delay=1)
    h0 = bc.by_dimension(0)
    h1 = bc.by_dimension(1)
    act_h0 = active_bars(bc, tau, dimension=0)
    act_h1 = active_bars(bc, tau, dimension=1)

    # ---- (4) 在线 merge tree：alive vs settled ----
    tree = OnlineMergeTree()
    for p in closes:
        tree.update(p)
    snap_live = tree.current_barcode()
    th = tree.trend_health(window=5)
    final = tree.finalize()              # 注意：finalize 后 tree 无 alive，故先取 settle/score
    mbars = list(final.all_bars)

    # ---- (2) 级别检测：log-gap 递归 + 自适应 ----
    lvl = detect_levels(mbars, bars_per_day=1.0, noise_floor=tau)
    lvl_ad = detect_levels_adaptive(mbars, bars_per_day=1.0, noise_floor=tau)

    # ---- (3) 中枢计数（merge tree ≥3 同级重叠）----
    zs = detect_zhongshu(mbars, bars_per_day=1.0, noise_floor=tau,
                         policy=ZhongshuPolicy(min_members=3, same_level_ratio=3.0,
                                               require_overlap=True))

    # ---- (5) settle 触发阈值（须在 finalize 之前的活树上）----
    #   重新构建一棵活树（上面那棵已 finalize）。
    live_tree = OnlineMergeTree()
    for p in closes:
        live_tree.update(p)
    trigs = settle_triggers(live_tree)
    near = nearest_rebound_settle(trigs)
    dom = dominant_trigger(trigs)

    # ---- (6) 买点质量评分（因果快照 current_barcode）----
    #   等权 + 显式给出加权和几何乘积两种综合。
    bp = score_buypoint(live_tree, highs=highs, lows=lows, closes=closes,
                        policy=ScorePolicy())

    def d(i):  # 安全取日期
        return dates[i] if 0 <= i < n else "?"

    return {
        "meta": {
            "symbol": symbol, "n_bars": n,
            "date_start": dates[0], "date_end": dates[-1],
            "price_start": round(closes[0], 2), "price_end": round(closes[-1], 2),
            "price_min": round(min(closes), 2), "price_max": round(max(closes), 2),
            "tau_atr": round(tau, 4),
        },
        "recent_closes": [{"date": dates[i], "close": round(closes[i], 2)}
                          for i in range(max(0, n - 12), n)],
        "barcode_h0": {
            "n_total": len(h0), "n_active": len(act_h0),
            "max_persistence": round(bc.max_persistence(0), 4),
            "total_persistence": round(bc.total_persistence(0), 4),
            "top8": [{"birth": round(b.birth, 2), "death": round(b.death, 2),
                      "pers": round(b.persistence, 3)} for b in act_h0[:8]],
        },
        "barcode_h1_loop": {
            "n_total": len(h1), "n_active": len(act_h1),
            "max_persistence": round(bc.max_persistence(1), 4) if h1 else 0.0,
            "top5": [{"birth": round(b.birth, 3), "death": round(b.death, 3),
                      "pers": round(b.persistence, 3)} for b in act_h1[:5]],
        },
        "levels_recursive": {
            "n_levels": len(lvl.levels),
            "n_roots": len(lvl.roots), "n_leaves": len(lvl.leaves),
            "detail": [{"id": L.level_id, "label": L.period_label,
                        "pers_range": [round(L.persistence_range[0], 3),
                                       round(L.persistence_range[1], 3)],
                        "n_bars": L.n_bars, "avg_span": round(L.avg_span, 1),
                        "parent": L.parent_id, "children": list(L.child_ids),
                        "depth": L.depth} for L in lvl.levels],
        },
        "levels_adaptive": {
            "n_levels": len(lvl_ad.levels),
            "detail": [{"id": L.level_id, "label": L.period_label,
                        "pers_range": [round(L.persistence_range[0], 3),
                                       round(L.persistence_range[1], 3)],
                        "n_bars": L.n_bars, "avg_span": round(L.avg_span, 1)}
                       for L in lvl_ad.levels],
        },
        "zhongshu": {
            "count": len(zs),
            "detail": [{"label": z.period_label, "n_members": z.n_members,
                        "zd": round(z.zd, 2), "zg": round(z.zg, 2),
                        "width": round(z.width, 2), "span": z.span,
                        "lo": z.lo, "hi": z.hi,
                        "date_lo": d(z.lo), "date_hi": d(z.hi),
                        "env_low": round(z.envelope_low, 2),
                        "env_high": round(z.envelope_high, 2),
                        "level_pers": round(z.level_persistence, 3)} for z in zs],
        },
        "online": {
            "n_settled_final": len(final.settled_bars),
            "n_alive_live": len(snap_live.alive_bars),
            "n_settled_live": len(snap_live.settled_bars),
            "alive_bars": [{"birth_price": round(b.birth_price, 2),
                            "death_price_est": round(b.death_price, 2),
                            "pers_est": round(b.persistence, 3),
                            "span": b.span, "lo": b.lo, "hi": b.hi,
                            "is_global": b.is_global,
                            "birth_date": d(b.birth_idx)}
                           for b in sorted(snap_live.alive_bars,
                                           key=lambda b: b.persistence, reverse=True)],
            "trend_health": None if th is None else {
                "dominant_persistence": round(th.dominant_persistence, 3),
                "dominant_span": th.dominant_span,
                "dominant_birth_date": d(th.dominant_birth_idx),
                "persistence_growth_rate": round(th.persistence_growth_rate, 4),
                "suppression_ratio": round(th.suppression_ratio, 3),
                "healthy": th.healthy,
            },
        },
        "settle_triggers": {
            "n_alive": len(trigs),
            "nearest": None if near is None else {
                "birth_date": d(near.birth_idx), "valley": round(near.birth_price, 2),
                "settle_price": round(near.settle_price, 2),
                "gap_to_settle": round(near.gap_to_settle, 2),
            },
            "dominant": None if dom is None else {
                "birth_date": d(dom.birth_idx), "valley": round(dom.birth_price, 2),
                "settle_price": None if dom.settle_price is None else round(dom.settle_price, 2),
                "can_settle_by_rebound": dom.can_settle_by_rebound,
                "reversal_amplitude": None if dom.reversal_amplitude is None
                                       else round(dom.reversal_amplitude, 2),
                "current_cap": round(dom.current_cap, 2),
            },
            "all": [{"birth_date": d(t.birth_idx), "valley": round(t.birth_price, 2),
                     "settle_price": None if t.settle_price is None else round(t.settle_price, 2),
                     "gap_to_settle": None if t.gap_to_settle is None else round(t.gap_to_settle, 2),
                     "is_dominant": t.is_dominant,
                     "can_settle_by_rebound": t.can_settle_by_rebound,
                     "reversal_amplitude": None if t.reversal_amplitude is None
                                           else round(t.reversal_amplitude, 2)}
                    for t in trigs],
        },
        "buypoint_score": {
            "structure_completion": round(bp.structure_completion, 4),
            "zhongshu_count": bp.zhongshu_count,
            "zhongshu_score": round(bp.zhongshu_score, 4),
            "nesting_depth": bp.nesting_depth,
            "nesting_score": round(bp.nesting_score, 4),
            "alive_cleanliness": round(bp.alive_cleanliness, 4),
            "amplitude_decay": round(bp.amplitude_decay, 4),
            "composite_weighted": bp.composite_weighted,
            "composite_product": bp.composite_product,
            "dimensions": [round(x, 4) for x in bp.dimensions],
            "explanations": list(bp.explanations),
        },
        "epistemic_summary": (
            "PH算法L0；OKLO单标的实测L2；级别↔周期L2；中枢/趋势L1~L2；"
            "'高分=买点候选'L2（仅形态学必要条件非充分，§17.3/521）；跨标的L3未做"
        ),
    }


def main() -> None:
    data = pull("OKLO", period="max", interval="1d")
    # 缓存原始数据（与 MOS 同格式，便于复现/比价扩展）
    (CACHE / "OKLO_1d_max.json").write_text(
        json.dumps(data, ensure_ascii=False, indent=2))
    result = analyze(data)
    out = CACHE / "oklo_ph_result.json"
    out.write_text(json.dumps(result, ensure_ascii=False, indent=2))

    m = result["meta"]
    print("=" * 72)
    print(f"OKLO PH 分析  ({m['date_start']} → {m['date_end']}, n={m['n_bars']})")
    print("=" * 72)
    pct = (m["price_end"] / m["price_start"] - 1) * 100
    print(f"价格：起 {m['price_start']} → 终 {m['price_end']}  "
          f"[{m['price_min']}, {m['price_max']}]  全程 {pct:+.1f}%")
    print(f"ATR14 噪声阈 τ = {m['tau_atr']}")
    print(f"\n[H0] {result['barcode_h0']['n_total']} 特征，"
          f"{result['barcode_h0']['n_active']} 超过 τ；"
          f"max_pers={result['barcode_h0']['max_persistence']}，"
          f"total={result['barcode_h0']['total_persistence']}")
    print(f"[H1] {result['barcode_h1_loop']['n_total']} loop，"
          f"{result['barcode_h1_loop']['n_active']} 活跃；"
          f"max_pers={result['barcode_h1_loop']['max_persistence']}")
    lr = result["levels_recursive"]
    print(f"[级别-递归] {lr['n_levels']} 级（根{lr['n_roots']}/叶{lr['n_leaves']}）：")
    for L in lr["detail"]:
        print(f"    id={L['id']} {L['label']} pers{L['pers_range']} "
              f"n={L['n_bars']} span={L['avg_span']} depth={L['depth']}")
    la = result["levels_adaptive"]
    print(f"[级别-自适应] {la['n_levels']} 级：{[L['label'] for L in la['detail']]}")
    print(f"[中枢] {result['zhongshu']['count']} 个：")
    for z in result["zhongshu"]["detail"]:
        print(f"    {z['label']} [zd={z['zd']}, zg={z['zg']}] "
              f"宽{z['width']} {z['n_members']}成员 {z['date_lo']}→{z['date_hi']}")
    o = result["online"]
    print(f"[在线] settled={o['n_settled_final']}（finalize），"
          f"实时 alive={o['n_alive_live']} / settled={o['n_settled_live']}")
    if o["trend_health"]:
        thh = o["trend_health"]
        print(f"    trend_health: dom_pers={thh['dominant_persistence']} "
              f"(诞生 {thh['dominant_birth_date']}) growth={thh['persistence_growth_rate']} "
              f"healthy={thh['healthy']}")
    st = result["settle_triggers"]
    print(f"[settle] alive={st['n_alive']}；近端={st['nearest']}")
    print(f"    主导={st['dominant']}")
    bp = result["buypoint_score"]
    print(f"[买点评分] 加权={bp['composite_weighted']}  几何乘积={bp['composite_product']}")
    print(f"    五维={bp['dimensions']} "
          f"[结构完成/中枢/区间套/alive干净/振幅衰减]")
    print(f"\n认识论：{result['epistemic_summary']}")
    print(f"\n已存 {out.relative_to(ROOT)} 和 OKLO_1d_max.json")


if __name__ == "__main__":
    main()
