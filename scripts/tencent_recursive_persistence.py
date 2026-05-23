"""腾讯 700 — 持续同调的递归级别分析 + persistence vs MACD 力度对照。

回应五个研究问题（2026-05-23 编排者追加）
------------------------------------------
Q1 级别映射：主导 H0 feature 的 birth→death **bar 跨度** = 这一"笔"的持续时间，
   持续时间决定级别（级别从数据推出，不预设）。
Q2 下跌趋势递归背驰：在 670→当前 的下跌内部分次级别推动段，逐段 barcode + Wasserstein-1。
Q3 递归对象 barcode：每一笔/线段/中枢挂 persistence；上级 = 组成部分聚合。
Q4 persistence-分布分层做区间套：按 persistence 长度自动分层（gap 检测），大级别确认趋势、
   次级别力度递减、最小级别 death = 转折。
Q5 persistence vs MACD 作为笔力度：同方向相邻笔 persistence 对比 vs MACD 面积对比，是否一致。

认识论等级
----------
- 位置感知 H0 / barcode / Wasserstein / MACD 面积：L0（纯算法）。
- 腾讯 700 真实 OHLCV 上的读出：L2（单标的单时段）。
- 级别映射的"60日≈日线笔/250日≈周线笔"换算：经验粗标，非严格定理（L1→L2 边界）。

关键诚实声明（贯穿全文，不可绕过）
----------------------------------
**H0 sublevel persistence ≈ 价格振幅 ∈ ker(D)（239号）。** 一笔（单调运动）的 H0 持续度
几乎等于该笔的价格幅度——时间盲（只看端点，不看快慢）。MACD 面积 = 动量的时间积分
（与持续时间/速度相关）。二者度量的不是同一对象：相同幅度但耗时不同的两笔，H0 持续度相同、
MACD 面积不同。故 persistence 作笔力度 = 用振幅作力度，会丢失 MACD 捕捉的"动量随时间积累"。
持续同调真正超出振幅的信息在 **H1（中枢 loop 几何）** 和 **diagram 间 Wasserstein 距离（结构重组）**。
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.a_persistence_barcode import barcode_from_prices, atr  # noqa: E402
from newchan.a_divergence_topo import topo_divergence  # noqa: E402
from newchan.a_macd import compute_macd, macd_area_for_range  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"
# HK 港股每交易日约 11 根 30 分钟 K（9:30-12:00 + 13:00-16:00 = 5.5h）
BARS_PER_DAY = {"日线 1D": 1.0, "30分钟": 11.0}


# ======================================================================
# 位置感知 H0（merge-tree，额外返回 birth_idx / death_idx）
# ======================================================================
@dataclass(frozen=True)
class PBar:
    birth: float
    death: float
    persistence: float
    birth_idx: int
    death_idx: int

    @property
    def span(self) -> int:
        """该特征形成所跨的 bar 数（|death_idx - birth_idx|）。"""
        return abs(self.death_idx - self.birth_idx)


def h0_positioned(values, finite_cap: float | None = None) -> list[PBar]:
    """1D sublevel H0，记录每个特征的 birth/death bar 位置。

    与 a_persistence_barcode.sublevel_h0_bars 同算法（union-find merge tree），
    额外跟踪 birth_idx（局部极小 bar）与 death_idx（合并 bar）。全局存活分量的
    death 设为 cap，death_idx 设为全局最大值所在 bar（= 主导运动的另一端）。
    """
    vals = [float(v) for v in values]
    n = len(vals)
    if n == 0:
        return []
    cap = max(vals) if finite_cap is None else float(finite_cap)
    argmax = int(np.argmax(vals))

    order = sorted(range(n), key=lambda i: vals[i])
    parent: dict[int, int] = {}
    bval: dict[int, float] = {}
    bidx: dict[int, int] = {}
    active = [False] * n
    bars: list[PBar] = []

    def find(x: int) -> int:
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for i in order:
        vi = vals[i]
        parent[i] = i
        bval[i] = vi
        bidx[i] = i
        active[i] = True
        roots: list[int] = []
        for j in (i - 1, i + 1):
            if 0 <= j < n and active[j]:
                r = find(j)
                if r not in roots:
                    roots.append(r)
        if not roots:
            continue
        comps = list(dict.fromkeys([find(i), *roots]))
        elder = min(comps, key=lambda r: bval[r])
        for r in comps:
            if r == elder:
                continue
            b = bval[r]
            if b < vi:
                bars.append(PBar(b, vi, vi - b, bidx[r], i))
            parent[r] = elder

    alive = {find(i) for i in range(n)}
    for r in alive:
        b = bval[r]
        bars.append(PBar(b, cap, cap - b, bidx[r], argmax))

    bars.sort(key=lambda b: b.persistence, reverse=True)
    return bars


# ======================================================================
# 数据加载
# ======================================================================
def load_closes(ofile: str):
    blob = json.loads((DATA / ofile).read_text())
    bars = blob["bars"]
    return (
        [float(b["high"]) for b in bars],
        [float(b["low"]) for b in bars],
        [float(b["close"]) for b in bars],
    )


# ======================================================================
# zigzag 分腿（笔代理）——拓扑自导出的笔，与 OHLCV 完全对齐
# ======================================================================
def zigzag_legs(closes, threshold: float):
    if len(closes) < 3:
        return []
    pivots = [0]
    direction = 0
    piv, pidx = closes[0], 0
    for i in range(1, len(closes)):
        c = closes[i]
        if direction >= 0 and c - piv >= threshold:
            direction = 1 if direction == 0 else direction
            piv, pidx = c, i
        elif direction <= 0 and piv - c >= threshold:
            direction = -1 if direction == 0 else direction
            piv, pidx = c, i
        if direction == 1 and piv - c >= threshold:
            pivots.append(pidx)
            direction = -1
            piv, pidx = c, i
        elif direction == -1 and c - piv >= threshold:
            pivots.append(pidx)
            direction = 1
            piv, pidx = c, i
    if pivots[-1] != len(closes) - 1:
        pivots.append(len(closes) - 1)
    return [(a, b, closes[b] - closes[a]) for a, b in zip(pivots[:-1], pivots[1:])]


# ======================================================================
# Q1 级别映射：主导特征 bar 跨度 → 级别
# ======================================================================
def classify_level_by_days(days: float) -> str:
    """从一笔的持续交易日数推断级别（经验粗标，L1）。"""
    if days >= 180:
        return "月线级别一笔"
    if days >= 80:
        return "周线级别一笔"
    if days >= 15:
        return "日线级别一笔"
    if days >= 3:
        return "30分钟级别一笔"
    return "次30分钟/噪声"


def q1_level_mapping(name: str, closes, bars: list[PBar]):
    bpd = BARS_PER_DAY[name]
    print(f"\n{'='*70}")
    print(f"【Q1 · {name}】级别映射——主导结构跨多少 bar = 哪个级别的一笔")
    print(f"{'='*70}")
    print(f"  换算基准: 1 交易日 ≈ {bpd:.0f} 根 K 线")
    print(f"  {'排名':<4}{'persistence':>12}{'bar跨度':>9}{'≈交易日':>9}  级别(数据推出)")
    for k, b in enumerate(bars[:5]):
        days = b.span / bpd
        print(f"  #{k+1:<3}{b.persistence:>12.2f}{b.span:>9}{days:>9.1f}  "
              f"{classify_level_by_days(days)}  [bar {b.birth_idx}→{b.death_idx}]")
    dom = bars[0]
    print(f"\n  → 主导级别力度 = {dom.persistence:.2f} HKD，"
          f"对应 {dom.span} 根K线 ≈ {dom.span/bpd:.0f} 交易日 "
          f"= {classify_level_by_days(dom.span/bpd)}")
    print(f"    （级别不是预设的；它从主导特征的时间跨度被读出来）")


# ======================================================================
# Q4 persistence 分布分层（gap 检测）→ 区间套
# ======================================================================
def auto_layer_thresholds(persistences: list[float]) -> tuple[float, float]:
    """从 persistence 分布用最大对数间隙自动确定两个分层阈值 P1>P2。"""
    ps = sorted([p for p in persistences if p > 0], reverse=True)
    if len(ps) < 3:
        return (ps[0] if ps else 0.0, 0.0)
    logs = np.log(ps)
    gaps = [(logs[i] - logs[i + 1], i) for i in range(len(logs) - 1)]
    gaps.sort(reverse=True)
    cut_idx = sorted([gaps[0][1], gaps[1][1]])  # 两个最大间隙的位置
    p1 = (ps[cut_idx[0]] + ps[cut_idx[0] + 1]) / 2
    p2 = (ps[cut_idx[1]] + ps[cut_idx[1] + 1]) / 2
    return (p1, p2)


def q4_nested_levels(name: str, closes, bars: list[PBar], tau: float):
    bpd = BARS_PER_DAY[name]
    p1, p2 = auto_layer_thresholds([b.persistence for b in bars])
    big = [b for b in bars if b.persistence >= p1]
    mid = [b for b in bars if p2 <= b.persistence < p1]
    small = [b for b in bars if b.persistence < p2 and b.persistence > tau * 0.3]
    last = len(closes) - 1

    print(f"\n{'='*70}")
    print(f"【Q4 · {name}】持续同调原生区间套（按 persistence 分布自动分层）")
    print(f"{'='*70}")
    print(f"  自动阈值: P1={p1:.2f}  P2={p2:.2f}（最大对数间隙切分）")
    print(f"  大级别(p≥P1) {len(big)} 个 | 中级别 {len(mid)} 个 | 小级别(p<P2,>噪声) {len(small)} 个")

    # 大级别：取极值最靠近末端的大级别特征 = 当前活跃趋势（非历史最大幅度）
    if big:
        b0 = max(big, key=lambda b: max(b.birth_idx, b.death_idx))
        ext_idx = max(b0.birth_idx, b0.death_idx)   # 较新的端点 = 当前趋势极值
        trend_down = b0.birth_idx > b0.death_idx     # 低点(birth)更新 = 下跌主导
        alive = (last - ext_idx) <= max(2, int(0.03 * len(closes)))
        print(f"\n  ① 大级别趋势确认（取极值最近的活跃趋势）:")
        print(f"     主导 [bar {min(b0.birth_idx,b0.death_idx)}→{max(b0.birth_idx,b0.death_idx)}]"
              f" 力度 {b0.persistence:.2f}，方向 {'下跌' if trend_down else '上涨'}")
        print(f"     趋势极值在 bar {ext_idx}，末 bar {last} → "
              f"{'★ 极值贴近末端（趋势刚到极值/仍在延伸）' if alive else '极值已过（已折返）'}")

    # 次级别：最近两个同向推动力度是否递减（背驰）
    legs = zigzag_legs(closes, threshold=max(tau, 1e-9))
    if len(legs) >= 3:
        last_leg = legs[-1]
        same = [lg for lg in legs[:-1] if (lg[2] > 0) == (last_leg[2] > 0)]
        if same:
            a, c = same[-1], last_leg
            fa = barcode_from_prices(closes[a[0]:a[1]+1], maxdim=0).total_persistence(0)
            fc = barcode_from_prices(closes[c[0]:c[1]+1], maxdim=0).total_persistence(0)
            direction = "下跌" if c[2] < 0 else "上涨"
            print(f"\n  ② 次级别推动力度递减检测（背驰）:")
            print(f"     最近两个{direction}推动: A力度={fa:.2f}  C力度={fc:.2f}  C/A={fc/fa if fa else 0:.3f}")
            print(f"     {'★ 力度递减 = 背驰 = 动能衰竭' if fc < fa else '力度未递减 = 不背驰（动能延续）'}")

    # 最小级别：是否出现 death（折返信号）在末端
    recent_small = [b for b in small if abs(b.death_idx - last) <= max(3, int(0.05*len(closes)))]
    print(f"\n  ③ 最小级别转折信号:")
    print(f"     末端附近完成 death 的小级别特征: {len(recent_small)} 个"
          f"{' → ★ 出现小级别折返' if recent_small else ' → 无明显小级别转折'}")


# ======================================================================
# Q2 下跌趋势递归背驰（670→当前 内部分推动段）
# ======================================================================
def q2_downtrend_recursive(name: str, closes, tau: float):
    print(f"\n{'='*70}")
    print(f"【Q2 · {name}】下跌趋势内部递归背驰（峰值→当前，逐推动段 Wasserstein-1）")
    print(f"{'='*70}")
    peak_idx = int(np.argmax(closes))
    seg = closes[peak_idx:]
    print(f"  下跌区间: bar {peak_idx}（峰值 {closes[peak_idx]:.1f}）→ "
          f"bar {len(closes)-1}（当前 {closes[-1]:.1f}），共 {len(seg)} 根")
    legs = zigzag_legs(seg, threshold=max(tau, 1e-9))
    downs = [(a, b, d) for (a, b, d) in legs if d < 0]
    print(f"  区间内下跌推动段: {len(downs)} 个")
    if len(downs) < 2:
        print("  下跌推动段不足 2，无法递归对比。")
        return
    print(f"  {'推动段':<6}{'bar区间':<14}{'跌幅':>9}{'H0力度':>9}{'相邻C/A':>10}")
    prev = None
    for k, (a, b, d) in enumerate(downs):
        force = barcode_from_prices(seg[a:b+1], maxdim=0).total_persistence(0)
        ratio = f"{force/prev:.3f}" if prev else "—"
        flag = ""
        if prev is not None:
            flag = " ★背驰" if force < prev else " 增强"
        print(f"  #{k+1:<5}[{a:>3}:{b:<3}]    {d:>+8.2f}{force:>9.2f}{ratio:>10}{flag}")
        prev = force
    # 首尾推动段 Wasserstein
    f_first, f_last = downs[0], downs[-1]
    div = topo_divergence(
        barcode_from_prices(seg[f_first[0]:f_first[1]+1], maxdim=1),
        barcode_from_prices(seg[f_last[0]:f_last[1]+1], maxdim=1),
        dimension=0, noise_floor=tau,
    )
    print(f"\n  首推动段 vs 末推动段: 力度 {div.force_a:.2f} → {div.force_c:.2f}  "
          f"W1(diagram)={div.wasserstein_ac:.2f}")
    print(f"  整体下跌动能: {'★ 衰竭（末段弱于首段）' if div.force_c < div.force_a else '未衰竭（末段不弱）'}")


# ======================================================================
# Q3 + Q5 递归对象 barcode + persistence vs MACD（逐笔力度对照）
# ======================================================================
def q3q5_per_bi_force(name: str, closes, tau: float):
    df = pd.DataFrame({"close": closes})
    df_macd = compute_macd(df)
    legs = zigzag_legs(closes, threshold=max(tau, 1e-9))
    print(f"\n{'='*70}")
    print(f"【Q3+Q5 · {name}】逐笔力度：persistence(H0) vs MACD 面积")
    print(f"{'='*70}")
    print(f"  笔(zigzag)数: {len(legs)}（拓扑自导出的笔，与 OHLCV 完全对齐）")
    print(f"  {'笔':<4}{'方向':<5}{'bar区间':<13}{'幅度':>8}{'maxH0':>8}{'totH0':>8}{'MACD面积':>10}")
    rows = []
    for k, (a, b, d) in enumerate(legs):
        bc = barcode_from_prices(closes[a:b+1], maxdim=0)
        maxh0 = bc.max_persistence(0)
        toth0 = bc.total_persistence(0)
        area = macd_area_for_range(df_macd, a, b)
        macd_force = abs(area["area_neg"]) if d < 0 else abs(area["area_pos"])
        rows.append((k, d, a, b, maxh0, toth0, macd_force))
        if k >= len(legs) - 8:  # 只打印最近 8 笔
            print(f"  #{k:<3}{'下跌' if d<0 else '上涨':<5}[{a:>3}:{b:<3}]   "
                  f"{abs(d):>7.2f}{maxh0:>8.2f}{toth0:>8.2f}{macd_force:>10.3f}")

    # 同方向相邻笔背驰对比：persistence 判定 vs MACD 判定
    def last_two_same_dir():
        if len(rows) < 2:
            return None
        last = rows[-1]
        for r in reversed(rows[:-1]):
            if (r[1] > 0) == (last[1] > 0):
                return r, last
        return None

    pair = last_two_same_dir()
    print(f"\n  ── 背驰判定对照（最近两个同向笔，A=较早 C=较晚）──")
    if not pair:
        print("     无同向相邻笔可比。")
        return
    a_r, c_r = pair
    direction = "下跌" if c_r[1] < 0 else "上涨"
    pers_div = c_r[5] < a_r[5]   # totH0
    pmax_div = c_r[4] < a_r[4]   # maxH0
    macd_div = c_r[6] < a_r[6]
    print(f"     方向: {direction}笔  A=#{a_r[0]}[{a_r[2]}:{a_r[3]}]  C=#{c_r[0]}[{c_r[2]}:{c_r[3]}]")
    print(f"     persistence(totH0): A={a_r[5]:.2f} C={c_r[5]:.2f} → "
          f"{'背驰' if pers_div else '不背驰'}")
    print(f"     persistence(maxH0): A={a_r[4]:.2f} C={c_r[4]:.2f} → "
          f"{'背驰' if pmax_div else '不背驰'}")
    print(f"     MACD 面积         : A={a_r[6]:.3f} C={c_r[6]:.3f} → "
          f"{'背驰' if macd_div else '不背驰'}")
    consistent = (pers_div == macd_div)
    print(f"     一致性: persistence(totH0) vs MACD = "
          f"{'一致 ✓' if consistent else '★ 不一致 ✗'}")
    if not consistent:
        print(f"     根因: H0持续度≈幅度(时间盲, ker(D)/239号)；MACD面积=动量×时间(速度敏感)。")
        print(f"          相同幅度但 C 笔耗时/速度不同 → 两度量分歧。MACD 对'缓跌vs急跌'更敏感。")


# ======================================================================
# 主流程
# ======================================================================
def main():
    print("#" * 70)
    print("# 腾讯 700（HKEX:700）持续同调递归级别分析")
    print("# 回应 Q1 级别映射 / Q2 下跌递归背驰 / Q3 递归对象 / Q4 区间套 / Q5 vs MACD")
    print("#" * 70)
    for name, ofile in [("日线 1D", "daily_ohlcv.json"), ("30分钟", "m30_ohlcv.json")]:
        highs, lows, closes = load_closes(ofile)
        tau = atr(highs, lows, closes, period=14)
        bars = h0_positioned(closes)
        print(f"\n\n{'#'*70}\n# {name}  (n={len(closes)}, 价 {min(closes):.1f}-{max(closes):.1f}, "
              f"ATR={tau:.2f})\n{'#'*70}")
        q1_level_mapping(name, closes, bars)
        q2_downtrend_recursive(name, closes, tau)
        q4_nested_levels(name, closes, bars, tau)
        q3q5_per_bi_force(name, closes, tau)


if __name__ == "__main__":
    main()
