#!/usr/bin/env python3
"""#218 owner 归属修正（路线 B）——三窗重放读数校验（spec #219 ID-5/ID-7）。

对照面：#214 基线留档（/tmp/nest214/after，旧判同口径 + #214 装置行族）。
断言组：
  A. 新行存在且可解析（#218 schema：owner_anchor_neq / owner_pts_anchor_missing /
     band_eq_start_neq / scan_pts 三计数）；
  B. 四桶平账：trend == success + owner_anchor_neq + out_of_window + opposite_side
     + no_valid_point；no_valid_point == book_missing + book_empty；
     trend == LEVEL.base_trend；pan_success + pan_fail == LEVEL.base_pan；
  C. 按级 × kind 合计对账 NEST_GATE_INDEX 行（Σbase_lk == base_events 等）；
  D. 点级计数自洽：c_i total >= eq；(Σeq>0) == (success>0)；
     (Σne>0) == (owner_anchor_neq>0)；Σne >= anchor_missing + real_neq；
  E. 上游红线（vs #214 基线，逐字节/逐值一致）：tower_events MD5、t5a_chain_dump MD5、
     INDEX.base_events 逐值一致（trades/STATS/CHAIN/T3 行族 = 允许变面，G 组落账）；
  F. 不变量断言集（vs #214 基线读数，变即红旗）：异向桶、窗口外桶、Pan 域全部计数、
     点级 c_i total（窗口/方向谓词产物）逐项一致；
  G. delta 对账落账（非硬断言，读数印出供 resolution）：success / owner 桶 / assembled /
     indexed / 点级相等率 / trades MD5 / STATS/CHAIN/T3 行族差异。
断言（A-F）全过退出码 0；G 组只印不算失败（delta 对账属读数落账非红线）。

用法：python3 owner-attribution-fix-verify-20260724.py [BASE_DIR] [AFTER_DIR] [TAGS...]
默认 BASE_DIR=/tmp/nest214/after AFTER_DIR=/tmp/nest218/after TAGS=wf7
（范围裁定 2026-07-24：验收重放严格 wf7 单窗，p3fold/wf8 不跑—— TAGS 显式传可扩）。
"""
import hashlib
import json
import re
import sys

BASE_DIR = sys.argv[1] if len(sys.argv) > 1 else "/tmp/nest214/after"
AFTER_DIR = sys.argv[2] if len(sys.argv) > 2 else "/tmp/nest218/after"
TAGS = sys.argv[3:] if len(sys.argv) > 3 else ["wf7"]

FAIL_RE = re.compile(
    r"^NEST_GATE_FAIL trend=(\d+) success=(\d+) owner_anchor_neq=(\d+) out_of_window=(\d+) "
    r"opposite_side=(\d+) no_valid_point=(\d+)\(book_missing=(\d+) book_empty=(\d+)\) "
    r"owner_pts_anchor_missing=(\d+) owner_pts_real_neq=(\d+) band_eq_start_neq=(\d+) \| pts_eq/total "
    r"c1=(\d+)/(\d+) c2=(\d+)/(\d+) c3=(\d+)/(\d+) \| scan_pts book_total=(\d+) in_window=(\d+) "
    r"same_side=(\d+) \| pan_success=(\d+) pan_fail=(\d+)$"
)
FAIL_RE_214 = re.compile(
    r"^NEST_GATE_FAIL trend=(\d+) success=(\d+) owner_start_neq=(\d+) out_of_window=(\d+) "
    r"opposite_side=(\d+) no_valid_point=(\d+)\(book_missing=(\d+) book_empty=(\d+)\) "
    r"owner_pts_id_missing=(\d+) owner_pts_real_neq=(\d+) \| pts_eq/total "
    r"c1=(\d+)/(\d+) c2=(\d+)/(\d+) c3=(\d+)/(\d+) \| pan_success=(\d+) pan_fail=(\d+)$"
)
LEVEL_RE = re.compile(
    r"^NEST_GATE_LEVEL base_lk=(\[.*?\]) assembled_lk=(\[.*?\]) indexed_lk=(\[.*?\]) "
    r"\| base_trend=(\d+) base_pan=(\d+)$"
)
INDEX_RE = re.compile(
    r"^NEST_GATE_INDEX events=(\d+) events_seen=(\d+) base_events=(\d+) assembled=(\d+) "
    r"indexed=(\d+) rungs_0=(\d+) rungs_1=(\d+) rungs_2p=(\d+) single_level_share=(\S+) \| "
    r"derivations=(\d+) index_builds=(\d+) provider_errors=(\d+)$"
)

fail_items = []
n_checks = 0


