"""MOS 卖点视角·settle 阶梯——上涨分量如何被回调逐级 settle 掉（OKLO 模板的对偶）。

框架（用户 2026-05-30：从 MOS 最高级别看卖点）
----------------------------------------------
之前的分析看"下跌什么时候结束"（买点，下跌分量被反弹 settle）。本脚本反过来看
"上涨什么时候结束了"（卖点，**上涨分量被回调 settle**）。

严格对偶（数学精确，非 workaround）
-----------------------------------
OnlineMergeTree 追踪 sublevel set（谷=下跌分量，反弹 settle）。把价格序列**取负**
`q = -p` 喂进同一棵树：
- `q` 的谷 = 真实价的**峰** = 一条上涨腿的诞生（birth_peak = -birth_price_neg）。
- `q` 的"反弹 settle" = 真实价的**回调 settle**：价格跌到 settle 屏障价
  (settle_level = -settle_price_neg)，该上涨腿作为 younger 被合并、死亡因果确定
  = **那个级别的上涨走势已确认结束**。
- persistence（=屏障−谷 在 neg 空间 = 峰−回调屏障 在真实空间）量值取负不变 = 上涨腿幅度。
- gap 量值不变 = 价格还需再跌多少才能 settle 这条涨腿。

全局分量翻转（必读，090号 / formalization-validity-domain）
---------------------------------------------------------
买点视角：全局分量 = 历史最低谷，**永不被反弹 settle**（栈底 elder，只在 finalize 封顶）。
卖点视角（取负后）：全局分量 = **历史最高峰**，**永不被回调 settle**——它是整个牛市
的顶，是**最大的牛市结构残余**。把它伪装成"跌到 X 即 settle"= 声明膨胀（090号）。
对它诚实报告 settle_level=None，并给出反向否定幅度（reversal_amplitude）。

操盘语义映射
------------
- alive 上涨分量 = **牛市结构残余**（顶尚未被足够深的回调确认结束，仍有再创新高可能）。
- settled 上涨分量 = 那个级别的涨势**已确认结束**（被回调 settle 掉）。
- 全部 settle = 牛市完全结束；最高级别仍 alive = 最高级别牛市结构未被否定。

数据纠正（用户假设 vs 真实数据，look-before-assert）
--------------------------------------------------
用户说 ATH ≈ 2022 年 $78。真实 max 历史：MOS ATH = **2008-06 收盘 $161.08**（农产品
超级周期顶）。$78（2022-04，俄乌钾肥行情）是**次级峰**。本脚本两口径都跑：
- W_full：全历史，全局上涨分量 = 2008 $161 超级周期顶。
- W_cycle：2020 COVID 低（$7 区）起，隔离 $7→$78→$24 这一完整涨落弧 = 用户操盘关心的窗口。

认识论等级
----------
- 取负对偶 + settle 阶梯算法：L0（merge tree 确定性属性，与买点侧同构）。
- "回调 settle ↔ 缠论上涨走势完成"同构：L1。
- MOS 单标的翻译为具体卖点价：L2（可否证，未来走势改写阈值）。
"""
from __future__ import annotations

import bisect
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_settle_trigger import settle_triggers  # noqa: E402
from newchan.a_level_detection import adaptive_gap_boundaries  # noqa: E402
from newchan.a_persistence_barcode import atr_noise_threshold  # noqa: E402
from newchan.a_buypoint_score import score_buypoint  # noqa: E402
from newchan.a_ph_zhongshu import detect_zhongshu  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"

# 级别带序号 → 缠论级别名（粗→细），基于 persistence 量级分布
LEVEL_NAMES = ["年线/超级周期级", "周线/月线级", "日线级(大)", "日线级(中)",
               "日线级(小)", "60分级(大)", "60分级(小)", "30分级或更小"]


