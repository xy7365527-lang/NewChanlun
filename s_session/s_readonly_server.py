#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""#1370 TB-01-A：S 正式结构会话的只读查询外壳（前端查询外壳，独立只读进程）。

C09.1：核心判断用 Rust；本进程只读 S 自己的 SQLite（`mode=ro`），承担协议与展示适配。
读端只读已提交 cut——不把文件存在、消息到达或内存对象当持久成功；本进程只 SELECT，
不写入、不重算结构、不选择操作级别、不推算经济资格。

- PY-H01：只读连接用 URI 编码的文件 URI（`Path.as_uri()` 对 `?`/`#`/`%` 正确转义），
  并加 `PRAGMA query_only=ON` 连接级防线——指定库的路径不会被截断成别的库。
- PY-M01 / R1-M01 / R1-M02：所有数据库读取、JSON 解码、JSON 序列化与 UTF-8 编码都在
  请求边界内完成并返回结构化 5xx；坏 `scope`/BLOB 值/孤立 surrogate 不再被吞成成功空值，
  也不越过 HTTP 错误边界裸断连。
- H1：每个 API 响应在单个读事务内读取同一已提交 cut（WAL 读快照），不跨查询拼状态。

用法：
  python3 s_readonly_server.py --db <S自己的.sqlite> --port 8787 \
      --browser s_session/browser/index.html
