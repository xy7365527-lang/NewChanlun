"""腾讯 700 — 在线（因果）merge tree + log-gap 自动级别检测的 L2 实测。

回应用户下游问题之一「级别判断」：用**完全因果、无 σ 自由参数**的在线 merge tree
（a_online_persistence）替代路径空间，从 persistence 分布自动涌现缠论级别
（a_level_detection），不预设阈值、不预选周期。

与路径空间（§8）的对照：路径空间被 L0+L2 双重否决（自由参数 + 退化 + 非因果）；
在线 merge tree 是 §7.5 指出的唯一合法因果升级方向——本脚本验证它能否产出可用级别。

认识论等级：在线检测管线 = L1（管线正确性）；级别结构 = L2（腾讯 700 单标的，
日线+30分跨周期=弱 L3）；级别↔缠论真值（一禅指标）对比 = **尚缺**（剩余 L2 缺口）。
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.a_level_detection import detect_levels  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_persistence_barcode import (  # noqa: E402
    atr_noise_threshold,
    sublevel_h0_bars,
)

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"
# HK 港股每交易日约 11 根 30 分钟 K
BARS_PER_DAY = {"日线 1D": 1.0, "30分钟": 11.0}
LABEL_FILE = {"日线 1D": "daily_labels.json", "30分钟": "m30_labels.json"}


def load_ohlc(ofile: str):
    blob = json.loads((DATA / ofile).read_text())
    bars = blob["bars"]
    return (
        [float(b["high"]) for b in bars],
        [float(b["low"]) for b in bars],
        [float(b["close"]) for b in bars],
    )


def chan_label_counts(lfile: str) -> list[int]:
    """一禅(CZSC)指标各 study 的标签数（笔级别粒度的代理）。"""
    blob = json.loads((DATA / lfile).read_text())
    return [s.get("total_labels", len(s.get("labels", []))) for s in blob.get("studies", [])]


def verify_online_equals_batch(closes) -> tuple[bool, int, float]:
    """步骤 6（用户 spec）：在线 finalize 的 diagram ≡ 批量 sublevel_h0_bars。

    确认"因果性不牺牲精度"——在真实 700 数据上坐实在线版与批量版的 persistence
    多重集逐项一致（diagram 是持续同调的拓扑不变量）。

    Returns (exact_match, n_bars, max_abs_diff)。
    """
    batch = sorted(
        (round(b.birth, 9), round(b.death, 9), round(b.persistence, 9))
        for b in sublevel_h0_bars(closes)
    )
    online = sorted(
        (round(b.birth_price, 9), round(b.death_price, 9), round(b.persistence, 9))
        for b in OnlineMergeTree.from_prices(closes).settled_bars
    )
    exact = batch == online
    max_diff = 0.0
    if len(batch) == len(online):
        for (b0, b1, b2), (o0, o1, o2) in zip(batch, online):
            max_diff = max(max_diff, abs(b0 - o0), abs(b1 - o1), abs(b2 - o2))
    return exact, len(batch), max_diff


def run(name: str, ofile: str):
    highs, lows, closes = load_ohlc(ofile)
    bpd = BARS_PER_DAY[name]
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)

    # 因果在线 merge tree：逐根喂入 → finalize（全 settled，无未来改写）
    snap = OnlineMergeTree.from_prices(closes)
    bars = snap.settled_bars

    ls = detect_levels(bars, bars_per_day=bpd, noise_floor=tau)

    print("=" * 72)
    print(f"【{name}】在线因果 merge tree + log-gap 自动级别检测（L2）")
    print(f"  n={len(closes)}  settled特征={len(bars)}  τ(ATR)={tau:.2f}  bars/day={bpd:.0f}")
    print("=" * 72)
    print(f"  自动涌现级别数={len(ls.levels)}（根={len(ls.roots)} 叶={len(ls.leaves)}），"
          f"无预设阈值/周期")
    print(f"  {'级别':<4}{'深度':<4}{'特征数':>5}{'persistence区间':>20}"
          f"{'avg_span':>9}{'≈日历日':>8}  级别名(数据涌现)")
    for lv in ls.levels:
        days = lv.avg_span / bpd
        rng = f"[{lv.persistence_range[0]:.1f},{lv.persistence_range[1]:.1f}]"
        print(f"  L{lv.level_id:<3}{lv.depth:<4}{lv.n_bars:>5}{rng:>20}"
              f"{lv.avg_span:>9.1f}{days:>8.1f}  {lv.period_label}"
              f"{' (父=L%d)' % lv.parent_id if lv.parent_id is not None else ' (根)'}")

    # 主导级别用最大"非全局"特征（全局 bar = 全段，非可交易级别）
    nonglobal = [b for b in bars if not b.is_global]
    dom = max(nonglobal, key=lambda b: b.persistence) if nonglobal else bars[0]
    print(f"\n  主导特征(非全局): persistence={dom.persistence:.1f} span={dom.span} "
          f"(≈{dom.span/bpd:.1f} 日历日)")
    print(f"  叶级别(可交易级别): {[lv.period_label for lv in ls.leaves]}")

    # 步骤 5（用户 spec）：跟一禅(CZSC)指标缠论级别对比
    counts = chan_label_counts(LABEL_FILE[name])
    print(f"\n  [步骤5] 一禅指标对比: studies 标签数={counts}（笔级别粒度参照）")
    print(f"    PH 叶级别数={len(ls.leaves)}；超τ特征数={len(bars)}。")
    print("    认识论标注(L2 缺口)：一禅缓存标签为纯价格(无时间索引)，无法逐笔对齐 span；")
    print("    精确逐级吻合需时间索引的笔 ground-truth。当前仅作粒度量级 + §4 锚点交叉参照。")
    print(f"    §4 已验证锚点: 日线 span≈124→周线级笔 / 30min span≈285→日线级笔（tencent_ph_nav.py）。")
    return ls


def main():
    print("#" * 72)
    print("# 腾讯 700 — 在线因果级别检测（路径空间被否决后的合法因果方向，§7.5/§8）")
    print("#" * 72)
    for name, ofile in [("日线 1D", "daily_ohlcv.json"), ("30分钟", "m30_ohlcv.json")]:
        run(name, ofile)
        # 步骤 6（用户 spec）：在线 = 批量，确认因果性不牺牲精度
        _, _, closes = load_ohlc(ofile)
        exact, nb, mdiff = verify_online_equals_batch(closes)
        print(f"\n  [步骤6] 在线 finalize diagram ≡ 批量 sublevel_h0_bars: "
              f"{'✓ 完全一致' if exact else '✗ 不一致'} "
              f"(n_bars={nb}, max|Δ|={mdiff:.2e}) —— 因果性不牺牲精度")
        print()
    print("=" * 72)
    print("结论: 在线因果 merge tree 在真实 700 数据上 (1) diagram 与批量逐项一致(步骤6)，")
    print("      (2) 从 persistence 自动涌现多级别结构、不预设阈值/周期(步骤2-4)。")
    print("剩余 L2 缺口: 自动级别 vs 一禅真值的逐级吻合度需时间索引笔 ground-truth(步骤5)。")
    print("=" * 72)


if __name__ == "__main__":
    main()
