"""腾讯 700 — 区间套买点真假区分的 PH 判据 L2 实测。

操盘问题（用户最高优先级）
--------------------------
下跌过程中，区间套会**多次**给出次级别背驰买点信号，但多数被后续下跌否定，
只有最后一次（或某一次）才是真的底。**PH 能不能分辨哪次真、哪次假？**

缠论理论锚点（原文，docs/chanlun/text/chan99/0027 区间套）
---------------------------------------------------------
> "低级别背驰是本级别背驰的**必要条件而非充分条件**。"
> "1分钟级别的背弛，不会制造周线级别的大顶，**除非日线上同时也出现背弛**。"

→ 次级别背驰每次都"有逆转"（小反弹），但只有**大级别同时背驰**才制造大级别底。
  假买点 = 次级别背驰成立但大级别下跌力度仍健康（未衰竭）。
  真买点 = 次级别背驰 ∧ 大级别背驰同时成立。

PH 假设（用户提出，本脚本检验）
-------------------------------
大级别 alive 全局分量的 persistence = running_max − running_min = **整个大级别下跌
的累积幅度**（a_online_persistence：栈底全局分量，cap=运行最高，birth=运行最低）。

- 其增长率 dP/dt = 创新低速度 = 大级别下跌力度的拓扑量。
- **健康下跌**（dP/dt 仍快）→ 次级别背驰更可能假（会被继续下跌破坏）。
- **力度衰竭**（dP/dt 减速）→ 次级别背驰更可能真。

这与 MACD 背驰**正交**：MACD 比较相邻两个次级别段（都是次级别）；PH 全局 alive
分量度量的是整个大级别下跌的减速——正是区间套定理里缺失的"大级别充分性"。

方法（严格因果 feature + 无泄漏 label）
---------------------------------------
1. fine zigzag（次级别摆动）→ 每个新低 swing-low 处用 MACD 绿柱面积对比判底背驰。
2. label（用未来，仅作真值）：FAKE=后续出现更低 swing-low；REAL=无更低低点且后续
   反弹 ≥ R%（反转确认）；边界未确认者剔除。
3. PH feature（严格因果，OnlineMergeTree 只喂到信号当根 t_v）：
   - P_main      : 大级别 alive 全局分量 persistence（累积大跌幅）
   - dPdt_recent : 最近 W 根的 dP/dt（近端创新低速度）
   - dPdt_trend  : P_main / 自大级别下跌起点的根数（全程平均速度）
   - accel_ratio : dPdt_recent / dPdt_trend（<1 = 大级别减速 = 衰竭）
   - sub_main    : 次级别 alive / 大级别 alive persistence 比值
4. 统计真假买点的 feature 分布差异 + 单阈值可分性（AUC）。

认识论等级（formalization-validity-domain）
-------------------------------------------
- zigzag / MACD 面积 / OnlineMergeTree feature：**L0**（纯算法，因果确定）。
- "accel_ratio 区分真假买点"经验断言：**L2**（腾讯 700 单标的日线，可否证）。
  否定性结果（PH 不区分）同样合法且有价值（缩小有效域边界）。

概念溯源标签
-----------
- 区间套真假判据 [新缠论:候选——区间套定理 0027 + §7.5 在线 merge tree]
- 大级别 alive persistence dP/dt = 大级别下跌力度 [新缠论:候选——拓扑↔背驰同构]
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import pandas as pd  # noqa: E402

from newchan.a_macd import compute_macd, macd_area_for_range  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"

# ---- 自主选定的参数（用户授权自决；每个都给出理由）-------------------------
ZIGZAG_PCT = 0.025       # 次级别摆动反转阈值 2.5%（日线上数日的笔级别摆动）
COARSE_PCT = 0.12        # 大级别趋势分段阈值 12%（区分主级别下跌趋势 vs 次级别回调）
REVERSAL_PCT = 0.08      # 真底反转确认：后续需反弹 ≥ 8%（大级别转折的最小幅度）
DPDT_WINDOW = 5          # dP/dt 近端窗口 5 根（≈ 一个次级别摆动的时间尺度）


# ====================================================================
# ZigZag（百分比反转摆动点提取）
# ====================================================================


@dataclass(frozen=True, slots=True)
class Pivot:
    idx: int
    price: float
    kind: str  # "H" | "L"


def zigzag(closes: list[float], pct: float) -> list[Pivot]:
    """百分比反转 zigzag，返回交替的 H/L 摆动点（含端点修正）。"""
    if not closes:
        return []
    pivots: list[Pivot] = []
    # 初始方向未知：以第一个点为试探极值
    ext_idx, ext_price = 0, closes[0]
    direction = 0  # +1 上行（找高点回落确认）, -1 下行, 0 未定

    for i in range(1, len(closes)):
        p = closes[i]
        if direction >= 0 and p > ext_price:
            ext_idx, ext_price = i, p
            direction = 1 if direction == 0 else direction
        elif direction <= 0 and p < ext_price:
            ext_idx, ext_price = i, p
            direction = -1 if direction == 0 else direction

        if direction == 1 and p <= ext_price * (1.0 - pct):
            # 上行中回落超阈值 → 确认 ext 为 swing high
            pivots.append(Pivot(ext_idx, ext_price, "H"))
            direction, ext_idx, ext_price = -1, i, p
        elif direction == -1 and p >= ext_price * (1.0 + pct):
            # 下行中反弹超阈值 → 确认 ext 为 swing low
            pivots.append(Pivot(ext_idx, ext_price, "L"))
            direction, ext_idx, ext_price = 1, i, p

    # 收尾：登记最后未确认的极值（端点）
    if not pivots or pivots[-1].idx != ext_idx:
        kind = "H" if direction == 1 else "L"
        pivots.append(Pivot(ext_idx, ext_price, kind))
    return pivots


# ====================================================================
# 底背驰买点信号（MACD 绿柱面积对比，相邻下跌段）
# ====================================================================


@dataclass(frozen=True, slots=True)
class BuySignal:
    """下跌延续中的次级别买点候选 = 创新低的 swing-low（区间套在此给信号）。"""

    idx: int           # swing-low 所在 bar 索引（信号当根，因果边界）
    price: float
    macd_div: bool     # 是否触发 MACD 底背驰（本段绿柱面积 < 前段）
    area_now: float    # 本段下跌 |MACD 绿柱面积|
    area_prev: float   # 前一段下跌 |MACD 绿柱面积|


def detect_buy_candidates(
    closes: list[float], pivots: list[Pivot], df_macd: pd.DataFrame
) -> list[BuySignal]:
    """每个"创新低"的 swing-low = 下跌延续中区间套给出的次级别买点候选。

    标注 MACD 底背驰（0016 课操作判据简化：相邻下跌段后段价格新低但绿柱面积更小）。
    用"创新低"而非"仅 MACD 背驰"作候选，是因为操盘者在下跌中每个次级别新低处都会
    面临"这是不是底"的判断——这正是用户问题的样本空间。
    """
    lows = [p for p in pivots if p.kind == "L"]
    highs = [p for p in pivots if p.kind == "H"]
    cands: list[BuySignal] = []

    for k in range(1, len(lows)):
        sl_prev, sl_now = lows[k - 1], lows[k]
        if sl_now.price >= sl_prev.price:
            continue  # 非创新低 → 非下跌延续，跳过

        h_now = _last_high_before(highs, sl_now.idx, sl_prev.idx)
        h_prev = _last_high_before(highs, sl_prev.idx, -1)
        area_now = area_prev = 0.0
        macd_div = False
        if h_now is not None and h_prev is not None:
            area_now = abs(macd_area_for_range(df_macd, h_now.idx, sl_now.idx)["area_neg"])
            area_prev = abs(macd_area_for_range(df_macd, h_prev.idx, sl_prev.idx)["area_neg"])
            macd_div = area_prev > 0 and area_now < area_prev

        cands.append(BuySignal(sl_now.idx, sl_now.price, macd_div, area_now, area_prev))

    return cands


def _last_high_before(highs: list[Pivot], before_idx: int, after_idx: int) -> Pivot | None:
    """返回 (after_idx, before_idx) 区间内最靠右的 swing high。"""
    cand = [h for h in highs if after_idx < h.idx < before_idx]
    return cand[-1] if cand else None


# ====================================================================
# 真假标签（用未来，仅作真值；不进 feature）
# ====================================================================


def label_signal(closes: list[float], pivots: list[Pivot], sig: BuySignal) -> str:
    """REAL / FAKE / UNCONFIRMED。

    FAKE       : 后续存在更低的 swing-low（被下跌否定）。
    REAL       : 无更低 swing-low，且信号后最高 close 较该低点反弹 ≥ REVERSAL_PCT。
    UNCONFIRMED: 无更低低点但尚未确认反转（数据边界）→ 剔除。
    """
    later_lows = [p for p in pivots if p.kind == "L" and p.idx > sig.idx]
    if any(p.price < sig.price for p in later_lows):
        return "FAKE"
    future = closes[sig.idx + 1:]
    if future and max(future) >= sig.price * (1.0 + REVERSAL_PCT):
        return "REAL"
    return "UNCONFIRMED"


# ====================================================================
# PH feature（严格因果：OnlineMergeTree 只喂到 t_v）
# ====================================================================


@dataclass(frozen=True, slots=True)
class PHFeature:
    P_main: float        # 大级别 alive 全局分量 persistence（累积大跌幅）
    dPdt_recent: float   # 最近 W 根 dP/dt
    dPdt_trend: float    # 全程平均 dP/dt（自大级别下跌起点）
    accel_ratio: float   # dPdt_recent / dPdt_trend（<1=减速=衰竭）
    sub_main: float      # 次级别 alive / 大级别 alive persistence


def ph_features_at(closes: list[float]) -> dict[int, PHFeature]:
    """逐根因果喂入，记录每根的 P_main 序列；在任意 t 可取因果 feature。

    **P_main(t) = 当前大级别下跌的累积幅度** = run_max(0..t) − min(closes[run_max_idx..t])，
    即自最近一次全局新高以来的最大回撤。等价于 OnlineMergeTree 中"被最近全局高点封顶
    的主 alive 下跌分量"的 persistence（current_barcode().alive_bars 里 birth_idx ≥
    run_max_idx 的最深分量）——其 cap=run_max（新高），birth=自高点以来最低 valley。

    为何不用全局 running_max−running_min：全局最低 valley 可能落在**当前下跌之前**
    （700 案例：全局底 idx23 在大下跌 idx147→298 之前），那样 P_main 在整个下跌中
    恒定、dP/dt≡0，无法反映当前大级别下跌的力度。新高 → run_max_idx 更新 → P_main
    归零 → 新大级别下跌段开始（与缠论"中枢新生/趋势重启"同构）。
    """
    tree = OnlineMergeTree()
    p_main_series: list[float] = []
    seg_start_series: list[int] = []   # 各 t 的大级别下跌起点（run_max_idx）
    run_max = float("-inf")
    run_max_idx = 0
    min_since_max = float("inf")

    feats: dict[int, PHFeature] = {}
    for t, c in enumerate(closes):
        tree.update(c)
        if c > run_max:
            run_max, run_max_idx = c, t
            min_since_max = c
        min_since_max = min(min_since_max, c)
        p_now = run_max - min_since_max
        p_main_series.append(p_now)
        seg_start_series.append(run_max_idx)

        # dP/dt（严格因果；窗口不跨越段起点 run_max_idx，避免跨段污染）
        start = max(t - DPDT_WINDOW, run_max_idx)
        dpdt_recent = (p_now - p_main_series[start]) / (t - start) if t > start else 0.0
        bars_since_top = max(1, t - run_max_idx)
        dpdt_trend = p_now / bars_since_top
        accel = dpdt_recent / dpdt_trend if dpdt_trend > 0 else 0.0

        # 次级别/大级别 alive 比值：限定在当前下跌段内（birth_idx ≥ run_max_idx）
        seg_alive = sorted(
            (b for b in tree.current_barcode().alive_bars if b.birth_idx >= run_max_idx),
            key=lambda b: b.persistence,
            reverse=True,
        )
        main_p = seg_alive[0].persistence if seg_alive else 0.0
        sub_p = seg_alive[1].persistence if len(seg_alive) > 1 else 0.0
        sub_main = sub_p / main_p if main_p > 0 else 0.0

        feats[t] = PHFeature(p_now, dpdt_recent, dpdt_trend, accel, sub_main)
    return feats


# ====================================================================
# 统计：单阈值可分性（AUC = 真>假 的成对胜率）
# ====================================================================


def auc(real_vals: list[float], fake_vals: list[float], higher_is_real: bool) -> float:
    """Mann-Whitney 风格成对 AUC：P(real 比 fake 更靠 real 一侧)。

    higher_is_real=True 时假设真买点该特征更大；False 时更小。0.5=无区分力。
    """
    if not real_vals or not fake_vals:
        return float("nan")
    wins = 0.0
    for r in real_vals:
        for f in fake_vals:
            d = r - f if higher_is_real else f - r
            wins += 1.0 if d > 0 else (0.5 if d == 0 else 0.0)
    return wins / (len(real_vals) * len(fake_vals))


def _mean(xs: list[float]) -> float:
    return sum(xs) / len(xs) if xs else float("nan")


# ====================================================================
# 主流程
# ====================================================================


def find_down_legs(closes: list[float]) -> list[tuple[int, int]]:
    """用 coarse zigzag 把序列切成大级别下跌段 [high_idx, low_idx]。"""
    cz = zigzag(closes, COARSE_PCT)
    legs: list[tuple[int, int]] = []
    for i in range(1, len(cz)):
        if cz[i - 1].kind == "H" and cz[i].kind == "L":
            legs.append((cz[i - 1].idx, cz[i].idx))
    return legs


def main() -> None:
    ofile = sys.argv[1] if len(sys.argv) > 1 else "daily_ohlcv.json"
    blob = json.loads((DATA / ofile).read_text())
    bars = blob["bars"]
    closes = [float(b["close"]) for b in bars]
    df = pd.DataFrame({"close": closes})
    df_macd = compute_macd(df)

    pivots = zigzag(closes, ZIGZAG_PCT)
    cands = detect_buy_candidates(closes, pivots, df_macd)
    feats = ph_features_at(closes)
    legs = find_down_legs(closes)

    tf = "30分钟" if "m30" in ofile else "日线"
    print("#" * 78)
    print(f"# 腾讯 700 {tf} — 区间套买点真假区分的 PH 判据（L2 实测）")
    print(f"# n={len(closes)} | fine={ZIGZAG_PCT:.1%} coarse={COARSE_PCT:.0%} "
          f"反转确认={REVERSAL_PCT:.0%}")
    print("#" * 78)
    print(f"次级别买点候选(创新低 swing-low): {len(cands)} | "
          f"其中 MACD 底背驰: {sum(c.macd_div for c in cands)}")
    print(f"大级别下跌段(coarse {COARSE_PCT:.0%}): "
          f"{[(h, l, f'{closes[h]:.0f}→{closes[l]:.0f}') for h, l in legs]}")
    print()

    rows = [(c, label_signal(closes, pivots, c), feats[c.idx]) for c in cands]

    # ---- 段内纵向分析：区间套"逐级收缩"，看 accel 是否随接近底部衰减 ----
    print("=" * 78)
    print("【段内纵向】每个大级别下跌段内，次级别买点候选的 PH 演化（按时间）")
    print("  核心读法：accel<1 = 大级别下跌减速(力度衰竭)；真底应出现在 accel 最低处")
    print("=" * 78)
    hdr = (f"{'idx':>4}{'price':>8}{'MACD背驰':>9}{'label':>12}"
           f"{'P_main':>8}{'dPdt_now':>9}{'accel':>7}{'sub/main':>9}")
    for h, lo in legs:
        seg_rows = [r for r in rows if h < r[0].idx <= lo]
        if not seg_rows:
            continue
        print(f"\n下跌段 [{h}→{lo}]  {closes[h]:.0f} → {closes[lo]:.0f}  "
              f"(跌 {(1-closes[lo]/closes[h]):.0%})")
        print(hdr)
        print("-" * len(hdr))
        for c, lab, f in seg_rows:
            mk = "✓背驰" if c.macd_div else "—"
            print(f"{c.idx:>4}{c.price:>8.1f}{mk:>9}{lab:>12}"
                  f"{f.P_main:>8.1f}{f.dPdt_recent:>9.3f}{f.accel_ratio:>7.2f}"
                  f"{f.sub_main:>9.2f}")

    # ---- 全局 REAL/FAKE 统计 ----
    real = [(s, f) for s, l, f in rows if l == "REAL"]
    fake = [(s, f) for s, l, f in rows if l == "FAKE"]
    unconf = [(s, f) for s, l, f in rows if l == "UNCONFIRMED"]
    print("\n" + "=" * 78)
    print(f"【全局统计】候选分类: REAL={len(real)} FAKE={len(fake)} "
          f"UNCONFIRMED(剔除)={len(unconf)}")
    print("=" * 78)

    if not real or not fake:
        print("⚠ 真或假样本不足以做跨段 AUC（这是 L2 的真实边界条件，不是 bug）：")
        print("  700 这 300 根日线只含约 1.5 个大级别下跌——一个已确认反转的底(idx≈23)，")
        print("  一个最终底落在数据末端尚未确认(idx≈298)。跨段二元统计样本不足。")
        print("  → 结论依据转向上方【段内纵向】的 accel 演化趋势（无需跨段对照）。")
        return

    def col(group, attr):
        return [getattr(f, attr) for _, f in group]

    specs = [
        ("P_main",      "P_main",      None),
        ("dPdt_recent", "dPdt_recent", False),
        ("dPdt_trend",  "dPdt_trend",  False),
        ("accel_ratio", "accel_ratio", False),
        ("sub_main",    "sub_main",    None),
    ]
    print(f"{'feature':>13}{'REAL均值':>12}{'FAKE均值':>12}{'AUC':>8}  解读")
    print("-" * 70)
    for name, attr, dir_real in specs:
        rv, fv = col(real, attr), col(fake, attr)
        if dir_real is None:
            a_hi = auc(rv, fv, True)
            a = max(a_hi, 1 - a_hi)
            dir_txt = "真>假" if a_hi >= 0.5 else "真<假"
        else:
            a = auc(rv, fv, dir_real)
            dir_txt = "真<假(衰竭)" if dir_real is False else "真>假"
        verdict = "★有区分力" if abs(a - 0.5) >= 0.2 else ("弱" if abs(a - 0.5) >= 0.1 else "无")
        print(f"{name:>13}{_mean(rv):>12.3f}{_mean(fv):>12.3f}{a:>8.2f}  {dir_txt} {verdict}")

    print("\nAUC 读法: 0.5=无区分力；越偏离 0.5 越能单阈值区分真假。")
    print("认识论等级 L2: 腾讯 700 单标的日线，样本量小，否定性结果同样有效。")


if __name__ == "__main__":
    main()
