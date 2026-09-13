#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""#1371/#1372：S 正式结构会话的独立只读查询进程。

在 #1370 TB-01-A 只读外壳之上补齐 B 片：修订历史（raw_history / withdrawn_objects / supersedes /
replaces）、AsKnown（`?as_of=`）与 RecomputedWithRevision（默认）、同源 Watch（`/api/delta?after_generation=`）。
本进程只读 S 自己的 SQLite（`mode=ro`），不写、不重算结构、不选择操作级别、不推算经济资格。

读取边界（延续 A）：参数错误 → 400 InvalidQuery；持久值损坏/缺失 → 503 StorageUnavailable；
不把坏 generation/frontier/Delta/query 静默回退成 0/-1/空并 200。
v2 增加同源全域审计、固定 cut 分页和有界 Watch；所有生产读路由共用严格根。
"""

import argparse
import json
import os
import sqlite3
import sys
import importlib.util
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qsl, urlsplit


class InvalidQuery(ValueError):
    """请求参数错误（非持久损坏），映射 HTTP 400。"""


_QUERY_MODULES = {}


def _query_module(name):
    """兼容正式脚本入口和既有按绝对路径 importlib 加载的 R13/R14 验证器。"""
    if name not in ("s_query_integrity", "s_query"):
        raise ValueError("未知本地查询模块")
    if name not in _QUERY_MODULES:
        path = Path(__file__).resolve().with_name(name + ".py")
        spec = importlib.util.spec_from_file_location(name, path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        _QUERY_MODULES[name] = module
    return _QUERY_MODULES[name]


def _query_integrity():
    return _query_module("s_query_integrity")


def open_readonly(db_path, *, check_same_thread=True):
    """PY-H01：URI 编码 + query_only。"""
    uri = Path(db_path).resolve().as_uri() + "?mode=ro"
    conn = sqlite3.connect(uri, uri=True, check_same_thread=check_same_thread)
    conn.execute("PRAGMA query_only=ON")
    return conn


def meta_dict(conn):
    out = {}
    for key, value in conn.execute("SELECT key, value FROM meta"):
        if type(key) is not str or type(value) is not str:
            raise ValueError("meta 的键和值必须都是文本")
        out[key] = value
    return out


def _validate_scope(scope):
    if scope != {"structure": "CompleteCut", "economic": "not_started"}:
        raise ValueError("scope 与本片结构/经济未启动声明不符")
    return scope


def _scope(meta):
    if "scope" not in meta:
        raise ValueError("meta.scope 缺失")
    return _validate_scope(_json_field(meta["scope"], dict))


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
    return _query_integrity().parse_json(text, expected)


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
    if len(s) > 19:
        raise ValueError("%s 超出 i64 精确整数域" % what)
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


def _legacy_payload_projection(value):
    if type(value) is list:
        return [_legacy_payload_projection(v) for v in value]
    if type(value) is dict:
        result = {}
        for k, v in value.items():
            if k == "detail":
                result[k] = _project_wire_integers(v)
            elif k == "raw_bars" and type(v) is list:
                result[k] = [dict(bar, supersedes_revision=_project_wire_integers(bar["supersedes_revision"]))
                             if type(bar) is dict and "supersedes_revision" in bar else bar for bar in v]
            else:
                result[k] = _legacy_payload_projection(v)
        return result
    return value


def _observation_id(ob):
    import hashlib
    content = {k: ob[k] for k in ("kind", "window_start", "window_mid", "window_end", "reason")}
    b = json.dumps(content, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()
    return "obs-" + hashlib.sha256(b).hexdigest()[:16]


def _verify_batch_profile(batch, binding):
    if type(batch.get("profile_id")) is not str or type(batch.get("profile_hash")) is not str:
        raise ValueError("batch.profile_id/hash缺失")
    if batch["profile_id"]==batch["profile_hash"]=="" and batch.get("raw_events")==[]:
        return
    if (batch["profile_id"],batch["profile_hash"])!=binding:
        raise ValueError("历史批次profile与固定输入绑定不一致")


def _profile_at_cut(conn, generation):
    if generation==0:
        return "", ""
    row=conn.execute("SELECT b.canonical_bytes FROM batches b JOIN structure_deltas d "
                     "ON d.index_frontier=b.batch_id WHERE d.generation=?",(generation,)).fetchone()
    if row is None:
        raise ValueError("cut来源不存在")
    batch=json.loads(row[0])
    if type(batch.get("profile_id")) is not str or type(batch.get("profile_hash")) is not str:
        raise ValueError("cut.profile_id/hash缺失")
    return batch["profile_id"],batch["profile_hash"]


def _verify_reachable_root(conn):
    """#1372：全 typed 图像与线性关联门；旧正式读接口和 Q 共用同一个判据。"""
    integrity = _query_integrity()
    return integrity.verify(integrity.capture(conn), globals())


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
    if cut != "cut-%d" % gen:
        raise ValueError("meta.structure_cut 与 generation 不一致")
    if gen == 0 and (idx != "" or pub_n != -1):
        raise ValueError("初态根/输入前沿不一致")
    _scope(meta)
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
        if "supersedes_revision" in b:
            sv = b.get("supersedes_revision")
            if sv is not None:
                b["supersedes_revision"] = _num_to_str(sv)
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


