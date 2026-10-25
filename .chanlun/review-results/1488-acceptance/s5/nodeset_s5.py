#!/usr/bin/env python3
"""#1488 S5（0075）：解释 compare_s86 的 CONDITIONAL「s_content_nodes 全集」差异（只读）。

对两臂 A、B：A−B 与 B−A 的节点数、标签分布、是否都落在本臂不可达集；各自差集中
由“裸序列顶点”（0x12/0x13 无父，append 作废中间根签名）向下可达的节点数。
用法：nodeset_s5.py RUN_A RUN_B [--out JSON]
"""

import argparse
import collections
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "s86"))
from compare_s86 import Run, children  # noqa: E402


def side(run, other_nodes):
    N = run.nodes()
    R = run.closure(run.public_roots())
    parents = collections.Counter()
    for tag, body in N.values():
        for c in children(tag, body):
            parents[c] += 1
    seq_tops = [d for d in N if d not in R and parents[d] == 0 and N[d][0] in (0x12, 0x13)]
    under, stack = set(), list(seq_tops)
    while stack:
        d = stack.pop()
        if d in under or d in R:
            continue
        under.add(d)
        stack.extend(children(*N[d]))
    only = [d for d in N if d not in other_nodes]
    return {"stored": len(N), "only_here": len(only),
            "only_here_reachable_here": sum(1 for d in only if d in R),
            "only_here_tags": {f"{t:#04x}": n for t, n in collections.Counter(N[d][0] for d in only).most_common()},
            "seq_top_count": len(seq_tops), "under_seq_tops": len(under),
            "only_here_under_seq_tops": sum(1 for d in only if d in under),
            "only_here_not_under_seq_tops_tags": {f"{t:#04x}": n for t, n in collections.Counter(
                N[d][0] for d in only if d not in under).most_common()}}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run_a")
    ap.add_argument("run_b")
    ap.add_argument("--out")
    a = ap.parse_args()
    A, B = Run(a.run_a), Run(a.run_b)
    out = {"a": a.run_a, "b": a.run_b,
           "A": side(A, B.nodes()), "B": side(B, A.nodes()),
           "common": len(set(A.nodes()) & set(B.nodes()))}
    text = json.dumps(out, indent=1, ensure_ascii=False)
    if a.out:
        Path(a.out).write_text(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