def level_namer(all_persistences: list[float], *, noise_tau: float = 0.0):
    """自适应 log-gap 带把 persistence 映射到级别序号（0=最粗），与买点侧同口径（L0）。

    长序列防碎裂（MOS 全历史有上千微小腿）：
    1. 先用 ATR 噪声阈 noise_tau 过滤——噪声腿不参与级别边界计算（否则 MAD 间隙谱
       被微小腿打碎成几十个带）。这是 detect_zhongshu/detect_levels 同款 noise_floor 口径。
    2. 边界数封顶到 len(LEVEL_NAMES)-1：保留**persistence 值最大的**几个分隔（最粗的
       级别分界），低于最小保留边界的腿全部归入最细命名。映射 1:1 到 8 个级别名。
    """
    sig = [p for p in all_persistences if p > max(0.0, noise_tau)]
    boundaries = sorted(adaptive_gap_boundaries(sig, n_mad=3.0), reverse=True)
    # 封顶：保留最粗的 (N_names-1) 个分界（值最大者），其余细腿同归最细带
    max_bounds = len(LEVEL_NAMES) - 1
    if len(boundaries) > max_bounds:
        boundaries = boundaries[:max_bounds]
    edges = [float("inf"), *boundaries, 0.0]

    def band_of(p: float) -> int:
        for k in range(len(edges) - 1):
            hi, lo = edges[k], edges[k + 1]
            if (p < hi or k == 0) and p >= lo:
                return k
        return len(edges) - 2

    return band_of, len(edges) - 1, boundaries


def lvl_name(band: int) -> str:
    return LEVEL_NAMES[band] if band < len(LEVEL_NAMES) else LEVEL_NAMES[-1]


