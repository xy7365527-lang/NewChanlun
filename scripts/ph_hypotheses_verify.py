"""PH 四假说 L2 验证 — QQQ 日线（2024-08-27 至 2026-05-29，440 根）。

认识论等级：L2（单标的单时段，可否证，不外推到其他标的/时段）

假说1：alive 分量递减 = 背驰（底背驰区域 alive_count 趋势性下降）
假说2：superlevel settle persistence > sublevel settle persistence = 趋势反转
假说3：双树 alive 差值变号 = 趋势转折（买卖点出现前 dual_diff 变号）
假说4：alive 数做连续力度指标（与未来 N 日收益的信息系数 IC）

数据源：
  价格：analysis/data_cache/QQQ_1d_max.json（6848 根，1999-2026）
  缠论标注：analysis/data_cache/qqq_daily_chanlun.json（440 根，label_time 0-440）

时间对齐：
  label_time T → QQQ bar index 6408 + T（6848 - 440 = 6408，2024-08-27 起）
  local_idx = label_time ∈ [0, 439]（共 440 根）

有效域约束：
  - 本分析仅使用 close 价（H0 sublevel/superlevel on close, not high/low）
  - Persistence ≈ 幅度（ker(D)），不可替代 MACD 动量
  - 115 个买卖点事件 / 440 根 K 线：统计量有限，结论为 L2 起点
"""
from __future__ import annotations

import json
import sys
import math
from pathlib import Path
from collections import defaultdict

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_dual_merge_tree import DualMergeTree  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
OUT_MD = ROOT / "analysis" / "ph_hypotheses_results.md"
QQQ_PRICE_FILE = CACHE / "QQQ_1d_max.json"
CHANLUN_FILE = CACHE / "qqq_daily_chanlun.json"

WINDOW_BARS = 440        # TV 标注覆盖的 bar 数
OFFSET = 6848 - WINDOW_BARS  # = 6408，QQQ 本地起始索引
LOOKBACK = 5             # 假说1/2/3 前瞻/回溯窗口（天）
FORWARD_WINDOWS = [5, 10, 20]  # 假说4 IC 计算窗口


# ====================================================================
# 工具函数
# ====================================================================

def pearson_r(xs: list[float], ys: list[float]) -> float:
    n = len(xs)
    if n < 3:
        return float("nan")
    mx = sum(xs) / n
    my = sum(ys) / n
    cov = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    sx = math.sqrt(sum((x - mx) ** 2 for x in xs) + 1e-12)
    sy = math.sqrt(sum((y - my) ** 2 for y in ys) + 1e-12)
    return cov / (sx * sy)


def sign(x: float) -> int:
    if x > 0:
        return 1
    if x < 0:
        return -1
    return 0


def linear_slope(ys: list[float]) -> float:
    """最小二乘斜率（每步变化）。"""
    n = len(ys)
    if n < 2:
        return 0.0
    xs = list(range(n))
    mx = sum(xs) / n
    my = sum(ys) / n
    num = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    den = sum((x - mx) ** 2 for x in xs) + 1e-12
    return num / den


def precision_recall(
    signal_bars: list[int],
    event_bars: list[int],
    n_bars: int,
    window: int,
) -> dict[str, float]:
    """计算信号 → 事件的精确率/召回率及 null baseline。

    precision = % 信号后 window 内有事件
    recall    = % 事件前 window 内有信号
    null_precision = P(至少 1 个事件在信号后 window 内 | 随机信号)
                   = 1 - ((n_bars - n_events) / n_bars)^window
                   这才是正确的 precision 零假设（不是 n_events/n_bars）。
    lift = precision / null_precision（>1 表示信号有效）
    """
    event_set = set(event_bars)
    signal_set = set(signal_bars)
    n_events = len(event_bars)
    base_rate = n_events / max(n_bars, 1)
    null_prec = 1.0 - ((n_bars - n_events) / max(n_bars, 1)) ** window

    if not signal_set:
        return {
            "precision": float("nan"), "recall": float("nan"),
            "base_rate": base_rate, "null_precision": null_prec,
            "lift": float("nan"), "n_signals": 0, "n_events": n_events,
        }

    prec_hits = sum(
        1 for s in signal_set
        if any(s <= e <= s + window for e in event_set)
    )
    rec_hits = sum(
        1 for e in event_set
        if any(e - window <= s <= e for s in signal_set)
    )
    prec = prec_hits / len(signal_set)
    lift = prec / null_prec if null_prec > 1e-9 else float("nan")
    return {
        "precision": prec,
        "recall": rec_hits / max(n_events, 1),
        "base_rate": base_rate,
        "null_precision": null_prec,
        "lift": lift,
        "n_signals": len(signal_set),
        "n_events": n_events,
    }


