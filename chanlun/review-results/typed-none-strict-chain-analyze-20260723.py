#!/usr/bin/env python3
"""T3（#172，严格链判定）阶段 B：shadow 双读 dump 全量分析 + 四条件终判（#164 同构三件套·分析件）。

090 纪律：全量测量（非抽样）；确证/未确证分项；不用统计估计；任何与阶段 A 回报不符的
数字以本脚本对 dump 的重算为准并在报告中注明。

输入（全部只读）：
  /tmp/t3_shadow_dump_{p3fold,wf7,wf8}.jsonl   阶段 A 逐候选双读 dump（after=链时代判定 + base=并集时代重构列）
  /tmp/t3_baseline/<tag>/stderr.log            改前 NEST_GATE_* 聚合行
  /tmp/t3_after/<tag>/stderr.log               改后 NEST_GATE_* + NEST_GATE_T3 聚合行
  /tmp/t3_baseline/<tag>/opsem/{trades,tower_events}.jsonl / /tmp/t3_after/... 同名
  <worktree>/chanlun/review-results/typed-none-joint-dump-candidates-20260722.jsonl  #164 wf7 逐候选 dump

输出：
  stdout = 报告全部数字（按节编号）。
  <worktree>/chanlun/review-results/typed-none-strict-chain-dump-candidates-20260723.jsonl
    = 三窗 dump 合并（4875 行），每行加 "window" 与 "migration"（§3 归因标签）两字段，原列不动。
"""
import json, re, hashlib, collections, sys, os

WS = "/tmp/kimi-nest-mainline/chanlun/review-results"
TAGS = ["p3fold", "wf7", "wf8"]
XZD = ("xzd_pass", "xzd_gate_fail", "cert_none")
D164 = f"{WS}/typed-none-joint-dump-candidates-20260722.jsonl"
OUT_JSONL = f"{WS}/typed-none-strict-chain-dump-candidates-20260723.jsonl"

def load(p):
    with open(p) as f:
        return [json.loads(l) for l in f]

def md5(p):
    return hashlib.md5(open(p, "rb").read()).hexdigest()

def gate_lines(path):
    out = {}
    for line in open(path):
        for k in ("NEST_GATE_STATS", "NEST_GATE_CHAIN", "NEST_GATE_INDEX", "NEST_GATE_T3"):
            if line.startswith(k):
                out[k] = line.strip()
    return out

def kv(line):
    return {m.group(1): m.group(2) for m in re.finditer(r"([A-Za-z_0-9]+)=([^\s|]+)", line or "")}

DUMP = {t: load(f"/tmp/t3_shadow_dump_{t}.jsonl") for t in TAGS}
D164_ROWS = load(D164)
BASE = {t: gate_lines(f"/tmp/t3_baseline/{t}/stderr.log") for t in TAGS}
AFTER = {t: gate_lines(f"/tmp/t3_after/{t}/stderr.log") for t in TAGS}
BS = {t: kv(BASE[t].get("NEST_GATE_STATS")) for t in TAGS}
AS = {t: kv(AFTER[t].get("NEST_GATE_STATS")) for t in TAGS}
BC = {t: kv(BASE[t].get("NEST_GATE_CHAIN")) for t in TAGS}
AC = {t: kv(AFTER[t].get("NEST_GATE_CHAIN")) for t in TAGS}
T3 = {t: AFTER[t].get("NEST_GATE_T3", "") for t in TAGS}

OK = []   # (检查项, 是否通过, 备注) —— 汇总打印
def check(name, cond, note=""):
    OK.append((name, bool(cond), note))
    return cond

key = lambda r: (r["bar"], r["level"], r["source_index"], r["dir"])

# ════════════════ §1 corpus 同一性 ════════════════
print("══ §1 corpus 同一性 ══")
w7 = DUMP["wf7"]
same_keys = [key(a) == key(b) for a, b in zip(w7, D164_ROWS)]
check("wf7↔#164 行数 1724=1724", len(w7) == len(D164_ROWS) == 1724, f"{len(w7)} vs {len(D164_ROWS)}")
check("wf7↔#164 候选键有序逐键一致", all(same_keys) and len(same_keys) == 1724,
      f"不一致 {same_keys.count(False)} 处")