def _publication_fields(conn, gen):
    if gen == 0:
        return dict(input_frontier="-1", seq_range={"from":"0", "to":"-1"},
                    catalog_run_status="not_run", catalog_evidence={})
    row = conn.execute("SELECT input_frontier,seq_range_json,catalog_run_status,catalog_evidence_json "
                       "FROM structure_deltas WHERE generation=?", (gen,)).fetchone()
    if row is None:
        raise ValueError("publication tuple 缺失")
    return dict(input_frontier=_num_to_str(row[0]), seq_range=_json_field(row[1], dict),
                catalog_run_status=row[2], catalog_evidence=_json_field(row[3], dict))


def read_catalog(conn, as_of=None):
    proof = _verify_reachable_root(conn)
    return _query_integrity().project_state(proof, as_of, globals())["catalog"]


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
    proof = _verify_reachable_root(conn)
    return _query_integrity().project_state(proof, as_of, globals())["snapshot"]


def _read_snapshot_in_tx(conn, as_of=None):
    """仅投影；调用者在同一短事务完成根验证。验证器使用它交叉核独立持久索引。"""
    meta = meta_dict(conn)
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

    profile_id, profile_hash = _profile_at_cut(conn, int(header_gen))
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
        **_publication_fields(conn, int(header_gen)),
        "session_id": meta.get("session_id", ""),
        "generation": header_gen,
        "structure_cut": header_cut,
        "catalog_revision": header_cat,
        "scope": _scope(meta),
        "index_frontier": header_idx,
        "profile_id": profile_id,
        "profile_hash": profile_hash,
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
    proof = _verify_reachable_root(conn)
    return _query_integrity().project_state(proof, as_of, globals())


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
        if key not in delta or not isinstance(delta[key], list):
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


def _delta_from_proof(proof, after_generation):
    if type(after_generation) is not int or after_generation < 0:
        raise InvalidQuery("after_generation 必须是非负整数")
    generation = proof["generation"]
    gap = ({"reason": "cursor_ahead_or_session_rebuilt", "rebuild_cut": f"cut-{generation}"}
           if after_generation > generation else None)
    return _project_wire_integers({"session_id": proof["meta"]["session_id"],
            "generation": str(generation), "structure_cut": f"cut-{generation}",
            "after_generation": str(after_generation), "gap": gap,
            "deltas": [] if gap else [_query_integrity().project_delta(proof, index)
                                      for index in range(after_generation, generation)]})


def read_delta(conn, after_generation):
    return _delta_from_proof(_verify_reachable_root(conn), after_generation)


