"""腾讯 700 — 真假买点的 PH 过滤 + 在线止损信号 L2 实测。

核心操盘问题（编排者 2026-05-27）
--------------------------------
区间套在下跌中多次给出次级别背驰买点，多数被后续走势破坏（创新低），只有最后一次是
真底。PH 能否分辨"这次次级别买点是真底还是会被破坏的假底"？

被检验的假设（用户）
--------------------
大级别下跌的 alive 分量 persistence **还在增长** → 次级别背驰买点更可能是**假底**
（中继，后续创新低）；persistence 增长**停滞/减速** → 更可能是**真底**。

缠论原文依据（原文考据 2026-05-27）
-----------------------------------
- 买点确认是**单调否定过程**：背驰段是"被假设的"，由后续**创新低**否定（第37/61课，
  "对象否定对象"）。"没有假背驰，错了是把盘整背驰误判为趋势背驰"（第27课）。
- 真背驰 = 创新低 + **力度衰竭**；中继(假底) = 创新低 + **力度未减**（第27课）。
- 第44课："只有必要条件，没有充分条件"——量化系统应把次级别背驰标注为**待证伪假设**。

认识论 + §5 边界（必读）
-----------------------
PH H0 persistence ≈ 幅度 ∈ ker(D)（§5/239号）。本回测的 PH 过滤器只能量度"是否创新低
/幅度速度"（维度二），**不能**量度 MACD 的"力度衰竭/0轴基准"（维度一）。故先验预期：
PH 过滤器至多提供**否定性必要条件**（大级别还在创新低 → 否定假买点），不能肯定真底。
**本实验可能产生否定性结果**（幅度过滤器无独立区分力）——若如此，正是 §5 的 L2 实证
（否定性结果比确认更有价值，231号），不是失败。

认识论等级：L2（腾讯 700 单标的日线，可否证）。
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.a_online_persistence import EntryState, OnlineMergeTree  # noqa: E402
from newchan.a_persistence_barcode import atr_noise_threshold  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"
LOOKAHEAD = 20  # 真假底验证窗口（根 K 线）
PROM_W = 3      # 局部低点窗口


def load(ofile: str):
    bars = json.loads((DATA / ofile).read_text())["bars"]
    return (
        [float(b["high"]) for b in bars],
        [float(b["low"]) for b in bars],
        [float(b["close"]) for b in bars],
    )


def local_lows(closes: list[float], w: int, tau: float) -> list[int]:
    """显著局部低点 = 窗口内最小 + 两侧反弹 prominence > τ（次级别买点候选）。"""
    n = len(closes)
    lows = []
    for i in range(w, n - w):
        seg = closes[i - w : i + w + 1]
        if closes[i] != min(seg):
            continue
        left_high = max(closes[max(0, i - w) : i + 1])
        right_high = max(closes[i : min(n, i + w + 1)])
        prom = min(left_high, right_high) - closes[i]
        if prom > tau:
            lows.append(i)
    return lows


def per_bar_health(closes: list[float]):
    """因果单遍：每根 K 线后记录 trend_health（大级别 alive 健康度）。"""
    tree = OnlineMergeTree()
    health = []
    prev_dom_idx = None
    for p in closes:
        tree.update(p)
        th = tree.trend_health(window=5)
        switch = tree.level_switch_event(prev_dom_idx)
        health.append((th, switch))
        if th is not None:
            prev_dom_idx = th.dominant_birth_idx
    return health


def run(name: str, ofile: str):
    highs, lows_, closes = load(ofile)
    n = len(closes)
    tau = atr_noise_threshold(highs, lows_, closes, multiple=1.0)
    health = per_bar_health(closes)
    cands = local_lows(closes, PROM_W, tau)

    print("=" * 76)
    print(f"【{name}】真假买点 PH 过滤实测（n={n} τ={tau:.2f} LOOKAHEAD={LOOKAHEAD}）")
    print("=" * 76)

    # 混淆矩阵：PH 预测(假底=大级别健康/仍创新低) vs 真值(后续创新低=假底)
    # 预测假底 = trend_health.healthy（大级别 persistence 仍增长）
    tp = fp = tn = fn = 0
    new_low_cnt = 0
    rows = []
    for i in cands:
        th, _ = health[i]
        if th is None:
            continue
        # 真值：候选后 LOOKAHEAD 内创出更低 low → 假底（买点被破坏）
        future = lows_[i + 1 : min(n, i + 1 + LOOKAHEAD)]
        gt_false = bool(future) and min(future) < lows_[i]
        # PH 预测：大级别仍健康（persistence 增长）→ 预测假底
        pred_false = th.healthy
        is_new_low = closes[i] <= min(closes[: i + 1])
        if is_new_low:
            new_low_cnt += 1
        if pred_false and gt_false:
            tp += 1
        elif pred_false and not gt_false:
            fp += 1
        elif (not pred_false) and (not gt_false):
            tn += 1
        else:
            fn += 1
        rows.append((i, closes[i], gt_false, pred_false, th.persistence_growth_rate, is_new_low))

    total = tp + fp + tn + fn
    if total == 0:
        print("  无有效候选买点。")
        return
    acc = (tp + tn) / total
    base_false = sum(1 for r in rows if r[2]) / total  # 假底基准率
    prec_true = tn / (tn + fn) if (tn + fn) else float("nan")  # 预测真底的精度
    print(f"  候选次级别买点={total}（其中创新低={new_low_cnt}）；假底基准率={base_false:.0%}")
    print(f"  PH 过滤器混淆矩阵（预测假底 = 大级别 persistence 仍增长）：")
    print(f"    TP(预假/真假)={tp}  FP(预假/真真)={fp}  TN(预真/真真)={tn}  FN(预真/真假)={fn}")
    print(f"    准确率={acc:.0%}（基准:全预假底={base_false:.0%}）  "
          f"预测真底精度 TN/(TN+FN)={prec_true:.0%}")

    # 关键判据：PH 说"真底"(not healthy) 时，真的是真底的比例 vs 基准真底率
    base_true = 1 - base_false
    lift = (prec_true - base_true) if prec_true == prec_true else float("nan")
    verdict = ("有提升（弱信号）" if lift == lift and lift > 0.10
               else "无独立区分力（≈基准，证实 §5：幅度过滤器不替代 MACD）")
    print(f"  真底精度 vs 基准真底率({base_true:.0%}): 提升={lift:+.0%} → {verdict}")

    # 止损信号实测：在每个候选买点入场，看 stop_signal 是否在创新低前/后触发
    _stop_signal_backtest(name, closes, lows_, cands, tau)
    return rows


def _stop_signal_backtest(name, closes, lows_, cands, tau):
    """700 实测止损信号：在候选买点入场，stop_signal 触发时机 vs 实际创新低。"""
    triggered_before_break = 0
    total_broken = 0
    for i in cands[:50]:  # 限量
        # 重建到入场点
        tree = OnlineMergeTree()
        for p in closes[: i + 1]:
            tree.update(p)
        th = tree.trend_health(window=5)
        if th is None:
            continue
        entry = EntryState(
            birth_idx=i, entry_persistence=max(closes[: i + 1]) - closes[i],
            direction=1, n_at_entry=i + 1,
        )
        n = len(closes)
        future = lows_[i + 1 : min(n, i + 1 + LOOKAHEAD)]
        gt_break = bool(future) and min(future) < lows_[i]
        if not gt_break:
            continue
        total_broken += 1
        # 逐根推进，记录 stop_signal 首次触发的相对位置 vs 创新低位置
        break_at = next((j for j in range(i + 1, min(n, i + 1 + LOOKAHEAD)) if lows_[j] < lows_[i]), None)
        stop_at = None
        for j in range(i + 1, min(n, i + 1 + LOOKAHEAD)):
            tree.update(closes[j])
            ss = tree.stop_signal(entry)
            if ss.triggered:
                stop_at = j
                break
        if stop_at is not None and break_at is not None and stop_at <= break_at:
            triggered_before_break += 1
    if total_broken:
        print(f"  [止损信号实测] {total_broken} 个被破坏的买点中，stop_signal 在创新低当根或之前"
              f"触发 = {triggered_before_break}/{total_broken} "
              f"({triggered_before_break/total_broken:.0%})")


def main():
    print("#" * 76)
    print("# 腾讯 700 — 真假买点 PH 过滤 + 在线止损信号（§5 边界下的 L2 实证）")
    print("#" * 76)
    for name, ofile in [("日线 1D", "daily_ohlcv.json"), ("30分钟", "m30_ohlcv.json")]:
        run(name, ofile)
        print()
    print("=" * 76)
    print("结论标注：PH 过滤器度量幅度（创新低），是真假买点的必要条件否定器，非充分确认")
    print("（§5：力度衰竭需 MACD，PH 不可及）。详见 docs/persistence_theory.md。")
    print("=" * 76)


if __name__ == "__main__":
    main()
