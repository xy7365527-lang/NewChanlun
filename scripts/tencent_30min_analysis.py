"""腾讯 700（HKEX:700）30分钟 vs 日线 持续同调分析 + 一禅指标交叉验证。

数据来源
--------
通过 tradingview-mcp 的 `tv` CLI（绕过 MCP 层直连 CDP，端口 9222）从本地
TradingView Desktop 实时拉取，落地到 tmp/tencent/*.json：
  - {daily,m30}_ohlcv.json       : OHLCV（direct_bars，上限 300 根）
  - {daily,m30}_labels_v.json    : 缠论指标 label.new() 标注（笔编号 + 买卖点），含 x（bar 索引）
  - {daily,m30}_boxes_v.json     : 缠论指标 box.new() 中枢矩形（仅 high/low，无时间坐标）

认识论等级（formalization-validity-domain 规则）
------------------------------------------------
- 持续同调计算：L0（纯算法）。
- 在腾讯 700 真实 OHLCV 上的结构读出：L2（单标的单时段真实数据）。
- "拓扑背驰 ↔ 一禅买卖点一致" 的断言：受数据对齐限制，停留 L1/L2 边界（见报告"对齐缺口"）。

关键数据约束（诚实声明，不可绕过）
----------------------------------
1. OHLCV `direct_bars` 硬上限 300 根。日线 300 根 ≈ 14 个月（价 300-683，覆盖完整波段）；
   30min 300 根 ≈ 1 个月（价 441-522，仅近期）。
2. 指标标注覆盖的 feed 历史 > OHLCV 窗口：30min 标注价位 260-680，远超 OHLCV 的 441-522。
   故 30min 上指标标注与 OHLCV 不同窗口，不可 bar 级对齐。
3. 中枢 box 导出仅 high/low，无左右时间边界 → 中枢无法定位到 bar。
4. label 的 x 字段为指标 bar 空间索引，与 OHLCV direct_bars 原点不保证一致。
→ 结论：交叉验证只能在【聚合计数 + 价位带】层面进行，bar 级精确对齐不可得。
  这不是实现缺陷，是一禅指标数据导出缺时间坐标造成的能力缺口（spec-execution-gap）。
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from newchan.a_persistence_barcode import (  # noqa: E402
    atr,
    atr_noise_threshold,
    active_bars,
    barcode_from_prices,
    dominant_bar,
)
from newchan.a_divergence_topo import topo_divergence  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"


# ----------------------------------------------------------------------
# 数据加载
# ----------------------------------------------------------------------
def load(name: str) -> dict:
    return json.loads((DATA / name).read_text())


def ohlcv_arrays(blob: dict) -> tuple[list[float], list[float], list[float]]:
    bars = blob["bars"]
    highs = [float(b["high"]) for b in bars]
    lows = [float(b["low"]) for b in bars]
    closes = [float(b["close"]) for b in bars]
    return highs, lows, closes


def chan_study0_labels(blob: dict) -> list[dict]:
    """取第一个 Chan study 的标注列表。"""
    return blob["studies"][0]["labels"]


def chan_study0_zones(blob: dict) -> list[dict]:
    return blob["studies"][0].get("zones", [])


# ----------------------------------------------------------------------
# 持续同调结构读出
# ----------------------------------------------------------------------
def analyze_persistence(name: str, highs, lows, closes, *, multiple: float = 1.0) -> dict:
    n = len(closes)
    bc = barcode_from_prices(closes, maxdim=1)
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=multiple)
    atr_val = atr(highs, lows, closes, period=14)

    h0 = bc.by_dimension(0)
    h1 = bc.by_dimension(1)
    h0_active = active_bars(bc, tau, dimension=0)
    h1_active = active_bars(bc, tau, dimension=1)
    dom = dominant_bar(bc, dimension=0, tau=tau)

    print(f"\n{'='*68}")
    print(f"【{name}】持续同调结构（L0 算法 / L2 真实数据）")
    print(f"{'='*68}")
    print(f"  样本数 n_points        : {n} 根 K 线")
    print(f"  收盘价区间             : {min(closes):.2f} – {max(closes):.2f} HKD")
    print(f"  ATR(14)                : {atr_val:.3f} HKD")
    print(f"  噪声阈值 τ = {multiple}×ATR  : {tau:.3f} HKD")
    print(f"  ── H0（笔/趋势级 prominence，sublevel set）──")
    print(f"     H0 特征总数         : {len(h0)}")
    print(f"     H0 超过 τ（成级别）  : {len(h0_active)}")
    print(f"     H0 总持续度         : {bc.total_persistence(0):.2f} HKD")
    if dom:
        print(f"     主导级别力度        : {dom.persistence:.2f} HKD"
              f"  (birth={dom.birth:.2f} → death={dom.death:.2f})")
    print(f"  ── H1（中枢级 loop，Takens 嵌入 + Rips）──")
    print(f"     H1 特征总数         : {len(h1)}")
    print(f"     H1 超过 τ           : {len(h1_active)}")
    print(f"     H1 总持续度         : {bc.total_persistence(1):.4f}")
    if h0_active:
        print(f"  ── 级别谱（前 6 个超阈 H0 特征，持续度降序）──")
        for i, b in enumerate(h0_active[:6]):
            print(f"     #{i+1}: persistence={b.persistence:7.2f}  "
                  f"[{b.birth:.2f} → {b.death:.2f}]")

    return {
        "barcode": bc,
        "tau": tau,
        "atr": atr_val,
        "n": n,
        "h0_total": len(h0),
        "h0_active": len(h0_active),
        "h1_total": len(h1),
        "h1_active": len(h1_active),
        "dominant": dom.persistence if dom else 0.0,
        "h0_total_pers": bc.total_persistence(0),
        "closes": closes,
    }


# ----------------------------------------------------------------------
# 一禅指标标注解析
# ----------------------------------------------------------------------
BUYSELL_RE = re.compile(r"([123])\s*([买卖])")
NUMERIC_RE = re.compile(r"^\s*-?\d+\s*$")


def parse_indicator(name: str, labels: list[dict], zones: list[dict],
                    price_lo: float, price_hi: float) -> dict:
    numeric = [l for l in labels if NUMERIC_RE.match(l["text"])]
    buysell = [l for l in labels if "买" in l["text"] or "卖" in l["text"]]

    # 买卖点按类型计数
    bs_types: dict[str, int] = {}
    for l in buysell:
        m = BUYSELL_RE.search(l["text"])
        if m:
            key = m.group(1) + m.group(2)  # "1买" / "2卖" / "3买"
            bs_types[key] = bs_types.get(key, 0) + 1

    # 落在 OHLCV 价格窗口内的标注（用于判断窗口是否对齐）
    in_window = lambda l: price_lo <= l.get("price", -1) <= price_hi  # noqa: E731
    numeric_in = [l for l in numeric if in_window(l)]
    buysell_in = [l for l in buysell if in_window(l)]

    # 中枢价位带
    zone_bands = [(z["high"], z["low"]) for z in zones if "high" in z and "low" in z]

    print(f"\n{'-'*68}")
    print(f"【{name}】一禅指标标注（缠论 Chan Theory study0，覆盖指标 feed 全历史）")
    print(f"{'-'*68}")
    print(f"  数字标注（笔/分型编号）总数 : {len(numeric)}")
    print(f"  买卖点标注总数              : {len(buysell)}")
    if bs_types:
        ordered = sorted(bs_types.items())
        print(f"  买卖点分类                  : " +
              "  ".join(f"{k}×{v}" for k, v in ordered))
    print(f"  中枢（box.new 矩形）总数    : {len(zone_bands)}")
    print(f"  ── 与 OHLCV 价格窗口 [{price_lo:.1f}, {price_hi:.1f}] 的重叠 ──")
    print(f"     落窗内数字标注           : {len(numeric_in)} / {len(numeric)}")
    print(f"     落窗内买卖点             : {len(buysell_in)} / {len(buysell)}")
    overlap_zones = [b for b in zone_bands
                     if not (b[1] > price_hi or b[0] < price_lo)]
    print(f"     与窗口价位重叠的中枢     : {len(overlap_zones)} / {len(zone_bands)}")

    # 最近的买卖点（x = bar 索引，与时间单调）——方向性交叉验证用
    recent = sorted(buysell, key=lambda l: l.get("x", -1))[-4:]
    latest_kind = None
    if recent:
        print(f"  ── 最近 4 个买卖点（按 bar 索引 x，越大越新）──")
        for l in recent:
            m = BUYSELL_RE.search(l["text"])
            kind = (m.group(1) + m.group(2)) if m else l["text"].strip()
            print(f"     x={l.get('x')}  price={l.get('price')}  → {kind}")
        m = BUYSELL_RE.search(recent[-1]["text"])
        latest_kind = (m.group(1) + m.group(2)) if m else recent[-1]["text"].strip()
        side = "买（看涨/底部）" if "买" in (latest_kind or "") else "卖（看跌/顶部）"
        print(f"     →→ 一禅最新信号        : {latest_kind}  [{side}]")

    return {
        "numeric": len(numeric),
        "buysell": len(buysell),
        "bs_types": bs_types,
        "zones": len(zone_bands),
        "zone_bands": zone_bands,
        "numeric_in": len(numeric_in),
        "buysell_in": len(buysell_in),
        "buysell_in_labels": buysell_in,
        "overlap_zones": len(overlap_zones),
        "latest_kind": latest_kind,
    }


# ----------------------------------------------------------------------
# zigzag 分腿（用于背驰：取最后两个同向腿）
# ----------------------------------------------------------------------
def zigzag_legs(closes: list[float], threshold: float) -> list[tuple[int, int, float]]:
    """以 threshold 为最小折返幅度提取转折点，返回腿列表 (i_start, i_end, delta)。

    delta > 0 为上涨腿，< 0 为下跌腿。threshold 建议传 ATR 量纲阈值。
    """
    if len(closes) < 3:
        return []
    pivots = [0]
    direction = 0  # 1 up, -1 down
    last_pivot = closes[0]
    last_idx = 0
    for i in range(1, len(closes)):
        c = closes[i]
        if direction >= 0 and c - last_pivot >= threshold:
            if direction == 0:
                direction = 1
            last_pivot, last_idx = c, i
        elif direction <= 0 and last_pivot - c >= threshold:
            if direction == 0:
                direction = -1
            last_pivot, last_idx = c, i
        # 折返检测
        if direction == 1 and last_pivot - c >= threshold:
            pivots.append(last_idx)
            direction = -1
            last_pivot, last_idx = c, i
        elif direction == -1 and c - last_pivot >= threshold:
            pivots.append(last_idx)
            direction = 1
            last_pivot, last_idx = c, i
    if pivots[-1] != len(closes) - 1:
        pivots.append(len(closes) - 1)

    legs = []
    for a, b in zip(pivots[:-1], pivots[1:]):
        legs.append((a, b, closes[b] - closes[a]))
    return legs


def topo_divergence_on_legs(name: str, closes: list[float], tau: float) -> dict:
    legs = zigzag_legs(closes, threshold=max(tau, 1e-9))
    print(f"\n{'-'*68}")
    print(f"【{name}】拓扑背驰（Wasserstein-1 力度，a_divergence_topo）")
    print(f"{'-'*68}")
    print(f"  zigzag 分腿数（阈值 τ={tau:.3f}）: {len(legs)}")
    if len(legs) < 3:
        print("  腿数不足，无法构造同向 A/C 段进行背驰判定。")
        return {"verdict": None, "direction": None}

    # 取最后两个同向腿作为 A（较早）/ C（较晚）
    last = legs[-1]
    same_dir = [lg for lg in legs[:-1] if (lg[2] > 0) == (last[2] > 0)]
    if not same_dir:
        print("  最后一腿无同向前腿可比，无法判定背驰。")
        return {"verdict": None, "direction": None}
    a_leg = same_dir[-1]
    c_leg = last
    direction = "上涨" if c_leg[2] > 0 else "下跌"

    prices_a = closes[a_leg[0]: a_leg[1] + 1]
    prices_c = closes[c_leg[0]: c_leg[1] + 1]
    if len(prices_a) < 4 or len(prices_c) < 4:
        print(f"  最后两个{direction}腿样本过短（A={len(prices_a)},C={len(prices_c)}），"
              f"H1 不可靠，仅用 H0 力度。")
        dim = 0
    else:
        dim = 0  # H0 = 幅度维度，与 MACD 面积同构；段太短 H1 退化

    div = topo_divergence(
        barcode_from_prices(prices_a, maxdim=1),
        barcode_from_prices(prices_c, maxdim=1),
        dimension=dim,
        noise_floor=tau,
    )
    print(f"  比较方向                : 最后两个【{direction}】腿")
    print(f"  A 段（较早）            : bars[{a_leg[0]}:{a_leg[1]}] "
          f"Δ={a_leg[2]:+.2f}  力度(H{dim})={div.force_a:.2f}")
    print(f"  C 段（较晚/当前）       : bars[{c_leg[0]}:{c_leg[1]}] "
          f"Δ={c_leg[2]:+.2f}  力度(H{dim})={div.force_c:.2f}")
    print(f"  力度比 C/A              : {div.ratio:.3f}")
    print(f"  Wasserstein-1(A,C)      : {div.wasserstein_ac:.2f}（结构重组程度）")
    print(f"  拓扑背驰判定            : {'★ 背驰（力度衰竭）' if div.is_divergent else '不背驰'}")
    print(f"  边界条件（239号）       : dim=0 时拓扑力度≈振幅∈ker(D)，"
          f"段近单调则不应作独立证据")
    return {"verdict": div.is_divergent, "direction": direction}


# ----------------------------------------------------------------------
# 主流程
# ----------------------------------------------------------------------
def main() -> None:
    print("#" * 68)
    print("# 腾讯 700（HKEX:700）持续同调分析 + 一禅指标交叉验证")
    print("# 数据：TradingView Desktop 实时（CDP 9222），2026-05-23")
    print("#" * 68)

    results = {}
    for tf, ofile, lfile, bfile in [
        ("日线 1D", "daily_ohlcv.json", "daily_labels_v.json", "daily_boxes_v.json"),
        ("30分钟", "m30_ohlcv.json", "m30_labels_v.json", "m30_boxes_v.json"),
    ]:
        oh = load(ofile)
        highs, lows, closes = ohlcv_arrays(oh)
        stats = analyze_persistence(tf, highs, lows, closes)
        ind = parse_indicator(
            tf,
            chan_study0_labels(load(lfile)),
            chan_study0_zones(load(bfile)),
            price_lo=min(closes),
            price_hi=max(closes),
        )
        div = topo_divergence_on_legs(tf, closes, stats["tau"])
        results[tf] = (stats, ind, div)

    # ── 对比汇总 ──
    print(f"\n{'#'*68}")
    print("# 跨周期对比 + 一禅交叉验证汇总")
    print(f"{'#'*68}")
    d_s, d_i, d_d = results["日线 1D"]
    m_s, m_i, m_d = results["30分钟"]
    print(f"\n{'维度':<22}{'日线 1D':>16}{'30分钟':>16}")
    print("-" * 54)
    rows = [
        ("OHLCV 样本数", d_s["n"], m_s["n"]),
        ("H0 特征总数", d_s["h0_total"], m_s["h0_total"]),
        ("H0 超阈(成级别)", d_s["h0_active"], m_s["h0_active"]),
        ("H1 特征总数", d_s["h1_total"], m_s["h1_total"]),
        ("一禅 数字标注", d_i["numeric"], m_i["numeric"]),
        ("一禅 买卖点", d_i["buysell"], m_i["buysell"]),
        ("一禅 中枢数", d_i["zones"], m_i["zones"]),
        ("买卖点落OHLCV窗", d_i["buysell_in"], m_i["buysell_in"]),
    ]
    for label, dv, mv in rows:
        print(f"{label:<22}{dv:>16}{mv:>16}")
    print(f"\n主导级别力度(H0)      日线 {d_s['dominant']:.2f} HKD"
          f"   30min {m_s['dominant']:.2f} HKD")

    # ── 方向性交叉验证（拓扑背驰 vs 一禅最新信号）──
    print(f"\n{'-'*54}")
    print("拓扑背驰判定 vs 一禅最新买卖点（方向性，不依赖 bar 对齐）")
    print(f"{'-'*54}")
    for tf, s, i, dd in [("日线 1D", d_s, d_i, d_d), ("30分钟", m_s, m_i, m_d)]:
        topo = ("背驰" if dd["verdict"] else "不背驰") if dd["verdict"] is not None else "无法判定"
        direction = dd.get("direction") or "?"
        latest = i.get("latest_kind") or "无"
        # 下跌段背驰 → 看涨(底部) → 应与"买"一致；上涨段背驰 → 看跌 → 应与"卖"一致
        expect = None
        if dd["verdict"]:
            expect = "买" if direction == "下跌" else "卖"
        consistent = "—"
        if expect and latest != "无":
            consistent = "一致 ✓" if expect in latest else "不一致 ✗"
        print(f"  {tf:<8} 拓扑:{direction}段{topo:<6} 一禅最新:{latest:<6} "
              f"预期方向:{expect or '—'}  → {consistent}")
    print("\n  注：一禅最新信号取 max(bar 索引 x) 的买卖点；拓扑背驰取最后两同向腿。")
    print("      两者覆盖的 bar 窗口不同（指标 feed 历史 > OHLCV 300 根），")
    print("      故为方向性参考而非 bar 级对齐验证（见各周期'对齐缺口'）。")


if __name__ == "__main__":
    main()