class _DeadlineInput:
    """覆盖请求行、所有头与 body 的总时间/字节界限，不能靠慢滴送重置超时。"""

    def __init__(self, source, connection, seconds, maximum):
        self.source, self.connection = source, connection
        self.deadline, self.maximum = time.monotonic() + seconds, maximum
        self.pending = bytearray()
        self.total = 0

    @property
    def closed(self):
        return self.source.closed

    def close(self):
        self.source.close()

    def _read(self, size):
        remaining = self.deadline - time.monotonic()
        if remaining <= 0:
            raise _query_integrity().QueryBudgetExceeded("HTTP 请求读取超过总时限")
        self.connection.settimeout(remaining)
        try:
            data = self.source.read1(size)
        except TimeoutError as exc:
            raise _query_integrity().QueryBudgetExceeded("HTTP 请求读取超过总时限") from exc
        self.total += len(data)
        if self.total > self.maximum:
            raise _query_integrity().QueryBudgetExceeded("HTTP 请求行/头/body 超过帧上限")
        return data

    def read1(self, size):
        if time.monotonic() > self.deadline:
            raise _query_integrity().QueryBudgetExceeded("HTTP 请求读取超过总时限")
        if self.pending:
            result = bytes(self.pending[:size])
            del self.pending[:size]
            return result
        return self._read(size)

    def readline(self, limit=-1):
        if time.monotonic() > self.deadline:
            raise _query_integrity().QueryBudgetExceeded("HTTP 请求读取超过总时限")
        limit = self.maximum + 1 if limit < 0 else limit
        result = bytearray()
        while len(result) < limit:
            if not self.pending:
                data = self._read(min(4096, limit-len(result)))
                if not data:
                    break
                self.pending.extend(data)
            newline = self.pending.find(b"\n")
            take = len(self.pending) if newline < 0 else newline+1
            take = min(take, limit-len(result))
            result.extend(self.pending[:take])
            del self.pending[:take]
            if result.endswith(b"\n"):
                break
        return bytes(result)


