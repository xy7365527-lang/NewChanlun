#!/usr/bin/env python3
"""#224 一/三类带判同真伪深挖——复算脚本（20260724）。

数据源（均为 #218 留档，本脚本只读）：
  探针  /tmp/nest218/debug_stderr.log        （NEST218_DEBUG band_cmp/t2_cmp 逐点行）
  塔    /tmp/nest218/after/wf7/opsem/tower_events.jsonl

产出：报告 `band-judgment-truth-audit-20260724.md` §0/§1/§2 全部读数。
口径：探针账本 append-only、跨索引重建按 (事件, 点) 去重取末次（沿 flip_analyze.py）。
"""
import re, json, collections, sys

LOG = "/tmp/nest218/debug_stderr.log"
TOWER = "/tmp/nest218/after/wf7/opsem/tower_events.jsonl"

BAND_RE = re.compile(
    r"NEST218_DEBUG band_cmp ev_turn=(\d+) ev_ib=\[?(\d+)\.\.=(\d+) b_start=Some\((\d+)\) "
    r"pt_src=(\d+) pt_start=(\d+) pt_band=\((-?\d+),(-?\d+)\) b_band=Some\(\((-?\d+), (-?\d+)\)\)"
)
NC_RE = re.compile(r"L(\d+) #(\d+) zd=(\d+) zg=(\d+) si=(\d+) ei=(\d+)")
EX_RE = re.compile(r"L(\d+) #(\d+) zd=(\d+) zg=(\d+) ei (\d+)->(\d+)")

# ---- 探针解析（去重取末次）----
band_pts = {}
for line in open(LOG):
    m = BAND_RE.search(line)
    if m:
        ev_turn, ib0, ib1, b_start, pt_src, pt_start, pt_zd, pt_zg, b_zd, b_zg = map(int, m.groups())
        band_pts[(ev_turn, ib0, ib1, pt_src)] = dict(
            turn=ev_turn, ib0=ib0, ib1=ib1, b_start=b_start, pt_src=pt_src,
            pt_start=pt_start, pt_band=(pt_zd, pt_zg), b_band=(b_zd, b_zg))

old_eq = {k: v for k, v in band_pts.items() if v["pt_start"] == v["b_start"]}
new_eq = {k: v for k, v in band_pts.items() if v["pt_band"] == v["b_band"]}
coll = {k: v for k, v in new_eq.items() if v["pt_start"] != v["b_start"]}

# ---- 对账断言（NEST_GATE_FAIL 行读数）----
assert len(band_pts) == 132, len(band_pts)           # c1+c3 total = 8+124
assert len(old_eq) == 18, len(old_eq)                # c1 4 + c3 14
assert len(new_eq) == 27, len(new_eq)                # c1 4 + c3 23
assert len(coll) == 9, len(coll)                     # band_eq_start_neq
assert all(k in new_eq for k in old_eq)              # 收紧 0 件（old ⊆ new）
assert set(k for k in new_eq if k not in old_eq) == set(coll)
print(f"[对账] band=132 旧判等=18 新判等=27 碰撞=9 old⊆new ✓（全部与行读数一致）")

# ---- 塔解析 ----
centers, by_si = {}, collections.defaultdict(list)
for line in open(TOWER):
    d = json.loads(line)
    if d["kind"] == "new_center":
        lv, num, zd, zg, si, ei = map(int, NC_RE.search(d["detail"]).groups())
        centers[(lv, num)] = dict(level=lv, num=num, zd=zd, zg=zg, si=si, ei=ei, ext=0)
        by_si[si].append((lv, num))
    elif d["kind"] == "extend":
        lv, num, zd, zg, e0, e1 = map(int, EX_RE.search(d["detail"]).groups())
        c = centers[(lv, num)]
        assert c["ei"] == e0
        c["ei"], c["ext"] = e1, c["ext"] + 1

def rec(si, band):
    """si 处带匹配的记录键列表。"""
    return [k for k in by_si.get(si, []) if (centers[k]["zd"], centers[k]["zg"]) == band]

print(f"[tower] 记录={len(centers)} 跨级别同si={sum(1 for v in by_si.values() if len(v) > 1)}")

# ---- 任务一：18 旧判等逐件 ----
print("\n== 任务一：旧判等 18 点次 ==")
for k, v in sorted(old_eq.items()):
    cands = by_si[v["b_start"]]
    match = rec(v["b_start"], v["pt_band"])
    assert len(match) == 1, (k, match)  # 带匹配唯一 ⟹ 点中枢 ≡ B
    tag = "唯一记录" if len(cands) == 1 else f"碰撞面L{sorted(centers[ck]['level'] for ck in cands if ck != match[0])}未咬到"
    mk = match[0]
    print(f"  turn={v['turn']} pt_src={v['pt_src']} b_start={v['b_start']} -> "
          f"L{mk[0]}#{mk[1]} 同一中枢（{tag}）")

# ---- 任务二：9 碰撞点次逐件 ----
print("\n== 任务二：碰撞 9 点次 ==")
for k, v in sorted(coll.items()):
    cC, cB = rec(v["pt_start"], v["pt_band"]), rec(v["b_start"], v["b_band"])
    assert len(cC) == len(cB) == 1, (k, cC, cB)
    c, b = centers[cC[0]], centers[cB[0]]
    gap = c["si"] - b["ei"]
    print(f"  turn={v['turn']} pt_src={v['pt_src']}: C=L{c['level']}#{c['num']}[{c['si']}..{c['ei']}] "
          f"vs B=L{b['level']}#{b['num']}[{b['si']}..{b['ei']}] gap(B.ei→C.si)={gap} 同带不同记录")

# ---- zona 同带链与全塔同带复用 ----
for zona, band in (("A", (2826025000000, 2834083000000)), ("B", (2907707000000, 2915146000000))):
    chain = sorted([c for c in centers.values() if (c["zd"], c["zg"]) == band], key=lambda c: c["si"])
    gaps = [chain[i + 1]["si"] - chain[i]["ei"] for i in range(len(chain) - 1)]
    print(f"[zona {zona}] 同带记录 {len(chain)} 条 [{chain[0]['si']}..{chain[-1]['ei']}] 间隙={gaps}")
band_cnt = collections.Counter((k[0], c["zd"], c["zg"]) for k, c in centers.items())
print(f"[tower] 同级别同带多记录组合={sum(1 for n in band_cnt.values() if n > 1)}（最多 ×{max(band_cnt.values())}）")
print("\n全部断言通过。")
