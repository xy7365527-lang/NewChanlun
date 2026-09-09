#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""#1371 TB-01-B：S 正式结构会话的只读查询外壳（前端查询外壳，独立只读进程）。

在 #1370 TB-01-A 已合的只读外壳之上补齐 B 片：修订历史（raw_history / withdrawn_objects /
supersedes / replaces）、AsKnown（`?as_of=`）与 RecomputedWithRevision（默认）、同源 Watch
（`/api/delta?after_generation=`）。本进程仍只读 S 自己的 SQLite（`mode=ro`），不写、不重算结构、
不选择操作级别、不推算经济资格；坏持久值/损坏库准确 StorageUnavailable，不吞成成功空值。
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
        if type(key) is not str or type(value) is not str:
            raise ValueError("meta 的键和值必须都是文本")
        out[key] = value
    return out


def _scope(meta):
    if "scope" not in meta:
        return {}
    return _json_field(meta["scope"], dict)


def _num_to_str(v):
    if type(v) is not int or not -(2**63) <= v < 2**63:
        raise ValueError("持久整数必须是 i64，不能是浮点、布尔、空值或文本")
    return str(v)


def _num_or_none_to_str(v):
    if v is None:
        return None
    return _num_to_str(v)


def _json_field(text, expected):
    if not isinstance(text, str):
        raise ValueError("持久 JSON 必须是文本")
    value = json.loads(text)
    if not isinstance(value, expected):
        raise ValueError("持久 JSON 形状不符")
    return value


def _project_input_refs(refs):
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
            if "revision" in item:
                item["revision"] = _num_to_str(item.get("revision"))
            projected_refs.append(item)
        g["raw_refs"] = projected_refs
        out.append(g)
    return out


def _project_raw_bars(bars):
    if not isinstance(bars, list):
        raise ValueError("raw_bars 必须是数组")
    out = []
    for b in bars:
        if not isinstance(b, dict):
            raise ValueError("raw_bars 成员必须是对象")
        b = dict(b)
        b["seq"] = _num_to_str(b.get("seq"))
        if "revision" in b:
            b["revision"] = _num_to_str(b.get("revision"))
        out.append(b)
    return out


def read_catalog(conn):
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
            "branches": _json_field(branches_json, (list, dict)),
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


_OBJECT_COLS = (
    "object_id, object_revision, kind, batch_id, branch, dir_ab, dir_bc, window_start, "
    "window_mid, window_end, comparisons_json, input_refs_json, source_coords_json, "
    "first_known_generation, first_known_cut, published_generation, withdrawn_generation, "
    "withdrawal_reason, superseded_by"
)


def _project_object_row(row, as_of=None):
    (oid, orev, kind, batch_id, branch, dir_ab, dir_bc, ws, wm, we, cmp_json, refs_json,
     sc_json, fkg, fkc, pg, wg, wr, sb) = row
    # AsKnown：若对象在 as_of 之后才撤回（withdrawn_generation > as_of），按当时仍活动投影，
    # 不把后来的撤回元数据倒填到旧 cut。
    effective_wg = wg
    if as_of is not None and wg is not None and wg > as_of:
        effective_wg = None
    return {
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
        "source_coords": _json_field(sc_json, list),
        "first_known_generation": _num_to_str(fkg),
        "first_known_cut": fkc,
        "published_generation": _num_to_str(pg),
        "withdrawn_generation": _num_or_none_to_str(effective_wg),
        "withdrawal_reason": wr if effective_wg is not None else None,
        "superseded_by": sb if effective_wg is not None else None,
        "lifecycle": "withdrawn" if effective_wg is not None else "active",
    }


def _read_objects_view(conn, as_of):
    if as_of is None:
        active_sql = ("SELECT %s FROM objects WHERE withdrawn_generation IS NULL "
                      "ORDER BY window_start") % _OBJECT_COLS
        withdrawn_sql = ("SELECT %s FROM objects WHERE withdrawn_generation IS NOT NULL "
                         "ORDER BY first_known_generation") % _OBJECT_COLS
        active = [_project_object_row(r, None) for r in conn.execute(active_sql)]
        withdrawn = [_project_object_row(r, None) for r in conn.execute(withdrawn_sql)]
    else:
        active_sql = ("SELECT %s FROM objects WHERE first_known_generation <= ? AND "
                      "(withdrawn_generation IS NULL OR withdrawn_generation > ?) "
                      "ORDER BY window_start") % _OBJECT_COLS
        withdrawn_sql = ("SELECT %s FROM objects WHERE withdrawn_generation IS NOT NULL "
                         "AND withdrawn_generation <= ? ORDER BY first_known_generation") % _OBJECT_COLS
        active = [_project_object_row(r, as_of) for r in conn.execute(active_sql, (as_of, as_of))]
        withdrawn = [_project_object_row(r, as_of) for r in conn.execute(withdrawn_sql, (as_of,))]
    return active, withdrawn


