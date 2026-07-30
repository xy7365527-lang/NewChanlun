#!/usr/bin/env python3
"""#798: ② 丢弃样本抽样核对——父级该不该丢 + 次级别有没有留住。

抽样口径（可复现）：
- 单元 = (dump 行, 账本级 g) 且该格按 #797 判别式归 ②（c2，unconf_exact>0 优先级判别）。
- 固定序 = (文件行号升序, 账本级升序)；分层 = 窗 × 账本级。
- 每层 n：L0=15, L1=15, L2=4, L3=2（层内不足则全取）。
- 系统抽样：step = N // n，取 index 0, step, 2*step, … 共 n 个。

判定规则（机械，教义锚见报告正文）：
Q1 父级该不该丢：
  - gap == "broken"（同链更高账本级已 closed）→ 谓词有偏(候选)（第20课区间套论证），入疑义清单
  - gap == "missing" → 该丢(推断，第53/36课 prima facie)
Q2 次级别留住（被丢事件在 event_level g+1，其次级别 = 级 g）：
  - 次级别实例 = 同窗 dump 中 level==g 且 (price,anchor) 相同的候选行（含本行自身当 k==g）
  - 留住 = 存在同向实例 admit==true；没留住 = 同向实例全 admit==false
  - 无同向实例（仅异向/无实例）→ 查不到/疑义
"""
import json, collections, sys

DUMP_DIR = sys.argv[1] if len(sys.argv) > 1 else "chanlun/review-results"

def classify(ev_level, d):
    if ev_level > d["max_event_level"]: return "c4"
    if d["reg_same_price"] > 0: return "c5"
    if d["unconf_exact"] > 0: return "c2"
    if d["unconf_same_price"] > 0: return "c2p"
    if d["miss_same_price"] > 0: return "c3"
    return "c1"

QUOTA = {0: 15, 1: 15, 2: 4, 3: 2}

def judge(cell, by_lpa):
    """returns dict of Q1/Q2 verdicts + evidence"""
    r, g = cell["row"], cell["g"]
    gl = g["level"]
    # Q1
    if g["gap"] == "broken":
        q1 = "不该丢?(谓词有偏候选)"
    else:
        q1 = "该丢(推断)"
    # Q2
    subs = by_lpa.get((gl, r["price"], r["anchor"]), [])
    same_dir = [s for s in subs if s["dir"] == r["dir"]]
    adm = [s for s in same_dir if s["admit"]]
    if not subs:
        q2 = "查不到(无次级别实例)"
    elif not same_dir:
        q2 = "查不到(仅异向实例)"
    elif adm:
        q2 = "留住"
    else:
        q2 = "没留住"
    # deeper retention: any level < gl, same foot, same dir, admitted
    deep = False
    for lv in range(gl):
        for s in by_lpa.get((lv, r["price"], r["anchor"]), []):
            if s["dir"] == r["dir"] and s["admit"]:
                deep = True
    # combo
    if q1.startswith("不该丢"):
        combo = "谓词有偏(候选)"
    elif q2 == "留住":
        combo = "教义正确"
    elif q2 == "没留住":
        combo = "真丢失"
    else:
        combo = "疑义"
    subch = collections.Counter(s["channel"] for s in same_dir)
    return {
        "q1": q1, "q2": q2, "combo": combo, "deep_retained": deep,
        "n_sub": len(subs), "n_same_dir": len(same_dir), "n_adm": len(adm),
        "sub_channels": dict(subch),
        "adm_bars": sorted(s["bar"] for s in adm)[:3],
    }