class Handler(BaseHTTPRequestHandler):
    db_path = None
    browser_path = None

    def setup(self):
        if hasattr(self.server, "resources"):
            self.request.settimeout(self.server.resources["socket_read_timeout_ms"] / 1000)
        super().setup()
        if hasattr(self.server, "resources"):
            self.rfile = _DeadlineInput(self.rfile, self.connection,
                self.server.resources["socket_read_timeout_ms"] / 1000, self.server.resources["max_frame_bytes"])

    def handle_one_request(self):
        try:
            super().handle_one_request()
        except _query_integrity().QueryBudgetExceeded as exc:
            self.close_connection = True
            if not hasattr(self, "request_version"):
                self.request_version = "HTTP/1.0"
                self.requestline = ""
                self.command = ""
            self._send_json({"ok": False, "status": "Incomplete", "error": "QueryBudgetExceeded", "detail": str(exc)}, 503)

    def log_message(self, fmt, *args):
        sys.stderr.write("[readonly] %s - %s\n" % (self.address_string(), fmt % args))

    def _send_bytes(self, body_bytes, status=200, content_type="application/json; charset=utf-8"):
        if hasattr(self.server, "resources"):
            self.connection.settimeout(self.server.resources["socket_write_timeout_ms"] / 1000)
        try:
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body_bytes)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body_bytes)
            return True
        except OSError:
            self.close_connection = True
            return False

    def _send_html(self, body_bytes, status=200):
        return self._send_bytes(body_bytes, status, "text/html; charset=utf-8")

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
        if raw == "" or len(raw) > 19 or not raw.isascii() or not raw.isdigit() or (len(raw) > 1 and raw[0] == "0"):
            raise InvalidQuery("%s 必须是规范十进制整数且 >= 0" % key)
        v = int(raw)
        if not -(2**63) <= v < 2**63:
            raise InvalidQuery("%s 超出 i64 精确整数域" % key)
        return v

    def _read_only(self, kind, query=None, as_of=None, after_generation=None):
        query = query or {}
        if hasattr(self.server, "query_reader"):
            self._verified_read(kind, as_of, after_generation)
            return
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
                payload = {"meta": _verify_reachable_root(conn)["meta"]}
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
        except (sqlite3.Error, json.JSONDecodeError, ValueError, TypeError, UnicodeError, KeyError, IndexError, OSError) as e:
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

    def _verified_read(self, kind, as_of, after_generation):
        audit = _query_integrity()
        try:
            proof = self.server.query_reader().capture_verified()
            if kind in ("ready", "state", "catalog", "snapshot"):
                state = audit.project_state(proof, as_of, globals())
                if proof["protocol"] is not None:
                    state["cut"]["session_generation"] = proof["protocol"]["session_generation"]
                payload = {"cut": state["cut"]} if kind == "ready" else state if kind == "state" else state[kind]
            elif kind == "delta":
                payload = _delta_from_proof(proof, after_generation)
            elif kind == "meta":
                payload = {"meta": proof["meta"]}
            else:
                raise InvalidQuery("未知只读操作")
            response = dict(payload, ok=True, producer_epoch=str(self.server.producer_epoch))
            if kind in ("ready", "state"):
                # 控制器启动诊断；不进入 v2 公共业务头、投影摘要或语义重放。
                response["control_instance_id"] = self.server.control_instance_id
            body = serialize_payload(response)
            if body is None:
                raise ValueError("已核响应无法序列化")
            if len(body) > self.server.resources["max_reply_bytes"]:
                raise audit.QueryBudgetExceeded("完整 legacy 响应超过回复上限")
            self._send_bytes(body)
        except InvalidQuery as exc:
            self._send_json({"ok": False, "error": "InvalidQuery", "detail": str(exc)}, 400)
        except audit.QueryBudgetExceeded as exc:
            self._send_json({"ok": False, "status": "Incomplete", "error": "QueryBudgetExceeded", "detail": str(exc)}, 503)
        except (sqlite3.Error, ValueError, TypeError, UnicodeError, KeyError, IndexError, OSError, RecursionError) as exc:
            self._send_json({"ok": False, "error": "StorageUnavailable", "detail": str(exc)}, 503)

    def do_POST(self):
        if not hasattr(self.server, "query_reader"):
            self._send_json({"ok": False, "error": "QueryNotConfigured"}, 503)
            return
        audit, query = _query_integrity(), _query_module("s_query")
        envelope, proof, header_verified = None, None, False

        def error_reply(payload, status):
            if header_verified:
                bound = proof if proof is not None and proof.get("protocol") is not None else None
                payload = dict(payload, identity_binding="verified_current_capture" if bound else "request_echo_unverified")
                response = query.response_envelope(envelope, payload, bound, self.server.producer_epoch, audit)
                body = audit.canonical(response)
                if len(body) <= self.server.resources["max_reply_bytes"]:
                    self._send_bytes(body, status)
                else:
                    self.close_connection = True
            else:
                self._send_json(payload, status)
        try:
            if self.path not in ("/api/v2/snapshot", "/api/v2/watch"):
                self._send_json({"ok": False, "error": "not found"}, 404)
                return
            lengths = self.headers.get_all("Content-Length", [])
            if len(lengths) != 1 or self.headers.get("Transfer-Encoding") is not None:
                raise InvalidQuery("需要唯一 Content-Length；不接受传输编码歧义")
            length = query._integer(lengths[0], "Content-Length", InvalidQuery, True)
            if length > self.server.resources["max_frame_bytes"]:
                raise audit.QueryBudgetExceeded("请求帧超过事前字节上限")
            deadline = time.monotonic() + self.server.resources["socket_read_timeout_ms"] / 1000
            chunks, remaining = [], length
            while remaining:
                seconds = deadline - time.monotonic()
                if seconds <= 0:
                    raise audit.QueryBudgetExceeded("读取请求帧达到总时限")
                self.connection.settimeout(seconds)
                chunk = self.rfile.read1(min(remaining, 65536))
                if not chunk:
                    raise InvalidQuery("请求帧截断")
                chunks.append(chunk)
                remaining -= len(chunk)
            try:
                envelope = audit.parse_json(b"".join(chunks))
            except (ValueError, UnicodeError, RecursionError) as exc:
                raise InvalidQuery("请求不是无重复键的完整 UTF-8 JSON") from exc
            payload = query.validate_envelope(envelope, audit, InvalidQuery)
            header_verified = True
            operation = "snapshot" if self.path.endswith("snapshot") else "watch"
            if payload.get("op") != operation:
                raise InvalidQuery("路径与 op 不一致")
            proof = self.server.query_reader().capture_verified()
            if proof["protocol"] is None:
                raise ValueError("v2 查询需要完整协议存储")
            if operation == "snapshot":
                if (envelope["session_id"], envelope["session_generation"]) != (proof["meta"]["session_id"], proof["protocol"]["session_generation"]):
                    raise InvalidQuery("公共头 session/化身与查询目标不符")
                result = query.snapshot_page(proof, payload, self.server.resources, globals())
            else:
                cursor = payload.get("cursor")
                if type(cursor) is not dict:
                    raise InvalidQuery("Watch cursor 必须是对象")
                if (envelope["session_id"], envelope["session_generation"]) != (cursor.get("session_id"), cursor.get("session_generation")):
                    raise InvalidQuery("公共头与 Watch cursor 身份不一致")
                result = query.watch_page(proof, payload, self.server.resources, globals(), self.server.leases)
            result = dict(result, ok=True)
            response = query.response_envelope(envelope, result, proof, self.server.producer_epoch, audit)
            body = audit.canonical(response)
            if len(body) > self.server.resources["max_reply_bytes"]:
                raise audit.QueryBudgetExceeded("完整公共头响应超过回复上限")
            sent = self._send_bytes(body)
            if sent and operation == "watch" and result["gap"] is None:
                self.server.leases.delivered(payload["client_id"], result["next_cursor"])
        except InvalidQuery as exc:
            error_reply({"ok": False, "error": "InvalidQuery", "detail": str(exc)}, 400)
        except _query_integrity().QueryBudgetExceeded as exc:
            error_reply({"ok": False, "status": "Incomplete", "error": "QueryBudgetExceeded", "detail": str(exc)}, 503)
        except (sqlite3.Error, ValueError, TypeError, UnicodeError, KeyError, IndexError, OSError, RecursionError) as exc:
            error_reply({"ok": False, "error": "StorageUnavailable", "detail": str(exc)}, 503)

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
        if path in ("/", "/index.html", "/tb01c-client.js"):
            try:
                # 固定静态文件集合；URL 不参与磁盘路径拼接。
                target = Path(self.browser_path)
                javascript = path == "/tb01c-client.js"
                if javascript:
                    target = target.with_name("tb01c-client.js")
                maximum = self.server.resources["max_reply_bytes"] if hasattr(self.server, "resources") else 8*1024*1024
                with target.open("rb") as f:
                    body = f.read(maximum+1)
                if len(body) > maximum:
                    raise _query_integrity().QueryBudgetExceeded("静态响应超过回复字节上限")
                self._send_bytes(body, content_type="application/javascript; charset=utf-8" if javascript else "text/html; charset=utf-8")
            except _query_integrity().QueryBudgetExceeded as e:
                self._send_json({"ok": False, "status": "Incomplete", "error": "QueryBudgetExceeded", "detail": str(e)}, 503)
            except FileNotFoundError:
                self._send_json({"ok": False, "error": "browser file not found"}, 404)
            except OSError as e:
                self._send_json({"ok": False, "error": "browser unreadable", "detail": str(e)}, 503)
            return
        if path == "/api/ready":
            if query:
                self._send_json({"ok": False, "error": "InvalidQuery", "detail": "就绪控制读口只核当前会话，不接受查询参数"}, 400)
                return
            self._read_only("ready")
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


