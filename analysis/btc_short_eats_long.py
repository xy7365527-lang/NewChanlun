"""BTC：加了做空后，做空吃掉了多少多头利润？（displacement vs 直接亏损）

═══════════════ 问题（用户精确化）═══════════════
v3 只做多+短差 → BTC +760%。加 root 做空后 → ε对称 −143% / nf涌现 +28.8% / T自复制 +11.8%。
**做空到底吃掉了多少多头利润？是多头本身利润缩水（displacement）还是空头亏损抵消？**
目标：修好做空（做多赚、做空也赚），不是关掉做空。

═══════════════ 会计（双通道分解）═══════════════
- long-only-root：多头 P&L = L0（从未被 root 做空 displacement 的多头利润），短差 P&L = SS0。
- 加做空版本：多头 P&L = L1，空头 P&L = S1（含短差 + root 做空）。
- **displacement = L0 − L1**（root 花时间做空→错过上涨腿→多头利润缩水）。
- **root 做空直接增量亏损 = S1 − SS0**。
- 总损失 = (L0+SS0) − (L1+S1) = displacement − (S1−SS0 的负值贡献)。

NAV-additivity 守卫：单核心 v3/T 引擎，trade 视图账之和 == strat（已验证 +28,754=+28.8%）⟹ 分解可加。

═══════════════ 做空为何不赚（机制）═══════════════
每笔空头按 entry→exit 价格方向分类：xp>ep（价格涨）= 逆势亏 / xp<ep（价格跌）= 顺势赚。
强牛市里空头多数落在"价格涨"区 = 逆势 = 结构性亏损。配合 operate.rs DBG 的 recover 失败率
（空头僵尸=开了不平=在牛市持续流血）。

认识论等级：L3（真实数据逐笔归因）。
"""

from __future__ import annotations

import json
import math
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parents[1]
ANALYSIS_CACHE = REPO / "analysis" / "data_cache"
TS_CACHE = REPO / "trading_system" / "data_cache"
BTC_RAW = ANALYSIS_CACHE / "btc_1m_full.json"
OUT_MD = REPO / "analysis" / "reports" / "btc_short_eats_long.md"

INIT = 100_000.0
LAD, EB, EP, XB, XP, SH, W, DFR, PART, REASON, POL = range(11)


def load_btc():
    raw = json.loads(BTC_RAW.read_text())
    o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    d_in = raw["dates"]
    closes, dates = [], []
    for idx in range(len(c_in)):
        o, h, l, c = float(o_in[idx]), float(h_in[idx]), float(l_in[idx]), float(c_in[idx])
        if math.isnan(o) or math.isnan(h) or math.isnan(l) or math.isnan(c):
            continue
        if o <= 0 or h <= 0 or l <= 0 or c <= 0:
            continue
        closes.append(c)
        dates.append(d_in[idx])
    drop = {i for i in range(1, len(closes) - 1)
            if abs(closes[i] / closes[i - 1] - 1) > 0.5
            and abs(closes[i + 1] / closes[i - 1] - 1) < 0.05}
    if drop:
        keep = [i for i in range(len(closes)) if i not in drop]
        closes = [closes[i] for i in keep]
        dates = [dates[i] for i in keep]
    return np.asarray(closes, dtype=float), dates


def pnl(t):
    ep, xp, sh, pol = float(t[EP]), float(t[XP]), float(t[SH]), t[POL]
    return sh * (xp - ep) if pol == "long" else sh * (ep - xp)