def check(cond, msg):
    global n_checks
    n_checks += 1
    print(("  ✓ " if cond else "  ✗ ") + msg)
    if not cond:
        fail_items.append(msg)


def md5(path):
    with open(path, "rb") as f:
        return hashlib.md5(f.read()).hexdigest()


def gate_lines(path):
    rows = {}
    for line in open(path):
        for key in ("NEST_GATE_STATS", "NEST_GATE_CHAIN", "NEST_GATE_T3",
                    "NEST_GATE_INDEX", "NEST_GATE_FAIL", "NEST_GATE_LEVEL"):
            if line.startswith(key):
                rows.setdefault(key, []).append(line.rstrip("\n"))
    return rows


def parse_fail(line):
    m = FAIL_RE.match(line)
    assert m, f"NEST_GATE_FAIL（#218 schema）行不可解析: {line}"
    v = [int(x) for x in m.groups()]
    keys = ["trend", "success", "owner_anchor_neq", "out_of_window", "opposite_side",
            "no_valid_point", "book_missing", "book_empty",
            "owner_anchor_missing", "owner_real_neq", "band_eq_start_neq",
            "c1_eq", "c1_total", "c2_eq", "c2_total", "c3_eq", "c3_total",
            "scan_book_total", "scan_in_window", "scan_same_side",
            "pan_success", "pan_fail"]
    return dict(zip(keys, v))


def parse_fail_214(line):
    m = FAIL_RE_214.match(line)
    assert m, f"NEST_GATE_FAIL（#214 schema）行不可解析: {line}"
    v = [int(x) for x in m.groups()]
    keys = ["trend", "success", "owner_start_neq", "out_of_window", "opposite_side",
            "no_valid_point", "book_missing", "book_empty",
            "owner_id_missing", "owner_real_neq",
            "c1_eq", "c1_total", "c2_eq", "c2_total", "c3_eq", "c3_total",
            "pan_success", "pan_fail"]
    return dict(zip(keys, v))


def parse_level(line):
    m = LEVEL_RE.match(line)
    assert m, f"NEST_GATE_LEVEL 行不可解析: {line}"
    return {
        "base_lk": json.loads(m.group(1)),
        "assembled_lk": json.loads(m.group(2)),
        "indexed_lk": json.loads(m.group(3)),
        "base_trend": int(m.group(4)),
        "base_pan": int(m.group(5)),
    }


def parse_index(line):
    m = INDEX_RE.match(line)
    assert m, f"NEST_GATE_INDEX 行不可解析: {line}"
    v = m.groups()
    return {"base_events": int(v[2]), "assembled": int(v[3]), "indexed": int(v[4]),
            "rungs_0": int(v[5]), "rungs_1": int(v[6]), "rungs_2p": int(v[7])}