def _frontier_at_generation(conn, gen):
    if gen is None or gen <= 0:
        return -1
    row = conn.execute(
        "SELECT input_frontier FROM structure_deltas WHERE generation = ?", (gen,)
    ).fetchone()
    return row[0] if row is not None else -1


def read_snapshot(conn, as_of=None):
    meta = meta_dict(conn)
    objects, withdrawn = _read_objects_view(conn, as_of)
    max_seq = _frontier_at_generation(conn, as_of) if as_of is not None else None

    if as_of is None:
        wit_sql = ("SELECT witness_id, object_id, slot, merged_source_index, merged_high, "
                   "merged_low, merged_open, merged_close, raw_json FROM witnesses ORDER BY object_id, slot")
        wit_rows = conn.execute(wit_sql)
    else:
        wit_sql = ("SELECT witness_id, object_id, slot, merged_source_index, merged_high, "
                   "merged_low, merged_open, merged_close, raw_json FROM witnesses "
                   "WHERE published_generation <= ? ORDER BY object_id, slot")
        wit_rows = conn.execute(wit_sql, (as_of,))
    witnesses = []
    for (wid, oid, slot, msi, mh, ml, mo, mc, raw_json) in wit_rows:
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

    if as_of is None:
        rel_sql = "SELECT subject, relation_type, object FROM relations ORDER BY subject, relation_type, object"
        rel_rows = conn.execute(rel_sql)
    else:
        rel_sql = ("SELECT subject, relation_type, object FROM relations "
                   "WHERE published_generation <= ? ORDER BY subject, relation_type, object")
        rel_rows = conn.execute(rel_sql, (as_of,))
    relations = []
    for (subj, rel, obj) in rel_rows:
        relations.append({"subject": subj, "relation_type": rel, "object": obj})

    if as_of is None:
        obs_sql = ("SELECT observation_id, batch_id, kind, window_start, window_mid, window_end, "
                   "reason, detail_json FROM observations ORDER BY kind, window_start")
        obs_rows = conn.execute(obs_sql)
    else:
        obs_sql = ("SELECT observation_id, batch_id, kind, window_start, window_mid, window_end, "
                   "reason, detail_json FROM observations WHERE published_generation <= ? "
                   "ORDER BY kind, window_start")
        obs_rows = conn.execute(obs_sql, (as_of,))
    observations = []
    for (oid, batch_id, kind, ws, wm, we, reason, detail_json) in obs_rows:
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

    if max_seq is None:
        rh_sql = ("SELECT identity_key, revision, input_revision, payload_hash, receipt_id, seq, "
                  "source_namespace, source_epoch, instrument, event_id, received_at, raw_text, "
                  "price, ts, volume, source_coord, supersedes_revision FROM raw_events ORDER BY seq ASC")
        rh_rows = conn.execute(rh_sql)
    else:
        rh_sql = ("SELECT identity_key, revision, input_revision, payload_hash, receipt_id, seq, "
                  "source_namespace, source_epoch, instrument, event_id, received_at, raw_text, "
                  "price, ts, volume, source_coord, supersedes_revision FROM raw_events "
                  "WHERE seq <= ? ORDER BY seq ASC")
        rh_rows = conn.execute(rh_sql, (max_seq,))
    raw_history = []
    for (ikey, rev, irev, ph, rid, seq, sns, sepoch, instr, eid, received, raw_text,
         price, ts, volume, coord, sup) in rh_rows:
        raw_history.append({
            "identity_key": ikey,
            "revision": _num_to_str(rev),
            "input_revision": irev,
            "payload_hash": ph,
            "receipt_id": rid,
            "seq": _num_to_str(seq),
            "source_namespace": sns,
            "source_epoch": sepoch,
            "instrument": instr,
            "event_id": eid,
            "received_at": received,
            "raw_text": raw_text,
            "price": price,
            "ts": ts,
            "volume": volume,
            "source_coord": coord,
            "supersedes_revision": _num_or_none_to_str(sup),
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
        "history_mode": "AsKnown" if as_of is not None else "RecomputedWithRevision",
        "as_of_generation": None if as_of is None else _num_to_str(as_of),
        "objects": objects,
        "withdrawn_objects": withdrawn,
        "witnesses": witnesses,
        "relations": relations,
        "observations": observations,
        "raw_history": raw_history,
    }


def read_state(conn, as_of=None):
    catalog = read_catalog(conn)
    snapshot = read_snapshot(conn, as_of)
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


def read_delta(conn, after_generation):
    meta = meta_dict(conn)
    try:
        current_gen = int(meta.get("generation", "0"))
    except ValueError:
        current_gen = 0
    current_cut = meta.get("structure_cut", "")
    rows = []
    for (gen, sid, crev, base_cut, next_cut, seq_range_json, idx_frontier, input_frontier, delta_json) in conn.execute(
        "SELECT generation, session_id, catalog_revision, base_cut, next_cut, seq_range_json, "
        "index_frontier, input_frontier, delta_json FROM structure_deltas WHERE generation > ? ORDER BY generation ASC",
        (after_generation,),
    ):
        rows.append({
            "generation": _num_to_str(gen),
            "session_id": sid,
            "catalog_revision": crev,
            "base_cut": base_cut,
            "next_cut": next_cut,
            "seq_range": _json_field(seq_range_json, dict),
            "index_frontier": idx_frontier,
            "input_frontier": _num_to_str(input_frontier),
            "delta": _json_field(delta_json, dict),
        })
    gap = None
    if 0 <= after_generation < current_gen:
        first = None
        if rows:
            try:
                first = int(rows[0]["generation"])
            except ValueError:
                first = None
        if first != after_generation + 1:
            gap = {
                "reason": "cursor_stale_or_retained_delta_missing",
                "rebuild_cut": current_cut,
            }
    return {
        "session_id": meta.get("session_id", ""),
        "generation": _num_to_str(current_gen),
        "structure_cut": current_cut,
        "after_generation": _num_to_str(after_generation),
        "gap": gap,
        "deltas": rows,
    }


class Handler(BaseHTTPRequestHandler):
    db_path = None
    browser_path = None

    def log_message(self, fmt, *args):
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
        body = serialize_payload(obj)
        if body is None:
            body = b'{"ok": false, "error": "StorageUnavailable"}'
            status = 503
        self._send_bytes(body, status)

    def _parse_query(self):
        query = {}
        if "?" in self.path:
            qs = self.path.split("?", 1)[1]
            for pair in qs.split("&"):
                if "=" in pair:
                    k, v = pair.split("=", 1)
                    query[k] = v
                elif pair:
                    query[pair] = ""
        return query

    def _read_only(self, kind, query=None):
        query = query or {}
        payload = None
        try:
            conn = open_readonly(self.db_path)
        except Exception as e:
            self._send_json({"ok": False, "error": "StorageUnavailable", "detail": str(e)}, 503)
            return
        try:
            conn.execute("BEGIN")
            if kind == "state":
                as_of = self._parse_as_of(query)
                payload = read_state(conn, as_of)
            elif kind == "catalog":
                payload = read_catalog(conn)
            elif kind == "snapshot":
                as_of = self._parse_as_of(query)
                payload = read_snapshot(conn, as_of)
            elif kind == "delta":
                payload = read_delta(conn, self._parse_after_generation(query))
            elif kind == "meta":
                payload = {"meta": meta_dict(conn)}
            else:
                self._send_json({"ok": False, "error": "not found"}, 404)
                return
            conn.commit()
        except (sqlite3.Error, json.JSONDecodeError, ValueError, TypeError, UnicodeError) as e:
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
        body = serialize_payload({"ok": True, **payload})
        if body is None:
            self._send_json(
                {"ok": False, "error": "StorageUnavailable", "detail": "响应序列化失败（坏持久值）"},
                503,
            )
            return
        self._send_bytes(body)

    def _parse_as_of(self, query):
        raw = query.get("as_of")
        if raw is None:
            return None
        if raw == "":
            raise ValueError("as_of 不能为空")
        if not raw.lstrip("-").isdigit():
            raise ValueError("as_of 必须是规范十进制整数")
        v = int(raw)
        if not -(2**63) <= v < 2**63:
            raise ValueError("as_of 超出 i64 精确整数域")
        return v

    def _parse_after_generation(self, query):
        raw = query.get("after_generation")
        if raw is None:
            return -1
        if not raw.lstrip("-").isdigit():
            raise ValueError("after_generation 必须是规范十进制整数")
        v = int(raw)
        if not -(2**63) <= v < 2**63:
            raise ValueError("after_generation 超出 i64 精确整数域")
        return v

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        query = self._parse_query()
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
            self._read_only("state", query)
            return
        if path == "/api/catalog":
            self._read_only("catalog", query)
            return
        if path == "/api/snapshot":
            self._read_only("snapshot", query)
            return
        if path == "/api/delta":
            self._read_only("delta", query)
            return
        if path == "/api/meta":
            self._read_only("meta", query)
            return
        self._send_json({"ok": False, "error": "not found"}, 404)


def serialize_payload(obj):
    try:
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
