#!/usr/bin/env python3
"""#940：把 Rust 探针的 metrics JSON 汇成报告用的 Markdown 表（只读，不做任何判定）。

用法：`python3 report_issue940.py [new|wide]`（缺省 new）
"""
import datetime
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
M = os.path.normpath(os.path.join(HERE, "..", "issue940-metrics-20260807.json"))
MODE = sys.argv[1] if len(sys.argv) > 1 else "new"


def d(ts):
    return datetime.datetime.fromtimestamp(ts, datetime.UTC).strftime("%Y-%m-%d")


def main():
    recs = [r for r in json.load(open(M))["records"] if r["stroke_mode"] == MODE]

    print(f"### 表 1：三套输入的结构元素计数（笔档位 = {MODE}）\n")
    print("| 标的 | 窗口(UTC) | 交易日 | bar A/B/C | 笔 A/B/C | 线段 A/B/C | "
          "笔中枢 A/B/C | 线段中枢 A/B/C | L1走势 A/B/C |")
    print("|---|---|---|---|---|---|---|---|---|")
    tot = {k: [0, 0, 0] for k in ("n_bars", "n_strokes", "n_segments",
                                  "n_bi_centers", "n_seg_centers", "n_moves_l1")}
    for r in recs:
        a, b, c = r["A"], r["B"], r["C"]
        for k in tot:
            for i, x in enumerate((a, b, c)):
                tot[k][i] += x[k]
        print(f"| {r['symbol']} | {d(r['window'][0])}→{d(r['window'][1])} | {r['n_sessions']} | "
              f"{a['n_bars']}/{b['n_bars']}/{c['n_bars']} | "
              f"{a['n_strokes']}/{b['n_strokes']}/{c['n_strokes']} | "
              f"{a['n_segments']}/{b['n_segments']}/{c['n_segments']} | "
              f"{a['n_bi_centers']}/{b['n_bi_centers']}/{c['n_bi_centers']} | "
              f"{a['n_seg_centers']}/{b['n_seg_centers']}/{c['n_seg_centers']} | "
              f"{a['n_moves_l1']}/{b['n_moves_l1']}/{c['n_moves_l1']} |")
    print(f"| **合计(10 标的)** | — | — | "
          + " | ".join("/".join(str(v) for v in tot[k]) for k in
                       ("n_bars", "n_strokes", "n_segments", "n_bi_centers",
                        "n_seg_centers", "n_moves_l1")) + " |")
    print()
    for k, name in (("n_strokes", "笔"), ("n_segments", "线段"),
                    ("n_bi_centers", "笔中枢"), ("n_seg_centers", "线段中枢")):
        a, b, c = tot[k]
        print(f"- **{name}**：B/A = {b / a:.2f}×，C/A = {c / a:.2f}×"
              f"（绝对数 A={a}，B={b}，C={c}）")
    a, b, c = tot["n_bars"]
    print(f"- **bar 数本身**：B/A = {b / a:.2f}×，C/A = {c / a:.2f}×")

    print(f"\n### 表 2：笔端点对齐率（容差扫描，笔档位 = {MODE}）\n")
    print("| 容差 | A→B 命中 | A→B 命中率 | A→B 零假设对照 | B 端点未被 A 认领 | "
          "A→C 命中率 | A→C 零假设对照 | C 端点未被 A 认领 |")
    print("|---|---|---|---|---|---|---|---|")
    tols = [t["tol_s"] for t in recs[0]["align"]]
    for i, tol in enumerate(tols):
        ab = sum(r["align"][i]["ab_matched"] for r in recs)
        ac = sum(r["align"][i]["ac_matched"] for r in recs)
        nab = sum(r["align"][i]["null_ab_mean"] for r in recs)
        nac = sum(r["align"][i]["null_ac_mean"] for r in recs)
        ub = sum(r["align"][i]["b_unmatched"] for r in recs)
        uc = sum(r["align"][i]["c_unmatched"] for r in recs)
        ea = sum(r["A"]["n_endpoints"] for r in recs)
        eb = sum(r["B"]["n_endpoints"] for r in recs)
        ec = sum(r["C"]["n_endpoints"] for r in recs)
        print(f"| ±{tol // 3600}h | {ab}/{ea} | {100 * ab / ea:.1f}% | {100 * nab / ea:.1f}% | "
              f"{ub}/{eb} = {100 * ub / eb:.1f}% | {100 * ac / ea:.1f}% | "
              f"{100 * nac / ea:.1f}% | {uc}/{ec} = {100 * uc / ec:.1f}% |")

    print(f"\n### 表 3：B 相对 A 多出来的笔端点落在哪里（±2h 口径，笔档位 = {MODE}）\n")
    print("| 标的 | 多出端点数 | 开市段 | 隔夜段 | 周末/整日休市 | 开市段占比 |")
    print("|---|---|---|---|---|---|")
    ts_, to_, tw_ = 0, 0, 0
    for r in recs:
        e = r["extra_B_endpoints_by_class"]
        s, o, w = e.get("session", 0), e.get("overnight", 0), e.get("weekend", 0)
        ts_, to_, tw_ = ts_ + s, to_ + o, tw_ + w
        n = s + o + w
        print(f"| {r['symbol']} | {n} | {s} | {o} | {w} | {100 * s / n:.1f}% |")
    n = ts_ + to_ + tw_
    print(f"| **合计** | {n} | {ts_} | {to_} | {tw_} | **{100 * ts_ / n:.1f}%** |")

    print(f"\n### 表 4：线段中枢的对应关系与递归塔（笔档位 = {MODE}）\n")
    print("| 标的 | 线段中枢 A/B/C | A 的中枢在 B 里找得到 | A 的中枢在 C 里找得到 | "
          "B 多出的中枢 | 递归塔顶 A/B/C |")
    print("|---|---|---|---|---|---|")
    for r in recs:
        m = r["seg_center_match"]
        def top(arm):
            t = r[arm]["tower"]
            hi = [k for k, v in t.items() if v[0] > 0]
            return hi[-1] if hi else "L1"
        print(f"| {r['symbol']} | {m['A_total']}/{m['B_total']}/{m['C_total']} | "
              f"{m['AB_matched']}/{m['A_total']} | {m['AC_matched']}/{m['A_total']} | "
              f"{m['B_total'] - m['AB_matched']} | {top('A')}/{top('B')}/{top('C')} |")
    ta = sum(r["seg_center_match"]["A_total"] for r in recs)
    tb = sum(r["seg_center_match"]["B_total"] for r in recs)
    tc = sum(r["seg_center_match"]["C_total"] for r in recs)
    mab = sum(r["seg_center_match"]["AB_matched"] for r in recs)
    mac = sum(r["seg_center_match"]["AC_matched"] for r in recs)
    print(f"| **合计** | {ta}/{tb}/{tc} | {mab}/{ta} = {100 * mab / ta:.0f}% | "
          f"{mac}/{ta} = {100 * mac / ta:.0f}% | {tb - mab} | — |")


    print(f"\n### 表 5：价格噪声对照臂 A′（同一批 bar、同一条网格，只加 0.051% 价格噪声；"
          f"笔档位 = {MODE}）\n")
    print("| 标的 | A 笔数 | A′ 笔数(3 种子均值) | A→A′ 端点命中率 | 参照：A→C | 参照：A→B |")
    print("|---|---|---|---|---|---|")
    i = [t["tol_s"] for t in recs[0]["align"]].index(7200)
    for r in recs:
        ea = r["A"]["n_endpoints"]
        print(f"| {r['symbol']} | {r['A']['n_strokes']} | {r['noise_ctrl']['strokes_mean']:.1f} | "
              f"{100 * r['noise_ctrl']['matched_mean'] / ea:.1f}% | "
              f"{100 * r['align'][i]['ac_matched'] / ea:.1f}% | "
              f"{100 * r['align'][i]['ab_matched'] / ea:.1f}% |")
    ea = sum(r["A"]["n_endpoints"] for r in recs)
    print(f"| **合计** | {sum(r['A']['n_strokes'] for r in recs)} | "
          f"{sum(r['noise_ctrl']['strokes_mean'] for r in recs):.1f} | "
          f"**{100 * sum(r['noise_ctrl']['matched_mean'] for r in recs) / ea:.1f}%** | "
          f"{100 * sum(r['align'][i]['ac_matched'] for r in recs) / ea:.1f}% | "
          f"{100 * sum(r['align'][i]['ab_matched'] for r in recs) / ea:.1f}% |")


if __name__ == "__main__":
    main()