for t in TAGS:
    check(f"{t} dump 行数 == STATS.total（base/after）",
          len(DUMP[t]) == int(BS[t]["total"]) == int(AS[t]["total"]),
          f"{len(DUMP[t])} vs {BS[t]['total']} vs {AS[t]['total']}")
    h1, h2 = md5(f"/tmp/t3_baseline/{t}/opsem/tower_events.jsonl"), md5(f"/tmp/t3_after/{t}/opsem/tower_events.jsonl")
    check(f"{t} tower_events base/after 逐字节一致", h1 == h2, h1[:12])
h164 = md5("/tmp/v4_C/wf7/tower_events.jsonl") if os.path.exists("/tmp/v4_C/wf7/tower_events.jsonl") else None
print(f"  tower MD5: p3fold={md5('/tmp/t3_baseline/p3fold/opsem/tower_events.jsonl')[:12]} "
      f"wf7={md5('/tmp/t3_baseline/wf7/opsem/tower_events.jsonl')[:12]}(#164源={h164[:12] if h164 else 'N/A'}) "
      f"wf8={md5('/tmp/t3_baseline/wf8/opsem/tower_events.jsonl')[:12]}")

# 键四元组 (bar,level,source_index,dir) 非唯一：L0 同脚双发候选（两候选 bits 不同，dump 键不含 bits）。
# 因此一切 T3↔#164 对照按**有序逐行（位置）**对齐（行序两文件一致已核 ⟺ 同一候选流行序）。
DUPN = {}
for t in TAGS:
    kc = collections.Counter(key(r) for r in DUMP[t])
    dups = {k: v for k, v in kc.items() if v > 1}
    DUPN[t] = len(dups)
    lv = collections.Counter(k[1] for k in dups)
    print(f"  {t}: 相异键 {len(kc)}/{len(DUMP[t])}；重复键对 {len(dups)}（候选级构成 {dict(lv)}，均 2 发）")
sg_mis = sum(1 for i in range(1724) if (w7[i]["single"] is True) != bool(D164_ROWS[i]["single_found"]))
check("wf7↔#164 single 桥读出逐位一致（1724 行，corpus 同一性强证据）", sg_mis == 0, f"不一致 {sg_mis}")
# 重复对内部一致性（corpus 特征落账：同脚双发两候选 bits 不同，可各自落到不同 Xzd 子通道——并集时代即如此）
for t in TAGS:
    posm = collections.defaultdict(list)
    for i, r in enumerate(DUMP[t]):
        posm[key(r)].append(r)
    pairs = [v for v in posm.values() if len(v) > 1]
    sp = sum(1 for rs in pairs if len({r["price"] for r in rs}) == 1)
    sv = sum(1 for rs in pairs if len({r["chain"]["verdict"] for r in rs}) == 1)
    sc = sum(1 for rs in pairs if len({r["channel"] for r in rs}) == 1)
    sc_base = sum(1 for rs in pairs if len({r["base"]["channel"] for r in rs}) == 1)
    diff_ok = all(all(r["base"]["channel"] == r["channel"] for r in rs) and len({r["base"]["channel"] for r in rs}) > 1
                  for rs in pairs if len({r["channel"] for r in rs}) > 1)
    check(f"{t} 通道异重复对均为 unchanged 行且 base 已异（bits 差异，并集时代已存在）", diff_ok)
    print(f"  {t}: 重复对 {len(pairs)}：同价 {sp} / 同裁决 {sv} / 同通道 {sc}（其中 base 同通道 {sc_base}；"
          f"通道异对均为 unchanged 行——bits 差异并集时代已存在，非 T3 迁移）")

