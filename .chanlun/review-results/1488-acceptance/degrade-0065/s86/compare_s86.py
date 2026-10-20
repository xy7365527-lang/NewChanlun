#!/usr/bin/env python3
"""#1488 验收 S1：S86 两次运行产物的三档对拍（INVARIANT / CONDITIONAL / ALWAYS_DIFFERENT）。

用法：compare_s86.py RUN_A RUN_B [--mode same-binary|cross-binary] [--out report.json]
RUN_* 为 run_s86.py 的 workdir（含 s.sqlite、init.json、responses.jsonl、readings.jsonl、
run-manifest.json）。只读打开数据库（mode=ro），不改写任一产物。

三档口径（详见同目录 README.md）：
  INVARIANT        任何二臂必须逐位相同（main r8 ↔ r8 双跑；main ↔ #1488 段 A）：
                   逐代公开根（manifest/snapshot/delta_root）、逐代 FinancialBasis 根
                   （按 financial_reader.rs current_basis 配方在 Python 内独立重算）、
                   init 回执、86 条回复帧（逐位，含 receipt_id/index_frontier）、
                   对象/见证/关系/观察索引行、全部非节点表行、meta（除环境键）、
                   以及从全部公开根出发的可达闭包节点集合（digest,tag,body 逐位，逐节点核 F(tag,body)）。
  CONDITIONAL      同一二进制双跑必须相同；跨二进制允许不同但须报告并逐项解释：
                   s_content_nodes 全集（含不可达中间根；#1488 R1 预期使不可达降为 0）、
                   逐代 stored_nodes/stored_body_bytes、sqlite_master DDL、二进制 sha256、
                   meta 环境键（sqlite_version/platform/journal_mode/synchronous）。
  ALWAYS_DIFFERENT 不比较，仅列出：耗时与 /proc 读数、墙钟起始时刻、workdir/socket/二进制路径、
                   pid、主机名、数据库物理文件字节（页布局、WAL 检查点时机）。
退出码：0=按 mode 判定通过；1=有不允许的差异；2=输入/结构错误（缺节点、摘要不符等）。
"""

import argparse
import hashlib
import json
import sqlite3
import sys
from pathlib import Path

