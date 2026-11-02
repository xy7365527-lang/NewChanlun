#!/usr/bin/env python3
"""#1488 S5（0075）D7：从派生诊断构建（c081bfdeee）native 运行的 serve.stderr 读候选 handoff 峰值。

每条 NativeTestOnlyPublication 行的 s5_charge（发布后仍保留候选时才有）：
  handoff_peak          CompleteCharge.handoff_peak（候选生命期内取 max，含 entry_reserved_bytes）
  entry_reserved_bytes  本请求入口已持费用
  resources_request_peak 候选父账 Budget::request_peak()（本请求增量峰值）
  limit / resources_total_limit  候选 limit 与父账总帽
配同代 request-budget-entry 的 budget{total, old_pool, allowance}。
D7 口径：请求增量峰值对 8 MiB（write_budget 锁夹具 S7 = legacy + held + 8MiB 的余量）的比例；
两种算法都给：lock 式 handoff_peak − entry_reserved_bytes（fix-0044 对齐后的断言量），与父账 request_peak。
用法：peak_s5.py RUN_DIR [--out JSON]
"""

import argparse
import json
import statistics
from pathlib import Path

MIB8 = 8 * 1024 * 1024


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run")
    ap.add_argument("--out")
    a = ap.parse_args()
    entry, rows = {}, []
    for line in (Path(a.run) / "serve.stderr").read_text(errors="replace").splitlines():
        if not line.startswith("{"):
            continue
        try:
            j = json.loads(line)
        except ValueError:
            continue
        mid = str(j.get("message_id", ""))
        if j.get("stage") == "request-budget-entry":
            entry[mid] = j["budget"]
        elif j.get("diagnostic") == "NativeTestOnlyPublication":
            g = int(mid.split("-")[-1])
            c = j.get("s5_charge")
            row = {"generation": g, "candidate_entry": j["candidate_entry"], "abandoned": j["abandoned"],
                   "candidate_after": j["candidate_after"], "budget_entry": entry.get(mid)}
            if c:
                ch = c["charge"]
                lock = ch["handoff_peak"] - ch["entry_reserved_bytes"]
                row.update({"handoff_peak": ch["handoff_peak"], "entry_reserved_bytes": ch["entry_reserved_bytes"],
                            "lock_request_peak": lock, "resources_request_peak": c["resources_request_peak"],
                            "charge_total": ch["total"], "limit": c["limit"],
                            "resources_total_limit": c["resources_total_limit"],
                            "lock_ratio_8mib": round(lock / MIB8, 3),
                            "request_peak_ratio_8mib": round(c["resources_request_peak"] / MIB8, 3)})
                b = entry.get(mid)
                if b and b.get("allowance"):
                    row["lock_ratio_allowance"] = round(lock / b["allowance"], 3)
            rows.append(row)
    have = [r for r in rows if "lock_request_peak" in r]

    def stat(key):
        v = [r[key] for r in have]
        return {"min": min(v), "median": statistics.median(v), "max": max(v),
                "argmax_generation": max(have, key=lambda r: r[key])["generation"]} if v else None

    out = {"run": a.run, "eight_mib": MIB8, "publications": len(rows),
           "with_charge": len(have),
           "abandoned_generations": [r["generation"] for r in rows if r["abandoned"]],
           "retained_after": sum(r["candidate_after"] for r in rows),
           "lock_request_peak": stat("lock_request_peak"),
           "resources_request_peak": stat("resources_request_peak"),
           "lock_ratio_8mib": stat("lock_ratio_8mib"),
           "request_peak_ratio_8mib": stat("request_peak_ratio_8mib"),
           "generations_at_or_above_90pct_of_8mib": sum(r["request_peak_ratio_8mib"] >= 0.9 for r in have),
           "verdict_d7": None, "rows": rows}
    if have:
        out["verdict_d7"] = ("转 #1612 切片 2" if out["request_peak_ratio_8mib"]["max"] >= 0.9
                             else "结案（读数 < 90%）")
    text = json.dumps(out, indent=1, ensure_ascii=False)
    if a.out:
        Path(a.out).write_text(text + "\n")
    print(json.dumps({k: v for k, v in out.items() if k != "rows"}, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