def decompose(path, label):
    d = json.loads(Path(path).read_text())
    tr = d["trades"]
    strat = d.get("strat_pct", (d.get("final_nav", INIT) / INIT - 1) * 100)
    longs = [t for t in tr if t[POL] == "long"]
    shorts = [t for t in tr if t[POL] == "short"]
    lp = sum(pnl(t) for t in longs)
    sp = sum(pnl(t) for t in shorts)
    # 短差(reduce/recover) vs root做空(F-entry→core_clear/eod/liq) 拆分
    swing_reasons = {"reduce", "recover"}
    sp_swing = sum(pnl(t) for t in shorts if t[REASON] in swing_reasons)
    sp_root = sp - sp_swing
    # 空头按价格方向（逆势亏/顺势赚）
    s_counter = sum(pnl(t) for t in shorts if float(t[XP]) > float(t[EP]))  # 价格涨=逆势
    s_with = sum(pnl(t) for t in shorts if float(t[XP]) <= float(t[EP]))    # 价格跌=顺势
    n_counter = sum(1 for t in shorts if float(t[XP]) > float(t[EP]))
    n_with = sum(1 for t in shorts if float(t[XP]) <= float(t[EP]))
    # NAV-additivity 守卫
    nav_sum_pct = (lp + sp) / INIT * 100
    additive = abs(nav_sum_pct - strat) < 1.0
    return {
        "label": label, "strat": strat, "lp": lp, "sp": sp,
        "n_long": len(longs), "n_short": len(shorts),
        "sp_swing": sp_swing, "sp_root": sp_root,
        "s_counter": s_counter, "s_with": s_with, "n_counter": n_counter, "n_with": n_with,
        "nav_sum_pct": nav_sum_pct, "additive": additive, "trades": tr,
    }


def segment_long_pnl(tr, closes, segs, dates):
    """每个牛市段内的多头 P&L（按 exit_bar 落在段内）。"""
    rows = []
    for i, (s0, s1, lo, hi, g) in enumerate(segs):
        lp = sum(pnl(t) for t in tr if t[POL] == "long" and s0 <= int(t[XB]) <= s1)
        sp = sum(pnl(t) for t in tr if t[POL] == "short" and s0 <= int(t[XB]) <= s1)
        rows.append((f"B{i+1}", dates[s0][:7], g, lp, sp))
    return rows


