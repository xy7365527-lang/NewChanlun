#!/usr/bin/env python3
"""#1488 S5（0075）R1：S86 运行库不可达节点比例与来源分解（只读，离线结构判读）。

产品没有“按发射阶段”的节点计数，这里只按库内结构归属：
1. 可达 = compare_s86 同一公开根集合（ROOT_COLUMNS）的闭包；不可达 U = 全部节点 − 可达。
2. 顶点 = U 中没有任何父节点（全库范围）的节点。每个顶点按形状/字段集归一类：
   seq-top        裸序列节点（0x12/0x13 无父）：fractal per-bar append 作废中间根的签名（D5 本票修，终面应为 0）
   array-memo     数组（0x05），元素序列是某可达数组的连续子段：coords 数组 memo 子数组（content_producer
                  array_from_node，发射阶段 coords_array_memo）→ #1491
   array-other    其余数组顶点
   obj:<族>       对象（0x06）按字段集分族，见 FAMILY；记录族另比对 s_content_objects 当前行
   other-top      其他标签的无父节点
3. 非顶点不可达节点沿子边从顶点向下归属；被多类顶点共享记 mixed。
4. 有 births.tsv（run_s5.py 产物）时另给按出生代分布；native 运行另从 serve.stderr 取放弃代，
   给该代出生的不可达节点数（冷根放弃残留候选）。

用法：unreach_s5.py RUN_DIR [--out JSON] [--label L]；compare_s86.py 须与本脚本同目录或在 PYTHONPATH。
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

FAMILY = [  # (族名, 必含字段)；按序取第一个命中
    ("record-internal", {"batch_id", "lifecycle"}),
    ("record-generated", {"object_id", "published_generation", "input_refs_json"}),
    ("manifest-delta-draft", {"base_cut", "next_cut"}),
    ("manifest-snapshot-draft", {"as_of_generation", "history_mode"}),
    ("source-mi", {"mi", "raw_bars"}),
    ("generation-scope", {"scope", "rule_revision"}),
]


def family(keys):
    for name, need in FAMILY:
        if need <= keys:
            return name
    return "other"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run")
    ap.add_argument("--out")
    ap.add_argument("--label")
    a = ap.parse_args()
    run = Run(a.run)
    N = run.nodes()
    R = run.closure(run.public_roots())
    U = set(N) - R
    parents = collections.Counter()
    for tag, body in N.values():
        for c in children(tag, body):
            parents[c] += 1
    tops = [u for u in U if parents[u] == 0]

    def sval(d):
        return run.string(d) if N[d][0] in (3, 4) else f"tag{N[d][0]}"

    # 可达数组的元素序列，用于 memo 子段判定
    host = collections.defaultdict(list)
    leaves = {}
    for d in R:
        if N[d][0] == 5:
            leaves[d] = run.seq_leaves(N[d][1][8:40])
            for i, x in enumerate(leaves[d]):
                host[x].append((d, i))

    def sublist(L):
        if not L:
            return "empty"
        for d, i in sorted(host.get(L[0], [])):  # 排序使 prefix/suffix/interior 判读可复现
            H = leaves[d]
            if H[i:i + len(L)] == L:
                return "prefix" if i == 0 else ("suffix" if i + len(L) == len(H) else "interior")
        return "none"

    rows = {r[0]: r for r in run.db.execute(
        "SELECT object_id,object_revision,published_generation,withdrawn_generation FROM s_content_objects")}
    cls, sub, keysets, record_rel = {}, collections.Counter(), collections.defaultdict(collections.Counter), \
        collections.Counter()
    kinds = collections.Counter()
    for u in tops:
        tag = N[u][0]
        if tag in (0x12, 0x13):
            c = "seq-top"
        elif tag == 0x05:
            s = sublist(run.seq_leaves(N[u][1][8:40]))
            sub[s] += 1
            c = "array-memo" if s in ("prefix", "suffix", "interior") else "array-other"
        elif tag == 0x06:
            f = run.object_fields(u)
            keys = frozenset(f)
            c = "obj:" + family(keys)
            keysets[c][tuple(sorted(keys))] += 1
            if c in ("obj:record-generated", "obj:record-internal") and "object_id" in f:
                kinds[(c, sval(f["kind"]) if "kind" in f else "?")] += 1
                row = rows.get(sval(f["object_id"]))
                if row is None:
                    record_rel[(c, "no-row")] += 1
                else:
                    rev = sval(f["object_revision"]) if "object_revision" in f else None
                    pg = sval(f["published_generation"]) if "published_generation" in f else None
                    record_rel[(c, "rev=" + ("same" if rev is None or str(row[1]) == rev else "diff"),
                                "pg=" + ("n/a" if pg is None else "same" if str(row[2]) == pg else "diff"),
                                "withdrawn" if row[3] else "live")] += 1
        else:
            c = "other-top"
        cls[u] = c

    # 向下归属
    owner = dict(cls)
    stack = list(tops)
    while stack:
        d = stack.pop()
        for ch in children(*N[d]):
            if ch not in U:
                continue
            prev = owner.get(ch)
            if prev is None:
                owner[ch] = owner[d]
                stack.append(ch)
            elif prev != owner[d] and prev != "mixed":
                owner[ch] = "mixed"
                stack.append(ch)
    unowned = [u for u in U if u not in owner]  # 无父顶点外的环/孤链（应为 0）

    births = {}
    bpath = Path(a.run) / "births.tsv"
    if bpath.exists():
        for line in bpath.read_text().splitlines():
            g, h = line.split("\t")
            births[bytes.fromhex(h)] = int(g)
    abandoned = []
    epath = Path(a.run) / "serve.stderr"
    if epath.exists():
        for line in epath.read_text(errors="replace").splitlines():
            if '"NativeTestOnlyPublication"' in line:
                try:
                    j = json.loads(line)
                except ValueError:
                    continue
                if j.get("abandoned"):
                    abandoned.append(int(str(j["message_id"]).split("-")[-1]))

    by_class = collections.Counter(owner.get(u, "unowned") for u in U)
    tops_by_class = collections.Counter(cls.values())
    tag_by_class = collections.defaultdict(collections.Counter)
    for u in U:
        tag_by_class[owner.get(u, "unowned")][f"{N[u][0]:#04x}"] += 1
    out = {
        "label": a.label or Path(a.run).name, "run": str(a.run),
        "binary_sha256": run.manifest.get("binary_sha256"),
        "nodes": len(N), "reachable": len(R), "unreachable": len(U),
        "unreachable_ratio": round(len(U) / len(N), 6),
        "tops": len(tops), "unowned": len(unowned),
        "classes": {c: {"tops": tops_by_class.get(c, 0), "nodes": n, "tags": dict(tag_by_class[c])}
                    for c, n in by_class.most_common()},
        "array_top_sublist": dict(sub),
        "object_top_keysets": {c: [[n, list(k)] for k, n in v.most_common(4)] for c, v in keysets.items()},
        "record_top_vs_object_row": {" ".join(k): n for k, n in record_rel.most_common()},
        "record_top_kinds": {" ".join(k): n for k, n in kinds.most_common()},
        "publications_rows": run.db.execute("SELECT count(*) FROM s_content_publications").fetchone()[0],
        "abandoned_generations": abandoned,
    }
    if births:
        missing = sum(1 for u in U if u not in births)
        gen_class = collections.defaultdict(collections.Counter)
        for u in U:
            gen_class[births.get(u, -1)][owner.get(u, "unowned")] += 1
        out["births_missing"] = missing
        out["unreachable_by_birth_generation"] = {
            str(g): dict(gen_class[g]) for g in sorted(gen_class)}
        out["unreachable_born_in_abandoned_generations"] = {
            str(g): dict(gen_class.get(g, {})) for g in abandoned}
        out["new_nodes_by_generation"] = dict(sorted(collections.Counter(births.values()).items()))
    text = json.dumps(out, indent=1, ensure_ascii=False)
    if a.out:
        Path(a.out).write_text(text + "\n")
    summary = {k: out[k] for k in ("label", "nodes", "reachable", "unreachable", "unreachable_ratio",
                                     "tops", "unowned")}
    print(json.dumps(summary, ensure_ascii=False))
    for c, v in out["classes"].items():
        print(f"  {c:28s} tops={v['tops']:7d} nodes={v['nodes']:8d}")
    print("  array_top_sublist", out["array_top_sublist"])
    print("  abandoned", abandoned)


if __name__ == "__main__":
    main()