def _project_wire_integers(v):
    """最终 wire 边界的安全递归精确整数投影：int → 规范十进制字符串；bool/null/float/str 原值保持。"""
    if type(v) is bool:
        return v
    if type(v) is int:
        return str(v)
    if isinstance(v, dict):
        return {k: _project_wire_integers(x) for k, x in v.items()}
    if isinstance(v, list):
        return [_project_wire_integers(x) for x in v]
    return v


def serialize_payload(obj):
    try:
        return json.dumps(_project_wire_integers(obj), ensure_ascii=False, allow_nan=False).encode("utf-8")
    except (TypeError, ValueError, UnicodeError, OverflowError):
        return None


class QueryHTTPServer(ThreadingHTTPServer):
    """固定 worker 数和有界待处理请求；每 worker 独占自己的 Q 连接。"""

    def __init__(self, address, handler, db_path, resources, producer_epoch):
        self.resources = resources
        self.producer_epoch = producer_epoch
        self.control_instance_id = os.environ.get("S_CONTROL_INSTANCE_ID")
        self.db_path = db_path
        self.request_queue_size = resources["max_pending_connections"]
        capacity = min(resources["max_pending_connections"], resources["max_query_workers"] + resources["max_query_queue"])
        self.slots = threading.BoundedSemaphore(capacity)
        self.local = threading.local()
        self.readers = []
        self.readers_lock = threading.Lock()
        self.leases = _query_module("s_query").ClientLeases(resources)
        self.executor = ThreadPoolExecutor(max_workers=resources["max_query_workers"], thread_name_prefix="s-query")
        try:
            super().__init__(address, handler)
        except Exception:
            self.executor.shutdown(wait=True)
            raise

    def query_reader(self):
        if not hasattr(self.local, "reader"):
            self.local.reader = _query_module("s_query").AuditReader(self.db_path, self.resources, globals())
            with self.readers_lock:
                self.readers.append(self.local.reader)
        return self.local.reader

    def process_request(self, request, client_address):
        if not self.slots.acquire(blocking=False):
            try:
                request.settimeout(self.resources["socket_write_timeout_ms"] / 1000)
                body = b'{"ok":false,"status":"Incomplete","error":"QueryBudgetExceeded"}'
                request.sendall(b"HTTP/1.0 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: " +
                                str(len(body)).encode() + b"\r\nConnection: close\r\n\r\n" + body)
            except OSError:
                pass
            finally:
                self.shutdown_request(request)
            return
        queued_at = time.monotonic()

        def serve():
            try:
                if (time.monotonic()-queued_at)*1000 > self.resources["socket_read_timeout_ms"]:
                    self.shutdown_request(request)
                    return
                self.process_request_thread(request, client_address)
            finally:
                self.slots.release()
        try:
            self.executor.submit(serve)
        except Exception:
            self.slots.release()
            self.shutdown_request(request)
            raise

    def server_close(self):
        super().server_close()
        self.executor.shutdown(wait=True)
        for reader in self.readers:
            reader.close()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", required=True)
    ap.add_argument("--port", type=int, default=8787)
    ap.add_argument("--browser", required=True)
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--resource-config", required=True)
    ap.add_argument("--producer-epoch", required=True)
    args = ap.parse_args()
    query = _query_module("s_query")
    epoch = query._integer(args.producer_epoch, "producer_epoch", ValueError, True)
    resources = query.load_resources(args.resource_config, _query_integrity())
    Handler.db_path = os.path.abspath(args.db)
    Handler.browser_path = os.path.abspath(args.browser)
    server = QueryHTTPServer((args.host, args.port), Handler, Handler.db_path, resources, epoch)
    sys.stderr.write(
        "[readonly] serving S 只读查询 {host}:{port} (db={db})\n".format(
            host=args.host, port=args.port, db=Handler.db_path
        )
    )
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