# ====================================================================
# 数据加载
# ====================================================================

def load_prices() -> dict[str, list[float]]:
    with open(QQQ_PRICE_FILE) as f:
        d = json.load(f)
    return {
        "closes": d["closes"][OFFSET: OFFSET + WINDOW_BARS],
        "highs":  d["highs"][OFFSET:  OFFSET + WINDOW_BARS],
        "lows":   d["lows"][OFFSET:   OFFSET + WINDOW_BARS],
    }


def load_bsp_labels() -> dict[str, list[int]]:
    """解析 TV 缠论标注，返回买点/卖点的 local_idx 列表。"""
    with open(CHANLUN_FILE) as f:
        d = json.load(f)
    labels = d["pine"]["labels"]
    buy_bars: list[int] = []
    sell_bars: list[int] = []
    for lb in labels:
        text = lb["text"].strip()
        if not text or lb["price"] is None:
            continue
        t = lb["time"]
        if t < 0 or t >= WINDOW_BARS:
            continue
        if "买" in text:
            buy_bars.append(t)
        elif "卖" in text:
            sell_bars.append(t)
    return {"buy": sorted(set(buy_bars)), "sell": sorted(set(sell_bars))}


# ====================================================================
# PH 特征提取：双树逐 bar 快照
# ====================================================================

def run_dual_trees(prices: dict[str, list[float]]) -> list[dict]:
    """逐 bar 运行双树（T_low + T_high），返回每根 bar 的特征快照。

    双树定义（a_dual_merge_tree.py）：
      T_low  = OnlineMergeTree(low)   — 追踪底（买压 settle）
      T_high = OnlineMergeTree(-high) — 追踪顶（卖压 settle，取负进树，输出还原）

    H1/H4 仍用 close 的 sublevel tree（原有语义：alive_count/dom_persistence on close）。
    H2/H3 用正式双树（T_low/T_high on HL）。
    """
    closes = prices["closes"]
    highs  = prices["highs"]
    lows   = prices["lows"]

    # H1/H4 用的 close sublevel tree
    close_tree = OnlineMergeTree()

    # H2/H3 用的 DualMergeTree（T_low + T_high）
    dual = DualMergeTree()

    last_settle_p_bot: float = 0.0   # T_low 最近新增 settle persistence（底）
    last_settle_p_top: float = 0.0   # T_high 最近新增 settle persistence（顶）
    # 用 birth_idx set 检测增量 settle（DualMergeTree.update() 不返回 newly_settled）
    seen_bot_idx: set[int] = set()
    seen_top_idx: set[int] = set()

    snapshots: list[dict] = []

    for i in range(len(closes)):
        c = closes[i]
        h = highs[i]
        lo = lows[i]

        # close tree：H1/H4
        close_tree.update(c)
        bc_close = close_tree.current_barcode()

        # dual tree：H2/H3
        dual.update(h, lo)
        alive_bot = dual.alive_components_low()   # 底 alive 分量
        alive_top = dual.alive_components_high()  # 顶 alive 分量
        settled_bot = dual.settled_components_low()
        settled_top = dual.settled_components_high()

        # 检测增量 settle（通过 birth_idx 去重）
        new_bots = [b for b in settled_bot if b.birth_idx not in seen_bot_idx]
        new_tops = [b for b in settled_top if b.birth_idx not in seen_top_idx]
        if new_bots:
            last_settle_p_bot = max(b.persistence for b in new_bots)
            seen_bot_idx.update(b.birth_idx for b in new_bots)
        if new_tops:
            last_settle_p_top = max(b.persistence for b in new_tops)
            seen_top_idx.update(b.birth_idx for b in new_tops)

        n_alive_close = len(bc_close.alive_bars)
        n_alive_bot   = len(alive_bot)
        n_alive_top   = len(alive_top)

        dom_p_close = (
            max(b.persistence for b in bc_close.alive_bars)
            if bc_close.alive_bars else 0.0
        )
        dom_p_bot = (
            max(b.persistence for b in alive_bot)
            if alive_bot else 0.0
        )
        dom_p_top = (
            max(b.persistence for b in alive_top)
            if alive_top else 0.0
        )

        snapshots.append({
            "i": i,
            "close": c,
            # H1/H4
            "n_alive_close": n_alive_close,
            "dom_p_close": dom_p_close,
            # H2
            "last_settle_p_bot": last_settle_p_bot,
            "last_settle_p_top": last_settle_p_top,
            # H3
            "n_alive_bot": n_alive_bot,
            "n_alive_top": n_alive_top,
            "dom_p_bot": dom_p_bot,
            "dom_p_top": dom_p_top,
        })

    return snapshots