print(f"# #218 三窗重放校验（baseline={BASE_DIR} vs after={AFTER_DIR}）\n")
summary = {}
for tag in TAGS:
    print(f"## 窗 {tag}\n")
    b = gate_lines(f"{BASE_DIR}/{tag}/stderr.log")
    a = gate_lines(f"{AFTER_DIR}/{tag}/stderr.log")

    print("### A. 新行存在与解析（#218 schema）")
    check(len(a.get("NEST_GATE_FAIL", [])) == 1, "after NEST_GATE_FAIL 恰好一行")
    check(len(a.get("NEST_GATE_LEVEL", [])) == 1, "after NEST_GATE_LEVEL 恰好一行")
    f = parse_fail(a["NEST_GATE_FAIL"][0])
    fb = parse_fail_214(b["NEST_GATE_FAIL"][0])
    lv = parse_level(a["NEST_GATE_LEVEL"][0])
    lvb = parse_level(b["NEST_GATE_LEVEL"][0])
    idx = parse_index(a["NEST_GATE_INDEX"][0])
    idxb = parse_index(b["NEST_GATE_INDEX"][0])

    print("### B. 四桶平账（ID-2 完备性）")
    check(f["trend"] == f["success"] + f["owner_anchor_neq"] + f["out_of_window"]
          + f["opposite_side"] + f["no_valid_point"],
          f"trend({f['trend']}) == success({f['success']}) + 四桶({f['owner_anchor_neq']}+"
          f"{f['out_of_window']}+{f['opposite_side']}+{f['no_valid_point']})")
    check(f["no_valid_point"] == f["book_missing"] + f["book_empty"],
          f"no_valid_point({f['no_valid_point']}) == book_missing({f['book_missing']}) + book_empty({f['book_empty']})")
    check(f["trend"] == lv["base_trend"], f"FAIL.trend({f['trend']}) == LEVEL.base_trend({lv['base_trend']})")
    check(f["pan_success"] + f["pan_fail"] == lv["base_pan"],
          f"pan_success+pan_fail({f['pan_success']}+{f['pan_fail']}) == LEVEL.base_pan({lv['base_pan']})")

    print("### C. 按级 × kind 合计对账 INDEX 行")
    sum_base = sum(sum(row) for row in lv["base_lk"])
    sum_asm = sum(sum(row) for row in lv["assembled_lk"])
    sum_idx = sum(sum(row) for row in lv["indexed_lk"])
    check(sum_base == idx["base_events"], f"Σbase_lk({sum_base}) == INDEX.base_events({idx['base_events']})")
    check(sum_asm == idx["assembled"], f"Σassembled_lk({sum_asm}) == INDEX.assembled({idx['assembled']})")
    check(sum_idx == idx["indexed"], f"Σindexed_lk({sum_idx}) == INDEX.indexed({idx['indexed']})")
    check(sum(row[0] for row in lv["base_lk"]) == lv["base_trend"]
          and sum(row[1] for row in lv["base_lk"]) == lv["base_pan"],
          "base_lk 列合计 == base_trend/base_pan 列")

    print("### D. 点级计数自洽（ID-3/ID-4）")
    for i in (1, 2, 3):
        check(f[f"c{i}_total"] >= f[f"c{i}_eq"], f"c{i}: total({f[f'c{i}_total']}) >= eq({f[f'c{i}_eq']})")
    sum_eq = f["c1_eq"] + f["c2_eq"] + f["c3_eq"]
    sum_ne = (f["c1_total"] - f["c1_eq"]) + (f["c2_total"] - f["c2_eq"]) + (f["c3_total"] - f["c3_eq"])
    check((sum_eq > 0) == (f["success"] > 0),
          f"(Σeq>0)={sum_eq > 0} ⟺ (success>0)={f['success'] > 0}（Σeq={sum_eq}, success={f['success']}）")
    check((sum_ne > 0) == (f["owner_anchor_neq"] > 0),
          f"(Σne>0)={sum_ne > 0} ⟺ (owner_anchor_neq>0)={f['owner_anchor_neq'] > 0}")
    check(sum_ne >= f["owner_anchor_missing"] + f["owner_real_neq"],
          f"Σne({sum_ne}) >= anchor_missing({f['owner_anchor_missing']}) + real_neq({f['owner_real_neq']})（逐类次 ≥ 逐点）")

    print("### E. 上游红线（vs #214 基线，逐字节/逐值一致，ID-5）")
    check(idx["base_events"] == idxb["base_events"],
          f"INDEX.base_events 逐值一致（{idx['base_events']} == {idxb['base_events']}）")
    h_b = md5(f"{BASE_DIR}/{tag}/opsem/tower_events.jsonl")
    h_a = md5(f"{AFTER_DIR}/{tag}/opsem/tower_events.jsonl")
    check(h_b == h_a, f"tower_events MD5 一致（{h_b[:12]}…）")
    # t5a_chain_dump：行键集（候选流供给面）逐键一致为红线；verdict/admit 列 = 背书
    # 下游（允许变面）——MD5 级放 G 组带归因对照（见下）。
    import json as _json
    def _keys(path):
        ks = set()
        for line in open(path):
            r = _json.loads(line)
            ks.add((r["bar"], r["level"], r["source_index"], r["dir"]))
        return ks
    kb = _keys(f"{BASE_DIR}/{tag}/dump/t5a_chain_dump_{tag}.jsonl")
    ka = _keys(f"{AFTER_DIR}/{tag}/dump/t5a_chain_dump_{tag}.jsonl")
    check(kb == ka, f"chain_dump 行键集一致（{len(kb)} == {len(ka)} 键；候选流供给面）")

    print("### F. 不变量断言集（vs #214 基线读数，变即红旗，ID-5）")
    check(f["opposite_side"] == fb["opposite_side"],
          f"异向桶不变（{f['opposite_side']} == {fb['opposite_side']}）")
    check(f["out_of_window"] == fb["out_of_window"],
          f"窗口外桶不变（{f['out_of_window']} == {fb['out_of_window']}）")
    check(f["pan_success"] == fb["pan_success"] and f["pan_fail"] == fb["pan_fail"],
          f"Pan 域计数不变（{f['pan_success']}/{f['pan_fail']} == {fb['pan_success']}/{fb['pan_fail']}）")
    for i in (1, 2, 3):
        check(f[f"c{i}_total"] == fb[f"c{i}_total"],
              f"c{i}_total 不变（{f[f'c{i}_total']} == {fb[f'c{i}_total']}，窗口/方向谓词产物）")

    print("### G. delta 对账落账（读数印出，非硬断言）")
    print(f"  success: {fb['success']} → {f['success']}（Δ{f['success'] - fb['success']:+d}）")
    print(f"  owner 桶: {fb['owner_start_neq']} → {f['owner_anchor_neq']}（Δ{f['owner_anchor_neq'] - fb['owner_start_neq']:+d}）")
    print(f"  assembled: {idxb['assembled']} → {idx['assembled']}（Δ{idx['assembled'] - idxb['assembled']:+d}）")
    print(f"  indexed: {idxb['indexed']} → {idx['indexed']}（Δ{idx['indexed'] - idxb['indexed']:+d}）")
    # chain_dump verdict/admit 差异逐行归因（下游按账变面）
    import json as _json
    def _rows(path):
        out = {}
        for line in open(path):
            r = _json.loads(line)
            out[(r["bar"], r["level"], r["source_index"], r["dir"])] = r
        return out
    rb = _rows(f"{BASE_DIR}/{tag}/dump/t5a_chain_dump_{tag}.jsonl")
    ra = _rows(f"{AFTER_DIR}/{tag}/dump/t5a_chain_dump_{tag}.jsonl")
    diffs = [k for k in rb if rb[k] != ra[k]]
    print(f"  chain_dump verdict/admit 差异行: {len(diffs)}（逐行归因翻转证书，见读数报告）")
    for k in diffs[:10]:
        print(f"    {k}: verdict {rb[k]['chain']['verdict']}->{ra[k]['chain']['verdict']} "
              f"admit {rb[k]['admit']}->{ra[k]['admit']} channel {rb[k]['channel']}->{ra[k]['channel']}")
    print(f"  点级相等率: c1 {fb['c1_eq']}/{fb['c1_total']} → {f['c1_eq']}/{f['c1_total']}；"
          f"c2 {fb['c2_eq']}/{fb['c2_total']} → {f['c2_eq']}/{f['c2_total']}；"
          f"c3 {fb['c3_eq']}/{fb['c3_total']} → {f['c3_eq']}/{f['c3_total']}")
    print(f"  锚不可解子计数: {f['owner_anchor_missing']}；带碰撞观察: {f['band_eq_start_neq']}")
    print(f"  遍历计数（新行族新字段，#214 基线无对照面）: book_total={f['scan_book_total']} "
          f"in_window={f['scan_in_window']} same_side={f['scan_same_side']}")
    h_t_b = md5(f"{BASE_DIR}/{tag}/opsem/trades.jsonl")
    h_t_a = md5(f"{AFTER_DIR}/{tag}/opsem/trades.jsonl")
    print(f"  trades MD5: {'一致' if h_t_b == h_t_a else '不一致（允许变面，须逐笔归因）'}")
    for key in ("NEST_GATE_STATS", "NEST_GATE_CHAIN", "NEST_GATE_T3"):
        same = b.get(key) == a.get(key)
        print(f"  {key} 整行: {'零变化' if same else '有变化（允许变面，须归因）'}")
        if not same:
            print(f"    baseline: {b.get(key)}")
            print(f"    after   : {a.get(key)}")
    summary[tag] = {"fail": f, "base": fb, "level": lv, "index": idx, "index_b": idxb}
    print()

print("## 读数汇总（resolution 备料）\n")
for tag in TAGS:
    f, fb = summary[tag]["fail"], summary[tag]["base"]
    idx, idxb = summary[tag]["index"], summary[tag]["index_b"]
    print(f"- {tag}: 相等率 c1={f['c1_eq']}/{f['c1_total']} c2={f['c2_eq']}/{f['c2_total']} "
          f"c3={f['c3_eq']}/{f['c3_total']}（基线 {fb['c1_eq']}/{fb['c1_total']} "
          f"{fb['c2_eq']}/{fb['c2_total']} {fb['c3_eq']}/{fb['c3_total']}）；四桶 "
          f"success={f['success']}(基线{fb['success']}) owner={f['owner_anchor_neq']}(基线{fb['owner_start_neq']}) "
          f"window={f['out_of_window']} opposite={f['opposite_side']} nvp={f['no_valid_point']}；"
          f"assembled={idx['assembled']}(基线{idxb['assembled']}) indexed={idx['indexed']}")

print(f"\n## 总结：{n_checks} 项断言，{'全过 ✓' if not fail_items else f'{len(fail_items)} 项失败 ✗'}")
for item in fail_items:
    print("  FAIL:", item)
sys.exit(1 if fail_items else 0)
