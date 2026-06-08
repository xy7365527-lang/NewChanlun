"""OKLO settle 阶梯的时间演化分析 —— 区间套逐级确认（操盘相关），隔离全局分量。

框架纠正（用户 2026-05-30 指出）
--------------------------------
前一版报告把"全局最低分量不可反弹 settle"误读为"反弹要走到 $174 才算数"。错。
- **全局最低分量**（valley=5.59/45.58，settle_price=None）= 整个历史走势的基底，
  **定义上永远不被反弹 settle**（只在序列终止封顶）→ **与当前操盘无关，必须隔离**。
- **近端分量逐级 settle**（settle_price 有限，可反弹触及）= 区间套从小级别到大级别
  **逐级确认** → **操盘相关**。这才是"走势在某级别上完成"的拓扑信号。

本脚本做 settle 阶梯的**时间演化**（非单时点快照）：
1. 喂到近端低点 $45.58（2026-03-30），记录该时刻 alive 下跌分量集合（低点 alive 集）。
2. 继续喂到当前（2026-05-29），累积这期间 **newly settled** 的分量
   = 被 $45→$67 反弹 **逐级 settle 掉** 的下跌腿。
3. 当前仍 alive 的分量 = 尚未被反弹确认完成的下跌腿（含全局分量，单独标注）。
4. 每个分量按 persistence 量级（自适应 log-gap 带）贴缠论级别标签。

复用 W_despac 窗口（借壳后真实交易，隔离 SPAC）。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_settle_trigger import settle_triggers  # noqa: E402
from newchan.a_level_detection import (  # noqa: E402
    adaptive_gap_boundaries,
    CalendarPeriodNamer,
)
from newchan.a_persistence_barcode import atr_noise_threshold  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"


def level_namer_by_persistence(all_persistences: list[float]):
    """用自适应 log-gap 带把 persistence 值映射到级别序号（0=最粗）。

    返回 (band_of(p)->int, n_bands)。带边界由全体 persistence 的 MAD 离群 log-gap 决定，
    与 detect_levels_adaptive / detect_zhongshu 的分层口径一致（L0 稳健统计）。
    """
    boundaries = sorted(adaptive_gap_boundaries(all_persistences, n_mad=3.0), reverse=True)
    edges = [float("inf"), *boundaries, 0.0]

    def band_of(p: float) -> int:
        for k in range(len(edges) - 1):
            hi, lo = edges[k], edges[k + 1]
            if (p < hi or k == 0) and p >= lo:
                return k
        return len(edges) - 2

    return band_of, len(edges) - 1, boundaries


# 级别带序号 → 缠论级别名（粗→细），基于本数据的 persistence 量级分布
LEVEL_NAMES = ["周线/月线级", "日线级(大)", "日线级(中)", "日线级(小)", "60分级(大)", "60分级(小)", "30分级或更小"]


def lvl_name(band: int, n_bands: int) -> str:
    # 把 band 序号线性映射到 LEVEL_NAMES（band 0=最粗）
    if band < len(LEVEL_NAMES):
        return LEVEL_NAMES[band]
    return LEVEL_NAMES[-1]


def main() -> None:
    data = json.load(open(CACHE / "OKLO_1d_max.json"))
    dates_all, closes_all = data["dates"], data["closes"]
    highs_all, lows_all = data["highs"], data["lows"]
    # W_despac 切片
    import bisect
    i0 = bisect.bisect_left(dates_all, "2024-05-01")
    dates = dates_all[i0:]
    closes = closes_all[i0:]
    highs = highs_all[i0:]
    lows = lows_all[i0:]
    n = len(closes)

    last = closes[-1]
    last_date = dates[-1]

    # 近端低点 $45.58 索引
    low_val = min(closes)
    low_idx = closes.index(low_val)
    low_date = dates[low_idx]

    # ---- 喂到近端低点：记录该时刻 alive 集合 ----
    tree_low = OnlineMergeTree()
    for p in closes[: low_idx + 1]:
        tree_low.update(p)
    alive_at_low = {b.birth_idx for b in tree_low.current_barcode().alive_bars}

    # ---- 从低点之后逐根喂，累积 newly settled（被反弹 settle 掉的腿）----
    tree = OnlineMergeTree()
    for p in closes[: low_idx + 1]:
        tree.update(p)
    settled_during_rebound = []  # MergeBar
    for j in range(low_idx + 1, n):
        newly = tree.update(closes[j])
        for b in newly:
            settled_during_rebound.append((b, j))  # j = settle 发生的索引

    # ---- 当前活树（喂完全部）----
    cur_tree = OnlineMergeTree()
    for p in closes:
        cur_tree.update(p)
    trigs = settle_triggers(cur_tree)

    # ---- 级别带 ----
    # 关键修正（用户 2026-05-30 框架纠正的第二化身）：alive 分量的"级别"不能用
    # current_persistence = cap − valley 定，因为 cap=历史最高价(174) 会把近端小腿
    # 误判为大级别（valley 67.82 的最新腿 current_pers=106 → 错标日线级大）。
    # 正确度量是 **settle_persistence = settle_price − valley**（因果屏障幅度，
    # 该腿作为独立下跌摆动要被多大反弹吸收）。settled 腿用真实 death−birth（已是屏障）。
    snap = cur_tree.current_barcode()
    alive_settle_pers = [t.settle_price - t.birth_price
                         for t in trigs if t.settle_price is not None]
    band_pers = alive_settle_pers + [b.persistence for b, _ in settled_during_rebound]
    band_of, n_bands, boundaries = level_namer_by_persistence([p for p in band_pers if p > 0])

    def d(i):
        return dates[i] if 0 <= i < n else "?"

    out_lines: list[str] = []

    def emit(s: str = ""):
        print(s)
        out_lines.append(s)

    emit("=" * 78)
    emit(f"OKLO settle 阶梯·时间演化分析（W_despac 借壳后, n={n}）")
    emit(f"当前价 {last:.2f}（{last_date}）｜近端低点 {low_val:.2f}（{low_date}, idx={low_idx}）")
    emit(f"反弹幅度：{low_val:.2f} → {last:.2f}  (+{(last/low_val-1)*100:.1f}%)")
    emit(f"persistence 级别带边界（自适应 log-gap, 粗→细）: {[round(b,1) for b in boundaries]}")
    emit("=" * 78)

    # ============================================================
    # (1) 当前所有 alive 分量，按 settle_price 从小到大（=从近端到深层）
    # ============================================================
    emit("\n【1】当前 alive 分量（settle 阶梯，从近端→深层 / 小级别→大级别）")
    emit("-" * 78)
    emit(f"{'级别':<12}{'诞生日':<12}{'valley':>8}{'settle价':>9}{'gap':>8}{'settle幅':>8}{'可反弹settle':>10}")
    # 排序：可反弹的按 settle_price 升序在前，全局分量（None）置末
    settleable = sorted([t for t in trigs if t.settle_price is not None],
                        key=lambda t: t.settle_price)
    globals_ = [t for t in trigs if t.settle_price is None]
    emit("  （级别按 settle_persistence=settle价−valley 定，即该腿被反弹吸收的因果幅度）")
    for t in settleable:
        settle_pers = t.settle_price - t.birth_price
        band = band_of(settle_pers)
        emit(f"{lvl_name(band,n_bands):<12}{d(t.birth_idx):<12}{t.birth_price:>8.2f}"
             f"{t.settle_price:>9.2f}{t.gap_to_settle:>8.2f}{settle_pers:>8.2f}"
             f"{'是':>10}")
    for t in globals_:
        emit(f"{'周线/月线级(全局)':<12}{d(t.birth_idx):<12}{t.birth_price:>8.2f}"
             f"{'—(全局)':>9}{'—':>8}{'—':>8}{'否':>10}")
    emit(f"\n  近端待确认：valley={settleable[0].birth_price:.2f}（{d(settleable[0].birth_idx)}）"
         f"→ settle价 {settleable[0].settle_price:.2f}，gap {settleable[0].gap_to_settle:.2f}"
         f"（当前 {last:.2f}）")
    emit(f"  全局分量（隔离, 不参与操盘）：valley={globals_[0].birth_price:.2f}，"
         f"reversal_amp={globals_[0].reversal_amplitude:.2f} —— 整个历史走势基底，永不反弹 settle")

    # ============================================================
    # (2) 被 $45→$67 反弹 settle 掉的分量（低点之后 newly settled）
    # ============================================================
    emit("\n【2】已被 $45→$67 反弹 settle 掉的下跌腿（低点后 newly settled = 逐级确认完成）")
    emit("-" * 78)
    emit(f"{'级别':<12}{'诞生日':<12}{'valley':>8}{'死亡价(屏障)':>12}{'persistence':>12}{'span':>6}{'settle日':>12}")
    # 只保留 persistence 有意义的（> 噪声），按 settle 时间排序
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)
    sig = [(b, j) for b, j in settled_during_rebound if b.persistence > 0]
    sig.sort(key=lambda x: x[1])
    n_above_tau = 0
    for b, j in sig:
        if b.persistence < tau * 0.3:  # 极小噪声腿略过明细（仍计数）
            continue
        band = band_of(b.persistence)
        was_alive = "★低点时alive" if b.birth_idx in alive_at_low else ""
        if b.persistence > tau:
            n_above_tau += 1
        emit(f"{lvl_name(band,n_bands):<12}{d(b.birth_idx):<12}{b.birth_price:>8.2f}"
             f"{b.death_price:>12.2f}{b.persistence:>12.2f}{b.span:>6}{d(j):>12} {was_alive}")
    emit(f"\n  低点后共 settle {len(sig)} 个下跌腿（{n_above_tau} 个 > τ={tau:.2f}）。")
    emit(f"  其中标 ★ 的是 $45.58 低点时就 alive、被反弹 settle 掉的 —— 即区间套已逐级确认的级别。")

    # ============================================================
    # (3) 级别完成度阶梯：每个级别带的 settle 状态
    # ============================================================
    emit("\n【3】级别完成度阶梯（每级别带：已 settle vs 仍 alive）")
    emit("-" * 78)
    # 统计每个 band 的 settled / alive 数
    band_stat: dict[int, dict] = {}
    for b, j in settled_during_rebound:
        if b.persistence <= 0:
            continue
        bd = band_of(b.persistence)
        band_stat.setdefault(bd, {"settled": 0, "alive": 0, "settled_max_p": 0.0, "alive_max_p": 0.0})
        band_stat[bd]["settled"] += 1
        band_stat[bd]["settled_max_p"] = max(band_stat[bd]["settled_max_p"], b.persistence)
    GLOBAL_BAND = 0  # 全局分量归最粗带
    for t in trigs:
        if t.settle_price is None:
            bd = GLOBAL_BAND
            ap = t.current_persistence
        else:
            sp = t.settle_price - t.birth_price
            bd = band_of(sp)
            ap = sp
        band_stat.setdefault(bd, {"settled": 0, "alive": 0, "settled_max_p": 0.0, "alive_max_p": 0.0})
        band_stat[bd]["alive"] += 1
        band_stat[bd]["alive_max_p"] = max(band_stat[bd]["alive_max_p"], ap)
    emit(f"{'级别':<14}{'已settle数':>10}{'仍alive数':>10}{'级别走势状态':>20}")
    for bd in sorted(band_stat):
        s = band_stat[bd]
        if s["alive"] == 0 and s["settled"] > 0:
            status = "✅ 该级别下跌已完成确认"
        elif s["settled"] > 0 and s["alive"] > 0:
            status = "🔄 部分确认/部分未完成"
        elif s["alive"] > 0:
            status = "⏳ 仍未确认（含全局/最新腿）"
        else:
            status = "—"
        emit(f"{lvl_name(bd,n_bands):<14}{s['settled']:>10}{s['alive']:>10}   {status}")

    # ============================================================
    # (4) 缠论买卖点定位
    # ============================================================
    emit("\n【4】缠论含义：区间套逐级确认 → 买卖点定位")
    emit("-" * 78)
    near = settleable[0]
    # 小级别是否已确认完成？看最小级别带是否 alive=0
    fine_bands = [bd for bd in band_stat if bd >= n_bands - 2]  # 最细两带
    fine_settled = sum(band_stat[bd]["settled"] for bd in fine_bands)
    fine_alive = sum(band_stat[bd]["alive"] for bd in fine_bands)
    coarse_bands = [bd for bd in band_stat if bd <= 1]  # 最粗两带
    coarse_alive = sum(band_stat[bd]["alive"] for bd in coarse_bands)
    emit(f"  · 近端最小级别下跌腿（{d(near.birth_idx)} valley {near.birth_price:.2f}）"
         f"→ 升破 {near.settle_price:.2f} 即 settle，当前 {last:.2f} 差 {near.gap_to_settle:.2f}。")
    emit(f"  · 最细级别带：{fine_settled} 已 settle / {fine_alive} 仍 alive "
         f"→ {'小级别下跌已基本逐级确认完成（区间套向上收敛）' if fine_alive<=2 else '小级别仍在演化'}")
    emit(f"  · 最粗级别带（周/月线）：{coarse_alive} 仍 alive "
         f"→ 大级别下跌分量未 settle（含 174→45 主回撤的全局基底）")
    emit("")
    emit("  缠论翻译（L1~L2，PH↔缠论同构, 需 MACD 动力学层确认充分性, 521号）：")
    emit(f"  - 区间套从最小级别向上逐级 settle = 缠论次级别走势逐个完成 → 向上的中枢/趋势确认在累积。")
    emit(f"  - 若小级别已逐级确认（向上）+ 价格回抽不破前低 = 次级别二买/三买的拓扑前提。")
    emit(f"  - '周线三买的天线二买'对应：周线级（最粗带）仍 alive=大级别中枢尚未被一个")
    emit(f"    向上分量彻底 settle，但近端小级别已逐级 settle 向上 → 价格站在")
    emit(f"    '大级别中枢上沿附近、小级别向上已确认'的位置 = 三买区的次级别二买，与 PH 阶梯一致。")
    emit(f"  - 关键否定条件：若价格跌破近端 valley {near.birth_price:.2f}（{d(near.birth_idx)}），")
    emit(f"    最小级别向上确认被否定（对象否定对象），二买证伪。")

    emit("\n" + "=" * 78)
    emit("认识论：settle 阶梯算法 L0；'区间套逐级 settle ↔ 缠论次级别完成' L1（同构）；")
    emit("       '对应周线三买天线二买' L2（单标的可否证，需 MACD 力度确认充分性, 521号）。")

    # 存 JSON
    result = {
        "window": "W_despac", "last": round(last, 2), "last_date": last_date,
        "near_low": round(low_val, 2), "near_low_date": low_date,
        "rebound_pct": round((last / low_val - 1) * 100, 1),
        "level_band_boundaries": [round(b, 2) for b in boundaries],
        "alive_ladder": [
            {"level": "周线/月线级(全局)" if t.settle_price is None
                      else lvl_name(band_of(t.settle_price - t.birth_price), n_bands),
             "birth_date": d(t.birth_idx), "valley": round(t.birth_price, 2),
             "settle_price": None if t.settle_price is None else round(t.settle_price, 2),
             "settle_persistence": None if t.settle_price is None
                                   else round(t.settle_price - t.birth_price, 2),
             "gap_to_settle": None if t.gap_to_settle is None else round(t.gap_to_settle, 2),
             "can_settle_by_rebound": t.can_settle_by_rebound,
             "is_global": not t.can_settle_by_rebound}
            for t in sorted(trigs, key=lambda t: (t.settle_price is None, t.settle_price or 0))
        ],
        "settled_during_rebound": [
            {"level": lvl_name(band_of(b.persistence), n_bands),
             "birth_date": d(b.birth_idx), "valley": round(b.birth_price, 2),
             "death_barrier": round(b.death_price, 2), "persistence": round(b.persistence, 2),
             "span": b.span, "settle_date": d(j),
             "was_alive_at_low": b.birth_idx in alive_at_low}
            for b, j in sig if b.persistence >= tau * 0.3
        ],
        "band_stat": {lvl_name(bd, n_bands): band_stat[bd] for bd in sorted(band_stat)},
        "epistemic": "L0 算法; L1 区间套↔次级别同构; L2 周线三买天线二买对应(需MACD确认)",
    }
    (CACHE / "oklo_settle_ladder.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2))
    emit(f"\n已存 analysis/data_cache/oklo_settle_ladder.json")

    # 返回 markdown 追加块
    (CACHE / "_settle_ladder_lines.txt").write_text("\n".join(out_lines))


if __name__ == "__main__":
    main()