# ====================================================================
# 假说验证
# ====================================================================

def verify_h1(snaps: list[dict], buy_bars: list[int]) -> dict:
    """H1：alive 分量递减 = 背驰。

    信号定义：当前 bar 前 LOOKBACK 根 alive_count（close sublevel tree）的线性斜率 < 0。
    事件定义：buy_bars（底背驰候选——买点出现意味底部区域）。
    指标：信号 precision/recall vs base_rate。
    """
    signal_bars: list[int] = []
    for i in range(LOOKBACK, len(snaps)):
        window = [snaps[j]["n_alive_close"] for j in range(i - LOOKBACK, i + 1)]
        slope = linear_slope(window)
        if slope < 0:
            signal_bars.append(i)

    stats = precision_recall(signal_bars, buy_bars, len(snaps), LOOKBACK)

    # 额外：buy_bars 前 LOOKBACK 内 slope 分布
    slopes_at_buy: list[float] = []
    for b in buy_bars:
        if b >= LOOKBACK:
            window = [snaps[j]["n_alive_close"] for j in range(b - LOOKBACK, b + 1)]
            slopes_at_buy.append(linear_slope(window))

    neg_slope_rate = (
        sum(1 for s in slopes_at_buy if s < 0) / len(slopes_at_buy)
        if slopes_at_buy else float("nan")
    )
    base_neg_slope_rate = len(signal_bars) / max(len(snaps) - LOOKBACK, 1)

    return {
        **stats,
        "neg_slope_before_buy": neg_slope_rate,
        "global_neg_slope_rate": base_neg_slope_rate,
        "slopes_at_buy_mean": (
            sum(slopes_at_buy) / len(slopes_at_buy) if slopes_at_buy else float("nan")
        ),
    }


def verify_h2(
    snaps: list[dict], buy_bars: list[int], sell_bars: list[int]
) -> dict:
    """H2：superlevel settle persistence > sublevel settle persistence = 趋势反转。

    实现：双树（T_high/T_low on HL）。
    方向区分：
      top > bot → 卖压 settle 大，预示下跌/卖点
      bot > top → 买压 settle 大，预示上涨/买点
    事件定义：buy_bars / sell_bars 分别验证。
    """
    all_bsp = sorted(set(buy_bars + sell_bars))

    signal_sell = [  # top settle 大 → 卖压，对应卖点
        s["i"] for s in snaps
        if s["last_settle_p_top"] > s["last_settle_p_bot"] and s["last_settle_p_bot"] > 0
    ]
    signal_buy = [  # bot settle 大 → 买压，对应买点
        s["i"] for s in snaps
        if s["last_settle_p_bot"] > s["last_settle_p_top"] and s["last_settle_p_top"] > 0
    ]

    stats_sell = precision_recall(signal_sell, sell_bars, len(snaps), LOOKBACK)
    stats_buy  = precision_recall(signal_buy,  buy_bars,  len(snaps), LOOKBACK)
    stats_all  = precision_recall(
        sorted(set(signal_sell + signal_buy)), all_bsp, len(snaps), LOOKBACK
    )

    # 事件前 LOOKBACK 内命中率
    def hit_rate(sigs: list[int], events: list[int]) -> float:
        if not events:
            return float("nan")
        return sum(
            any(e - LOOKBACK <= i <= e for i in sigs) for e in events
        ) / len(events)

    return {
        "sell_signal": {**stats_sell, "event_hit": hit_rate(signal_sell, sell_bars)},
        "buy_signal":  {**stats_buy,  "event_hit": hit_rate(signal_buy,  buy_bars)},
        "all_signal":  {**stats_all,  "event_hit": hit_rate(sorted(set(signal_sell + signal_buy)), all_bsp)},
    }