# dump 聚合 ↔ stderr 聚合（base 重构列保真度 + after 一致性）
print("\n══ §1b dump 聚合 ↔ stderr 聚合对照 ══")
for t in TAGS:
    ch = collections.Counter(r["channel"] for r in DUMP[t])
    bc = collections.Counter(r["base"]["channel"] for r in DUMP[t])
    ad = sum(1 for r in DUMP[t] if r["admit"]); bad = sum(1 for r in DUMP[t] if r["base"]["admit"])
    check(f"{t} after 通道计数 == after STATS",
          ch.get("nest_pass", 0) == int(AS[t]["nest_pass"]) and
          ch.get("xzd_pass", 0) == int(AS[t]["xzd_pass"]) and
          ch.get("xzd_gate_fail", 0) == int(AS[t]["xzd_gate_fail"]) and
          ch.get("cert_none", 0) == int(AS[t]["cert_none"]) and
          ch.get("nest_n_delta_false", 0) == int(AS[t]["nest_n_delta_false"]) and
          ad == int(AS[t]["admitted"]) and len(DUMP[t]) - ad == int(AS[t]["rejected"]),
          f"{dict(ch)} admitted={ad}")
    check(f"{t} base 重构通道计数 == baseline STATS",
          bc.get("nest_pass", 0) == int(BS[t]["nest_pass"]) and
          bc.get("xzd_pass", 0) == int(BS[t]["xzd_pass"]) and
          bc.get("xzd_gate_fail", 0) == int(BS[t]["xzd_gate_fail"]) and
          bc.get("cert_none", 0) == int(BS[t]["cert_none"]) and
          bc.get("nest_n_delta_false", 0) == int(BS[t]["nest_n_delta_false"]) and
          bad == int(BS[t]["admitted"]),
          f"{dict(bc)} admitted={bad}")
    # 并集时代 reuse/fallback 可由 dump 列重构（multi None=merged_pass null；旧臂通道）
    re_reuse = sum(1 for r in DUMP[t] if r["union"]["merged_pass"] is None and r["old_arm"]["channel"] in ("xzd_pass", "xzd_gate_fail"))
    re_fallback = sum(1 for r in DUMP[t] if r["union"]["merged_pass"] is None and r["old_arm"]["channel"] in ("nest_pass", "nest_n_delta_false"))
    check(f"{t} 并集时代 reuse/fallback 由 dump 重构 == baseline CHAIN",
          re_reuse == int(BC[t]["reuse"]) and re_fallback == int(BC[t]["xzd_fallback"]),
          f"reuse {re_reuse} vs {BC[t]['reuse']}；fallback {re_fallback} vs {BC[t]['xzd_fallback']}")
    re_reuse_a = sum(1 for r in DUMP[t] if r["xzd"]["reused"]); re_fa = sum(1 for r in DUMP[t] if r["xzd"]["fallback"])
    check(f"{t} after reuse/fallback == after CHAIN",
          re_reuse_a == int(AC[t]["reuse"]) and re_fa == int(AC[t]["xzd_fallback"]),
          f"reuse {re_reuse_a} vs {AC[t]['reuse']}；fallback {re_fa} vs {AC[t]['xzd_fallback']}")
    check(f"{t} union.merged_pass=True 数 == typed_found", 
          sum(1 for r in DUMP[t] if r["union"]["merged_pass"] is True) == int(AC[t]["typed_found"]) == int(BC[t]["typed_found"]),
          f"{sum(1 for r in DUMP[t] if r['union']['merged_pass'] is True)} vs {AC[t]['typed_found']}")

# ════════════════ 条件① 差异归因全量复核 ════════════════
print("\n══ 条件① 逐候选迁移归因（全量，非抽样）══")
def classify(r):
    """独立重实现的归因分类（与 ADR 四分类对齐；谱系差按机制拆两亚型）。"""
    b, n = r["base"]["channel"], r["channel"]
    v = r["chain"]["verdict"]
    fg = (r["chain"].get("first_gap") or {}).get("kind")
    if b == n:
        return ("unchanged", r["admit"] == r["base"]["admit"])
    if b == "nest_pass" and n == "nest_n_delta_false" and v == "reject" and fg == "missing":
        return ("缺环拒", True)
    if b == "nest_pass" and n == "nest_n_delta_false" and v == "reject" and fg == "broken":
        return ("断环拒", True)
    if b in XZD and n == "nest_pass" and v == "pass":
        return ("换锚恢复", True)
    if b == "nest_pass" and n in XZD and v == "no_chain":
        return ("谱系差·链下NoChain", True)
    if b in XZD and n == "nest_n_delta_false" and v == "reject":
        return ("谱系差·链拒自回退出", True)
    if b in XZD and n in XZD:
        return ("★Xzd子通道漂移", False)
    return ("★未分类", False)

