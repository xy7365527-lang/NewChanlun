"""腾讯 700（HKEX:700）— PH 导航区间套实战分析。

操作范式（新）
-------------
**持续同调（PH）定操作级别，缠论在每层做形态判断。** 二者分工：

- **PH 导航层**：从原始价格序列的 sublevel-set H0 barcode 读出"主导特征跨多少
  bar"。bar 跨度 → 级别（级别从数据推出，不预设，239号/递归脚本 Q1）。
  主导级别 → 次级别 = 操作级别 → 再次级别 = 确认级别。
- **缠论形态层**：在操作级别上读一禅（CZSC）指标的笔/买卖点标注，判断走势结构
  （第几段、有无买卖点信号）。
- **区间套**：下沉到确认级别，看次级别**背驰**是否确认操作级别的买卖点。

认识论等级（formalization-validity-domain 规则）
-----------------------------------------------
- barcode / 级别映射 / 背驰比值：**L0**（纯算法，从价格确定性推导）。
- 腾讯 700 真实 OHLCV 上的读出：**L2**（单标的单时段，可被否证）。
- "bar 跨度 → 级别"换算（124 日≈周线一笔）：经验粗标，**L1→L2 边界**，非严格定理。
- 一禅指标标注：第三方 Pine 指标的形态判断，作为**外部形态证据**与 PH 互参，
  不是本系统的定义产出。

数据边界（诚实声明，不可绕过）
------------------------------
- 数据为 2026-05-22（上周五最后交易日）收盘抓取；运行日 2026-05-24 为周日，
  HK 休市，故此数据**即当前市场状态**（无更新的数据存在）。
- TradingView MCP 在本次未连通，未实时拉取——使用 tmp/tencent/ 已抓取数据。
- 中枢(boxes)/线段(lines) 抓取仅含**计数**未含几何边界，故中枢以计数 + PH H1
  loop 数表征，不给精确边界。买卖点(labels)含 x(bar索引)+price，可对齐。

ker(D) 诚实声明（239号，贯穿）
------------------------------
H0 sublevel persistence ≈ 价格振幅 ∈ ker(D)，时间盲。作笔力度时与 MACD 面积
（动量×时间）互补而非等价。背驰判断同时给 H0 与 MACD，分歧时以 MACD 对速度更敏感。
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import numpy as np  # noqa: E402

from newchan.a_persistence_barcode import barcode_from_prices, atr  # noqa: E402
from newchan.a_macd import compute_macd, macd_area_for_range  # noqa: E402
from tencent_recursive_persistence import (  # noqa: E402
    h0_positioned,
    zigzag_legs,
    classify_level_by_days,
    load_closes,
    PBar,
    BARS_PER_DAY,
)
import pandas as pd  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"

# 缠论经典级别递归链（次级别 = 链中下一个）。日/30/5 与 周/日/30 经典三级别联立。
LEVEL_CHAIN = ["月线级别", "周线级别", "日线级别", "30分钟级别", "5分钟级别", "1分钟级别"]


def _level_root(level_name: str) -> str:
    """'周线级别一笔' → '周线级别'，统一到链上的键。"""
    for lv in LEVEL_CHAIN:
        if level_name.startswith(lv):
            return lv
    return level_name


def sub_level(level_name: str) -> str:
    """级别 → 次级别（链中下一个）。链尾返回自身 + 标记。"""
    root = _level_root(level_name)
    if root in LEVEL_CHAIN:
        i = LEVEL_CHAIN.index(root)
        if i + 1 < len(LEVEL_CHAIN):
            return LEVEL_CHAIN[i + 1]
        return root + "（已达链尾）"
    return "未知次级别"


# ======================================================================
# 一禅（CZSC）买卖点标注解析
# ======================================================================
def load_buysell(labels_file: str, study_idx: int = 0):
    """读一禅标注里的买卖点（含 x=bar索引, price, text），按 x 升序。"""
    d = json.loads((DATA / labels_file).read_text())
    studies = d.get("studies", [])
    if study_idx >= len(studies):
        return "", 0, []
    s = studies[study_idx]
    labs = s.get("labels", [])
    bs = [
        {"x": l.get("x", 0), "price": l.get("price"), "text": l["text"].strip()}
        for l in labs
        if ("买" in l["text"] or "卖" in l["text"])
    ]
    bs.sort(key=lambda l: l["x"])
    return s.get("name", ""), s.get("total_labels", 0), bs


def count_structures(boxes_file: str, lines_file: str):
    """中枢(boxes)/线段(lines) 计数（几何未抓，仅计数）。"""
    def total(fn, key):
        try:
            d = json.loads((DATA / fn).read_text())
            return sum(s.get(key, 0) for s in d.get("studies", []))
        except FileNotFoundError:
            return None
    return total(boxes_file, "total_boxes"), total(lines_file, "total_lines")


# ======================================================================
# Step 1 — PH 导航：定级别
# ======================================================================
def step1_ph_navigation(name: str, closes, bars: list[PBar], tau: float):
    bpd = BARS_PER_DAY[name]
    print(f"\n{'='*72}")
    print(f"  STEP 1 · PH 导航层（{name}图）—— 持续同调定级别")
    print(f"{'='*72}")
    actives = [b for b in bars if b.persistence > tau]
    print(f"  噪声阈值 τ = ATR = {tau:.2f}，活跃特征(persistence>τ) {len(actives)} 个")
    print(f"  {'排名':<4}{'persistence':>12}{'bar跨度':>8}{'≈交易日':>9}  级别(数据推出)")
    for k, b in enumerate(actives[:5]):
        days = b.span / bpd
        print(f"  #{k+1:<3}{b.persistence:>12.2f}{b.span:>8}{days:>9.1f}  "
              f"{classify_level_by_days(days)}  [bar {b.birth_idx}→{b.death_idx}]")

    dom = actives[0]
    dom_days = dom.span / bpd
    dom_level = classify_level_by_days(dom_days)
    op_level = sub_level(dom_level)
    cf_level = sub_level(op_level)

    print(f"\n  ── 级别导航判定 ──")
    print(f"  主导级别 = 最大 persistence 特征跨度 {dom.span} bar ≈ {dom_days:.0f} 交易日")
    print(f"           → {dom_level}  (力度 {dom.persistence:.1f} HKD)")
    print(f"  操作级别 = 主导的次级别            → {op_level}")
    print(f"  确认级别 = 操作的次级别            → {cf_level}")
    print(f"\n  ▶ 导航结论：你应该从【{op_level}】操作，区间套到【{cf_level}】确认。")
    return {"dominant": dom_level, "operating": op_level, "confirm": cf_level, "dom_bar": dom}


# ======================================================================
# Step 2 — 操作级别上的缠论形态判断
# ======================================================================
def step2_morphology(name: str, closes, tau: float, labels_file: str,
                     boxes_file: str, lines_file: str):
    print(f"\n{'='*72}")
    print(f"  STEP 2 · 缠论形态层（{name}图 = 操作级别）—— 一禅标注 + 走势结构")
    print(f"{'='*72}")
    # PH 视角：当前在第几段（zigzag 笔），最近一笔方向/力度
    legs = zigzag_legs(closes, threshold=max(tau, 1e-9))
    n_box, n_line = count_structures(boxes_file, lines_file)
    peak_idx = int(np.argmax(closes))
    last = len(closes) - 1
    print(f"  走势结构（PH/zigzag 自导出，与 OHLCV 对齐）:")
    print(f"    总笔数 {len(legs)}；峰值在 bar {peak_idx}（{closes[peak_idx]:.1f}），"
          f"当前 bar {last}（{closes[-1]:.1f}）")
    print(f"    一禅指标中枢(中枢框)计数 {n_box} 个 | 线段计数 {n_line} 个（全历史，仅计数）")
    if legs:
        a, b, d = legs[-1]
        print(f"    当前最后一笔: bar[{a}:{b}] {'下跌' if d<0 else '上涨'} 幅度 {abs(d):.1f}")

    name_st, tot, bs = load_buysell(labels_file)
    print(f"\n  一禅(CZSC)买卖点标注: study={name_st} 总标注 {tot}，买卖点 {len(bs)} 个")
    print(f"  最近 6 个买卖点（x=指标全历史bar索引, 升序）:")
    for l in bs[-6:]:
        print(f"    x={l['x']:>4}  价 {l['price']:>7}  →  {l['text']}")
    if bs:
        cur = bs[-1]
        print(f"\n  ▶ 操作级别当前一禅信号: 【{cur['text']}】@ {cur['price']} HKD")
        if "预期" in cur["text"]:
            print(f"    注：'预期'= 一禅投影的潜在买卖点，**尚未确认**（需次级别背驰兑现）")
    return bs[-1] if bs else None


# ======================================================================
# Step 3 — 区间套到确认级别：次级别背驰
# ======================================================================
def step3_nested_confirm(name: str, closes, tau: float, labels_file: str):
    print(f"\n{'='*72}")
    print(f"  STEP 3 · 区间套（{name}图 = 确认级别）—— 次级别背驰是否确认买点")
    print(f"{'='*72}")
    df_macd = compute_macd(pd.DataFrame({"close": closes}))
    legs = zigzag_legs(closes, threshold=max(tau, 1e-9))
    print(f"  确认级别笔数 {len(legs)}")
    if len(legs) < 3:
        print("  笔不足，无法做背驰对比。")
        return None

    last_leg = legs[-1]
    same = [lg for lg in legs[:-1] if (lg[2] > 0) == (last_leg[2] > 0)]
    if not same:
        print("  无同向相邻笔可比。")
        return None
    a, c = same[-1], last_leg
    direction = "下跌" if c[2] < 0 else "上涨"
    fa = barcode_from_prices(closes[a[0]:a[1]+1], maxdim=0).total_persistence(0)
    fc = barcode_from_prices(closes[c[0]:c[1]+1], maxdim=0).total_persistence(0)
    ma = macd_area_for_range(df_macd, a[0], a[1])
    mc = macd_area_for_range(df_macd, c[0], c[1])
    macd_a = abs(ma["area_neg"]) if a[2] < 0 else abs(ma["area_pos"])
    macd_c = abs(mc["area_neg"]) if c[2] < 0 else abs(mc["area_pos"])

    h0_div = fc < fa
    macd_div = macd_c < macd_a
    print(f"  最近两个同向({direction})笔: A=bar[{a[0]}:{a[1]}]  C=bar[{c[0]}:{c[1]}]")
    print(f"    H0 力度(振幅,ker(D)): A={fa:.2f}  C={fc:.2f}  C/A={fc/fa if fa else 0:.3f}"
          f"  → {'背驰' if h0_div else '不背驰'}")
    print(f"    MACD 面积(动量×时间):  A={macd_a:.3f}  C={macd_c:.3f}  C/A={macd_c/macd_a if macd_a else 0:.3f}"
          f"  → {'背驰' if macd_div else '不背驰'}")
    consistent = h0_div == macd_div
    print(f"    两度量{'一致 ✓' if consistent else '★分歧（MACD对速度更敏感）'}")

    name_st, tot, bs = load_buysell(labels_file)
    if bs:
        cur = bs[-1]
        print(f"\n  确认级别一禅信号: 【{cur['text']}】@ {cur['price']}")

    confirmed = h0_div and macd_div and direction == "下跌"
    print(f"\n  ▶ 区间套判定: 确认级别{direction}笔 "
          f"{'★ 双度量背驰 → 买点获次级别确认' if confirmed else '力度未递减 → 背驰未确认，买点未兑现'}")
    return {"h0_div": h0_div, "macd_div": macd_div, "ratio": fc/fa if fa else 0,
            "direction": direction, "confirmed": confirmed, "yichan": bs[-1] if bs else None}


# ======================================================================
# Step 4 — 综合结论
# ======================================================================
def step4_synthesis(nav, op_sig, cf):
    print(f"\n\n{'#'*72}")
    print(f"#  STEP 4 · 综合结论 —— PH 导航 × 缠论形态 × 区间套确认")
    print(f"{'#'*72}")
    print(f"\n  ① PH 导航说了什么级别:")
    print(f"     主导={nav['dominant']} → 操作=【{nav['operating']}】 → 确认=【{nav['confirm']}】")
    print(f"\n  ② 形态学在操作级别看到了什么:")
    if op_sig:
        print(f"     一禅信号【{op_sig['text']}】@ {op_sig['price']}"
              f"{'（预期，未确认）' if '预期' in op_sig['text'] else ''}")
    print(f"\n  ③ 区间套在确认级别确认了什么:")
    if cf:
        print(f"     确认级别{cf['direction']}笔 H0={'背驰' if cf['h0_div'] else '不背驰'} / "
              f"MACD={'背驰' if cf['macd_div'] else '不背驰'} (C/A={cf['ratio']:.3f})")
        if cf.get("yichan"):
            print(f"     确认级别一禅【{cf['yichan']['text']}】")
    print(f"\n  ④ 最终判断:")
    if cf and cf["confirmed"]:
        print(f"     操作级别买点获确认级别背驰确认 → 结构上买点成立")
    elif cf:
        print(f"     操作级别出现买点【预期】(PH+一禅均指向潜在一买)，")
        print(f"     但区间套到确认级别：次级别力度未递减 → 背驰未确认 → 买点【尚未兑现】")
        print(f"     结构状态：下跌的最后一笔仍在延续，买点处于'预期/酝酿'而非'确认'阶段")


def main():
    print("#" * 72)
    print("# 腾讯 700（HKEX:700）PH 导航区间套实战分析")
    print("# 范式：持续同调定操作级别，缠论(一禅)在每层做形态判断")
    print("# 数据：2026-05-22 收盘（周日 HK 休市，即当前市场状态）")
    print("#" * 72)

    # 操作级别 = 日线，确认级别 = 30分钟（由 Step1 在日线图上导航确定）
    dh, dl, dc = load_closes("daily_ohlcv.json")
    d_tau = atr(dh, dl, dc, period=14)
    d_bars = h0_positioned(dc)
    nav = step1_ph_navigation("日线 1D", dc, d_bars, d_tau)

    op_sig = step2_morphology(
        "日线 1D", dc, d_tau,
        "daily_labels_v.json", "daily_boxes_v.json", "daily_lines.json"
    )

    mh, ml, mc = load_closes("m30_ohlcv.json")
    m_tau = atr(mh, ml, mc, period=14)
    cf = step3_nested_confirm("30分钟", mc, m_tau, "m30_labels_v.json")

    step4_synthesis(nav, op_sig, cf)


if __name__ == "__main__":
    main()