D = b"NewChanlun/SContent/1\x00"
NODES = "s_content_nodes"
ENV_META_KEYS = {"sqlite_version", "platform", "journal_mode", "synchronous"}
ROOT_COLUMNS = {
    "s_content_publications": ("manifest_root", "snapshot_root", "delta_root"),
    "s_content_objects": ("generated_root", "internal_root", "immutable_root", "birth_wire_root"),
    "s_content_witnesses": ("generated_root", "wire_root"),
    "s_content_relations": ("generated_root",),
    "s_content_observations": ("generated_root", "birth_wire_root"),
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canon(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def u64(n):
    return n.to_bytes(8, "big")


def frame(tag, body):
    return hashlib.sha256(D + bytes([tag]) + u64(len(body)) + body).digest()


def merkle(hashes):
    """M(h)：RFC 9162 式按数量唯一二分；k=严格小于 n 的最大 2 的幂。"""
    n = len(hashes)
    if n == 0:
        return frame(0x11, b"")
    if n == 1:
        return frame(0x12, hashes[0])
    k = 1
    while k * 2 < n:
        k *= 2
    return frame(0x13, u64(n) + merkle(hashes[:k]) + merkle(hashes[k:]))


def v_string(text):
    data = text.encode("utf-8")
    chunks = [frame(0x10, data[i:i + 4096]) for i in range(0, len(data), 4096)]
    return frame(0x03, u64(len(data)) + merkle(chunks))


def v_object(fields):
    """fields: [(key, value_digest)]；按键 UTF-8 字节序。"""
    items = sorted(fields, key=lambda kv: kv[0].encode("utf-8"))
    return frame(0x06, u64(len(items)) + merkle([frame(0x14, v_string(k) + v) for k, v in items]))


def children(tag, body):
    """§5.2 节点表：返回直接子摘要；形状不符即抛错。"""
    if tag in (0x00, 0x01, 0x02, 0x08, 0x10, 0x11):
        return []
    if tag in (0x03, 0x04, 0x05, 0x06):
        if len(body) != 40:
            raise ValueError(f"tag {tag:#x} body 长度 {len(body)}")
        return [body[8:40]]
    if tag in (0x07, 0x12):
        if len(body) != 32:
            raise ValueError(f"tag {tag:#x} body 长度 {len(body)}")
        return [body]
    if tag == 0x13:
        if len(body) != 72:
            raise ValueError("0x13 body 长度")
        return [body[8:40], body[40:72]]
    if tag == 0x14:
        if len(body) != 64:
            raise ValueError("0x14 body 长度")
        return [body[0:32], body[32:64]]
    raise ValueError(f"未知 tag {tag:#x}")


class Run:
    def __init__(self, path):
        self.path = Path(path)
        self.db = sqlite3.connect(f"file:{self.path / 's.sqlite'}?mode=ro", uri=True)
        self.manifest = json.loads((self.path / "run-manifest.json").read_text())
        self._nodes = None

    def tables(self):
        return [r[0] for r in self.db.execute(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")]

    def ddl(self):
        return sorted(self.db.execute(
            "SELECT type,name,COALESCE(sql,'') FROM sqlite_master ORDER BY type,name").fetchall())

    def rows(self, table):
        cols = [r[1] for r in self.db.execute(f'PRAGMA table_info("{table}")')]
        out = []
        for row in self.db.execute(f'SELECT * FROM "{table}"'):
            out.append(canon([[c, typed(v)] for c, v in zip(cols, row)]))
        return sorted(out)

    def nodes(self):
        if self._nodes is None:
            self._nodes = {d: (t, b) for d, t, b in self.db.execute(
                f"SELECT digest,tag,body FROM {NODES}")}
        return self._nodes

    def public_roots(self):
        roots = set()
        for table, cols in ROOT_COLUMNS.items():
            for row in self.db.execute(f'SELECT {",".join(cols)} FROM "{table}"'):
                roots.update(row)
        return roots

    def closure(self, roots):
        nodes, seen, stack = self.nodes(), set(), list(roots)
        while stack:
            d = stack.pop()
            if d in seen:
                continue
            if d not in nodes:
                raise LookupError(f"缺节点 {d.hex()}")
            seen.add(d)
            tag, body = nodes[d]
            stack.extend(children(tag, body))
        return seen

    # —— 只读值解码（仅用于取出 manifest 内的少量标量） ——
    def object_fields(self, digest):
        tag, body = self.nodes()[digest]
        if tag != 0x06:
            raise ValueError("不是对象")
        out = {}
        for leaf in self.seq_leaves(body[8:40]):
            ftag, fbody = self.nodes()[leaf]
            assert ftag == 0x14
            out[self.string(fbody[:32])] = fbody[32:]
        return out

    def seq_leaves(self, digest):
        out, stack = [], [digest]
        while stack:
            tag, body = self.nodes()[stack.pop()]
            if tag == 0x12:
                out.append(body)
            elif tag == 0x13:
                stack.extend([body[40:72], body[8:40]])
            elif tag != 0x11:
                raise ValueError("非序列节点")
        return out

    def string(self, digest):
        tag, body = self.nodes()[digest]
        if tag not in (0x03, 0x04):
            raise ValueError(f"不是字符串 tag {tag:#x}")
        return b"".join(self.nodes()[c][1] for c in self.seq_leaves(body[8:40])).decode("utf-8")

    def find_scalar(self, digest, key):
        """manifest 顶层字段（不下钻，避免取到嵌套同名键）。"""
        fields = self.object_fields(digest)
        return self.string(fields[key]) if key in fields else None

    def basis_roots(self):
        """financial_reader.rs current_basis 配方：七字段 FinancialBasis，snapshot 为边。"""
        meta = dict(self.db.execute("SELECT key,value FROM meta"))
        out = {}
        for gen, manifest, snapshot in self.db.execute(
                "SELECT generation,manifest_root,snapshot_root FROM s_content_publications ORDER BY generation"):
            found = {k: self.find_scalar(manifest, k) for k in
                     ("semantic_commit_ns", "session_id", "session_generation", "writer_epoch")}
            if None in found.values() or found["session_id"] != meta["session_id"]:
                raise LookupError(f"第 {gen} 代 manifest 元数据不全：{found}")
            commit_ns = found["semantic_commit_ns"]
            root = v_object([
                ("kind", v_string("FinancialBasis")),
                ("session_id", v_string(found["session_id"])),
                ("session_generation", v_string(found["session_generation"])),
                ("writer_epoch", v_string(found["writer_epoch"])),
                ("pending_revision", frame(0x01, b"")),
                ("commit_ns", v_string(commit_ns)),
                ("snapshot", snapshot),
            ])
            out[str(gen)] = {"basis_root": "sc1-" + root.hex(), "commit_ns": commit_ns}
        return out


def typed(v):
    if isinstance(v, bytes):
        return {"blob": v.hex()}
    if isinstance(v, float):
        return {"real": repr(v)}
    return v


EXPECTED_GENERATIONS = 86


def lines(path):
    """逐行（保留行尾），缺文件即结构错误。"""
    return Path(path).read_text(encoding="utf-8").splitlines(keepends=True)


def diff_list(a, b, limit=5):
    if a == b:
        return None
    sa, sb = set(a), set(b)
    return {"len": [len(a), len(b)], "only_a": sorted(sa - sb)[:limit],
            "only_b": sorted(sb - sa)[:limit],
            "order_or_multiplicity_only": sa == sb}


def check(results, tier, name, a, b, detail=None):
    same = a == b
    entry = {"tier": tier, "item": name, "equal": same,
             "digest_a": sha(canon(a).encode()), "digest_b": sha(canon(b).encode())}
    if not same:
        entry["diff"] = detail if detail is not None else (
            diff_list(a, b) if isinstance(a, list) else {"a": a, "b": b})
    results.append(entry)


def compare(path_a, path_b):
    A, B = Run(path_a), Run(path_b)
    results, info = [], {}
    # 完整性前提：两臂都必须是 86 代全部 Committed 的完整运行，否则不是可比对象。
    for name, run in (("A", A), ("B", B)):
        m = run.manifest
        gens = run.db.execute("SELECT count(*) FROM s_content_publications").fetchone()[0]
        if m.get("failure") or m.get("generations") != EXPECTED_GENERATIONS or gens != EXPECTED_GENERATIONS \
                or m.get("basis_ref_returncode") != 0:
            raise ValueError(f"{name} 不是完整 86 代运行：failure={m.get('failure')} generations="
                             f"{m.get('generations')} publications={gens} basis_rc={m.get('basis_ref_returncode')}")
    # ---------- INVARIANT ----------
    check(results, "INVARIANT", "init.json", lines(A.path / "init.json"), lines(B.path / "init.json"))
    ra, rb = lines(A.path / "responses.jsonl"), lines(B.path / "responses.jsonl")
    check(results, "INVARIANT", "responses.jsonl(86 回复帧逐位)", ra, rb,
          detail={"len": [len(ra), len(rb)],
                  "first_diff_generation": next((i + 1 for i, (x, y) in enumerate(zip(ra, rb)) if x != y), None)})
    pub = lambda r: [canon([g, b_, m.hex(), s.hex(), d.hex()]) for g, b_, m, s, d in r.db.execute(
        "SELECT generation,batch_id,manifest_root,snapshot_root,delta_root FROM s_content_publications ORDER BY generation")]
    pa, pb = pub(A), pub(B)
    check(results, "INVARIANT", "逐代公开根 manifest/snapshot/delta_root", pa, pb,
          detail={"len": [len(pa), len(pb)], "differing_generations":
                  [json.loads(x)[0] for x, y in zip(pa, pb) if x != y][:20]})
    ba, bb = A.basis_roots(), B.basis_roots()
    check(results, "INVARIANT", "逐代 FinancialBasis 根（独立重算）", ba, bb,
          detail={"differing_generations": [g for g in ba if ba.get(g) != bb.get(g)][:20]})
    check(results, "INVARIANT", "basis-ref.json（产品 financial-basis-ref 末代回执）",
          lines(A.path / "basis-ref.json"), lines(B.path / "basis-ref.json"))
    # 自核：独立重算的末代 basis 根必须等于产品冷路径给出的 basis_ref.root，否则重算配方失效。
    for name, run, basis in (("A", A, ba), ("B", B, bb)):
        product = json.loads((run.path / "basis-ref.json").read_text())["basis_ref"]["root"]
        mine = basis[max(basis, key=int)]["basis_root"]
        if product != mine:
            raise ValueError(f"{name} 重算末代 basis 根 {mine} ≠ 产品 {product}")
    info["basis_recompute_matches_product"] = True
    info["generations"] = [len(pa), len(pb)]
    info["final_snapshot_root"] = [json.loads(pa[-1])[3] if pa else None, json.loads(pb[-1])[3] if pb else None]
    info["final_basis_root"] = [ba[max(ba, key=int)]["basis_root"] if ba else None,
                                bb[max(bb, key=int)]["basis_root"] if bb else None]
    ta, tb = A.tables(), B.tables()
    for table in sorted(set(ta) | set(tb)):
        if table == NODES:
            continue
        if table not in ta or table not in tb:
            check(results, "INVARIANT", f"表存在性 {table}", table in ta, table in tb)
            continue
        xa, xb = A.rows(table), B.rows(table)
        if table == "meta":
            env = lambda rows: [r for r in rows if json.loads(r)[0][1] in ENV_META_KEYS]
            check(results, "CONDITIONAL", "meta 环境键", env(xa), env(xb))
            xa = [r for r in xa if r not in env(xa)]
            xb = [r for r in xb if r not in env(xb)]
        check(results, "INVARIANT", f"表 {table}（{len(xa)} 行）", xa, xb)
    roots_a, roots_b = A.public_roots(), B.public_roots()
    check(results, "INVARIANT", "公开根集合", sorted(r.hex() for r in roots_a), sorted(r.hex() for r in roots_b))
    ca, cb = A.closure(roots_a), B.closure(roots_b)
    for name, run, closure in (("A", A, ca), ("B", B, cb)):
        bad = [d.hex() for d in closure if frame(*run.nodes()[d]) != d]
        if bad:
            raise ValueError(f"{name} 节点摘要不符：{bad[:3]}")
    node_line = lambda run, ds: sorted(d.hex() + ":" + format(run.nodes()[d][0], "02x") + ":" + sha(run.nodes()[d][1]) for d in ds)
    check(results, "INVARIANT", f"可达闭包节点（{len(ca)}/{len(cb)}，digest:tag:sha(body)，均已核 F）",
          node_line(A, ca), node_line(B, cb))
    # ---------- CONDITIONAL ----------
    na, nb = A.nodes(), B.nodes()
    check(results, "CONDITIONAL", f"s_content_nodes 全集（{len(na)}/{len(nb)}）",
          node_line(A, na), node_line(B, nb))
    for name, run, closure in (("a", A, ca), ("b", B, cb)):
        total = len(run.nodes())
        info[f"nodes_{name}"] = {"stored": total, "reachable": len(closure), "unreachable": total - len(closure),
                                 "unreachable_fraction": round((total - len(closure)) / total, 6) if total else None,
                                 "stored_body_bytes": sum(len(b) for _, b in run.nodes().values())}
    check(results, "CONDITIONAL", "sqlite_master DDL", [canon(r) for r in A.ddl()], [canon(r) for r in B.ddl()])
    check(results, "CONDITIONAL", "binary_sha256", A.manifest["binary_sha256"], B.manifest["binary_sha256"])
    check(results, "INVARIANT", "输入文件 sha256（同输入前提）",
          {k: v["sha256"] for k, v in A.manifest["inputs"].items()},
          {k: v["sha256"] for k, v in B.manifest["inputs"].items()})
    rd = lambda run: [json.loads(x) for x in lines(run.path / "readings.jsonl")]
    stored = lambda run: [[r["generation"], r["kind"], r["stored_nodes"], r["stored_body_bytes"]] for r in rd(run)]
    check(results, "CONDITIONAL", "逐代 stored_nodes/stored_body_bytes", stored(A), stored(B))
    # ---------- ALWAYS_DIFFERENT（只列不比） ----------
    always = {
        "readings": "elapsed_ns、rchar/wchar/syscr/syscw/read_bytes/write_bytes、cpu_ticks",
        "run-manifest": "started_unix、serve_startup_ns、workdir、binary 路径、host/kernel",
        "process": "pid、socket 路径、serve.stderr",
        "db_files": {"a": {p.name: sha(p.read_bytes()) for p in sorted(A.path.glob("s.sqlite*")) if p.is_file()},
                     "b": {p.name: sha(p.read_bytes()) for p in sorted(B.path.glob("s.sqlite*")) if p.is_file()}},
    }
    return results, info, always


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run_a")
    ap.add_argument("run_b")
    ap.add_argument("--mode", choices=["same-binary", "cross-binary"], default="same-binary")
    ap.add_argument("--out")
    a = ap.parse_args()
    try:
        results, info, always = compare(a.run_a, a.run_b)
    except (LookupError, ValueError, KeyError, TypeError, OSError, sqlite3.Error) as exc:
        print(f"STRUCTURE_ERROR {exc}", file=sys.stderr)
        sys.exit(2)
    gated = {"INVARIANT"} | ({"CONDITIONAL"} if a.mode == "same-binary" else set())
    failures = [r for r in results if r["tier"] in gated and not r["equal"]]
    cond_diffs = [r for r in results if r["tier"] == "CONDITIONAL" and not r["equal"]]
    verdict = "PASS" if not failures else "FAIL"
    report = {"mode": a.mode, "run_a": a.run_a, "run_b": a.run_b, "verdict": verdict,
              "invariant_items": sum(r["tier"] == "INVARIANT" for r in results),
              "invariant_equal": sum(r["tier"] == "INVARIANT" and r["equal"] for r in results),
              "conditional_items": sum(r["tier"] == "CONDITIONAL" for r in results),
              "conditional_equal": sum(r["tier"] == "CONDITIONAL" and r["equal"] for r in results),
              "conditional_differences_to_explain": [r["item"] for r in cond_diffs],
              "info": info, "results": results, "always_different_not_compared": always}
    text = json.dumps(report, ensure_ascii=False, indent=1)
    if a.out:
        Path(a.out).write_text(text + "\n", encoding="utf-8")
    for r in results:
        print(f'{r["tier"]:<12} {"EQUAL" if r["equal"] else "DIFF ":<6} {r["item"]}')
    print("INFO", json.dumps(info, ensure_ascii=False))
    print("VERDICT", verdict, a.mode)
    sys.exit(0 if verdict == "PASS" else 1)


if __name__ == "__main__":
    main()