MIG = {}
UNEXPLAINED = []
for t in TAGS:
    cnt = collections.Counter(); bad = []
    for r in DUMP[t]:
        lab, coherent = classify(r)
        r["migration"] = lab          # 写入合并 jsonl 用
        cnt[lab] += 1
        if not coherent or lab.startswith("★"):
            bad.append((lab, key(r), r["base"]["channel"], r["channel"], r["chain"]["verdict"]))
    MIG[t] = cnt
    UNEXPLAINED += [(t, b) for b in bad]
    print(f"  {t}: {dict(cnt.most_common())}")
    check(f"{t} 迁移 100% 归因零未解释", not bad, f"异常 {len(bad)} 例")
if UNEXPLAINED:
    print("  ★未解释清单:", UNEXPLAINED[:50])

# 迁移动合计对账（缺/断环拒 + 谱系差 == 变化候选总数；与 STATS Δ 对账）
for t in TAGS:
    moved = len(DUMP[t]) - MIG[t]["unchanged"]
    delta_admit = int(AS[t]["admitted"]) - int(BS[t]["admitted"])
    d_rows = sum((1 if r["admit"] else 0) - (1 if r["base"]["admit"] else 0) for r in DUMP[t])
    check(f"{t} admit 差逐候选平账 == admitted Δ", d_rows == delta_admit, f"{d_rows} vs {delta_admit}")
    print(f"  {t}: 迁移 {moved} 候选；候选 admit 差合计 {d_rows} == admitted Δ={delta_admit}（逐候选平账）；"
          f"nest_pass {BS[t]['nest_pass']}→{AS[t]['nest_pass']}")