def verify_h3(snaps: list[dict], all_bsp_bars: list[int]) -> dict:
    """H3：双树 alive 差值变号 = 趋势转折。

    实现：DualMergeTree（T_low low / T_high -high）。
    dual_diff_dom = dom_p_bot - dom_p_top（底主导 persistence - 顶主导 persistence）
    dual_diff_cnt = n_alive_bot - n_alive_top（alive 分量个数差）

    信号定义：dual_diff_dom 相邻 bar 变号（从正变负或从负变正）。
    补充信号：dual_diff_cnt 变号。
    事件定义：所有买卖点。
    """
    signal_bars_dom: list[int] = []
    signal_bars_cnt: list[int] = []
    for i in range(1, len(snaps)):
        prev_diff_dom = snaps[i - 1]["dom_p_bot"] - snaps[i - 1]["dom_p_top"]
        curr_diff_dom = snaps[i]["dom_p_bot"] - snaps[i]["dom_p_top"]
        if sign(prev_diff_dom) != 0 and sign(curr_diff_dom) != 0 and sign(prev_diff_dom) != sign(curr_diff_dom):
            signal_bars_dom.append(i)

        prev_diff_cnt = snaps[i - 1]["n_alive_bot"] - snaps[i - 1]["n_alive_top"]
        curr_diff_cnt = snaps[i]["n_alive_bot"] - snaps[i]["n_alive_top"]
        if sign(prev_diff_cnt) != 0 and sign(curr_diff_cnt) != 0 and sign(prev_diff_cnt) != sign(curr_diff_cnt):
            signal_bars_cnt.append(i)

    stats_dom = precision_recall(signal_bars_dom, all_bsp_bars, len(snaps), LOOKBACK)
    stats_cnt = precision_recall(signal_bars_cnt, all_bsp_bars, len(snaps), LOOKBACK)

    # 事件前 LOOKBACK 内有无变号信号
    sign_near_event_dom: list[bool] = []
    sign_near_event_cnt: list[bool] = []
    for e in all_bsp_bars:
        sign_near_event_dom.append(any(e - LOOKBACK <= i <= e for i in signal_bars_dom))
        sign_near_event_cnt.append(any(e - LOOKBACK <= i <= e for i in signal_bars_cnt))

    hit_rate_dom = (
        sum(sign_near_event_dom) / len(sign_near_event_dom)
        if sign_near_event_dom else float("nan")
    )
    hit_rate_cnt = (
        sum(sign_near_event_cnt) / len(sign_near_event_cnt)
        if sign_near_event_cnt else float("nan")
    )
    return {
        "dom": {**stats_dom, "sign_change_near_event": hit_rate_dom},
        "cnt": {**stats_cnt, "sign_change_near_event": hit_rate_cnt},
    }


def verify_h4(snaps: list[dict], closes: list[float]) -> dict:
    """H4：alive 数做连续力度指标。

    计算以下指标与未来 N 日收益的 Pearson IC：
      - n_alive_close：close sublevel tree alive 个数
      - dom_p_close：close sublevel tree dominant persistence
      - dom_p_bot：T_low 双树底 dominant persistence（HL 版本）
    """
    ics_cnt: dict[int, float] = {}
    ics_dom: dict[int, float] = {}
    ics_bot: dict[int, float] = {}

    for fw in FORWARD_WINDOWS:
        xs_cnt, xs_dom, xs_bot, ys = [], [], [], []
        for i in range(len(snaps) - fw):
            fwd_ret = (closes[i + fw] - closes[i]) / closes[i]
            xs_cnt.append(float(snaps[i]["n_alive_close"]))
            xs_dom.append(snaps[i]["dom_p_close"])
            xs_bot.append(snaps[i]["dom_p_bot"])
            ys.append(fwd_ret)
        ics_cnt[fw] = pearson_r(xs_cnt, ys)
        ics_dom[fw] = pearson_r(xs_dom, ys)
        ics_bot[fw] = pearson_r(xs_bot, ys)

    return {
        "ic_alive_count": ics_cnt,
        "ic_dom_persistence_close": ics_dom,
        "ic_dom_persistence_bot": ics_bot,
    }


