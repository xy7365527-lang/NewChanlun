#!/usr/bin/env python3
"""#944：把 p944 探针的读数整成报告用表（只读，不改仓内任何生产件）。

用法：`python3 report_issue944.py [new|wide]`（默认 new = #813 生产默认档）

读 `../issue944-bsp-metrics-20260808.json`（p944_bsp_layer_mismatch 产出），
出六项要测的表。买卖点的挂钟落点用 `zoneinfo` 换成美东时间做时段分桶——
**这一步只在报告侧做**，探针只吐 UTC 秒，不引入第二份日历口径。
"""
import json
import os
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone

try:
    from zoneinfo import ZoneInfo
    ET = ZoneInfo("America/New_York")
except Exception:  # noqa: BLE001
    ET = None

HERE = os.path.dirname(os.path.abspath(__file__))
METRICS = os.path.normpath(os.path.join(HERE, "..", "issue944-bsp-metrics-20260808.json"))
MODE = sys.argv[1] if len(sys.argv) > 1 else "new"
LAYERS = [("bi_layer", "笔中枢买卖点层"), ("seg_layer", "线段中枢买卖点层")]


def pct(a, b):
    return f"{100.0 * a / b:.1f}%" if b else "n/a(0)"


def et_slot(ts):
    """买卖点所在 bar 的**起点**在美东挂钟上的 HH:MM。

    不做「开盘段/收盘段」的语义分桶——A 臂 bar 从 09:30 起、C 臂从整点起，两条网格
    天生错半小时，任何跨臂共用的语义桶都会把网格错位算进「时段效应」。直接报 HH:MM，
    读者自己看首末根（A 的首根 09:30、末根 15:30；regular session 09:30–16:00 ET）。
    """
    if ET is None:
        return "?"
    d = datetime.fromtimestamp(ts, tz=timezone.utc).astimezone(ET)
    return f"{d.hour:02d}:{d.minute:02d} ET"