# ════════════════ 判定↔通道结构一致性 + NEST_GATE_T3 行对账 ════════════════
print("\n══ 链裁决↔通道结构一致性（逐候选恒等式）══")
for t in TAGS:
    rows = DUMP[t]
    c1 = all((r["channel"] == "nest_pass" and r["admit"]) == (r["chain"]["verdict"] == "pass") for r in rows)
    c2 = all((r["channel"] == "nest_n_delta_false") == (r["chain"]["verdict"] == "reject") for r in rows)
    c3 = all((r["channel"] in XZD) == (r["chain"]["verdict"] == "no_chain") for r in rows)
    check(f"{t} pass⟺nest_pass ∧ reject⟺nest_n_delta_false ∧ no_chain⟺xzd/cert", c1 and c2 and c3)
    # pass ⟹ closed_down_to=0；reject-missing ⟹ None；reject-broken ⟹ Some(k>0)；no_chain ⟹ None
    e1 = all(r["chain"]["closed_down_to"] == 0 for r in rows if r["chain"]["verdict"] == "pass")
    e2 = all(r["chain"]["closed_down_to"] is None for r in rows if r["chain"]["verdict"] == "no_chain")
    e3 = all((r["chain"]["closed_down_to"] is None) == ((r["chain"].get("first_gap") or {}).get("kind") == "missing")
             for r in rows if r["chain"]["verdict"] == "reject")
    e4 = all(len(r["chain"]["levels"]) == r["chain"]["top"] + 1 for r in rows if r["chain"]["top"] is not None)
    e5 = all(r["chain"]["first_gap"] is None for r in rows if r["chain"]["verdict"] == "pass")
    check(f"{t} 闭合恒等式（pass⟹cdt0；nochain⟹None；rej: cdt None⟺missing；len=top+1；pass 无 gap）",
          e1 and e2 and e3 and e4 and e5)
    # NEST_GATE_T3 行对账
    m = re.search(r"chain_pass=(\d+) chain_reject=(\d+)\(missing=(\d+) broken=(\d+)\) chain_none=(\d+)", T3[t])
    vd = collections.Counter(r["chain"]["verdict"] for r in rows)
    gaps = collections.Counter((r["chain"].get("first_gap") or {}).get("kind") for r in rows if r["chain"]["verdict"] == "reject")
    wit = sum(1 for r in rows if any(g["dir_witness"] != "agree" for g in r["chain"]["levels"]))
    mt = re.search(r"top_dist=\{([^}]*)\}", T3[t])
    td_line = {int(k): int(v) for k, v in re.findall(r"(\d+): (\d+)", mt.group(1))} if mt else {}
    td_dump = collections.Counter(r["chain"]["top"] for r in rows if r["chain"]["top"] is not None)
    mw = re.search(r"dir_witness_divergence=(\d+)", T3[t])
    check(f"{t} NEST_GATE_T3 行 == dump 重算（三态/拒首因/top_dist/方向见证）",
          vd["pass"] == int(m.group(1)) and vd["reject"] == int(m.group(2)) and
          gaps["missing"] == int(m.group(3)) and gaps["broken"] == int(m.group(4)) and
          vd["no_chain"] == int(m.group(5)) and dict(td_dump) == td_line and wit == int(mw.group(1)),
          f"vd={dict(vd)} gaps={dict(gaps)} wit={wit}")
    # single=false（并集时代 n_delta 拒）零发生；旧臂对照 cross old_rej_new_pass=0 逐候选版
    sf = sum(1 for r in rows if r["single"] is False)
    xpass = sum(1 for r in rows if r["chain"]["verdict"] == "pass" and r["old_arm"]["channel"] != "nest_pass")
    check(f"{t} single=false 零发生（{sf}）∧ 链pass⊆旧臂nest_pass（越界 {xpass}）", sf == 0 and xpass == 0)

# ════════════════ 条件② Xzd 子通道迁移分解 ════════════════
print("\n══ 条件② Xzd 子通道迁移分解（机制逐字节不变 + 人口迁移 100% 归因）══")
for t in TAGS:
    flow = collections.defaultdict(collections.Counter)
    for r in DUMP[t]:
        b, n = r["base"]["channel"], r["channel"]
        if b != n:
            flow[b][n] += 1
    print(f"  {t}: " + "; ".join(f"{b}→{n}:{c}" for b, ns in sorted(flow.items()) for n, c in sorted(ns.items())))
    for sub in XZD:
        d = collections.Counter(r["channel"] for r in DUMP[t]).get(sub, 0) - \
            collections.Counter(r["base"]["channel"] for r in DUMP[t]).get(sub, 0)
        inn = sum(flow[b][sub] for b in flow if b != sub); out = sum(flow[sub].values())
        print(f"    {sub}: Δ={d:+d}（迁入 {inn} / 迁出 {out}，逐候选可归因）")
    intra = sum(1 for r in DUMP[t] if r["base"]["channel"] in XZD and r["channel"] in XZD and r["base"]["channel"] != r["channel"])
    check(f"{t} Xzd 族内子通道零漂移（机制不变直接证据）", intra == 0, f"{intra} 例")

# ════════════════ §2 链×并集裁决矩阵 + 按候选级分层 ════════════════
print("\n══ §2 链×并集四象限裁决矩阵 ══")
QUAD = {}
for t in TAGS:
    q = collections.Counter()
    for r in DUMP[t]:
        u = "union_pass" if r["union"]["merged_pass"] is True else "union_none"
        q[(r["chain"]["verdict"], u)] += 1
    QUAD[t] = q
    tot = len(DUMP[t])
    print(f"  {t} (n={tot}):")
    for v in ("pass", "reject", "no_chain"):
        print(f"    链{v:9s} × 并集pass {q[(v,'union_pass')]:5d} | × 并集none {q[(v,'union_none')]:5d}")
    check(f"{t} 换锚恢复象限（pass×union_none）== 0", q[("pass", "union_none")] == 0,
          f"{q[('pass','union_none')]}")