def run_window(label: str, dates, closes, highs, lows, emit, *, detail: bool = True) -> dict:
    """对一个价格窗口跑卖点 settle 阶梯（取负对偶）。返回 JSON 结果块。

    detail=False（如 W_full 全历史）：section 2 只显示 > τ 的显著腿计数，不刷全部明细。
    """
    n = len(closes)
    last = closes[-1]
    last_date = dates[-1]

    # 窗口内历史最高峰（上涨分量的全局顶）
    peak_val = max(closes)
    peak_idx = closes.index(peak_val)
    peak_date = dates[peak_idx]

    def d(i):
        return dates[i] if 0 <= i < n else "?"

    # ---- 取负对偶：q = -p ----
    neg = [-p for p in closes]

    # ---- 喂到历史最高峰：记录该时刻 alive 上涨分量集合 ----
    tree_peak = OnlineMergeTree()
    for q in neg[: peak_idx + 1]:
        tree_peak.update(q)
    alive_at_peak = {b.birth_idx for b in tree_peak.current_barcode().alive_bars}

    # ---- 从峰之后逐根喂，累积 newly settled（被回调 settle 掉的上涨腿）----
    tree = OnlineMergeTree()
    for q in neg[: peak_idx + 1]:
        tree.update(q)
    settled_during_decline = []  # (MergeBar_neg, settle_idx)
    for j in range(peak_idx + 1, n):
        for b in tree.update(neg[j]):
            settled_during_decline.append((b, j))

    # ---- 当前活树（喂完全部 neg）----
    cur_tree = OnlineMergeTree()
    for q in neg:
        cur_tree.update(q)
    trigs = settle_triggers(cur_tree)

    # ---- 噪声阈 τ（ATR）：级别边界计算 + 显著腿过滤的共同口径 ----
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)

    # ---- 级别带（用 settle_persistence = 该涨腿被回调吸收的因果幅度）----
    alive_settle_pers = [t.settle_persistence for t in trigs if t.settle_price is not None]
    band_pers = [p for p in alive_settle_pers if p and p > 0] + \
                [b.persistence for b, _ in settled_during_decline if b.persistence > 0]
    band_of, n_bands, boundaries = level_namer(band_pers, noise_tau=tau)

    emit("=" * 84)
    emit(f"MOS 卖点·settle 阶梯 [{label}]（取负对偶, n={n}）")
    emit(f"窗口 {dates[0]} → {last_date}｜当前价 {last:.2f}")
    emit(f"窗口最高峰（全局上涨分量顶）{peak_val:.2f}（{peak_date}, idx={peak_idx}）"
         f"｜峰→今 {(last/peak_val-1)*100:+.1f}%")
    emit(f"persistence 级别带边界（自适应 log-gap, 粗→细）: {[round(b,1) for b in boundaries]}")
    emit("=" * 84)

    # ============================================================
    # (1) 当前 alive 上涨分量 = 牛市结构残余，settle 阶梯（近端顶→深层顶）
    # ============================================================
    emit("\n【1】当前 alive 上涨分量 = 牛市结构残余（顶尚未被回调确认结束）")
    emit("-" * 84)
    emit("  级别按 settle_persistence=峰−回调屏障 定（该涨腿被回调吸收的因果幅度）")
    emit(f"{'级别':<16}{'诞生顶日':<12}{'峰价':>8}{'settle屏障':>10}{'涨腿幅':>8}"
         f"{'还需跌':>8}{'可回调settle':>11}")
    settleable = sorted([t for t in trigs if t.settle_price is not None],
                        key=lambda t: t.settle_persistence)
    globals_ = [t for t in trigs if t.settle_price is None]
    for t in settleable:
        real_peak = -t.birth_price
        real_settle = -t.settle_price
        up_amp = t.settle_persistence
        band = band_of(up_amp)
        emit(f"{lvl_name(band):<16}{d(t.birth_idx):<12}{real_peak:>8.2f}"
             f"{real_settle:>10.2f}{up_amp:>8.2f}{t.gap_to_settle:>8.2f}{'是':>11}")
    for t in globals_:
        real_peak = -t.birth_price
        emit(f"{lvl_name(0)+'(全局顶)':<16}{d(t.birth_idx):<12}{real_peak:>8.2f}"
             f"{'—(全局)':>10}{t.reversal_amplitude:>8.2f}{'—':>8}{'否':>11}")
    if globals_:
        g = globals_[0]
        emit(f"\n  全局上涨分量（隔离, 牛市最高顶）：峰={-g.birth_price:.2f}"
             f"（{d(g.birth_idx)}），reversal_amp={g.reversal_amplitude:.2f}")
        emit("  → 这是整个牛市的顶，永不被回调 settle（只在序列终止封顶）= 最大牛市结构残余。")
        emit("  → 要否定它（确认牛市彻底结束），需一个反向下跌结构 persistence 超过此幅度。")

    # ============================================================
    # (2) 已被回调 settle 掉的上涨腿（峰后 newly settled = 那级别涨势已确认结束）
    # ============================================================
    emit("\n【2】已被回调 settle 掉的显著上涨腿（峰后逐级确认结束 = 卖点逐级兑现, 按 settle 时间）")
    emit("-" * 84)
    sig_all = [(b, j) for b, j in settled_during_decline if b.persistence > 0]
    above_tau = [(b, j) for b, j in sig_all if b.persistence > tau]
    above_tau.sort(key=lambda x: x[1])  # 按 settle 时间 = 逐级 settle 的时间阶梯
    if detail:
        emit(f"{'级别':<16}{'诞生顶日':<12}{'峰价':>8}{'回调屏障(死)':>12}{'涨腿幅':>8}"
             f"{'span':>6}{'settle日':>12}")
        for b, j in above_tau:
            band = band_of(b.persistence)
            star = "★峰时alive" if b.birth_idx in alive_at_peak else ""
            emit(f"{lvl_name(band):<16}{d(b.birth_idx):<12}{-b.birth_price:>8.2f}"
                 f"{-b.death_price:>12.2f}{b.persistence:>8.2f}{b.span:>6}{d(j):>12} {star}")
    else:
        # W_full：只显示最粗级别（年线/周线/日线大）的 settle，细腿计数
        coarse = [(b, j) for b, j in above_tau if band_of(b.persistence) <= 3]
        emit(f"  （全历史明细过多，仅列最粗级别 band≤3 的 {len(coarse)} 条；细腿见计数）")
        emit(f"{'级别':<16}{'诞生顶日':<12}{'峰价':>8}{'回调屏障(死)':>12}{'涨腿幅':>8}{'settle日':>12}")
        for b, j in sorted(coarse, key=lambda x: x[1]):
            emit(f"{lvl_name(band_of(b.persistence)):<16}{d(b.birth_idx):<12}{-b.birth_price:>8.2f}"
                 f"{-b.death_price:>12.2f}{b.persistence:>8.2f}{d(j):>12}")
    emit(f"\n  峰后共 settle {len(sig_all)} 条上涨腿，其中 {len(above_tau)} 条 > τ={tau:.2f}（显著）。")
    emit("  ★ = 历史最高峰时就 alive、被随后下跌 settle 掉的涨腿 = 区间套逐级向下确认的级别。")

    # ============================================================
    # (3) 级别完成度阶梯：每个级别带的涨势 settle 状态
    # ============================================================
    emit("\n【3】级别完成度阶梯（每级别带：上涨已 settle vs 仍 alive）")
    emit("-" * 84)
    band_stat: dict[int, dict] = {}
    for b, _ in settled_during_decline:
        if b.persistence <= 0:
            continue
        bd = band_of(b.persistence)
        band_stat.setdefault(bd, {"settled": 0, "alive": 0})
        band_stat[bd]["settled"] += 1
    for t in trigs:
        if t.settle_price is None:
            bd = 0  # 全局顶归最粗带
        else:
            bd = band_of(t.settle_persistence)
        band_stat.setdefault(bd, {"settled": 0, "alive": 0})
        band_stat[bd]["alive"] += 1
    # 多个 band 可能共享同一级别名（细腿统归最细名）→ 按名字合并，避免重复行
    name_stat: dict[str, dict] = {}
    name_order: list[str] = []
    for bd in sorted(band_stat):
        nm = lvl_name(bd)
        if nm not in name_stat:
            name_stat[nm] = {"settled": 0, "alive": 0}
            name_order.append(nm)
        name_stat[nm]["settled"] += band_stat[bd]["settled"]
        name_stat[nm]["alive"] += band_stat[bd]["alive"]
    emit(f"{'级别':<18}{'已settle数':>10}{'仍alive数':>10}   {'级别涨势状态':<22}")
    for nm in name_order:
        s = name_stat[nm]
        if s["alive"] == 0 and s["settled"] > 0:
            status = "✅ 该级别涨势已确认结束"
        elif s["settled"] > 0 and s["alive"] > 0:
            status = "🔄 部分级别涨势已结束/部分残余"
        elif s["alive"] > 0:
            status = "⏳ 涨势未被回调确认（含全局顶/残余）"
        else:
            status = "—"
        emit(f"{nm:<18}{s['settled']:>10}{s['alive']:>10}   {status}")

    # ============================================================
    # (4) 缠论卖点定位
    # ============================================================
    emit("\n【4】缠论含义：上涨分量逐级 settle → 卖点定位")
    emit("-" * 84)
    coarse_alive = sum(band_stat[bd]["alive"] for bd in band_stat if bd <= 1)
    fine_settled = sum(band_stat[bd]["settled"] for bd in band_stat if bd >= n_bands - 2)
    fine_alive = sum(band_stat[bd]["alive"] for bd in band_stat if bd >= n_bands - 2)
    if settleable:
        near = settleable[0]  # 最小涨腿幅 = 最近端、最易被回调 settle
        emit(f"  · 近端最小级别上涨腿（顶 {-near.birth_price:.2f}@{d(near.birth_idx)}）"
             f"→ 跌破 {-near.settle_price:.2f} 即 settle，当前 {last:.2f} 还需跌 {near.gap_to_settle:.2f}。")
    emit(f"  · 最细级别带：{fine_settled} 已 settle / {fine_alive} 仍 alive "
         f"→ {'小级别涨势已基本逐级确认结束（卖点逐级兑现）' if fine_alive<=2 else '小级别仍在演化'}")
    emit(f"  · 最粗级别带（年/周线）：{coarse_alive} 仍 alive "
         f"→ {'最高级别牛市结构未被否定（含全局顶）' if coarse_alive>0 else '最高级别涨势也已 settle = 牛市完全结束'}")
    emit("")
    emit("  缠论翻译（L1~L2，PH↔缠论同构, 卖点充分性需 MACD 顶背驰确认, 521号）：")
    emit("  - 上涨腿被回调逐级 settle = 缠论次级别上涨走势逐个完成 → 卖点逐级兑现/趋势转折累积。")
    emit("  - 最高级别（全局顶）仍 alive = 牛市最大结构残余未被否定 → 反弹仍可能（但级别越来越小）。")
    emit("  - 关键否定条件：价格跌破 alive 上涨腿的 settle 屏障 = 该级别涨势确认结束（对象否定对象）。")
    if globals_:
        emit(f"  - 全局顶 {-globals_[0].birth_price:.2f} 永不被回调 settle：要确认牛市彻底终结，"
             f"需反向下跌结构幅度 > {globals_[0].reversal_amplitude:.2f}。")

    return {
        "label": label,
        "window": [dates[0], last_date],
        "last": round(last, 2),
        "peak": round(peak_val, 2), "peak_date": peak_date,
        "peak_to_now_pct": round((last / peak_val - 1) * 100, 1),
        "level_band_boundaries": [round(b, 2) for b in boundaries],
        "alive_uplegs": [
            {"level": lvl_name(0) + "(全局顶)" if t.settle_price is None
                      else lvl_name(band_of(t.settle_persistence)),
             "peak_date": d(t.birth_idx), "peak_price": round(-t.birth_price, 2),
             "settle_level": None if t.settle_price is None else round(-t.settle_price, 2),
             "up_amplitude": round(t.settle_persistence if t.settle_price is not None
                                   else t.reversal_amplitude, 2),
             "gap_to_settle": None if t.gap_to_settle is None else round(t.gap_to_settle, 2),
             "can_settle_by_pullback": t.can_settle_by_rebound,
             "is_global_top": not t.can_settle_by_rebound}
            for t in sorted(trigs, key=lambda t: (t.settle_price is None,
                                                  t.settle_persistence or 0))
        ],
        "settled_during_decline": [
            {"level": lvl_name(band_of(b.persistence)),
             "peak_date": d(b.birth_idx), "peak_price": round(-b.birth_price, 2),
             "pullback_barrier": round(-b.death_price, 2),
             "up_amplitude": round(b.persistence, 2), "span": b.span,
             "settle_date": d(j), "was_alive_at_peak": b.birth_idx in alive_at_peak}
            for b, j in above_tau
        ],
        "band_stat": name_stat,
    }


