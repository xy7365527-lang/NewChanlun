#!/usr/bin/env python3
"""#797：missing_cert 四类拆分计数（读 t5a_chain_dump jsonl 的 chain.levels[].diag）。

判别优先级（报告 §3 逐条说明；重叠情况另表列账，不抹平）：
  ④ 结构性不可满足  event_level > diag.max_event_level
  ⑤ 键错配（票面四类之外）  reg_same_price > 0
  ② 谓词不成立（精确）      unconf_exact > 0
  ②' 谓词不成立（同价异锚）  unconf_same_price > 0
  ③ 装配缺口（精确）        miss_same_price > 0
  ① 产出缺口               以上全零
"""

import json
import sys
from collections import Counter, defaultdict

CLASSES = ["c4_structural", "c5_key_mismatch", "c2_predicate", "c2p_predicate_sameprice",
           "c3_assembly", "c1_no_output"]


def classify(ev_level, d):
    if ev_level > d["max_event_level"]:
        return "c4_structural"
    if d["reg_same_price"] > 0:
        return "c5_key_mismatch"
    if d["unconf_exact"] > 0:
        return "c2_predicate"
    if d["unconf_same_price"] > 0:
        return "c2p_predicate_sameprice"
    if d["miss_same_price"] > 0:
        return "c3_assembly"
    return "c1_no_output"


def run(path, window):
    status_by_level = defaultdict(Counter)          # 对账用：逐级 status 全量
    split = defaultdict(Counter)                    # level -> class -> n
    overlap = Counter()                             # 非零指示器组合
    index_miss_rows = Counter()                     # 旁注：missing_causal 中的索引缺失
    index_miss_total = 0
    global_only = defaultdict(Counter)              # 不可分上界（按事件级）
    n_rows = 0
    with open(path) as fh:
        for line in fh:
            rec = json.loads(line)
            n_rows += 1
            for g in rec["chain"]["levels"]:
                lvl = g["level"]
                st = g["status"]
                status_by_level[lvl][st] += 1
                d = g["diag"]
                if g["diag"]["index_miss"] > 0:
                    index_miss_rows[st] += 1
                    index_miss_total += d["index_miss"]
                if st != "missing_cert":
                    continue
                cls = classify(g["event_level"], d)
                split[lvl][cls] += 1
                sig = tuple(k for k in ("reg_same_price", "unconf_exact", "unconf_same_price",
                                        "miss_same_price") if d[k] > 0)
                if g["event_level"] > d["max_event_level"]:
                    sig = ("STRUCT",) + sig
                overlap[sig] += 1
                if cls == "c1_no_output":
                    if d["unconf_no_anchor"] > 0:
                        global_only[lvl]["unconf_no_anchor_lvl>0"] += 1
                    if d["miss_no_price"] > 0:
                        global_only[lvl]["miss_no_price_lvl>0"] += 1
    return {
        "window": window, "rows": n_rows,
        "status_by_level": {k: dict(v) for k, v in sorted(status_by_level.items())},
        "split": {k: dict(v) for k, v in sorted(split.items())},
        "overlap": {"|".join(k) if k else "(none)": v for k, v in overlap.most_common()},
        "index_miss_rows_by_status": dict(index_miss_rows),
        "index_miss_cert_total": index_miss_total,
        "unresolvable_upper_bound": {k: dict(v) for k, v in sorted(global_only.items())},
    }


if __name__ == "__main__":
    out = {}
    for window in ("wf7", "wf8", "p3fold"):
        out[window] = run(f"{sys.argv[1]}/t5a_chain_dump_{window}.jsonl", window)
    print(json.dumps(out, indent=2, ensure_ascii=False))