print("\n  按候选级分层（链 pass/reject/no_chain × 并集 pass）：")
for t in TAGS:
    print(f"  {t}: 级 | n | 链pass | 链reject | 链nochain | 并集pass | 链pass∩并集pass")
    for cl in sorted({r["level"] for r in DUMP[t]}):
        rows = [r for r in DUMP[t] if r["level"] == cl]
        vd = collections.Counter(r["chain"]["verdict"] for r in rows)
        up = sum(1 for r in rows if r["union"]["merged_pass"] is True)
        both = sum(1 for r in rows if r["chain"]["verdict"] == "pass" and r["union"]["merged_pass"] is True)
        print(f"    L{cl} | {len(rows):5d} | {vd['pass']:4d} | {vd['reject']:4d} | {vd['no_chain']:5d} | {up:4d} | {both:4d}")

# ════════════════ §4 链谱系分布（「链即身份」首次实证）══
print("\n══ §4 链谱系分布 ══")
for t in TAGS:
    rows = DUMP[t]
    print(f"  {t}:")
    for v in ("pass", "reject", "no_chain"):
        sub = [r for r in rows if r["chain"]["verdict"] == v]
        td = collections.Counter(r["chain"]["top"] for r in sub)
        cdt = collections.Counter(str(r["chain"]["closed_down_to"]) for r in sub)
        print(f"    {v:9s} n={len(sub):5d} top={dict(sorted(td.items(), key=lambda x: (x[0] is None, x[0])))} closed_down_to={dict(sorted(cdt.items()))}")
    rejs = [r for r in rows if r["chain"]["verdict"] == "reject"]
    gl = collections.Counter((r["chain"].get("first_gap") or {}).get("level") for r in rejs)
    nc = collections.Counter(sum(1 for g in r["chain"]["levels"] if g["status"] == "closed") for r in rejs)
    print(f"    reject 首 gap 级别分布={dict(sorted(gl.items()))}；reject 闭合级数分布={dict(sorted(nc.items()))}")
    st = collections.Counter(g["status"] for r in rows for g in r["chain"]["levels"])
    print(f"    谱系级底质计数（条目 {sum(st.values())}）={dict(st.most_common())}；broken 底质 {st.get('broken', 0)} 例")

# ════════════════ §5 与 #164 三层对照（wf7，同 corpus 正式结算面）══
print("\n══ §5 与 #164 三层对照（wf7）══")
none164_rows = [not c["single_found"] for c in D164_ROWS]   # 逐位（重复键见 §1）
check("#164 typed_none 行数 n=1651", sum(none164_rows) == 1651, f"{sum(none164_rows)}")
w7_pass_idx = [i for i, r in enumerate(w7) if r["chain"]["verdict"] == "pass"]
pass_in_none = [i for i in w7_pass_idx if none164_rows[i]]
print(f"  严格链确认 {len(w7_pass_idx)}/1724 = {len(w7_pass_idx)/1724*100:.2f}%（全门候选口径）")
print(f"  落在 #164 typed_none（逐位对读，n=1651）：{len(pass_in_none)} → {len(pass_in_none)/1651*100:.2f}%（与 #164 1.03% 同分母口径）")
print(f"  #164 三层：代理上界 423/1651=25.62% ｜ 并集实恢 17/1651=1.03% ｜ 严格链：typed_none 内新增恢复 0；"
      f"链确认总量 39/1724=2.26%")
print(f"  方向守卫③：链确认 {len(w7_pass_idx)} vs 并集基线 17 → {len(w7_pass_idx)/17:.2f}×（>1，未低于基线）")
check("③ 方向守卫 wf7 39 > 17", len(w7_pass_idx) > 17, f"{len(w7_pass_idx)/17:.4f}×")
print(f"  换锚恢复（链pass∩并集none）= {QUAD['wf7'][('pass','union_none')]}；链pass⊆并集pass："
      f"{QUAD['wf7'][('pass','union_pass')]}/{len(w7_pass_idx)}")
