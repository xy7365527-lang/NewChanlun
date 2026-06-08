"""比价关系的 PH 改造 —— 对 MOS/SPY 比值序列做持续同调分析。

存在论位置（§17 形态学层扩展到 K4 比价边）
-------------------------------------------
K4 框架的六条边是**比价关系**（相对强弱）。本脚本把 §17.1 的形态学层 PH 引擎
（H0 级别/中枢、online merge tree alive/settled、H1 相空间循环）施加到**比价序列**
ratio = MOS_close / SPY_close 上。

操盘意义
--------
- **比价中枢 [zd, zg]** = MOS 相对 SPY（大盘）的"均衡价格区间"——比价在此区间往返
  = 相对强弱无明确方向。
- **比价趋势（≥2 中枢）/ alive 下跌分量** = MOS 相对大盘的持续走弱/走强。
- **比价 H1 loop** = 相对强弱的周期性往返（相空间循环）。
- **比价振幅衰减**（持续同调 persistence 递减）= 相对强弱摆动幅度收窄。
  **诚实标注（521号）**：这是比价的**形态学振幅**信号，**非动力学力度背驰**。

认识论等级（formalization-validity-domain，逐条标注）
---------------------------------------------------
- PH 算法（H0/H1/级别/中枢）：**L0**（确定性）。
- "比价中枢=相对均衡区间""H1 loop=相对强弱往返"：单对(MOS/SPY)实测为 **L2**
  （可否证）；"=相对强弱"的语义断言在多标的验证前停留 **L1**。
- 级别↔周期映射：**L2**。

边界条件
--------
- 比价信号随 MOS 或 SPY 任一创新极值而改写（与 §10 alive→settled 同构）。
- H1 loop 对 Takens embedding_delay 敏感——脚本试多个 delay，仅报告稳定出现的 loop。

直接运行：``.venv/bin/python scripts/mos_ratio_ph.py``
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_level_detection import detect_levels, detect_levels_adaptive  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_persistence_barcode import rips_h1_bars, sublevel_h0_bars  # noqa: E402
from newchan.a_ph_zhongshu import detect_zhongshu  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"


def load_ratio() -> dict:
    """读缓存的 MOS/SPY 日线，按日期对齐算 ratio = MOS/SPY。"""
    mos = json.loads((CACHE / "MOS_1d_1y.json").read_text())
    spy = json.loads((CACHE / "SPY_1d_1y.json").read_text())
    if mos["dates"] != spy["dates"]:
        # 取交集对齐（防御性；本数据已同期）
        common = [d for d in mos["dates"] if d in set(spy["dates"])]
        mi = {d: i for i, d in enumerate(mos["dates"])}
        si = {d: i for i, d in enumerate(spy["dates"])}
        dates = common
        ratio = [mos["closes"][mi[d]] / spy["closes"][si[d]] for d in dates]
    else:
        dates = mos["dates"]
        ratio = [m / s for m, s in zip(mos["closes"], spy["closes"])]
    return {"dates": dates, "ratio": ratio,
            "mos_last": mos["closes"][-1], "spy_last": spy["closes"][-1]}


def analyze() -> dict:
    d = load_ratio()
    ratio, dates = d["ratio"], d["dates"]
    n = len(ratio)
    rmin, rmax = min(ratio), max(ratio)
    result: dict = {
        "n": n, "first_date": dates[0], "last_date": dates[-1],
        "ratio_first": ratio[0], "ratio_last": ratio[-1],
        "ratio_min": rmin, "ratio_max": rmax,
        "mos_last": d["mos_last"], "spy_last": d["spy_last"],
    }

    # --- 比价噪声阈值（比价无 high/low，用 ratio 自身的日间真实波动代理）---
    diffs = [abs(ratio[i] - ratio[i - 1]) for i in range(1, n)]
    noise = (sum(diffs) / len(diffs)) if diffs else 0.0  # 平均日间变动作 τ 量纲基准
    result["noise_tau"] = noise

    # --- 1) H0 sublevel persistence（批量，验证）+ online merge tree（因果）---
    h0_batch = sublevel_h0_bars(ratio)
    tree = OnlineMergeTree()
    for r in ratio:
        tree.update(r)
    snap = tree.current_barcode()          # 因果快照
    final = tree.finalize()                # 完整结构（含全局）
    settled = final.settled_bars
    result["h0"] = {
        "n_bars_batch": len(h0_batch),
        "n_settled_final": len(settled),
        "n_alive_live": len(snap.alive_bars),
        "dominant_persistence": settled[0].persistence if settled else 0.0,
        "level_L0_note": "H0 算法 L0",
    }

    # --- 2) 级别识别（吃 MergeBar）---
    lvl = detect_levels(settled, noise_floor=noise)
    lvl_ad = detect_levels_adaptive(settled, noise_floor=noise)
    result["levels"] = {
        "n_levels_recursive": len(lvl.levels),
        "leaves": [lv.period_label for lv in lvl.leaves],
        "n_levels_adaptive": len(lvl_ad.levels),
        "adaptive_labels": [
            {"label": lv.period_label,
             "persistence_range": [round(x, 6) for x in lv.persistence_range],
             "n_bars": lv.n_bars}
            for lv in lvl_ad.levels
        ],
        "epistemic": "log-gap L0；级别↔周期 L2",
    }

    # --- 3) 中枢计数（比价中枢=相对均衡区间）---
    zss = detect_zhongshu(settled, noise_floor=noise)
    result["zhongshu"] = {
        "count": len(zss),
        "list": [
            {"period": z.period_label, "zd": round(z.zd, 6), "zg": round(z.zg, 6),
             "width": round(z.width, 6), "n_members": z.n_members,
             "span": z.span, "lo": z.lo, "hi": z.hi,
             "date_range": [dates[z.lo], dates[z.hi]]}
            for z in zss
        ],
        "interpretation": "比价中枢[zd,zg]=MOS相对SPY的均衡价格区间（L2）",
    }

    # --- 4) online merge tree 当前 alive（比价是否还在创相对新低/高）---
    alive_sorted = sorted(snap.alive_bars, key=lambda b: b.persistence, reverse=True)
    dom = alive_sorted[0] if alive_sorted else None
    result["online"] = {
        "n_alive": len(snap.alive_bars),
        "dominant_alive": None if dom is None else {
            "birth_ratio": round(dom.birth_price, 6),
            "birth_date": dates[dom.birth_idx],
            "persistence_est": round(dom.persistence, 6),
        },
        "current_ratio": round(ratio[-1], 6),
        "is_near_min": abs(ratio[-1] - rmin) < noise,
        "interpretation": "主导 alive 比价下跌分量未 settle → MOS 相对大盘走弱未完成（L2）",
    }

    # --- 5) H1 Rips 周期检测（多 delay，仅报稳定 loop）---
    h1_by_delay = {}
    for delay in (1, 2, 3, 5):
        bars = rips_h1_bars(ratio, embedding_dim=3, embedding_delay=delay)
        h1_by_delay[delay] = {
            "n_loops": len(bars),
            "max_persistence": round(bars[0].persistence, 6) if bars else 0.0,
            "top3": [round(b.persistence, 6) for b in bars[:3]],
        }
    # 稳定性：在多数 delay 下都出现显著 loop（max_persistence > noise）才算稳定
    significant = sum(1 for v in h1_by_delay.values() if v["max_persistence"] > noise)
    result["h1"] = {
        "by_delay": h1_by_delay,
        "n_delays_with_significant_loop": significant,
        "stable_cycle": significant >= 3,
        "interpretation": (
            "H1 loop=比价相对强弱的相空间循环（往返）。"
            "稳定 loop（多 delay 一致）才报为周期结构；否则如实报无显著循环（L1）"
        ),
    }

    # --- 6) 比价背驰（形态学振幅，非力度背驰——521号）---
    finite = sorted((b for b in settled if b.death_idx is not None),
                    key=lambda b: b.death_idx)
    if len(finite) >= 2:
        prev_p, recent_p = finite[-2].persistence, finite[-1].persistence
        decay = max(0.0, 1.0 - recent_p / prev_p) if prev_p > 0 else 0.0
        div = {"recent_swing_persistence": round(recent_p, 6),
               "prev_swing_persistence": round(prev_p, 6),
               "amplitude_decay": round(decay, 4)}
    else:
        div = {"amplitude_decay": 0.0, "note": "settled 摆动 <2，无法评估"}
    div["tag"] = "形态学层比价振幅信号，**非**MACD力度背驰（521号/ker(D)）"
    result["divergence"] = div

    result["epistemic_summary"] = (
        "PH算法L0；MOS/SPY单对比价实测L2；'H1=相对强弱往返'语义停留L1（多标的未验证）"
    )
    return result


def main() -> None:
    r = analyze()
    print("=" * 70)
    print(f"MOS/SPY 比价 PH 分析  ({r['first_date']} → {r['last_date']}, n={r['n']})")
    print("=" * 70)
    print(f"ratio: 起 {r['ratio_first']:.5f} → 终 {r['ratio_last']:.5f}  "
          f"[{r['ratio_min']:.5f}, {r['ratio_max']:.5f}]  "
          f"(MOS={r['mos_last']:.2f}, SPY={r['spy_last']:.2f})")
    pct = (r['ratio_last'] / r['ratio_first'] - 1) * 100
    print(f"比价全程 {pct:+.1f}%  → MOS 相对大盘{'大幅走弱' if pct < -10 else '走弱' if pct<0 else '走强'}")
    print(f"噪声阈 τ(平均日间变动)={r['noise_tau']:.6f}")

    print(f"\n[1] H0: settled={r['h0']['n_settled_final']} 主导persistence={r['h0']['dominant_persistence']:.5f} "
          f"当前alive={r['h0']['n_alive_live']}")
    print(f"[2] 级别(自适应): {r['levels']['n_levels_adaptive']} 级 → "
          f"{[l['label'] for l in r['levels']['adaptive_labels']]}")
    print(f"[3] 比价中枢: {r['zhongshu']['count']} 个")
    for z in r['zhongshu']['list']:
        print(f"      [{z['zd']:.5f}, {z['zg']:.5f}] {z['period']} "
              f"({z['date_range'][0]}→{z['date_range'][1]}, {z['n_members']}成员)")
    o = r['online']
    print(f"[4] online alive: {o['n_alive']} 个; "
          f"主导={o['dominant_alive']}; 当前比价{'≈历史最低(相对最弱)' if o['is_near_min'] else '未达最低'}")
    print(f"[5] H1 周期(多delay): 稳定循环={r['h1']['stable_cycle']} "
          f"(显著delay数={r['h1']['n_delays_with_significant_loop']}/4)")
    for dl, v in r['h1']['by_delay'].items():
        print(f"      delay={dl}: {v['n_loops']} loops, max_pers={v['max_persistence']:.5f}")
    print(f"[6] 比价振幅衰减: {r['divergence'].get('amplitude_decay')} "
          f"[{r['divergence']['tag']}]")
    print(f"\n认识论: {r['epistemic_summary']}")

    out = CACHE / "s3_ratio_result.json"
    out.write_text(json.dumps(r, indent=2, ensure_ascii=False))
    print(f"\n已存 {out.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