def main():
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "diag", REPO / "analysis" / "btc_bull_throwback_diagnosis.py")
    diag = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(diag)

    closes, dates = load_btc()
    bh = (closes[-1] / closes[0] - 1) * 100
    segs = diag.find_bull_segments(closes, dates)

    lines = []
    P = lines.append
    P("# BTC：做空吃掉了多少多头利润？（displacement vs 直接亏损）\n")
    P(f"- BH +{bh:.0f}%；问题：v3 只做多+短差 +760% → 加做空后踏空。做空吃了多少多头利润？")
    P("- 目标：**修好做空**（多空都赚），不是关掉做空。认识论等级 **L3**。\n")

    # —— 各版本可用数据 ——
    versions = [
        ("v3 long-only (受控)", ANALYSIS_CACHE / "fugue_v3_BTC_longonly.json"),
        ("v3 nf涌现 (+28.8%)", TS_CACHE / "fugue_v3_BTC_nf_backup.json"),
        ("T自复制 structural", ANALYSIS_CACHE / "t_engine_BTC_structural_trades.json"),
        ("T自复制 and", ANALYSIS_CACHE / "t_engine_BTC_and_trades.json"),
        ("T自复制 or", ANALYSIS_CACHE / "t_engine_BTC_or_trades.json"),
        ("t_fugue(有root) and", TS_CACHE / "t_fugue_BTC_and.json"),
    ]
    results = {}
    P("## 1. 多空 P&L 分解（视图账，NAV-additivity 守卫）\n")
    P("| 版本 | strat% | 多头P&L | 空头P&L | 多+空 | 可加? | 空头-短差 | 空头-root | 多头笔 | 空头笔 |")
    P("|------|--------|---------|---------|-------|-------|----------|----------|--------|--------|")
    for label, path in versions:
        if not path.exists():
            P(f"| {label} | 文件缺失 `{path.name}` | | | | | | | | |")
            continue
        r = decompose(path, label)
        results[label] = r
        P(f"| {label} | {r['strat']:+.1f} | {r['lp']:+,.0f} | {r['sp']:+,.0f} | "
          f"{r['lp']+r['sp']:+,.0f} | {'✓' if r['additive'] else '✗'} | "
          f"{r['sp_swing']:+,.0f} | {r['sp_root']:+,.0f} | {r['n_long']} | {r['n_short']} |")
    P("")

    # —— displacement 会计 ——
    P("## 2. displacement 会计（做空吃掉的多头利润）\n")
    lo = results.get("v3 long-only (受控)")
    nf = results.get("v3 nf涌现 (+28.8%)")
    if lo and nf:
        L0, SS0 = lo["lp"], lo["sp"]
        L1, S1 = nf["lp"], nf["sp"]
        disp = L0 - L1
        root_extra = S1 - SS0
        P(f"- **long-only-root**（受控，root 不做空，H¹ 短差仍在）：多头 L0={L0:+,.0f}，短差 SS0={SS0:+,.0f}，总={L0+SS0:+,.0f}（{(L0+SS0)/INIT*100:+.1f}%）")
        P(f"- **nf 加 root 做空**：多头 L1={L1:+,.0f}，空头 S1={S1:+,.0f}，总={L1+S1:+,.0f}（{(L1+S1)/INIT*100:+.1f}%）")
        P(f"- **displacement = L0 − L1 = {disp:+,.0f}**（root 花时间做空→错过上涨腿→多头利润缩水 {disp/INIT*100:+.1f}pp）")
        P(f"- **root 做空直接增量亏损 = S1 − SS0 = {root_extra:+,.0f}**（{root_extra/INIT*100:+.1f}pp）")
        tot = (L0 + SS0) - (L1 + S1)
        P(f"- **总损失 = {tot:+,.0f}（{tot/INIT*100:+.1f}pp）** = displacement({disp/INIT*100:+.1f}) + 直接亏损({root_extra/INIT*100:+.1f})")
        if abs(disp) > abs(root_extra):
            P(f"- ⟹ **主因是 displacement**（多头被挤掉），不是空头直接亏损。修做空必须先解决「root 做空时多头停摆」。\n")
        else:
            P(f"- ⟹ **主因是空头直接亏损**。修做空必须先解决「空头本身在牛市亏钱」。\n")
    else:
        P("- ⚠ long-only 受控数据缺失（需 FUGUE_LONG_ONLY=1 重跑 v3 BTC）。\n")

    # —— 做空为何不赚（逆势分类）——
    P("## 3. 做空为何不赚（每笔空头按价格方向）\n")
    P("> 逆势 = 平仓时价格高于开仓（牛市里做空被涨势打），顺势 = 价格跌（做空赚）。\n")
    P("| 版本 | 逆势空头P&L(笔) | 顺势空头P&L(笔) | 逆势占比 |")
    P("|------|---------------|---------------|---------|")
    for label, r in results.items():
        tot_n = r["n_counter"] + r["n_with"]
        frac = r["n_counter"] / max(1, tot_n)
        P(f"| {label} | {r['s_counter']:+,.0f}({r['n_counter']}) | {r['s_with']:+,.0f}({r['n_with']}) | {frac*100:.0f}% |")
    P("")

    # —— 段内多头 P&L 对比 ——
    P("## 4. 牛市段内多头 P&L（displacement 在哪些段发生）\n")
    P("| 段 | 起始 | 涨幅 | " + " | ".join(r for r in results) + " |")
    P("|----|------|------|" + "|".join("------" for _ in results) + "|")
    seg_rows = {label: segment_long_pnl(r["trades"], closes, segs, dates) for label, r in results.items()}
    for i, (s0, s1, lo_, hi_, g) in enumerate(segs):
        cells = []
        for label in results:
            lp = seg_rows[label][i][3]
            cells.append(f"{lp:+,.0f}")
        P(f"| B{i+1} | {dates[s0][:7]} | +{g*100:.0f}% | " + " | ".join(cells) + " |")
    P("\n> 单元格 = 该段内多头 P&L（视图账）。long-only vs nf 的差 = 该段被 displacement 的多头利润。\n")

    # —— 僵尸假设反驳（FUGUE_DBG 实测 recover 成功率）——
    P("## 5. 「空头僵尸化」假设被数据反驳（FUGUE_DBG recover 成功率）\n")
    P("> operate.rs DBG 计数器实测（FUGUE_DBG=1 跑 BTC）：recover=把次级别空头平回核心。")
    P("> 若空头僵尸（开了不平），recover 成功率应很低。实测两版本几乎相同 ~97%：\n")
    P("| 版本 | recover attempt | ok | 成功率 | 失败分类 |")
    P("|------|----------------|-----|--------|---------|")
    P("| long-only (+901.7%) | 4715 | 4576 | **97.1%** | L4 wrongdir=139 |")
    P("| nf (+28.8%) | 5038 | 4899 | **97.2%** | L4 wrongdir=139 |")
    P("")
    P("- 两版本 recover 成功率几乎相同（97.1% vs 97.2%），L4 wrongdir 都恰好 139 笔（结构性，非 root 做空引起）。")
    P("- ⟹ **「空头僵尸化」不是踏空机制**——空头覆盖率两版本一致。差异全部来自 root 翻空的 displacement + 短差在 short-root 模式下的 P&L 崩坏，不是覆盖失败。\n")

    # —— 机制结论 ——
    P("## 6. 机制结论（修做空的方向）\n")
    P("**做空吃掉了 873pp，拆成两个通道：**")
    P("- **displacement −660pp（76%，主因）**：加 F-entry-short 后 root 在 secular bull 的回调处翻空，")
    P("  多头核心停摆 → 多头 P&L 从 +907k 崩到 +247k。root 每空一段 = 错过一段上涨腿。")
    P("- **短差 P&L 崩坏 −213pp（24%）**：H¹ 短差在 **root 做多时盈利 +18k**（long-only）/ +185k（T-struct），")
    P("  但 root 翻空后整套 sink/recover 在 short-root 模式下运作 → 翻成 −180k。\n")
    P("**对抗验证修正（重要诚实性，B 验证 holds=False）：做空机制本身没有 edge。**")
    P("- long-only 短差 +18k **是噪声不是盈利**：胜率 49.8%、median≈0、删 top1% 翻 −463k、bootstrap P(sum>0)=0.52 与零不可区分。")
    P("- nf 短差 −180k 则是**稳健为负**（删 top1% 仍 −152k）——root 可翻空后短差从「零 edge」退化为「稳健亏」。")
    P("- 对比：多头 P&L 是**真实 edge**（long-only 胜率 55.3%/median+96/删 top1% 仍 +701k）。")
    P("- ⟹ **做空（次级别顶背驰 sink/recover）是 50/50 硬币翻转，没有方向 edge**；杀手是 root 整仓翻空")
    P("  （displacement 挤掉真实多头 edge + 整核心强平爆仓，C 验证：T-and root 强平 = 总亏损 211%）。\n")
    P("**修做空的真实难度（诚实）**：当前做空构造（`root_dir=Down`→翻空 + 次级别顶背驰 sink）在 secular bull 里")
    P("既无 edge（硬币翻转）又有害（displacement）。`root_dir=Down` 多数是回调噪声非趋势反转。")
    P("**让做空真正赚钱不是调参，而是需要一个真实的做空 alpha 源**——现有 sink/recover 机制净期望≈0。\n")
    P("**修复方向**：①最低限度——root 维持多头底仓、做空仅作受限次级别 overlay（消除 660pp displacement，")
    P("把真实多头 edge 还回来）；②若要做空盈利——需给空头注入真实下跌 edge（更高级别确认的下跌 regime，")
    P("非对称门槛），而非依赖当前零 edge 的 sink/recover。详见对抗验证 + 修复方案评审。\n")

    OUT_MD.parent.mkdir(parents=True, exist_ok=True)
    OUT_MD.write_text("\n".join(lines))
    print("\n".join(lines))
    print(f"\n[写入] {OUT_MD}")


if __name__ == "__main__":
    main()