"""

import argparse
import json
import os
import sqlite3
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


def open_readonly(db_path):
    """PY-H01：URI 编码 + query_only。"""
    uri = Path(db_path).resolve().as_uri() + "?mode=ro"
    conn = sqlite3.connect(uri, uri=True)
    conn.execute("PRAGMA query_only=ON")
    return conn


def meta_dict(conn):
    out = {}
    for key, value in conn.execute("SELECT key, value FROM meta"):
        out[key] = value
    return out


def _scope(meta):
    """scope 字段：缺失 → 空对象（兼容缺项）；存在但损坏 → 抛出（进入 503 边界，不吞成 {}）。"""
    if "scope" not in meta:
        return {}
    # 坏 JSON / BLOB 等类型错误都向上抛，由请求边界转结构化 5xx。
    return _json_field(meta["scope"], dict)


def _num_to_str(v):
    """与 Rust 的持久 i64 同域；损坏值不经 str() 冒充成功整数。"""
    if type(v) is not int or not -(2**63) <= v < 2**63:
        raise ValueError("持久整数必须是 i64，不能是浮点、布尔、空值或文本")
    return str(v)


def _json_field(text, expected):
    """已存在的持久 JSON 必须符合声明形状，缺失/损坏不能补成空结果。"""
    if not isinstance(text, str):
        raise ValueError("持久 JSON 必须是文本")
    value = json.loads(text)
    if not isinstance(value, expected):
        raise ValueError("持久 JSON 形状不符")
    return value


def _project_input_refs(refs):
    """objects[].input_refs：merged_index（结构坐标）与 raw_refs[].seq（接纳序）→ 字符串。"""
    if not isinstance(refs, list):
        raise ValueError("input_refs 必须是数组")
    out = []
    for g in refs:
        if not isinstance(g, dict) or not isinstance(g.get("raw_refs"), list):
            raise ValueError("input_refs 成员必须含 raw_refs 数组")
        g = dict(g)
        g["merged_index"] = _num_to_str(g.get("merged_index"))
        projected_refs = []
        for item in g["raw_refs"]:
            if not isinstance(item, dict):
                raise ValueError("raw_refs 成员必须是对象")
            item = dict(item)
            item["seq"] = _num_to_str(item.get("seq"))
            projected_refs.append(item)
        g["raw_refs"] = projected_refs
        out.append(g)
    return out


def _project_raw_bars(bars):
    """witnesses[].raw_bars：seq（接纳序）→ 字符串。"""
    if not isinstance(bars, list):
        raise ValueError("raw_bars 必须是数组")
    out = []
    for b in bars:
        if not isinstance(b, dict):
            raise ValueError("raw_bars 成员必须是对象")
        b = dict(b)
        b["seq"] = _num_to_str(b.get("seq"))
        out.append(b)
    return out


def read_catalog(conn):
    """在调用方已开启的读事务内读取目录。"""
    meta = meta_dict(conn)
    rows = []
    for (cid, kind, title, domain, branches_json, impl, proof, run, evidence_json) in conn.execute(
        "SELECT catalog_id, kind, title, domain, branches_json, impl_status, proof_status, "
        "run_status, evidence_json FROM catalog ORDER BY catalog_id"
    ):
        rows.append({
            "id": cid,
            "kind": kind,
            "title": title,
            "domain": domain,
            "branches": _json_field(branches_json, list),
            "implementation_status": impl,
            "proof_status": proof,
            "run_status": run,
            "evidence": _json_field(evidence_json, dict),
        })
    return {
        "session_id": meta.get("session_id", ""),
        "generation": meta.get("generation", ""),
        "structure_cut": meta.get("structure_cut", ""),
        "catalog_revision": meta.get("catalog_revision", ""),
        "scope": _scope(meta),
        "items": rows,
        "counts": {
            "implemented": sum(1 for r in rows if r["implementation_status"] == "implemented"),
            "not_implemented": sum(1 for r in rows if r["implementation_status"] == "not_implemented"),
            "run": sum(1 for r in rows if r["run_status"] == "run"),
            "not_run": sum(1 for r in rows if r["run_status"] == "not_run"),
        },
    }


def read_snapshot(conn):
    """在调用方已开启的读事务内读取快照。"""
    meta = meta_dict(conn)
    objects = []
    for (oid, orev, kind, batch_id, branch, dir_ab, dir_bc, ws, wm, we, cmp_json, refs_json) in conn.execute(
        "SELECT object_id, object_revision, kind, batch_id, branch, dir_ab, dir_bc, "
        "window_start, window_mid, window_end, comparisons_json, input_refs_json "
        "FROM objects ORDER BY window_start"
    ):
        objects.append({
            "object_id": oid,
            "object_revision": _num_to_str(orev),
            "kind": kind,
            "batch_id": batch_id,
            "branch": branch,
            "dir_ab": dir_ab,
            "dir_bc": dir_bc,
            "window_start": _num_to_str(ws),
            "window_mid": _num_to_str(wm),
            "window_end": _num_to_str(we),
            "comparisons": _json_field(cmp_json, list),
            "input_refs": _project_input_refs(_json_field(refs_json, list)),
        })
    witnesses = []
    for (wid, oid, slot, msi, mh, ml, mo, mc, raw_json) in conn.execute(
        "SELECT witness_id, object_id, slot, merged_source_index, merged_high, merged_low, "
        "merged_open, merged_close, raw_json FROM witnesses ORDER BY object_id, slot"
    ):
        witnesses.append({
            "witness_id": wid,
            "object_id": oid,
            "slot": _num_to_str(slot),
            "merged_source_index": _num_to_str(msi),
            "merged_high": mh,
            "merged_low": ml,
            "merged_open": mo,
            "merged_close": mc,
            "raw_bars": _project_raw_bars(_json_field(raw_json, list)),
        })
    relations = []
    for (subj, rel, obj) in conn.execute(
        "SELECT subject, relation_type, object FROM relations ORDER BY subject, relation_type, object"
    ):
        relations.append({"subject": subj, "relation_type": rel, "object": obj})
    observations = []
    for (oid, batch_id, kind, ws, wm, we, reason, detail_json) in conn.execute(
        "SELECT observation_id, batch_id, kind, window_start, window_mid, window_end, reason, "
        "detail_json FROM observations ORDER BY kind, window_start"
    ):
        observations.append({
            "observation_id": oid,
            "batch_id": batch_id,
            "kind": kind,
            "window_start": None if ws is None else _num_to_str(ws),
            "window_mid": None if wm is None else _num_to_str(wm),
            "window_end": None if we is None else _num_to_str(we),
            "reason": reason,
            "detail": _json_field(detail_json, dict),
        })
    return {
        "session_id": meta.get("session_id", ""),
        "generation": meta.get("generation", ""),
        "structure_cut": meta.get("structure_cut", ""),
        "catalog_revision": meta.get("catalog_revision", ""),
        "scope": _scope(meta),
        "index_frontier": meta.get("index_frontier", ""),
        "profile_id": meta.get("profile_id", ""),
        "profile_hash": meta.get("profile_hash", ""),
        "objects": objects,
        "witnesses": witnesses,
        "relations": relations,
        "observations": observations,
    }


def read_state(conn):
    """在同一读事务内一次读出目录 + 快照（同一已提交 cut，供浏览器单请求使用）。"""
    catalog = read_catalog(conn)
    snapshot = read_snapshot(conn)
    return {
        "cut": {
            "session_id": snapshot["session_id"],
            "generation": snapshot["generation"],
            "structure_cut": snapshot["structure_cut"],
            "catalog_revision": snapshot["catalog_revision"],
            "index_frontier": snapshot["index_frontier"],
        },
        "catalog": catalog,
        "snapshot": snapshot,
    }


class Handler(BaseHTTPRequestHandler):
    db_path = None
    browser_path = None

    def log_message(self, fmt, *args):  # 安静日志
        sys.stderr.write("[readonly] %s - %s\n" % (self.address_string(), fmt % args))

    def _send_bytes(self, body_bytes, status=200):
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body_bytes)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body_bytes)

    def _send_html(self, body_bytes, status=200):
        self.send_response(status)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body_bytes)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body_bytes)

    def _send_json(self, obj, status=200):
        """在发送任何状态行之前完成序列化 + 编码（R1-M02：序列化失败不越界断连）。"""
        body = serialize_payload(obj)
        if body is None:
            body = b'{"ok": false, "error": "StorageUnavailable"}'
            status = 503
        self._send_bytes(body, status)

    def _read_only(self, kind):
        payload = None
        try:
            conn = open_readonly(self.db_path)
        except Exception as e:  # 数据库不可读 → StorageUnavailable，不伪造成功
            self._send_json({"ok": False, "error": "StorageUnavailable", "detail": str(e)}, 503)
            return
        try:
            conn.execute("BEGIN")  # H1：显式读事务，固定本次快照 cut
            if kind == "state":
                payload = read_state(conn)
            elif kind == "catalog":
                payload = read_catalog(conn)
            elif kind == "snapshot":
                payload = read_snapshot(conn)
            elif kind == "meta":
                payload = {"meta": meta_dict(conn)}
            else:
                self._send_json({"ok": False, "error": "not found"}, 404)
                return
            conn.commit()
        except (sqlite3.Error, json.JSONDecodeError, ValueError, TypeError, UnicodeError) as e:
            # PY-M01/R1-M01：读取/解码失败不越过 HTTP 错误边界，也不把损坏当合法空状态。
            try:
                conn.rollback()
            except sqlite3.Error:
                pass
            self._send_json({"ok": False, "error": "StorageUnavailable", "detail": str(e)}, 503)
            return
        finally:
            try:
                conn.close()
            except sqlite3.Error:
                pass
        # 序列化 + 编码也在错误边界内（R1-M02：坏持久值/BLOB/孤立 surrogate → 结构化 503）。
        body = serialize_payload({"ok": True, **payload})
        if body is None:
            self._send_json(
                {"ok": False, "error": "StorageUnavailable", "detail": "响应序列化失败（坏持久值）"},
                503,
            )
            return
        self._send_bytes(body)

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        if path in ("/", "/index.html"):
            try:
                with open(self.browser_path, "rb") as f:
                    body = f.read()
                self._send_html(body)
            except FileNotFoundError:
                self._send_json({"ok": False, "error": "browser file not found"}, 404)
            except OSError as e:
                self._send_json({"ok": False, "error": "browser unreadable", "detail": str(e)}, 503)
            return
        if path == "/api/state":
            self._read_only("state")
            return
        if path == "/api/catalog":
            self._read_only("catalog")
            return
        if path == "/api/snapshot":
            self._read_only("snapshot")
            return
        if path == "/api/meta":
            self._read_only("meta")
            return
        self._send_json({"ok": False, "error": "not found"}, 404)


def serialize_payload(obj):
    """把 dict 序列化为 UTF-8 字节；失败返回 None（调用方转 503），不抛异常越界。"""
    try:
        # #1370：损坏 Unicode 与非 JSON 数值必须进入失败边界，不能改编码后作为成功返回。
        return json.dumps(obj, ensure_ascii=False, allow_nan=False).encode("utf-8")
    except (TypeError, ValueError, UnicodeError, OverflowError):
        return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True)
    ap.add_argument("--port", type=int, default=8787)
    ap.add_argument("--browser", required=True)
    ap.add_argument("--host", default="127.0.0.1")
    args = ap.parse_args()
    Handler.db_path = os.path.abspath(args.db)
    Handler.browser_path = os.path.abspath(args.browser)
    server = ThreadingHTTPServer((args.host, args.port), Handler)
    sys.stderr.write(
        "[readonly] serving S 只读查询 {host}:{port} (db={db})\n".format(
            host=args.host, port=args.port, db=Handler.db_path
        )
    )
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
