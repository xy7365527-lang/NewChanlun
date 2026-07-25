#!/usr/bin/env python3
"""#214 背书失败原因测量装置——三窗重放读数校验（spec ID-7 / US-07）。

解析 stderr 新行（NEST_GATE_FAIL / NEST_GATE_LEVEL），断言：
  A. 新行存在且可解析（每窗各恰好一行）；
  B. 四桶平账：trend == success + owner_start_neq + out_of_window + opposite_side
     + no_valid_point；no_valid_point == book_missing + book_empty；
     trend == LEVEL.base_trend；pan_success + pan_fail == LEVEL.base_pan；
  C. 按级 × kind 合计对账 NEST_GATE_INDEX 行：Σbase_lk == base_events、
     Σassembled_lk == assembled、Σindexed_lk == indexed；
  D. 点级计数自洽：c_i total >= eq；(Σeq>0) == (success>0)；(Σne>0) == (owner_start_neq>0)；
     Σne >= owner_pts_id_missing + owner_pts_real_neq（逐点 ≤ 逐类次，多位点放宽方向）；
  E. 红线对照（baseline vs after 逐字节一致）：NEST_GATE_STATS/CHAIN/T3/INDEX 整行、
     tower_events.jsonl / trades.jsonl / t5a_chain_dump_<tag>.jsonl 字节（MD5）；
     baseline 无 NEST_GATE_FAIL/LEVEL 行（纯增量新行）；
断言全过退出码 0（T3 60 项 / T5a 114 项先例）。

用法：python3 endorsement-failure-instrument-verify-20260724.py [BASE_DIR] [AFTER_DIR]
默认 BASE_DIR=/tmp/nest214/baseline AFTER_DIR=/tmp/nest214/after。
"""
import hashlib
import json
import re
import sys

BASE_DIR = sys.argv[1] if len(sys.argv) > 1 else "/tmp/nest214/baseline"
AFTER_DIR = sys.argv[2] if len(sys.argv) > 2 else "/tmp/nest214/after"
TAGS = ["p3fold", "wf7", "wf8"]

FAIL_RE = re.compile(
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
    assert m, f"NEST_GATE_FAIL 行不可解析: {line}"
    v = [int(x) for x in m.groups()]
    keys = ["trend", "success", "owner_start_neq", "out_of_window", "opposite_side",
            "no_valid_point", "book_missing", "book_empty",
            "owner_pts_id_missing", "owner_pts_real_neq",
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
    return {"base_events": int(v[2]), "assembled": int(v[3]), "indexed": int(v[4])}


print(f"# #214 三窗重放校验（baseline={BASE_DIR} vs after={AFTER_DIR}）\n")
summary = {}
for tag in TAGS:
    print(f"## 窗 {tag}\n")
    b = gate_lines(f"{BASE_DIR}/{tag}/stderr.log")
    a = gate_lines(f"{AFTER_DIR}/{tag}/stderr.log")

    print("### A. 新行存在与解析")
    check(len(a.get("NEST_GATE_FAIL", [])) == 1, "after NEST_GATE_FAIL 恰好一行")
    check(len(a.get("NEST_GATE_LEVEL", [])) == 1, "after NEST_GATE_LEVEL 恰好一行")
    check("NEST_GATE_FAIL" not in b and "NEST_GATE_LEVEL" not in b,
          "baseline 无新行（纯增量，ID-5）")
    f = parse_fail(a["NEST_GATE_FAIL"][0])
    lv = parse_level(a["NEST_GATE_LEVEL"][0])
    idx = parse_index(a["NEST_GATE_INDEX"][0])

    print("### B. 四桶平账（ID-2 完备性）")
    check(f["trend"] == f["success"] + f["owner_start_neq"] + f["out_of_window"]
          + f["opposite_side"] + f["no_valid_point"],
          f"trend({f['trend']}) == success({f['success']}) + 四桶({f['owner_start_neq']}+"
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

    print("### D. 点级计数自洽（ID-3）")
    for i in (1, 2, 3):
        check(f[f"c{i}_total"] >= f[f"c{i}_eq"], f"c{i}: total({f[f'c{i}_total']}) >= eq({f[f'c{i}_eq']})")
    sum_eq = f["c1_eq"] + f["c2_eq"] + f["c3_eq"]
    sum_ne = (f["c1_total"] - f["c1_eq"]) + (f["c2_total"] - f["c2_eq"]) + (f["c3_total"] - f["c3_eq"])
    check((sum_eq > 0) == (f["success"] > 0),
          f"(Σeq>0)={sum_eq > 0} ⟺ (success>0)={f['success'] > 0}（Σeq={sum_eq}, success={f['success']}）")
    check((sum_ne > 0) == (f["owner_start_neq"] > 0),
          f"(Σne>0)={sum_ne > 0} ⟺ (owner_start_neq>0)={f['owner_start_neq'] > 0}")
    check(sum_ne >= f["owner_pts_id_missing"] + f["owner_pts_real_neq"],
          f"Σne({sum_ne}) >= id_missing({f['owner_pts_id_missing']}) + real_neq({f['owner_pts_real_neq']})（逐类次 ≥ 逐点）")

    print("### E. 红线对照（baseline vs after 逐字节一致，ID-6.3）")
    for key in ("NEST_GATE_STATS", "NEST_GATE_CHAIN", "NEST_GATE_T3", "NEST_GATE_INDEX"):
        check(b.get(key) == a.get(key), f"{key} 整行逐字节一致")
        if b.get(key) != a.get(key):
            print(f"    baseline: {b.get(key)}")
            print(f"    after   : {a.get(key)}")
    for name, rel in [("tower_events", "opsem/tower_events.jsonl"),
                      ("trades", "opsem/trades.jsonl"),
                      ("chain_dump(候选流)", f"dump/t5a_chain_dump_{tag}.jsonl")]:
        h_b = md5(f"{BASE_DIR}/{tag}/{rel}")
        h_a = md5(f"{AFTER_DIR}/{tag}/{rel}")
        check(h_b == h_a, f"{name} MD5 一致（{h_b[:12]}…）")

    summary[tag] = {"fail": f, "level": lv, "index": idx}
    print()

print("## 读数汇总（resolution 备料）\n")
for tag in TAGS:
    f, lv = summary[tag]["fail"], summary[tag]["level"]
    c23_eq = f["c2_eq"] + f["c3_eq"]
    c23_tot = f["c2_total"] + f["c3_total"]
    rate = f"{c23_eq}/{c23_tot}" + (f" ({c23_eq / c23_tot:.4f})" if c23_tot else " (n/a)")
    print(f"- {tag}: 二/三类点相等率 = {rate}；四桶 owner_start_neq={f['owner_start_neq']} "
          f"out_of_window={f['out_of_window']} opposite_side={f['opposite_side']} "
          f"no_valid_point={f['no_valid_point']}(missing={f['book_missing']},empty={f['book_empty']}) "
          f"success={f['success']}；kind trend={lv['base_trend']} pan={lv['base_pan']}")

print(f"\n## 总结：{n_checks} 项断言，{'全过 ✓' if not fail_items else f'{len(fail_items)} 项失败 ✗'}")
for item in fail_items:
    print("  FAIL:", item)
sys.exit(1 if fail_items else 0)
