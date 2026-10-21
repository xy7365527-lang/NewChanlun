#!/usr/bin/env python3
"""R3 离线预判：只读打开归档旧库（38c61eee/48e2386463，new-bi/1+center-seed/1），逐代统计结构写入量。

不启动产品；immutable 只读。输出 JSON：每代 delta/batch 字节、新发布/撤回事实（按 kind）、新关系数。
0055/F4：immutable=1 会忽略未 checkpoint 的 WAL（main 库因此只见 g1..51）。旁有非空 -wal 时先在临时副本上
wal_checkpoint(TRUNCATE) 再只读打开，原库不动；输出记 wal_applied。
用法：python3 r3_offline_churn.py <session.sqlite> > out.json
"""
import collections, json, os, shutil, sqlite3, sys, tempfile

db = sys.argv[1]
src, wal_applied = db, False
if os.path.exists(db + "-wal") and os.path.getsize(db + "-wal") > 0:
    tmp = tempfile.mkdtemp(prefix="r3churn-")
    src = os.path.join(tmp, "session.sqlite")
    shutil.copy2(db, src)
    shutil.copy2(db + "-wal", src + "-wal")
    w = sqlite3.connect(src)
    assert w.execute("PRAGMA wal_checkpoint(TRUNCATE)").fetchone()[0] == 0, "checkpoint busy"
    w.close()
    wal_applied = True
c = sqlite3.connect(f"file:{src}?mode=ro&immutable=1", uri=True)
gens = {}
for g, dlen, bf in c.execute("select generation, length(delta_json), index_frontier from structure_deltas order by generation"):
    blen = c.execute("select byte_len from batches where batch_id=?", (bf,)).fetchone()
    gens[g] = {"generation": g, "delta_json_bytes": dlen, "batch_bytes": blen[0] if blen else None,
               "published": collections.Counter(), "withdrawn": collections.Counter(), "relations_published": 0}
for table in ("structure_facts", "objects"):
    for kind, pub, wd in c.execute(f"select kind, published_generation, withdrawn_generation from {table}"):
        if pub in gens:
            gens[pub]["published"][kind] += 1
        if wd in gens:
            gens[wd]["withdrawn"][kind] += 1
for (pub,) in c.execute("select published_generation from relations"):
    if pub in gens:
        gens[pub]["relations_published"] += 1
out = []
for g in sorted(gens):
    r = gens[g]
    r["published_total"] = sum(r["published"].values())
    r["withdrawn_total"] = sum(r["withdrawn"].values())
    r["published"] = dict(sorted(r["published"].items()))
    r["withdrawn"] = dict(sorted(r["withdrawn"].items()))
    out.append(r)
json.dump({"db": db, "wal_applied": wal_applied, "generations": out}, sys.stdout, ensure_ascii=False, indent=1)