def main():
    sample_rows_md = []
    summary = collections.Counter()
    summary_by = collections.defaultdict(collections.Counter)
    full = collections.Counter()
    full_by = collections.defaultdict(collections.Counter)
    doubts = []
    n_sampled = 0
    for w in ("wf7","wf8","p3fold"):
        rows = [json.loads(l) for l in open(f"{DUMP_DIR}/t5a_chain_dump_{w}-i797-20260730.jsonl")]
        by_lpa = collections.defaultdict(list)
        for i, r in enumerate(rows):
            r["_line"] = i+1
            if r["price"] is not None:
                by_lpa[(r["level"], r["price"], r["anchor"])].append(r)
        strata = collections.defaultdict(list)
        for r in rows:
            for g in r["chain"]["levels"]:
                if g["status"] != "missing_cert": continue
                if classify(g["event_level"], g["diag"]) != "c2": continue
                strata[g["level"]].append({"row": r, "g": g})
        # full-population mechanical readout
        for gl, cells in sorted(strata.items()):
            for cell in cells:
                v = judge(cell, by_lpa)
                full[v["combo"]] += 1
                full_by[(w, gl)][v["combo"]] += 1
        # stratified systematic sample
        for gl, n_want in sorted(QUOTA.items()):
            cells = strata.get(gl, [])
            if not cells: continue
            n = min(n_want, len(cells))
            step = max(1, len(cells) // n)
            picked = [cells[i*step] for i in range(n)]
            for cell in picked:
                r, g = cell["row"], cell["g"]
                v = judge(cell, by_lpa)
                n_sampled += 1
                summary[v["combo"]] += 1
                summary_by[(w, gl)][v["combo"]] += 1
                ev = f"sub@L{gl}:{v['n_same_dir']}同向/{v['n_adm']}admit"
                if v["q1"].startswith("不该丢"):
                    basis = "第20课区间套:更高级已closed而本级事件未确认"
                elif v["combo"] == "教义正确":
                    basis = f"第53课:次级别L{gl}实例admit(bar{','.join(map(str,v['adm_bars']))})"
                elif v["combo"] == "真丢失":
                    basis = f"次级别L{gl}实例全拒({';'.join(f'{k}={n}' for k,n in v['sub_channels'].items())})"
                else:
                    basis = "见疑义清单"
                sample_rows_md.append(
                    f"| {w} | {r['_line']} | {r['bar']} | k={r['level']} | L{gl} | {r['dir']} | "
                    f"{r['price']} | {r['anchor']} | {v['q1']} | {v['q2']} | {v['combo']} | "
                    f"{ev}{'；更低级有admit' if v['deep_retained'] and v['combo']=='真丢失' else ''} | {basis} |")
                if v["combo"] in ("谓词有偏(候选)", "疑义"):
                    doubts.append((w, r["_line"], r["bar"], gl, r["dir"], r["price"], r["anchor"], v))
                elif v["combo"] == "真丢失" and v["deep_retained"]:
                    doubts.append((w, r["_line"], r["bar"], gl, r["dir"], r["price"], r["anchor"],
                                   {**v, "note": "L(g)没留住但更低级有admit——按第53课字面判真丢失(次级别=紧邻下一级)，把握不足"}))
    print("=== SAMPLE TABLE (n=%d) ===" % n_sampled)
    for line in sample_rows_md: print(line)
    print("\n=== SAMPLE SUMMARY ===")
    for k, v in summary.most_common(): print(f"  {k}: {v} ({v/n_sampled:.1%})")
    print("\n=== SAMPLE BY STRATUM ===")
    for (w, gl), c in sorted(summary_by.items()):
        print(f"  {w} L{gl}: " + ", ".join(f"{k}={v}" for k,v in c.most_common()))
    tot = sum(full.values())
    print("\n=== FULL POPULATION (mechanical rule extrapolation, n=%d) ===" % tot)
    for k, v in full.most_common(): print(f"  {k}: {v} ({v/tot:.1%})")
    print("\n=== FULL BY (window, ledger) ===")
    for (w, gl), c in sorted(full_by.items()):
        s = sum(c.values())
        print(f"  {w} L{gl} (n={s}): " + ", ".join(f"{k}={v}" for k,v in c.most_common()))
    print("\n=== DOUBTS (%d) ===" % len(doubts))
    for d in doubts:
        print(" ", d)

main()
