#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""#1371 TB-01-B：S 正式结构会话的只读查询外壳（前端查询外壳，独立只读进程）。

在 #1370 TB-01-A 只读外壳之上补齐 B 片：修订历史（raw_history / withdrawn_objects / supersedes /
replaces）、AsKnown（`?as_of=`）与 RecomputedWithRevision（默认）、同源 Watch（`/api/delta?after_generation=`）。
本进程只读 S 自己的 SQLite（`mode=ro`），不写、不重算结构、不选择操作级别、不推算经济资格。

读取边界（延续 A）：参数错误 → 400 InvalidQuery；持久值损坏/缺失 → 503 StorageUnavailable；
不把坏 generation/frontier/Delta/query 静默回退成 0/-1/空并 200。
"""

import argparse
import json
import os
import sqlite3
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qsl, urlsplit


class InvalidQuery(ValueError):
    """请求参数错误（非持久损坏），映射 HTTP 400。"""


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


def _is_canonical_dec(s):
    """ASCII 规范十进制整数（无 +、无空白、无前导零、无 Unicode 数字）。"""
    if type(s) is not str or s == "":
        return False
    if not s.isascii() or not s.isdigit():
        return False
    if len(s) > 1 and s[0] == "0":
        return False
    return True


def _canonical_i64_str(s, what):
    if not _is_canonical_dec(s):
        raise ValueError("%s 不是规范十进制整数（要求 ASCII、无前导零）" % what)
    n = int(s)
    if not -(2**63) <= n < 2**63:
        raise ValueError("%s 超出 i64 精确整数域" % what)
    return n


def _canonical_gen(v):
    """meta.generation 必填并校验规范十进制（缺失/损坏 → 503，不默认 0）。"""
    if type(v) is not str or v == "":
        raise ValueError("meta.generation 缺失或非文本")
    return _canonical_i64_str(v, "meta.generation")


def _frontier_i64(v):
    if type(v) is not int or not -(2**63) <= v < 2**63:
        raise ValueError("input_frontier 必须是 i64，不能是浮点/布尔/文本")
    # 正式域：-1 仅初态/空输入；其余必须是非负接纳前沿。
    if v < -1:
        raise ValueError("input_frontier 越域：只允许 -1（初态）或非负接纳前沿")
    return v


def _verify_batch_integrity(conn, batch_id):
    import hashlib
    if type(batch_id) is not str or not batch_id.startswith("batch-") or len(batch_id) != 6 + 64:
        raise ValueError("batch_id 非规范内容寻址形态")
    row = conn.execute(
        "SELECT canonical_bytes, byte_len FROM batches WHERE batch_id = ?", (batch_id,)
    ).fetchone()
    if row is None:
        raise ValueError("可达 batch %s 不存在（引用不完整）" % batch_id)
    b, l = row
    if not isinstance(b, (bytes, bytearray)) or type(l) is not int:
        raise ValueError("batch 内容/长度类型损坏")
    if hashlib.sha256(bytes(b)).hexdigest() != batch_id[6:] or l != len(b):
        raise ValueError("batch 规范字节/长度与内容寻址不符")


def _verify_reachable_root(conn):
    """reader 与 writer 共享的可达根/元数据/代际链校验（坏根/坏链 → 503）。"""
    meta = meta_dict(conn)
    gen = _validate_required_meta(meta)
    if gen == 0:
        cnt = conn.execute("SELECT COUNT(*) FROM structure_deltas").fetchone()[0]
        if cnt != 0:
            raise ValueError("初态 generation=0 却存在持久 Delta")
        return
    idx = meta["index_frontier"]
    _verify_batch_integrity(conn, idx)
    rows = conn.execute(
        "SELECT generation, index_frontier, input_frontier FROM structure_deltas "
        "WHERE generation <= ? ORDER BY generation ASC", (gen,)
    ).fetchall()
    if len(rows) != gen:
        raise ValueError("已发布代链断裂（generation=%d，仅 %d 行 Delta）" % (gen, len(rows)))
    pub_raw = meta["last_advance_frontier"]
    pub = -1 if pub_raw == "-1" else _canonical_i64_str(pub_raw, "meta.last_advance_frontier")
    for i, (g, didx, df) in enumerate(rows):
        if g != i + 1:
            raise ValueError("Delta 代际断链（第 %d 行 generation=%s）" % (i + 1, g))
        if type(didx) is not str or didx == "":
            raise ValueError("delta[%d].index_frontier 为空" % g)
        _verify_batch_integrity(conn, didx)
        if g == gen and _frontier_i64(df) != pub:
            raise ValueError("meta.last_advance_frontier 与 delta[%d].input_frontier 不一致" % g)


def _req_str(d, key, path):
    if not isinstance(d, dict) or type(d.get(key)) is not str:
        raise ValueError("%s.%s 缺失或非文本" % (path, key))
    return d[key]


def _req_str_or_none(d, key, path):
    v = d.get(key)
    if v is not None and type(v) is not str:
        raise ValueError("%s.%s 必须是文本或 null" % (path, key))
    return v


def _validate_required_meta(meta):
    """所有结构读取入口共享的必需 meta 校验：缺失/损坏 → 503，不默认空串/0。"""
    sid = meta.get("session_id")
    if type(sid) is not str or sid == "":
        raise ValueError("meta.session_id 缺失或非文本")
    gen = _canonical_gen(meta.get("generation"))
    cut = meta.get("structure_cut")
    if type(cut) is not str or cut == "":
        raise ValueError("meta.structure_cut 缺失或非文本")
    cat = meta.get("catalog_revision")
    if type(cat) is not str or cat == "":
        raise ValueError("meta.catalog_revision 缺失或非文本")
    idx = meta.get("index_frontier")
    if type(idx) is not str:
        raise ValueError("meta.index_frontier 缺失或非文本")
    if gen > 0 and idx == "":
        raise ValueError("meta.index_frontier 为空但 generation>0（缺根 meta）")
    pub_raw = meta.get("last_advance_frontier")
    if pub_raw == "-1":
        pub_n = -1
    else:
        pub_n = _canonical_i64_str(pub_raw, "meta.last_advance_frontier")
    if pub_n < -1:
        raise ValueError("meta.last_advance_frontier 越域：只允许 -1 或非负")
    return gen


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


def _header_at_generation(conn, gen):
    """历史 cut 头：返回 (index_frontier, catalog_revision)；gen==0 → ("", 当前 catalog_revision)。"""
    if gen == 0:
        return "", meta_dict(conn).get("catalog_revision", "")
    row = conn.execute(
        "SELECT index_frontier, catalog_revision FROM structure_deltas WHERE generation = ?", (gen,)
    ).fetchone()
    if row is None:
        raise ValueError("Unavailable：请求的 cut generation=%d 尚未发布（无对应持久 Delta）" % gen)
    if type(row[0]) is not str or type(row[1]) is not str:
        raise ValueError("structure_deltas 历史头损坏（非文本）")
    return row[0], row[1]


def _frontier_at_generation(conn, gen):
    """指定 generation 的输入前沿；gen==0 → -1；delta 缺失 → 显式不可用（不默认 -1 泄漏空/未来）。"""
    if gen == 0:
        return -1
    row = conn.execute(
        "SELECT input_frontier FROM structure_deltas WHERE generation = ?", (gen,)
    ).fetchone()
    if row is None:
        raise ValueError("Unavailable：请求的 cut generation=%d 尚未发布（无对应持久 Delta）" % gen)
    return _frontier_i64(row[0])


def read_catalog(conn, as_of=None):
    meta = meta_dict(conn)
    _verify_reachable_root(conn)
    current_gen = _validate_required_meta(meta)
    if as_of is None:
        header_gen = meta.get("generation", "")
        header_cut = meta.get("structure_cut", "")
        header_cat = meta.get("catalog_revision", "")
        cc006_run = None
        cc006_evidence = None
    else:
        if as_of < 0:
            raise InvalidQuery("as_of 必须 >= 0")
        if as_of > current_gen:
            raise ValueError("Unavailable：请求的 cut generation=%d 尚未发布" % as_of)
        idx, cat = _header_at_generation(conn, as_of)
        header_gen = str(as_of)
        header_cut = "cut-%d" % as_of
        header_cat = cat
        if as_of == 0:
            cc006_run, cc006_evidence = "not_run", {}
        else:
            row = conn.execute(
                "SELECT catalog_run_status, catalog_evidence_json FROM structure_deltas WHERE generation = ?",
                (as_of,),
            ).fetchone()
            if row is None:
                raise ValueError("Unavailable：请求的 cut generation=%d 尚未发布" % as_of)
            cc006_run = row[0]
            cc006_evidence = _json_field(row[1], dict)

    rows = []
    for (cid, kind, title, domain, branches_json, impl, proof, run, evidence_json) in conn.execute(
        "SELECT catalog_id, kind, title, domain, branches_json, impl_status, proof_status, "
        "run_status, evidence_json FROM catalog ORDER BY catalog_id"
    ):
        run_status = run
        evidence = _json_field(evidence_json, dict)
        if cid == "CC-006" and cc006_run is not None:
            run_status = cc006_run
            evidence = cc006_evidence
        rows.append({
            "id": cid,
            "kind": kind,
            "title": title,
            "domain": domain,
            "branches": _json_field(branches_json, (list, dict)),
            "implementation_status": impl,
            "proof_status": proof,
            "run_status": run_status,
            "evidence": evidence,
        })
    return {
        "session_id": meta.get("session_id", ""),
        "generation": header_gen,
        "structure_cut": header_cut,
        "catalog_revision": header_cat,
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
    invalid_lifecycle = conn.execute(
        "SELECT COUNT(*) FROM objects WHERE withdrawn_generation IS NOT NULL "
        "AND withdrawn_generation <= first_known_generation"
    ).fetchone()[0]
    if invalid_lifecycle != 0:
        raise ValueError("对象生命周期非法（withdrawn_generation <= first_known_generation）")
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


def read_snapshot(conn, as_of=None):
    meta = meta_dict(conn)
    _verify_reachable_root(conn)
    current_gen = _validate_required_meta(meta)
    objects, withdrawn = _read_objects_view(conn, as_of)

    if as_of is None:
        header_gen = meta.get("generation", "")
        header_cut = meta.get("structure_cut", "")
        header_cat = meta.get("catalog_revision", "")
        header_idx = meta.get("index_frontier", "")
        # current：raw_history 只到已发布前沿（未 Advance 的接纳日志不得混入当前 cut）。
        published = meta.get("last_advance_frontier", "")
        if published == "":
            raise ValueError("meta.last_advance_frontier 缺失")
        try:
            max_seq = _frontier_i64(int(published))
        except ValueError:
            raise ValueError("meta.last_advance_frontier 不是规范十进制整数")
    else:
        if as_of < 0:
            raise InvalidQuery("as_of 必须 >= 0")
        if as_of > current_gen:
            raise ValueError("Unavailable：请求的 cut generation=%d 尚未发布" % as_of)
        header_idx, header_cat = _header_at_generation(conn, as_of)
        header_gen = str(as_of)
        header_cut = "cut-%d" % as_of
        max_seq = _frontier_at_generation(conn, as_of)

    # 见证/关系/观察按发布代际过滤（AsKnown）；current 全量。
    def _exec(sql, params=()):
        return conn.execute(sql, params)

    if as_of is None:
        wit_rows = _exec("SELECT witness_id, object_id, slot, merged_source_index, merged_high, "
                         "merged_low, merged_open, merged_close, raw_json FROM witnesses ORDER BY object_id, slot")
    else:
        wit_rows = _exec("SELECT witness_id, object_id, slot, merged_source_index, merged_high, "
                         "merged_low, merged_open, merged_close, raw_json FROM witnesses "
                         "WHERE published_generation <= ? ORDER BY object_id, slot", (as_of,))
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
        rel_rows = _exec("SELECT subject, relation_type, object FROM relations ORDER BY subject, relation_type, object")
    else:
        rel_rows = _exec("SELECT subject, relation_type, object FROM relations "
                         "WHERE published_generation <= ? ORDER BY subject, relation_type, object", (as_of,))
    relations = [{"subject": s, "relation_type": t, "object": o} for (s, t, o) in rel_rows]

    if as_of is None:
        obs_rows = _exec("SELECT observation_id, batch_id, kind, window_start, window_mid, window_end, "
                         "reason, detail_json FROM observations ORDER BY kind, window_start")
    else:
        obs_rows = _exec("SELECT observation_id, batch_id, kind, window_start, window_mid, window_end, "
                         "reason, detail_json FROM observations WHERE published_generation <= ? "
                         "ORDER BY kind, window_start", (as_of,))
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

    if max_seq is None or max_seq < 0:
        rh_rows = []
    else:
        rh_rows = conn.execute(
            "SELECT identity_key, revision, input_revision, payload_hash, receipt_id, seq, "
            "source_namespace, source_epoch, instrument, event_id, received_at, raw_text, "
            "price, ts, volume, source_coord, supersedes_revision FROM raw_events "
            "WHERE seq <= ? ORDER BY seq ASC", (max_seq,))
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
        "generation": header_gen,
        "structure_cut": header_cut,
        "catalog_revision": header_cat,
        "scope": _scope(meta),
        "index_frontier": header_idx,
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
    catalog = read_catalog(conn, as_of)
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


def _canonical_int_field(d, key, path):
    v = _req_str(d, key, path)
    _canonical_i64_str(v, "%s.%s" % (path, key))
    return v


def _validate_delta_shape(delta):
    """持久 Delta 逐字段形状 + 精确整数 wire 校验：任何坏持久值 → 503，不以成功 wire 传出。"""
    if not isinstance(delta, dict):
        raise ValueError("delta_json 顶层必须是对象")
    for key in ("upserts", "withdrawals", "replaces", "witnesses", "relations",
                "observations", "raw_history_added"):
        if key in delta and not isinstance(delta[key], list):
            raise ValueError("delta.%s 必须是数组" % key)
    if not isinstance(delta.get("seq_range"), dict):
        raise ValueError("delta.seq_range 必须是对象")
    for _k in ("from", "to"):
        _v = _req_str(delta["seq_range"], _k, "delta.seq_range")
        if _v == "-1":
            continue
        _canonical_i64_str(_v, "delta.seq_range." + _k)
    _req_str(delta, "base_cut", "delta")
    _req_str(delta, "next_cut", "delta")
    _req_str(delta, "generation", "delta")
    _req_str(delta, "index_frontier", "delta")
    _req_str(delta, "session_id", "delta")
    _req_str(delta, "catalog_revision", "delta")
    # 内层 input_frontier 为必需精确整数（允许 -1 空输入）；不得 null。
    _if_raw = delta.get("input_frontier")
    if _if_raw == "-1":
        pass
    else:
        _canonical_i64_str(_if_raw, "delta.input_frontier")

    for i, u in enumerate(delta.get("upserts", [])):
        p = "delta.upserts[%d]" % i
        if not isinstance(u, dict):
            raise ValueError("%s 必须是对象" % p)
        for k in ("object_id", "kind", "batch_id", "branch", "dir_ab", "dir_bc",
                  "first_known_cut"):
            _req_str(u, k, p)
        for k in ("object_revision", "window_start", "window_mid", "window_end",
                  "first_known_generation", "published_generation"):
            _canonical_int_field(u, k, p)
        if u.get("lifecycle") not in ("active", "withdrawn"):
            raise ValueError("%s.lifecycle 必须是 active/withdrawn" % p)
        # 生命周期一致性：withdrawn ⟺ withdrawn_generation 非空且 > first_known_generation；
        # active ⟹ 无撤回代际/理由/替代。
        wg = u.get("withdrawn_generation")
        if u["lifecycle"] == "withdrawn":
            if wg is None:
                raise ValueError("%s.lifecycle=withdrawn 但 withdrawn_generation 为空" % p)
            _canonical_i64_str(wg, p + ".withdrawn_generation")
            if u.get("withdrawal_reason") is None or u.get("superseded_by") is None:
                raise ValueError("%s.lifecycle=withdrawn 但 withdrawal_reason/superseded_by 缺失" % p)
        else:
            if wg is not None or u.get("withdrawal_reason") is not None or u.get("superseded_by") is not None:
                raise ValueError("%s.lifecycle=active 却带撤回代际/理由/替代" % p)
        if not isinstance(u.get("comparisons"), list):
            raise ValueError("%s.comparisons 必须是数组" % p)
        if not isinstance(u.get("input_refs"), list):
            raise ValueError("%s.input_refs 必须是数组" % p)
        for g in u["input_refs"]:
            if not isinstance(g, dict):
                raise ValueError("%s.input_refs 成员必须是对象" % p)
            _req_str(g, "merged_index", p + ".input_refs")
            if not isinstance(g.get("raw_refs"), list):
                raise ValueError("%s.input_refs.raw_refs 必须是数组" % p)
            for rr in g["raw_refs"]:
                if not isinstance(rr, dict):
                    raise ValueError("%s.input_refs.raw_refs 成员必须是对象" % p)
                for k in ("identity_key", "receipt_id", "event_id", "input_revision",
                          "source_coord"):
                    _req_str(rr, k, p + ".input_refs.raw_refs")
                _canonical_int_field(rr, "revision", p + ".input_refs.raw_refs")
                _canonical_int_field(rr, "seq", p + ".input_refs.raw_refs")
        if not isinstance(u.get("source_coords"), list) or any(type(s) is not str for s in u["source_coords"]):
            raise ValueError("%s.source_coords 必须是字符串数组" % p)
        if u.get("withdrawn_generation") is not None:
            _canonical_int_field(u, "withdrawn_generation", p)
        _req_str_or_none(u, "withdrawal_reason", p)
        _req_str_or_none(u, "superseded_by", p)

    for i, w in enumerate(delta.get("withdrawals", [])):
        p = "delta.withdrawals[%d]" % i
        if not isinstance(w, dict):
            raise ValueError("%s 必须是对象" % p)
        _req_str(w, "object_id", p)
        _req_str(w, "reason", p)
        for k in ("window_start", "window_mid", "window_end"):
            _canonical_int_field(w, k, p)
        _req_str_or_none(w, "superseded_by", p)

    for i, r in enumerate(delta.get("replaces", [])):
        p = "delta.replaces[%d]" % i
        if not isinstance(r, dict):
            raise ValueError("%s 必须是对象" % p)
        _req_str(r, "new_object_id", p)
        _req_str(r, "old_object_id", p)

    for i, w in enumerate(delta.get("witnesses", [])):
        p = "delta.witnesses[%d]" % i
        if not isinstance(w, dict):
            raise ValueError("%s 必须是对象（不得为 null）" % p)
        for k in ("witness_id", "object_id", "merged_high", "merged_low", "merged_open",
                  "merged_close"):
            _req_str(w, k, p)
        _canonical_int_field(w, "slot", p)
        _canonical_int_field(w, "merged_source_index", p)
        if not isinstance(w.get("raw_bars"), list):
            raise ValueError("%s.raw_bars 必须是数组" % p)
        for j, rb in enumerate(w["raw_bars"]):
            if not isinstance(rb, dict):
                raise ValueError("%s.raw_bars[%d] 必须是对象（不得为 null/标量）" % (p, j))
            _canonical_int_field(rb, "seq", "%s.raw_bars[%d]" % (p, j))
            if "revision" in rb:
                _canonical_int_field(rb, "revision", "%s.raw_bars[%d]" % (p, j))

    for i, r in enumerate(delta.get("relations", [])):
        p = "delta.relations[%d]" % i
        if not isinstance(r, dict):
            raise ValueError("%s 必须是对象" % p)
        _req_str(r, "subject", p)
        _req_str(r, "relation_type", p)
        _req_str(r, "object", p)

    for i, o in enumerate(delta.get("observations", [])):
        p = "delta.observations[%d]" % i
        if not isinstance(o, dict):
            raise ValueError("%s 必须是对象（不得为标量）" % p)
        _req_str(o, "observation_id", p)
        _req_str(o, "batch_id", p)
        _req_str(o, "kind", p)
        _req_str(o, "reason", p)
        if not isinstance(o.get("detail"), dict):
            raise ValueError("%s.detail 必须是对象" % p)
        for k in ("window_start", "window_mid", "window_end"):
            if o.get(k) is not None:
                _canonical_int_field(o, k, p)

    for i, rh in enumerate(delta.get("raw_history_added", [])):
        p = "delta.raw_history_added[%d]" % i
        if not isinstance(rh, dict):
            raise ValueError("%s 必须是对象" % p)
        for k in ("identity_key", "input_revision", "payload_hash", "receipt_id",
                  "event_id", "source_coord", "price"):
            _req_str(rh, k, p)
        _canonical_int_field(rh, "revision", p)
        _canonical_int_field(rh, "seq", p)
        _req_str_or_none(rh, "supersedes_revision", p)


def _validate_seq_range(seq_range_json):
    sr = _json_field(seq_range_json, dict)
    for k in ("from", "to"):
        v = _req_str(sr, k, "seq_range")
        # 空输入发布的合法前沿 -1（to）与首代 from=0；-1 是合法前沿哨兵。
        if v == "-1":
            continue
        _canonical_i64_str(v, "seq_range." + k)
    return sr


def read_delta(conn, after_generation):
    meta = meta_dict(conn)
    _verify_reachable_root(conn)
    current_gen = _validate_required_meta(meta)
    current_cut = meta["structure_cut"]
    session_id = meta["session_id"]

    if after_generation > current_gen:
        # 游标超前（旧页面游标用于新会话/被重建的会话）→ 显式重建，不静默“无更新”。
        return {
            "session_id": session_id,
            "generation": _num_to_str(current_gen),
            "structure_cut": current_cut,
            "after_generation": _num_to_str(after_generation),
            "gap": {"reason": "cursor_ahead_or_session_rebuilt", "rebuild_cut": current_cut},
            "deltas": [],
        }

    rows = []
    for (gen, sid, crev, base_cut, next_cut, seq_range_json, idx_frontier, input_frontier,
         run_status, evidence_json, delta_json) in conn.execute(
        "SELECT generation, session_id, catalog_revision, base_cut, next_cut, seq_range_json, "
        "index_frontier, input_frontier, catalog_run_status, catalog_evidence_json, delta_json "
        "FROM structure_deltas WHERE generation > ? ORDER BY generation ASC",
        (after_generation,),
    ):
        if gen > current_gen:
            raise ValueError("structure_deltas 存在超出当前 generation 的未来行（generation=%d）" % gen)
        if sid != session_id:
            raise ValueError("structure_deltas 行的 session_id 与 meta.session_id 不一致")
        delta = _json_field(delta_json, dict)
        _validate_delta_shape(delta)
        sr = _validate_seq_range(seq_range_json)
        if type(run_status) is not str or run_status == "":
            raise ValueError("structure_deltas.catalog_run_status 缺失或非文本")
        evidence = _json_field(evidence_json, dict)
        # 内层头与外层行/当前 meta 同一身份（冲突 → 503，不作为正常增量发布）。
        if delta.get("session_id") != sid:
            raise ValueError("delta 内层 session_id 与外层行不一致")
        if delta.get("generation") != str(gen):
            raise ValueError("delta 内层 generation 与外层行不一致")
        if delta.get("base_cut") != base_cut:
            raise ValueError("delta 内层 base_cut 与外层行不一致")
        if delta.get("next_cut") != next_cut:
            raise ValueError("delta 内层 next_cut 与外层行不一致")
        if delta.get("catalog_revision") != crev:
            raise ValueError("delta 内层 catalog_revision 与外层行不一致")
        if delta.get("index_frontier") != idx_frontier:
            raise ValueError("delta 内层 index_frontier 与外层行不一致")
        if delta.get("seq_range") != sr:
            raise ValueError("delta 内层 seq_range 与外层 seq_range_json 不一致")
        if delta.get("catalog_run_status") != run_status:
            raise ValueError("delta 内层 catalog_run_status 与外层列不一致")
        if delta.get("catalog_evidence") != evidence:
            raise ValueError("delta 内层 catalog_evidence 与外层 catalog_evidence_json 不一致")
        rows.append({
            "generation": _num_to_str(gen),
            "session_id": sid,
            "catalog_revision": crev,
            "base_cut": base_cut,
            "next_cut": next_cut,
            "seq_range": sr,
            "index_frontier": idx_frontier,
            "input_frontier": _frontier_i64(input_frontier),
            "catalog_run_status": run_status,
            "catalog_evidence": evidence,
            "delta": delta,
        })

    gap = None
    if after_generation < current_gen:
        # 完整连续性 + 末端抵达 current_gen：首条 gen == after+1，逐条 +1，末条 == current_gen。
        gens = [int(r["generation"]) for r in rows]
        expected = list(range(after_generation + 1, current_gen + 1))
        if gens != expected:
            gap = {"reason": "cursor_stale_or_retained_delta_missing", "rebuild_cut": current_cut}
        else:
            # cut 链完整性：首条 base_cut == cut-{after}，逐条 next == 下一 base，末条 next == 当前 cut。
            # 断裂是持久损坏 → 503（不是 Gap）。
            prev_cut = "cut-%d" % after_generation
            for r in rows:
                if r["base_cut"] != prev_cut:
                    raise ValueError(
                        "structure_deltas cut 链断裂：期望 base_cut=%s，实际 %s（generation=%s）"
                        % (prev_cut, r["base_cut"], r["generation"]))
                prev_cut = r["next_cut"]
            if prev_cut != current_cut:
                raise ValueError(
                    "structure_deltas cut 链末端不一致：末 next_cut=%s，当前 structure_cut=%s"
                    % (prev_cut, current_cut))
    return {
        "session_id": session_id,
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
        """标准 URL/query 解码；保留重复项供后续拒绝歧义。"""
        qs = urlsplit(self.path).query
        return parse_qsl(qs, keep_blank_values=True)

    def _query_map(self, pairs):
        out = {}
        for k, v in pairs:
            if k in out:
                raise InvalidQuery("重复参数 %s 不被允许" % k)
            out[k] = v
        return out

    def _parse_nonneg(self, query, key, default):
        raw = query.get(key)
        if raw is None:
            return default
        # ASCII 规范十进制：拒绝空、非 ASCII 数字（如 ²）、前导零、负号。
        if raw == "" or not raw.isascii() or not raw.isdigit() or (len(raw) > 1 and raw[0] == "0"):
            raise InvalidQuery("%s 必须是规范十进制整数且 >= 0" % key)
        v = int(raw)
        if not -(2**63) <= v < 2**63:
            raise InvalidQuery("%s 超出 i64 精确整数域" % key)
        return v

    def _read_only(self, kind, query=None, as_of=None, after_generation=None):
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
                payload = read_state(conn, as_of)
            elif kind == "catalog":
                payload = read_catalog(conn, as_of)
            elif kind == "snapshot":
                payload = read_snapshot(conn, as_of)
            elif kind == "delta":
                payload = read_delta(conn, after_generation)
            elif kind == "meta":
                payload = {"meta": meta_dict(conn)}
            else:
                self._send_json({"ok": False, "error": "not found"}, 404)
                return
            conn.commit()
        except InvalidQuery as e:
            try:
                conn.rollback()
            except sqlite3.Error:
                pass
            self._send_json({"ok": False, "error": "InvalidQuery", "detail": str(e)}, 400)
            return
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

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        try:
            pairs = self._parse_query()
            query = self._query_map(pairs)
            # 参数解析在开库前完成：参数错误 → 400（缺库也先报参数错误），不把参数错误当存储故障。
            as_of = self._parse_nonneg(query, "as_of", None)
            after_generation = self._parse_nonneg(query, "after_generation", 0)
        except InvalidQuery as e:
            self._send_json({"ok": False, "error": "InvalidQuery", "detail": str(e)}, 400)
            return
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
            self._read_only("state", query, as_of=as_of)
            return
        if path == "/api/catalog":
            self._read_only("catalog", query, as_of=as_of)
            return
        if path == "/api/snapshot":
            self._read_only("snapshot", query, as_of=as_of)
            return
        if path == "/api/delta":
            self._read_only("delta", query, after_generation=after_generation)
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