# ====================================================================
# 报告生成
# ====================================================================

def verdict(h: dict, key_metric: str, threshold: float) -> str:
    val = h.get(key_metric, float("nan"))
    if math.isnan(val):
        return "待定（数据不足）"
    if val > threshold:
        return "初步成立（待 L3 交叉验证）"
    return "否证（当前数据不支持）"


def fmt(x: float | None, pct: bool = False) -> str:
    if x is None or math.isnan(x):
        return "NaN"
    if pct:
        return f"{x * 100:.1f}%"
    return f"{x:.4f}"


def build_report(
    h1: dict, h2: dict, h3: dict, h4: dict,
    closes: list[float],  # for reference only
    buy_bars: list[int], sell_bars: list[int],
    snaps: list[dict],
) -> str:
    all_bsp = sorted(set(buy_bars + sell_bars))

    lines = [
        "# PH 四假说 L2 验证报告 — QQQ 日线",
        "",
        "**标的**：NASDAQ:QQQ  **周期**：日线  **时间**：2024-08-27 至 2026-05-29（440 根）",
        "**认识论等级**：L2（单标的单时段，不外推）",
        "**数据源**：analysis/data_cache/QQQ_1d_max.json + qqq_daily_chanlun.json",
        "",
        f"**TV 缠论标注统计**：买点 {len(buy_bars)} 个 / 卖点 {len(sell_bars)} 个 / 合计 {len(all_bsp)} 个",
        "",
        "---",
        "",
    ]

    # --- 假说1 ---
    null_p_buy = h1.get("null_precision", float("nan"))
    lines += [
        "## 假说1：alive 分量递减 = 背驰",
        "",
        "**假设**：sublevel tree（on close）的 alive 分量数在背驰区域前趋势性递减。",
        "**信号**：前 5 根 alive_count 的线性斜率 < 0。",
        "**事件**：TV 缠论买点（底部背驰代理）。",
        f"**Null baseline precision@5bar**：{fmt(null_p_buy, pct=True)}",
        "",
        "| 指标 | 值 |",
        "|------|-----|",
        f"| 信号总数 | {h1.get('n_signals', 0)} |",
        f"| 买点事件数 | {h1.get('n_events', 0)} |",
        f"| 信号 precision（5bar 内有买点） | {fmt(h1.get('precision'), pct=True)} |",
        f"| null_precision（随机基线） | {fmt(h1.get('null_precision'), pct=True)} |",
        f"| lift = prec / null | {fmt(h1.get('lift'))} |",
        f"| 信号 recall（买点前 5bar 有信号） | {fmt(h1.get('recall'), pct=True)} |",
        f"| 买点前 slope 均值 | {fmt(h1.get('slopes_at_buy_mean'))} |",
        f"| 买点前 slope < 0 比率 | {fmt(h1.get('neg_slope_before_buy'), pct=True)} |",
        f"| 全局 slope < 0 比率 | {fmt(h1.get('global_neg_slope_rate'), pct=True)} |",
        "",
        "**关键比较**：买点前 neg_slope 比率 vs 全局 neg_slope 比率",
        f"→ {fmt(h1.get('neg_slope_before_buy'), pct=True)} vs {fmt(h1.get('global_neg_slope_rate'), pct=True)}",
        "",
    ]
    pre_buy_neg = h1.get('neg_slope_before_buy', float('nan'))
    global_neg = h1.get('global_neg_slope_rate', float('nan'))
    if not math.isnan(pre_buy_neg) and not math.isnan(global_neg):
        lift = pre_buy_neg / global_neg if global_neg > 1e-9 else float('nan')
        h1_verdict = (
            "初步成立（买点前 neg_slope 率高于全局，有一定区分力）"
            if pre_buy_neg > global_neg + 0.05
            else "否证（买点前 neg_slope 率未显著高于全局，区分力不足）"
            if pre_buy_neg <= global_neg
            else "待定（差异较小，需更多数据）"
        )
        lines.append(f"**结论**：{h1_verdict}（lift={fmt(lift)}）")
    else:
        lines.append("**结论**：待定（数据不足）")

    lines += ["", "---", ""]

    # --- 假说2 ---
    h2s = h2["sell_signal"]
    h2b = h2["buy_signal"]
    null_p = h2s.get("null_precision", float("nan"))
    lines += [
        "## 假说2：superlevel settle persistence > sublevel settle persistence = 趋势反转",
        "",
        "**假设**：双向测试——top_settle > bot_settle → 卖压 → 卖点；bot_settle > top_settle → 买压 → 买点。",
        "**实现**：DualMergeTree T_high(-high) + T_low(low)，比较最近新增 settle persistence。",
        f"**Null baseline precision@5bar**：{fmt(null_p, pct=True)}（事件密度 24%，随机信号期望 precision）",
        "",
        "| 信号方向 | 信号数 | 事件数 | precision | null_prec | lift | event 命中率 |",
        "|---------|-------|-------|-----------|-----------|------|------------|",
        f"| top>bot → 卖点 | {h2s.get('n_signals',0)} | {h2s.get('n_events',0)} | {fmt(h2s.get('precision'), pct=True)} | {fmt(h2s.get('null_precision'), pct=True)} | {fmt(h2s.get('lift'))} | {fmt(h2s.get('event_hit'), pct=True)} |",
        f"| bot>top → 买点 | {h2b.get('n_signals',0)} | {h2b.get('n_events',0)} | {fmt(h2b.get('precision'), pct=True)} | {fmt(h2b.get('null_precision'), pct=True)} | {fmt(h2b.get('lift'))} | {fmt(h2b.get('event_hit'), pct=True)} |",
        "",
        "**注意**：lift > 1 表示信号 precision 高于随机基线；lift ≈ 1 = 无区分力；lift < 1 = 反向。",
        "",
    ]
    lift_sell = h2s.get("lift", float("nan"))
    lift_buy  = h2b.get("lift", float("nan"))
    if not math.isnan(lift_sell) and not math.isnan(lift_buy):
        best_lift = max(lift_sell, lift_buy)
        h2_verdict = (
            "初步成立（至少一个方向 lift > 1.2，信号有效区分力）"
            if best_lift > 1.2
            else "否证（两个方向 lift 均 ≤ 1，信号无有效区分力）"
            if best_lift <= 1.0
            else "待定（lift 略高于 1，区分力弱，需 L3 验证）"
        )
    else:
        h2_verdict = "待定（数据不足）"
    lines.append(f"**结论**：{h2_verdict}")

    lines += ["", "---", ""]

    # --- 假说3 ---
    h3d = h3["dom"]
    h3c = h3["cnt"]
    lines += [
        "## 假说3：双树 alive 差值变号 = 趋势转折",
        "",
        "**假设**：DualMergeTree（T_low/T_high on HL）的 alive 差值变号预示趋势转折。",
        "测试两个差值：",
        "  (a) dom_diff = dom_p_bot - dom_p_top（dominant persistence 差）",
        "  (b) cnt_diff = n_alive_bot - n_alive_top（alive 个数差）",
        "**信号**：相邻两根 bar 的差值符号翻转。",
        "**事件**：TV 缠论买卖点。",
        "",
        "### (a) dominant persistence 差变号",
        "",
        "| 指标 | 值 |",
        "|------|-----|",
        f"| 信号总数（dom 变号次数） | {h3d.get('n_signals', 0)} |",
        f"| 信号 precision | {fmt(h3d.get('precision'), pct=True)} |",
        f"| null_precision（随机基线） | {fmt(h3d.get('null_precision'), pct=True)} |",
        f"| lift | {fmt(h3d.get('lift'))} |",
        f"| 信号 recall | {fmt(h3d.get('recall'), pct=True)} |",
        f"| 买卖点前 dom 变号命中率 | {fmt(h3d.get('sign_change_near_event'), pct=True)} |",
        "",
        "### (b) alive 个数差变号",
        "",
        "| 指标 | 值 |",
        "|------|-----|",
        f"| 信号总数（cnt 变号次数） | {h3c.get('n_signals', 0)} |",
        f"| 信号 precision | {fmt(h3c.get('precision'), pct=True)} |",
        f"| null_precision（随机基线） | {fmt(h3c.get('null_precision'), pct=True)} |",
        f"| lift | {fmt(h3c.get('lift'))} |",
        f"| 信号 recall | {fmt(h3c.get('recall'), pct=True)} |",
        f"| 买卖点前 cnt 变号命中率 | {fmt(h3c.get('sign_change_near_event'), pct=True)} |",
        "",
    ]
    # 取更好的一个做结论，但严格检查信号量
    n_dom = h3d.get('n_signals', 0)
    n_cnt = h3c.get('n_signals', 0)
    lift_dom = h3d.get('lift', float('nan'))
    lift_cnt = h3c.get('lift', float('nan'))
    recall_dom = h3d.get('recall', 0.0)
    recall_cnt = h3c.get('recall', 0.0)

    # 需同时满足：信号量≥10、recall≥15%、lift>1.2
    h3d_valid = (n_dom >= 10 and recall_dom >= 0.15
                 and not math.isnan(lift_dom) and lift_dom > 1.2)
    h3c_valid = (n_cnt >= 10 and recall_cnt >= 0.15
                 and not math.isnan(lift_cnt) and lift_cnt > 1.2)

    if h3d_valid or h3c_valid:
        h3_verdict = "初步成立（lift>1.2 且信号量/recall 达标）"
    elif (not math.isnan(lift_dom) and lift_dom < 1.0) or (not math.isnan(lift_cnt) and lift_cnt < 1.0):
        if n_dom < 10 or n_cnt < 10:
            h3_verdict = f"待定（信号数量不足：dom={n_dom}个 cnt={n_cnt}个；无法拒绝随机假设）"
        else:
            h3_verdict = "否证（dom 方向 lift<1，变号信号无预测效力）"
    else:
        h3_verdict = f"待定（信号数量不足：dom={n_dom}个 cnt={n_cnt}个；统计量不足以拒绝随机假设）"
    lines.append(f"**结论**：{h3_verdict}")

    lines += ["", "---", ""]

    # --- 假说4 ---
    ic_cnt  = h4["ic_alive_count"]
    ic_dom  = h4["ic_dom_persistence_close"]
    ic_bot  = h4["ic_dom_persistence_bot"]
    lines += [
        "## 假说4：alive 数做连续力度指标",
        "",
        "**假设**：alive_count 或 dominant_persistence 与未来 N 日收益存在显著 IC。",
        "**方法**：Pearson IC，|IC| > 0.1 为有意义阈值（量化惯例）。",
        "",
        "| 前瞻窗口 | IC(alive_count close) | IC(dom_p close) | IC(dom_p T_low HL) |",
        "|---------|-----|-----|-----|",
    ]
    for fw in FORWARD_WINDOWS:
        lines.append(
            f"| {fw}日 | {fmt(ic_cnt.get(fw))} | {fmt(ic_dom.get(fw))} | {fmt(ic_bot.get(fw))} |"
        )

    max_ic = max(
        max(abs(ic_cnt.get(fw, 0.0)) for fw in FORWARD_WINDOWS),
        max(abs(ic_dom.get(fw, 0.0)) for fw in FORWARD_WINDOWS),
        max(abs(ic_bot.get(fw, 0.0)) for fw in FORWARD_WINDOWS),
    )
    lines += ["", f"**max |IC| 综合**：{max_ic:.4f}"]
    if max_ic > 0.1:
        lines.append(
            "→ 初步成立（存在 >0.1 的 IC，alive 数具有一定预测信息含量；实用性待 L3 交叉验证）"
        )
    else:
        lines.append(
            "→ 否证（所有指标 IC 接近零，alive 数在当前窗口内与未来收益无显著线性关联）"
        )

    lines += [
        "",
        "---",
        "",
        "## 边界条件与翻转声明",
        "",
        "以下条件下，上述结论可能翻转：",
        "",
        "1. **数据窗口**：本报告仅 440 根 K 线（~1.75 年），结论对 2024-08-27 至 2026-05-29 有效。"
        " 不同市场环境（牛/熊市转换）可能导致显著差异。",
        "2. **Close 价限制**：使用 close 价运行 merge tree（而非 high/low 价），"
        " 会低估 persistence（tick 内的振幅不在 close 序列中体现）。",
        "3. **TV 标注对齐**：label_time 0 假设对应 QQQ 第 6408 根 K 线（2024-08-27）。"
        " 如果 TV 图表加载了不同历史长度，对齐关系需重新校正。",
        "4. **买卖点代理问题**：用全部买卖点代理背驰事件（H1），"
        " 实际 type2/type3 买卖点不需要背驰，会降低信号纯度。",
        "5. **DualMergeTree 实现**：H2/H3 使用 T_high(-high) + T_low(low)，"
        " H1/H4 使用 close 的 sublevel tree。两套树的信号互不干扰但也不完全等价。",
        "",
        "## 谱系引用",
        "",
        "- 不确定是否有直接的谱系记录针对这四条假说的分离过程，以下为相关记录：",
        "- `a_online_persistence.py` 注释中的 §7.5 升级方向（alive/settled 概念来源）",
        "- persistence_theory.md §7.5（因果 merge tree 的形式化位置）",
        "- `.chanlun/genealogy/` — 未检索到 PH 四假说专属的谱系条目",
        "",
        "## 影响声明",
        "",
        "本脚本为只读验证，不修改任何模块。",
        "产出：`analysis/ph_hypotheses_results.md`（当前文件）。",
        "对下游系统无直接影响，结论供 L3 交叉验证和信号设计参考。",
    ]

    return "\n".join(lines) + "\n"