def main():
    recs = [r for r in json.load(open(METRICS))["records"] if r["stroke_mode"] == MODE]
    if not recs:
        print(f"no records for mode={MODE}")
        return
    print(f"# #944 买卖点层失配（笔档位 = {MODE}）\n")

    # ── 端点层交叉校验（与 #940 同口径，验证本探针跑的是同一读数）──
    ea = sum(r["endpoint_xcheck"]["n_A"] for r in recs)
    ec = sum(r["endpoint_xcheck"]["n_C"] for r in recs)
    eac = sum(r["endpoint_xcheck"]["ac"] for r in recs)
    eab = sum(r["endpoint_xcheck"]["ab"] for r in recs)
    enz = sum(r["endpoint_xcheck"]["noise_mean"] for r in recs)
    print("## 0. 端点层交叉校验（#940 同口径，±2h）")
    print(f"- A 端点 {ea}、C 端点 {ec}；A→C 命中 {eac} = {pct(eac, ea)}"
          f"（#940 报 75.4%）；A→B {pct(eab, ea)}；噪声天花板 A→A′ {pct(enz, ea)}（#940 报 86.0%）\n")

    for key, name in LAYERS:
        print(f"\n## {name}（`{key}`）\n")
        nA = sum(r[key]["n_A"] for r in recs)
        nB = sum(r[key]["n_B"] for r in recs)
        nC = sum(r[key]["n_C"] for r in recs)
        acs = sum(r[key]["ac_strict"] for r in recs)
        acl = sum(r[key]["ac_loose"] for r in recs)
        abs_ = sum(r[key]["ab_strict"] for r in recs)
        print(f"### 计数：A={nA}  B={nB}  C={nC}（10 标的合计，窗口见下表）\n")
        if nA == 0:
            print("**A 臂零个买卖点 ⟹ 本层在本窗口无法给出任何对齐率读数（090 照实：没测出来，"
                  "不是「无差异」）。**\n")

        # 主表：对齐率 + 两个对照组
        tol_rows = defaultdict(lambda: [0, 0, 0, 0, 0, 0.0, 0.0])
        for r in recs:
            for t in r[key]["tol_scan"]:
                a = tol_rows[t["tol_s"]]
                a[0] += t["ac_strict"]; a[1] += t["ac_loose"]; a[2] += t["ab_strict"]
                a[3] += t["c_unclaimed"]; a[4] += t["b_unclaimed"]
                a[5] += t["null_ac_mean"]; a[6] += t["null_ab_mean"]
        noise_m = sum(r[key]["noise"]["matched_mean"] for r in recs)
        noise_n = sum(r[key]["noise"]["n_mean"] for r in recs)
        print("### 表 1：对齐率（容差扫描，严配 = 类型∧方向都同）\n")
        print("| 容差 | A→C 严配 | A→C 松配(只同方向) | A→C 零假设 | C 未被 A 认领 | A→B 严配 | A→B 零假设 |")
        print("|---|---|---|---|---|---|---|")
        for tol in sorted(tol_rows):
            a = tol_rows[tol]
            print(f"| ±{tol // 3600}h | {a[0]}/{nA} = {pct(a[0], nA)} | {pct(a[1], nA)} | "
                  f"{pct(a[5], nA)} | {a[3]}/{nC} = {pct(a[3], nC)} | {pct(a[2], nA)} | {pct(a[6], nA)} |")
        print(f"\n- **噪声天花板 A→A′**（0.051% basis 噪声，3 种子均值）："
              f"{noise_m:.1f}/{nA} = {pct(noise_m, nA)}；A′ 买卖点数均值 {noise_n:.1f}（A={nA}）")
        print(f"- 主读数（±2h）：A→C 严配 **{pct(acs, nA)}**、松配 {pct(acl, nA)}；A→B 严配 {pct(abs_, nA)}\n")

        if nA == 0 and nC == 0:
            continue

        # 按类型分
        print("### 表 2：按买卖点类型分（±2h 严配）\n")
        by_kind = defaultdict(lambda: [0, 0, 0, 0])  # A 总, A 命中, C 总, C 被认领
        for r in recs:
            for p in r[key]["A"]:
                by_kind[p["kind"]][0] += 1
                by_kind[p["kind"]][1] += 1 if p["in_C_strict"] else 0
            for p in r[key]["C"]:
                by_kind[p["kind"]][2] += 1
                by_kind[p["kind"]][3] += 1 if p["claimed_by_A_strict"] else 0
        print("| 类型 | A 侧个数 | A→C 对齐率 | C 侧个数 | C 中回测里不存在的 |")
        print("|---|---|---|---|---|")
        for k in ["type1", "type2", "type3"]:
            v = by_kind.get(k, [0, 0, 0, 0])
            print(f"| {k} | {v[0]} | {v[1]}/{v[0]} = {pct(v[1], v[0])} | {v[2]} | "
                  f"{v[2] - v[3]}/{v[2]} = {pct(v[2] - v[3], v[2])} |")
        print(f"| **合计** | {nA} | {acs}/{nA} = {pct(acs, nA)} | {nC} | "
              f"{nC - sum(1 for r in recs for p in r[key]['C'] if p['claimed_by_A_strict'])}/{nC} = "
              f"{pct(nC - sum(1 for r in recs for p in r[key]['C'] if p['claimed_by_A_strict']), nC)} |\n")

        # 按方向
        print("### 表 3：按方向分（±2h 严配）\n")
        by_side = defaultdict(lambda: [0, 0, 0, 0])
        for r in recs:
            for p in r[key]["A"]:
                by_side[p["side"]][0] += 1
                by_side[p["side"]][1] += 1 if p["in_C_strict"] else 0
            for p in r[key]["C"]:
                by_side[p["side"]][2] += 1
                by_side[p["side"]][3] += 1 if p["claimed_by_A_strict"] else 0
        print("| 方向 | A 侧个数 | A→C 对齐率 | C 侧个数 | C 中回测里不存在的 |")
        print("|---|---|---|---|---|")
        for s in ["buy", "sell"]:
            v = by_side.get(s, [0, 0, 0, 0])
            print(f"| {s} | {v[0]} | {pct(v[1], v[0])} | {v[2]} | {pct(v[2] - v[3], v[2])} |")
        print()

        # 按标的
        print("### 表 4：按标的分（±2h 严配）\n")
        print("| 标的 | 窗口(UTC 日) | A 买卖点 | A→C 对齐 | C 买卖点 | C 中回测里不存在的 |")
        print("|---|---|---|---|---|---|")
        for r in recs:
            L = r[key]
            unclaimed = sum(1 for p in L["C"] if not p["claimed_by_A_strict"])
            d0 = datetime.fromtimestamp(r["window"][0], tz=timezone.utc).strftime("%Y-%m-%d")
            d1 = datetime.fromtimestamp(r["window"][1], tz=timezone.utc).strftime("%Y-%m-%d")
            print(f"| {r['symbol']} | {d0}→{d1} | {L['n_A']} | "
                  f"{L['ac_strict']}/{L['n_A']} = {pct(L['ac_strict'], L['n_A'])} | {L['n_C']} | "
                  f"{unclaimed}/{L['n_C']} = {pct(unclaimed, L['n_C'])} |")
        print()

        # 时段分桶：全体 vs 失配
        print("### 表 5：A 侧买卖点的美东时段分布——全体 vs 失配（±2h 严配）\n")
        allc, missc = Counter(), Counter()
        for r in recs:
            for p in r[key]["A"]:
                s = et_slot(p["t0"])
                allc[s] += 1
                if not p["in_C_strict"]:
                    missc[s] += 1
        print("| 美东时段（bar 起点） | A 侧全体 | 其中失配 | 该时段失配率 |")
        print("|---|---|---|---|")
        for s in sorted(allc):
            print(f"| {s} | {allc[s]} | {missc[s]} | {pct(missc[s], allc[s])} |")
        print(f"| **合计** | {sum(allc.values())} | {sum(missc.values())} | "
              f"{pct(sum(missc.values()), sum(allc.values()))} |\n")

        # confirmed 子集（只有 confirmed 的买卖点才是可下单信号）
        ca = [p for r in recs for p in r[key]["A"] if p["confirmed"]]
        cc = [p for r in recs for p in r[key]["C"] if p["confirmed"]]
        ca_hit = sum(1 for p in ca if p["in_C_strict"])
        cc_un = sum(1 for p in cc if not p["claimed_by_A_strict"])
        print("### 表 5b：只看 `confirmed` 的买卖点（±2h 严配）\n")
        print(f"- A 侧 confirmed {len(ca)}/{nA}，其中在 C 里有对应物 {ca_hit} = {pct(ca_hit, len(ca))}")
        print(f"- C 侧 confirmed {len(cc)}/{nC}，其中 A 里没有对应物 {cc_un} = {pct(cc_un, len(cc))}")
        print("- ⚠ 匹配池仍是对方臂的**全部**买卖点（含 unconfirmed），故这是条件率不是"
              "confirmed↔confirmed 的双向限定率\n")

        # 失配是「消失」还是「改判类型」
        gone = sum(1 for r in recs for p in r[key]["A"] if not p["in_C_loose"])
        retyped = sum(1 for r in recs for p in r[key]["A"]
                      if p["in_C_loose"] and not p["in_C_strict"])
        print(f"### 表 6：A 侧失配的成分拆解（n={nA - acs}）\n")
        print(f"- **同位置同方向但类型被改判**：{retyped} 个（{pct(retyped, nA)} of A）"
              f"——实盘那个位置**有**买卖点，只是判成了别的类型")
        print(f"- **该位置在 C 里根本没有买卖点**：{gone} 个（{pct(gone, nA)} of A）"
              f"——这才是「回测开仓、实盘没有信号」的那一类\n")


if __name__ == "__main__":
    main()
