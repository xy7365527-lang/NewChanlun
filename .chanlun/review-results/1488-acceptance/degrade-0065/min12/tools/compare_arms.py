#!/usr/bin/env python3
"""degrade-0065：MIN12 native 臂 ↔ ordinary 臂逐代逐位对拍（同一 testonly 二进制，只差两个 testonly 开关）。

比较项（全部要求相等）：
  - 52 条回执的 publication 对象（generation/cut/各根，逐字段）；
  - g=1..52 每代 Q3 服务端 cut_projection_digest（qcut3 内容树根）、counts、structure_cut、rule_revision；
  - 每代本地 canonical 投影 sha256（rows_sha256）。
用法：compare_arms.py <native-work> <ordinary-work> [--out PATH]
"""
import argparse
import json
from pathlib import Path


def load(work):
    run = json.loads((work / "RUN.json").read_text())
    ingest = [json.loads(x) for x in (work / "INGEST.jsonl").read_text().splitlines()]
    return run, ingest


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("native", type=Path)
    ap.add_argument("ordinary", type=Path)
    ap.add_argument("--out", type=Path)
    a = ap.parse_args()
    (rn, inn), (ro, ino) = load(a.native), load(a.ordinary)
    items = []
    pub_diff = [x["index"] for x, y in zip(inn, ino) if x.get("publication") != y.get("publication")]
    items.append({"item": "回执 publication（逐条）", "count": [len(inn), len(ino)],
                  "equal": len(inn) == len(ino) == 52 and not pub_diff, "diff_index": pub_diff})
    qn = {q["generation"]: q for q in rn.get("queries", [])}
    qo = {q["generation"]: q for q in ro.get("queries", [])}
    for key in ("digest", "counts", "structure_cut", "rule_revision", "rows_sha256"):
        diff = [g for g in range(1, 53) if g not in qn or g not in qo or qn[g].get(key) != qo[g].get(key)]
        items.append({"item": f"逐代 {key}", "equal": not diff, "diff_generations": diff})
    verdict = "PASS" if all(i["equal"] for i in items) else "FAIL"
    result = {"native": str(a.native), "ordinary": str(a.ordinary),
              "binary_sha256": [rn.get("binary_sha256"), ro.get("binary_sha256")],
              "env_testonly": [rn.get("env_testonly"), ro.get("env_testonly")], "items": items, "verdict": verdict}
    for i in items:
        print(f"{'EQUAL' if i['equal'] else 'DIFF '}  {i['item']}")
    print("VERDICT", verdict)
    if a.out:
        a.out.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    raise SystemExit(0 if verdict == "PASS" else 1)


if __name__ == "__main__":
    main()