# ====================================================================
# 主流程
# ====================================================================

def main() -> None:
    print("加载价格数据...")
    prices = load_prices()
    closes = prices["closes"]
    print(f"  QQQ 日线 {WINDOW_BARS} 根（local_idx 0-{WINDOW_BARS-1}）")

    print("加载 TV 缠论标注...")
    bsp = load_bsp_labels()
    buy_bars = bsp["buy"]
    sell_bars = bsp["sell"]
    all_bsp = sorted(set(buy_bars + sell_bars))
    print(f"  买点 {len(buy_bars)} 个，卖点 {len(sell_bars)} 个，合计 {len(all_bsp)} 个")

    print("运行双树 PH 特征提取...")
    snaps = run_dual_trees(prices)
    print("  完成。")

    print("验证假说1...")
    h1 = verify_h1(snaps, buy_bars)

    print("验证假说2...")
    h2 = verify_h2(snaps, buy_bars, sell_bars)

    print("验证假说3...")
    h3 = verify_h3(snaps, all_bsp)

    print("验证假说4...")
    h4 = verify_h4(snaps, closes)

    print("生成报告...")
    report = build_report(h1, h2, h3, h4, closes, buy_bars, sell_bars, snaps)

    with open(OUT_MD, "w", encoding="utf-8") as f:
        f.write(report)

    print(f"✓ 报告已写入 {OUT_MD}")

    # 控制台摘要
    print("\n=== 四假说结论速览 ===")
    pre_buy_neg = h1.get('neg_slope_before_buy', float('nan'))
    global_neg = h1.get('global_neg_slope_rate', float('nan'))
    print(f"H1 alive 递减→背驰: neg_slope_before_buy={fmt(pre_buy_neg, pct=True)}, global={fmt(global_neg, pct=True)}")
    _h2s = h2["sell_signal"]; _h2b = h2["buy_signal"]
    print(f"H2 卖方向(top>bot): prec={fmt(_h2s.get('precision'), pct=True)}, lift={fmt(_h2s.get('lift'))}")
    print(f"H2 买方向(bot>top): prec={fmt(_h2b.get('precision'), pct=True)}, lift={fmt(_h2b.get('lift'))}")
    _h3d = h3['dom']
    _h3c = h3['cnt']
    print(f"H3 双树变号→转折 dom: prec={fmt(_h3d.get('precision'), pct=True)} lift={fmt(_h3d.get('lift'))}, cnt: prec={fmt(_h3c.get('precision'), pct=True)} lift={fmt(_h3c.get('lift'))}")
    print(f"H4 alive count IC: {[(fw, fmt(h4['ic_alive_count'].get(fw))) for fw in FORWARD_WINDOWS]}")


if __name__ == "__main__":
    main()
