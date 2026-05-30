"""布伦特原油 BZ=F 持续同调(PH) settle 阶梯分析 —— 与 OKLO/700/美团 完全同一套引擎。

本脚本对 Brent 期货连续合约 BZ=F 最近 2 年日线做同款分析：

  PH 七模块管线（a_persistence_barcode / a_level_detection / a_ph_zhongshu /
  a_online_persistence / a_settle_trigger / a_buypoint_score）
  + settle 阶梯时间演化（锚点 → 当前，区间套逐级确认）
  + 用户持仓水下分析（$109 均价 BZ 期货 + COIL C130 期权的 settle 反转确认价位）

数据诚实点（no-patch-mentality / formalization-validity-domain）
--------------------------------------------------------------
period="2y" 窗口的真实形态由脚本运行时输出决定，注释不预设叙事。Brent 是大宗商品
连续合约，2024 年中以来在地缘/OPEC+/需求三重压制下持续走弱。本脚本如实分析窗口内
真实走势，不弯曲方法贴叙事。

关键诚实点（与 700/美团 模板一致）
-----------------------------------
1. **因果屏障定级别**：级别一律用 settle_persistence = settle价 − valley（"未来一个
   反弹要多高才吸收这条下跌腿"，纯局部纯因果），**绝不用 cap−valley**。全局最低分量
   settle_price=None，永不反弹 settle，隔离为"全局腿"。

2. **级别名不过度声明**：级别映射直接复用引擎 detect_levels_adaptive 在**本数据上实际
   分辨出的簇**，不硬贴日线数据分辨不出的更细级别名。band_stat 按 level_id 统计。

3. **持仓 settle 确认价位**：用户 $109 成本的"确认反转"问题，严格映射到 settle_trigger
   的 settle_price——近端腿 settle = 近端止跌（nearest_rebound_settle），主导腿
   settle_price=None = 全局下跌未完成（dominant_trigger，§17.3 规则3）。不把全局低点
   伪装成"反弹到 X 即反转"（090号声明膨胀禁令）。

认识论：数据=真实(yfinance BZ=F 日线，L2 底材)；PH 算法 L0；
级别↔周期映射 L2；中枢/趋势判定 L1~L2；跨标的 L3 未做。
持仓水下/盈亏是纯算术 L0；"确认反转价位"是 settle 屏障读出 L0，翻译为操盘断言 L2。
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
SYMBOL = "BZ=F"
PERIOD = "2y"
COST_BASIS = 109.0  # 用户 BZ 期货持仓均价
OPTION_STRIKE = 130.0  # COIL C130 期权行权价（call）


def pull(symbol: str, period: str, interval: str = "1d") -> dict:
    df = yf.download(symbol, period=period, interval=interval,
                     auto_adjust=False, progress=False)
    if df.empty:
        raise SystemExit(f"yfinance 返回空数据: {symbol}")

    def col(name: str):
        if (name,) in df.columns or name in df.columns:
            s = df[name]
        else:
            s = df.xs(name, axis=1, level=0)
        return [float(x) for x in s.to_numpy().ravel()]

    # BZ=F 可能含 NaN 行（结算日缺口），剔除任一字段为 NaN 的根
    raw = {
        "closes": col("Close"), "highs": col("High"),
        "lows": col("Low"), "opens": col("Open"),
        "dates": [d.strftime("%Y-%m-%d") for d in df.index],
    }
    import math
    keep = [i for i in range(len(raw["closes"]))
            if not any(math.isnan(raw[k][i]) for k in ("closes", "highs", "lows", "opens"))]
    return {
        "symbol": symbol,
        "closes": [raw["closes"][i] for i in keep],
        "highs": [raw["highs"][i] for i in keep],
        "lows": [raw["lows"][i] for i in keep],
        "opens": [raw["opens"][i] for i in keep],
        "dates": [raw["dates"][i] for i in keep],
    }


# ====================================================================
# 级别映射：复用引擎 detect_levels_adaptive 的实际分级（非硬编码名池）
# ====================================================================


def build_engine_level_mapper(levels):
    """从 detect_levels_adaptive 的级别构建 persistence→(label, level_id, range) 映射。"""
    sl = sorted(levels, key=lambda L: L.persistence_range[1], reverse=True)
    boundaries: list[float] = []
    for i in range(len(sl) - 1):
        upper_lo = sl[i].persistence_range[0]
        lower_hi = sl[i + 1].persistence_range[1]
        if upper_lo > 0 and lower_hi > 0:
            boundaries.append((upper_lo * lower_hi) ** 0.5)
        else:
            boundaries.append((upper_lo + lower_hi) / 2.0)

    def mapper(p: float):
        for i, b in enumerate(boundaries):
            if p >= b:
                L = sl[i]
                return L.period_label, L.level_id, tuple(L.persistence_range)
        L = sl[-1]
        return L.period_label, L.level_id, tuple(L.persistence_range)

    return mapper, sl, boundaries


def analyze_ph(data: dict) -> dict:
    """PH 七模块（与 oklo_ph_driver / hk700 / 美团 同结构）。"""
    closes, highs, lows, dates = data["closes"], data["highs"], data["lows"], data["dates"]
    n = len(closes)
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)

    bc = barcode_from_prices(closes, maxdim=1, embedding_dim=3, embedding_delay=1)
    h0, h1 = bc.by_dimension(0), bc.by_dimension(1)
    act_h0 = active_bars(bc, tau, dimension=0)
    act_h1 = active_bars(bc, tau, dimension=1)

    tree = OnlineMergeTree()
    for p in closes:
        tree.update(p)
    snap_live = tree.current_barcode()
    th = tree.trend_health(window=5)
    final = tree.finalize()
    mbars = list(final.all_bars)

    lvl = detect_levels(mbars, bars_per_day=1.0, noise_floor=tau)
    lvl_ad = detect_levels_adaptive(mbars, bars_per_day=1.0, noise_floor=tau)
    zs = detect_zhongshu(mbars, bars_per_day=1.0, noise_floor=tau,
                         policy=ZhongshuPolicy(min_members=3, same_level_ratio=3.0,
                                               require_overlap=True))

    live_tree = OnlineMergeTree()
    for p in closes:
        live_tree.update(p)
    trigs = settle_triggers(live_tree)
    near = nearest_rebound_settle(trigs)
    dom = dominant_trigger(trigs)
    bp = score_buypoint(live_tree, highs=highs, lows=lows, closes=closes, policy=ScorePolicy())

    def d(i):
        return dates[i] if 0 <= i < n else "?"

    return {
        "meta": {
            "symbol": data["symbol"], "n_bars": n,
            "date_start": dates[0], "date_end": dates[-1],
            "price_start": round(closes[0], 2), "price_end": round(closes[-1], 2),
            "price_min": round(min(closes), 2), "price_max": round(max(closes), 2),
            "price_min_date": dates[closes.index(min(closes))],
            "price_max_date": dates[closes.index(max(closes))],
            "tau_atr": round(tau, 4),
        },
        "recent_closes": [{"date": dates[i], "close": round(closes[i], 2)}
                          for i in range(max(0, n - 12), n)],
        "barcode_h0": {
            "n_total": len(h0), "n_active": len(act_h0),
            "max_persistence": round(bc.max_persistence(0), 4),
            "total_persistence": round(bc.total_persistence(0), 4),
        },
        "barcode_h1_loop": {
            "n_total": len(h1), "n_active": len(act_h1),
            "max_persistence": round(bc.max_persistence(1), 4) if h1 else 0.0,
            "top5": [{"birth": round(b.birth, 3), "death": round(b.death, 3),
                      "pers": round(b.persistence, 3)} for b in act_h1[:5]],
        },
        "levels_recursive": {
            "n_levels": len(lvl.levels), "n_roots": len(lvl.roots), "n_leaves": len(lvl.leaves),
            "detail": [{"id": L.level_id, "label": L.period_label,
                        "pers_range": [round(L.persistence_range[0], 3),
                                       round(L.persistence_range[1], 3)],
                        "n_bars": L.n_bars, "depth": L.depth} for L in lvl.levels],
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
                        "date_lo": d(z.lo), "date_hi": d(z.hi),
                        "env_low": round(z.envelope_low, 2), "env_high": round(z.envelope_high, 2),
                        "level_pers": round(z.level_persistence, 3)} for z in zs],
        },
        "online": {
            "n_settled_final": len(final.settled_bars),
            "n_alive_live": len(snap_live.alive_bars),
            "n_settled_live": len(snap_live.settled_bars),
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
        },
        "buypoint_score": {
            "structure_completion": round(bp.structure_completion, 4),
            "zhongshu_count": bp.zhongshu_count,
            "zhongshu_score": round(bp.zhongshu_score, 4),
            "nesting_depth": bp.nesting_depth, "nesting_score": round(bp.nesting_score, 4),
            "alive_cleanliness": round(bp.alive_cleanliness, 4),
            "amplitude_decay": round(bp.amplitude_decay, 4),
            "composite_weighted": bp.composite_weighted,
            "composite_product": bp.composite_product,
            "dimensions": [round(x, 4) for x in bp.dimensions],
            "explanations": list(bp.explanations),
        },
    }


def settle_ladder(data: dict, anchor_idx: int, anchor_label: str) -> dict:
    """settle 阶梯时间演化（与 oklo/hk700/美团 同逻辑，因果屏障定级别）。"""
    closes, highs, lows, dates = data["closes"], data["highs"], data["lows"], data["dates"]
    n = len(closes)
    last, last_date = closes[-1], dates[-1]
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)

    low_idx = anchor_idx
    low_val = closes[low_idx]
    low_date = dates[low_idx]

    tree_low = OnlineMergeTree()
    for p in closes[: low_idx + 1]:
        tree_low.update(p)
    alive_at_low = {b.birth_idx for b in tree_low.current_barcode().alive_bars}

    tree = OnlineMergeTree()
    for p in closes[: low_idx + 1]:
        tree.update(p)
    settled_after_anchor = []
    for j in range(low_idx + 1, n):
        for b in tree.update(closes[j]):
            settled_after_anchor.append((b, j))

    cur_tree = OnlineMergeTree()
    for p in closes:
        cur_tree.update(p)
    trigs = settle_triggers(cur_tree)
    final_all = cur_tree.finalize()

    lvl_ad = detect_levels_adaptive(list(final_all.all_bars), bars_per_day=1.0, noise_floor=tau)
    level_of, sorted_levels, boundaries = build_engine_level_mapper(lvl_ad.levels)

    def d(i):
        return dates[i] if 0 <= i < n else "?"

    band_stat: dict[int, dict] = {}

    def ensure(lid, label, rng):
        if lid not in band_stat:
            band_stat[lid] = {"label": label, "pers_range": [round(rng[0], 1), round(rng[1], 1)],
                              "settled": 0, "alive": 0}
        return band_stat[lid]

    for b, j in settled_after_anchor:
        if b.persistence <= 0:
            continue
        label, lid, rng = level_of(b.persistence)
        ensure(lid, label, rng)["settled"] += 1
    for t in trigs:
        rep = (t.settle_price - t.birth_price) if t.settle_price is not None else t.current_persistence
        label, lid, rng = level_of(rep)
        ensure(lid, label, rng)["alive"] += 1

    sig = sorted([(b, j) for b, j in settled_after_anchor if b.persistence > 0],
                 key=lambda x: x[1])
    n_above_tau = sum(1 for b, _ in sig if b.persistence > tau)

    def alive_row(t):
        if t.settle_price is None:
            label, lid, _ = level_of(t.current_persistence)
            label = label + "(全局)"
        else:
            label, lid, _ = level_of(t.settle_price - t.birth_price)
        return {
            "level": label, "level_id": lid,
            "birth_date": d(t.birth_idx), "valley": round(t.birth_price, 2),
            "settle_price": None if t.settle_price is None else round(t.settle_price, 2),
            "settle_persistence": None if t.settle_price is None
                                  else round(t.settle_price - t.birth_price, 2),
            "gap_to_settle": None if t.gap_to_settle is None else round(t.gap_to_settle, 2),
            "current_persistence": round(t.current_persistence, 2),
            "is_dominant": t.is_dominant,
            "can_settle_by_rebound": t.can_settle_by_rebound,
            "reversal_amplitude": None if t.reversal_amplitude is None
                                  else round(t.reversal_amplitude, 2),
        }

    return {
        "window": PERIOD, "anchor_label": anchor_label,
        "last": round(last, 2), "last_date": last_date,
        "anchor_low": round(low_val, 2), "anchor_date": low_date, "anchor_idx": low_idx,
        "move_pct_from_anchor": round((last / low_val - 1) * 100, 1),
        "n_settled_after_anchor": len(sig), "n_above_tau": n_above_tau, "tau": round(tau, 2),
        "level_boundaries": [round(b, 2) for b in boundaries],
        "levels_used": [{"id": L.level_id, "label": L.period_label,
                         "pers_range": [round(L.persistence_range[0], 1),
                                        round(L.persistence_range[1], 1)]} for L in sorted_levels],
        "alive_ladder": [alive_row(t)
                         for t in sorted(trigs, key=lambda t: (t.settle_price is None, t.settle_price or 0))],
        "settled_after_anchor": [
            {"level": level_of(b.persistence)[0], "level_id": level_of(b.persistence)[1],
             "birth_date": d(b.birth_idx), "valley": round(b.birth_price, 2),
             "death_barrier": round(b.death_price, 2), "persistence": round(b.persistence, 2),
             "span": b.span, "settle_date": d(j),
             "was_alive_at_anchor": b.birth_idx in alive_at_low}
            for b, j in sig if b.persistence >= tau * 0.3
        ],
        "band_stat": {str(lid): band_stat[lid] for lid in sorted(band_stat)},
        "n_alive_total": len(trigs),
        "n_settleable": sum(1 for t in trigs if t.settle_price is not None),
        "n_globals": sum(1 for t in trigs if t.settle_price is None),
    }


def position_analysis(data: dict, cost_basis: float, option_strike: float) -> dict:
    """用户持仓水下分析 + settle 反转确认价位映射（任务问题2）。

    严格区分（090号声明膨胀禁令 + a_settle_trigger 顶部诚实点）：
    - 近端腿 settle_price（nearest_rebound_settle）= 近端止跌信号（最近一段下跌被吸收），
      **不等于**主级别反转。
    - 主导腿 settle_price=None（dominant_trigger）= 全局下跌未完成，只能被反向 persistence
      超过 reversal_amplitude 的结构"对象否定"（§17.3 规则3）。
    - 把全局低点伪装成"反弹到 X 即反转"= 声明膨胀，本函数拒绝这样做。

    认识论：水下/盈亏 L0 算术；settle 屏障读出 L0；翻译为操盘断言 L2。
    """
    closes = data["closes"]
    last = closes[-1]

    cur_tree = OnlineMergeTree()
    for p in closes:
        cur_tree.update(p)
    trigs = settle_triggers(cur_tree)
    near = nearest_rebound_settle(trigs)
    dom = dominant_trigger(trigs)

    underwater = last < cost_basis
    pnl_pct = (last / cost_basis - 1) * 100

    # 各级别 settle 屏障价（可反弹 settle 的腿），从近端到深层排序
    settleable = sorted(
        [t for t in trigs if t.settle_price is not None],
        key=lambda t: t.settle_price)  # type: ignore[arg-type]

    confirm_ladder = []
    for t in settleable:
        confirm_ladder.append({
            "settle_price": round(t.settle_price, 2),
            "settle_persistence": round(t.settle_price - t.birth_price, 2),
            "valley": round(t.birth_price, 2),
            "gap_from_last": round(t.settle_price - last, 2),
            "gap_pct_from_last": round((t.settle_price / last - 1) * 100, 1),
            "reaches_cost_basis": t.settle_price >= cost_basis,
            "reaches_option_strike": t.settle_price >= option_strike,
            "is_dominant": t.is_dominant,
        })

    # 主导（最深）下跌腿：通常 settle_price=None（全局未完成）
    dom_info = None
    if dom is not None:
        dom_info = {
            "valley": round(dom.birth_price, 2),
            "can_settle_by_rebound": dom.can_settle_by_rebound,
            "settle_price": None if dom.settle_price is None else round(dom.settle_price, 2),
            "reversal_amplitude": None if dom.reversal_amplitude is None
                                  else round(dom.reversal_amplitude, 2),
            "current_persistence": round(dom.current_persistence, 2),
            "current_cap": round(dom.current_cap, 2),
        }

    return {
        "cost_basis": cost_basis,
        "option_strike": option_strike,
        "last": round(last, 2),
        "underwater": underwater,
        "pnl_pct_futures": round(pnl_pct, 1),
        "drawdown_from_cost": round(cost_basis - last, 2),
        "n_settleable_legs": len(settleable),
        "nearest_settle": None if near is None else {
            "settle_price": round(near.settle_price, 2),
            "valley": round(near.birth_price, 2),
            "gap_from_last": round(near.settle_price - last, 2),
            "gap_pct_from_last": round((near.settle_price / last - 1) * 100, 1),
        },
        "confirm_ladder": confirm_ladder,
        "dominant_leg": dom_info,
        # 持仓回本/期权价内所需的纯算术目标（与 settle 屏障并列对照，不混淆）
        "breakeven_gap_pct": round((cost_basis / last - 1) * 100, 1),
        "option_itm_gap_pct": round((option_strike / last - 1) * 100, 1),
    }


def main() -> None:
    data = pull(SYMBOL, PERIOD)
    (CACHE / "BRN_1d_2y.json").write_text(json.dumps(data, ensure_ascii=False, indent=2))

    ph = analyze_ph(data)
    closes = data["closes"]
    gmin_idx = closes.index(min(closes))
    gmax_idx = closes.index(max(closes))
    ladder_global = settle_ladder(data, gmin_idx, "全局底锚点(2年最低, ≈当前)")
    ladder_decline = settle_ladder(data, gmax_idx, "见顶后回撤锚点(2024高位, 当前操盘相关)")
    pos = position_analysis(data, COST_BASIS, OPTION_STRIKE)
    result = {"ph": ph, "ladder_global": ladder_global,
              "ladder_decline": ladder_decline, "position": pos}
    (CACHE / "brn_settle_ladder.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2))
    ladder = ladder_decline

    m = ph["meta"]
    print("=" * 78)
    print(f"Brent 原油 BZ=F PH settle 阶梯  ({m['date_start']} → {m['date_end']}, n={m['n_bars']})")
    print("=" * 78)
    pct = (m["price_end"] / m["price_start"] - 1) * 100
    print(f"价格：起 {m['price_start']} → 终 {m['price_end']}  [{m['price_min']}, {m['price_max']}]  全程 {pct:+.1f}%")
    print(f"  2年最高 {m['price_max']}（{m['price_max_date']}）→ 2年最低 {m['price_min']}（{m['price_min_date']}）")
    print(f"ATR14 噪声阈 τ = {m['tau_atr']}")
    print(f"settle 锚点（操盘相关）: {ladder['anchor_label']} = {ladder['anchor_low']}（{ladder['anchor_date']}）→ 当前移动 {ladder['move_pct_from_anchor']:+.1f}%")
    print(f"\n[H0] {ph['barcode_h0']['n_total']} 特征 / {ph['barcode_h0']['n_active']} 活跃；max_pers={ph['barcode_h0']['max_persistence']}")
    print(f"[H1] {ph['barcode_h1_loop']['n_total']} loop / {ph['barcode_h1_loop']['n_active']} 活跃；max_pers={ph['barcode_h1_loop']['max_persistence']}")
    print(f"[级别-递归] {ph['levels_recursive']['n_levels']} 级")
    print(f"[级别-自适应] {ph['levels_adaptive']['n_levels']} 级：")
    for L in ph["levels_adaptive"]["detail"]:
        print(f"    id={L['id']} {L['label']} pers{L['pers_range']} n={L['n_bars']} span={L['avg_span']}")
    print(f"[中枢] {ph['zhongshu']['count']} 个")
    for z in ph["zhongshu"]["detail"]:
        print(f"    {z['label']} [zd={z['zd']}, zg={z['zg']}] 宽{z['width']} {z['n_members']}成员 {z['date_lo']}→{z['date_hi']}")
    o = ph["online"]
    print(f"[在线] settled={o['n_settled_final']}（finalize），实时 alive={o['n_alive_live']}")
    if o["trend_health"]:
        thh = o["trend_health"]
        print(f"    trend_health: dom_pers={thh['dominant_persistence']} (诞生 {thh['dominant_birth_date']}) growth={thh['persistence_growth_rate']} healthy={thh['healthy']}")
    st = ph["settle_triggers"]
    print(f"[settle] alive={st['n_alive']}；近端={st['nearest']}")
    print(f"    主导={st['dominant']}")
    bp = ph["buypoint_score"]
    print(f"[买点评分] 加权={bp['composite_weighted']}  几何乘积={bp['composite_product']}")
    print(f"    五维={bp['dimensions']} [结构完成/中枢/区间套/alive干净/振幅衰减]")

    print(f"\n--- settle 阶梯（{ladder['anchor_label']}）---")
    print(f"级别（引擎 detect_levels_adaptive 实际分级）边界（粗→细）: {ladder['level_boundaries']}")
    print(f"使用级别: {[(L['label'], L['pers_range']) for L in ladder['levels_used']]}")
    print(f"锚点后已 settle {ladder['n_settled_after_anchor']} 腿（{ladder['n_above_tau']} 个 >τ={ladder['tau']}）")
    print(f"[对照] 全局底锚点后已 settle {ladder_global['n_settled_after_anchor']} 腿"
          f"（{ladder_global['n_above_tau']} 个 >τ）")
    print(f"\nalive 阶梯（{ladder['n_settleable']} 可反弹 + {ladder['n_globals']} 全局, 共 {ladder['n_alive_total']}）：")
    for t in ladder["alive_ladder"]:
        sp = "—(全局)" if t["settle_price"] is None else f"{t['settle_price']}"
        gap = "—" if t["gap_to_settle"] is None else f"{t['gap_to_settle']}"
        spers = "—" if t["settle_persistence"] is None else f"{t['settle_persistence']}"
        dom = "★主导" if t["is_dominant"] else ""
        print(f"    {t['level']:<14} {t['birth_date']} valley={t['valley']:<8} "
              f"settle={sp:<8} gap={gap:<7} settle幅={spers:<7} {dom}")
    print(f"\n级别完成度（按 level_id）：")
    for lid, s in ladder["band_stat"].items():
        if s["alive"] == 0 and s["settled"] > 0:
            status = "✅ 该级别下跌已完成确认"
        elif s["settled"] > 0 and s["alive"] > 0:
            status = "🔄 部分确认/部分未完成"
        elif s["alive"] > 0:
            status = "⏳ 仍未确认（含全局/最新腿）"
        else:
            status = "—"
        print(f"    [{lid}] {s['label']:<8} pers{s['pers_range']}  settled={s['settled']:<3} alive={s['alive']:<3} {status}")

    print(f"\n--- 持仓水下分析（BZ 期货均价 ${pos['cost_basis']} / COIL C{int(pos['option_strike'])} 期权）---")
    wu = "🔴 水下" if pos["underwater"] else "🟢 水上"
    print(f"    当前 {pos['last']}  vs 成本 {pos['cost_basis']}  →  {wu}  期货浮盈亏 {pos['pnl_pct_futures']:+.1f}%（每桶亏 {pos['drawdown_from_cost']}）")
    print(f"    回本需 {pos['breakeven_gap_pct']:+.1f}%（到 {pos['cost_basis']}）；期权 C{int(pos['option_strike'])} 价内需 {pos['option_itm_gap_pct']:+.1f}%（到 {pos['option_strike']}）")
    if pos["nearest_settle"]:
        ns = pos["nearest_settle"]
        print(f"    近端止跌确认价（nearest settle）= {ns['settle_price']}（距今 {ns['gap_pct_from_last']:+.1f}%）—— 仅确认最近一段下跌被吸收，非主级别反转")
    print(f"    settle 反转确认阶梯（{pos['n_settleable_legs']} 条可反弹 settle 腿，价升到该价该腿下跌因果死亡）：")
    for c in pos["confirm_ladder"]:
        marks = []
        if c["reaches_cost_basis"]:
            marks.append("≥成本")
        if c["reaches_option_strike"]:
            marks.append("≥C130")
        if c["is_dominant"]:
            marks.append("★主导")
        print(f"      settle={c['settle_price']:<8} (距今 {c['gap_pct_from_last']:+5.1f}%) valley={c['valley']:<8} settle幅={c['settle_persistence']:<7} {' '.join(marks)}")
    if pos["dominant_leg"]:
        dl = pos["dominant_leg"]
        if dl["can_settle_by_rebound"]:
            print(f"    主导下跌腿：valley={dl['valley']}，settle 价={dl['settle_price']}（可反弹 settle）")
        else:
            print(f"    主导下跌腿：valley={dl['valley']}，**不可反弹 settle**（全局最深，§17.3 规则3）")
            print(f"      → 当前下跌假设只能被反向 persistence 超过 {dl['reversal_amplitude']} 的结构否定（对象否定对象）")
    print(f"\n已存 analysis/data_cache/brn_settle_ladder.json + BRN_1d_2y.json")


if __name__ == "__main__":
    main()