# 人口分解（阶段 B 新数）：并集 90 = single 73 + multi-only 17；链确认全部来自 single 人口
print("  ── 并集 nest_pass 人口 × 链裁决分解（链确认 vs #164-17 的人口关系，只呈现不裁定）──")
for t in TAGS:
    rows = DUMP[t]
    up = [r for r in rows if r["union"]["merged_pass"] is True]
    sg = [r for r in up if r["single"] is True]
    mo = [r for r in up if r["single"] is None]
    vd_sg = collections.Counter(r["chain"]["verdict"] for r in sg)
    vd_mo = collections.Counter(r["chain"]["verdict"] for r in mo)
    ps = sum(1 for r in rows if r["chain"]["verdict"] == "pass")
    ps_sg = sum(1 for r in sg if r["chain"]["verdict"] == "pass")
    print(f"  {t}: 并集 {len(up)} = single {len(sg)} + multi-only {len(mo)}；链pass {ps}（其中 single 人口 {ps_sg} / multi-only {ps-ps_sg}）")
    print(f"      single 人口链下：{dict(vd_sg)}｜multi-only 人口链下：{dict(vd_mo)}")
    if t == "wf7":
        check("wf7 并集=single73+multi17 且 链pass39⊆single 人口",
              len(up) == 90 and len(sg) == 73 and len(mo) == 17 and ps == 39 and ps_sg == 39)
print(f"  并集时代 nest_pass=90 在链下：仍确认 {QUAD['wf7'][('pass','union_pass')]}；缺环拒 {MIG['wf7']['缺环拒']}；"
      f"断环拒 {MIG['wf7']['断环拒']}；链下NoChain {MIG['wf7']['谱系差·链下NoChain']}（合计 {QUAD['wf7'][('pass','union_pass')]+MIG['wf7']['缺环拒']+MIG['wf7']['断环拒']+MIG['wf7']['谱系差·链下NoChain']}）")

# ── 真 nowhere 100（#164 §4.3 平铺口径）链下状态复核：同法重算（#107 初始区间代理，#164 脚本逐字口径）──
init = collections.defaultdict(list)
with open("/tmp/t3_baseline/wf7/opsem/tower_events.jsonl") as f:  # MD5 与 #164 源 /tmp/v4_C/wf7 相同（§1 已核）
    for line in f:
        e = json.loads(line)
        if e["kind"] == "level_upgrade":
            continue
        parts = e["detail"].split()
        if e["kind"] == "new_center":
            init[e["level"]].append((int(parts[4][3:]), int(parts[5][3:])))
LEVELS = sorted(init)
cov = lambda lvl, q: any(si <= q <= ei for si, ei in init[lvl])
nw_idx = [i for i in range(1724) if none164_rows[i] and not any(cov(l, D164_ROWS[i]["source_index"]) for l in LEVELS)]
check("真 nowhere 集重算 n=100（#164 §4.3 平铺口径，逐位）", len(nw_idx) == 100, f"{len(nw_idx)}")
nw_rows = [w7[i] for i in nw_idx]   # 逐位对齐（§1 已核两文件行序一致）
vd_nw = collections.Counter(r["chain"]["verdict"] for r in nw_rows)
ch_nw = collections.Counter(r["channel"] for r in nw_rows)
top_nw = collections.Counter(str(r["chain"]["top"]) for r in nw_rows)
print(f"  真 nowhere 100 链下：裁决={dict(vd_nw)}；通道={dict(ch_nw.most_common())}；top={dict(top_nw)}")
nw_pass = [(i, key(w7[i])) for i in nw_idx if w7[i]["chain"]["verdict"] == "pass"]
check("nowhere 100 链下零确认（全 no_chain）", vd_nw.get("pass", 0) == 0 and vd_nw.get("reject", 0) == 0,
      dict(vd_nw))
if nw_pass:
    print(f"  ★ nowhere 中链确认 {len(nw_pass)} 例（键：{nw_pass[:10]}）——并集代理之外的真恢复？留档待裁")