def main() -> None:
    data = json.load(open(CACHE / "MOS_1d_max.json"))
    dates_all, closes_all = data["dates"], data["closes"]
    highs_all, lows_all = data["highs"], data["lows"]

    out_lines: list[str] = []

    def emit(s: str = ""):
        print(s)
        out_lines.append(s)

    # ---- 对照：买点评分 + 中枢计数（真实序列，不取负）----
    cur_tree = OnlineMergeTree()
    for p in closes_all:
        cur_tree.update(p)
    bp = score_buypoint(cur_tree, highs=highs_all, lows=lows_all, closes=closes_all)
    final = cur_tree.finalize()
    zs = detect_zhongshu(final.settled_bars, bars_per_day=1.0,
                         noise_floor=atr_noise_threshold(highs_all, lows_all, closes_all))

    # ---- 卖点两口径 ----
    res_full = run_window("W_full 全历史", dates_all, closes_all, highs_all, lows_all,
                          emit, detail=False)

    i0 = bisect.bisect_left(dates_all, "2020-03-15")
    res_cycle = run_window("W_cycle 2020COVID低起($7→$78→今)",
                           dates_all[i0:], closes_all[i0:], highs_all[i0:], lows_all[i0:], emit)

    # ---- 对照块 ----
    emit("\n" + "=" * 84)
    emit("【对照】买点评分 + 中枢计数（真实序列, 全历史, 买点视角）")
    emit("-" * 84)
    emit(f"  综合分(加权)={bp.composite_weighted:.1f}/100  (几何)={bp.composite_product:.1f}/100")
    emit(f"  结构完成度={bp.structure_completion:.2f}  中枢数={bp.zhongshu_count}"
         f"  区间套深度={bp.nesting_depth}  alive洁净度={bp.alive_cleanliness:.2f}"
         f"  振幅衰减={bp.amplitude_decay:.2f}")
    emit(f"  detect_zhongshu 识别中枢 {len(zs)} 个（全历史 settled bars）。")
    emit("  注：买点评分为买点（下跌分量）形态前置，与本脚本卖点（上涨分量）视角互为镜像。")
    emit("      高分=结构上具备买点候选形态前提，非买入信号（充分性需 MACD 力度, 521号）。")

    emit("\n" + "=" * 84)
    emit("认识论：取负对偶 + settle 阶梯 L0；'回调 settle ↔ 上涨走势完成' L1（同构）；")
    emit("       MOS 单标的卖点价 L2（可否证，未来走势改写阈值，需 MACD 顶背驰确认充分性, 521号）。")

    result = {
        "symbol": "MOS",
        "all_time_high": {"close": round(max(closes_all), 2),
                          "date": dates_all[closes_all.index(max(closes_all))]},
        "data_correction": "真实 ATH=2008 $161.08（超级周期顶）, 用户假设的 $78 是 2022 次级峰",
        "W_full": res_full,
        "W_cycle": res_cycle,
        "buypoint_contrast": {
            "composite_weighted": round(bp.composite_weighted, 1),
            "composite_product": round(bp.composite_product, 1),
            "structure_completion": round(bp.structure_completion, 2),
            "zhongshu_count": bp.zhongshu_count,
            "nesting_depth": bp.nesting_depth,
            "alive_cleanliness": round(bp.alive_cleanliness, 2),
            "amplitude_decay": round(bp.amplitude_decay, 2),
            "detect_zhongshu_n": len(zs),
        },
        "epistemic": "L0 取负对偶+阶梯; L1 回调settle↔上涨完成同构; L2 单标的卖点价(需MACD顶背驰)",
    }
    (CACHE / "mos_sellpoint_settle.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2))
    (CACHE / "_mos_sellpoint_lines.txt").write_text("\n".join(out_lines))
    emit("\n已存 analysis/data_cache/mos_sellpoint_settle.json")


if __name__ == "__main__":
    main()