# ════════════════ §5.5 方向见证分歧 cohort（只呈现不裁定）══
print("\n══ §5.5 方向见证分歧 cohort ══")
for t in TAGS:
    rows = DUMP[t]
    divid = {id(r) for r in rows if any(g["dir_witness"] != "agree" for g in r["chain"]["levels"])}
    div = [r for r in rows if id(r) in divid]
    agr = [r for r in rows if id(r) not in divid]
    vd = collections.Counter(r["chain"]["verdict"] for r in div)
    va = collections.Counter(r["chain"]["verdict"] for r in agr)
    ty = collections.Counter()
    for r in div:
        kinds = {g["dir_witness"] for g in r["chain"]["levels"] if g["dir_witness"] != "agree"}
        ty["+".join(sorted(kinds))] += 1
    pr_d = vd["pass"] / len(div) * 100 if div else 0
    pr_a = va["pass"] / len(agr) * 100 if agr else 0
    print(f"  {t}: 分歧候选 {len(div)}/{len(rows)}（{len(div)/len(rows)*100:.1f}%）类型={dict(ty.most_common())}")
    print(f"    分歧内裁决={dict(vd)}（确认率 {pr_d:.2f}%）｜ 非分歧裁决={dict(va)}（确认率 {pr_a:.2f}%）")
    fg = collections.Counter((r["chain"].get("first_gap") or {}).get("kind") for r in div if r["chain"]["verdict"] == "reject")
    print(f"    分歧内 reject 首因={dict(fg)}；分歧级占其缺口级比例：", end="")
    num = den = 0
    for r in div:
        for g in r["chain"]["levels"]:
            if g["status"] != "closed":
                den += 1
                if g["dir_witness"] != "agree":
                    num += 1
    print(f"{num}/{den} = {(num/den*100) if den else 0:.1f}%")

# ════════════════ trades 前缀逐字节 + exit 侧 ════════════════
print("\n══ trades / exit 侧 ══")
for t in TAGS:
    b = open(f"/tmp/t3_baseline/{t}/opsem/trades.jsonl", "rb").read()
    a = open(f"/tmp/t3_after/{t}/opsem/trades.jsonl", "rb").read()
    bl, al = b.splitlines(), a.splitlines()
    pre = 0
    for x, y in zip(bl, al):
        if x != y:
            break
        pre += 1
    print(f"  {t}: trades {len(bl)}→{len(al)} 笔；前缀逐字节一致 {pre} 笔")
    check(f"{t} trades 前缀逐字节一致（阶段 A 口径 8/8/57）",
          (t, pre) in (("p3fold", 8), ("wf7", 8), ("wf8", 57)), f"prefix={pre}")
n_exit = sum(1 for t in TAGS for f in (f"/tmp/t3_baseline/{t}/stderr.log", f"/tmp/t3_after/{t}/stderr.log")
             for l in open(f) if "NEST_GATE_EXIT" in l)
print(f"  NEST_GATE_EXIT 行（四日志合计）= {n_exit}（exit 侧 m8 路径无聚合行；exit 时序证据=trades 前缀+tower MD5）")

# ════════════════ 合并 jsonl（交付件 2）══
with open(OUT_JSONL, "w") as f:
    for t in TAGS:
        for r in DUMP[t]:
            r2 = {"window": t, **r}   # migration 字段已在条件①写入
            f.write(json.dumps(r2, ensure_ascii=False, separators=(",", ":")) + "\n")
n_out = sum(1 for _ in open(OUT_JSONL))
check("合并 jsonl 行数 == 4875", n_out == 4875, f"{n_out}")
print(f"\n合并 jsonl 已写：{OUT_JSONL}（{n_out} 行，含 window/migration 字段）")

# ════════════════ 总检 ════════════════
print("\n══ 检查项总检 ══")
fails = [x for x in OK if not x[1]]
for name, ok, note in OK:
    print(f"  {'✓' if ok else '✗✗✗'} {name}  {note if not ok else ''}")
print(f"\n合计 {len(OK)} 项检查，失败 {len(fails)} 项")
sys.exit(1 if fails else 0)
