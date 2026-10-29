"""#1372 Q 定点测试；结构数据不手编。

历史读取消费 #1768 冻结 pre-W 旧规则 oracle 重建夹具（正式 writer 产物，见 fixtures/issue1768-frozen-pre-w-oracle/README.md）；
真实后续提交仍经 --binary 指向的正式 S writer。

协议分页单元臂仅给已核 v1 图像附加局部协议模型，不作为 v2 持久面或 C 实跑证据。
"""

import argparse
import copy
import hashlib
import json
import os
import sqlite3
import subprocess
import sys
import tempfile
import unittest
import threading
import http.client
import socket
import time
from contextlib import closing
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "s_session"))
import s_readonly_server as reader

audit = reader._query_integrity()
query = reader._query_module("s_query")
BINARY = None
FROZEN = ROOT / "s_session/tests/fixtures/issue1768-frozen-pre-w-oracle"


def load_frozen(name, dest):
    """冻结原件带持久 WAL 头；有侧件即可能有未检查点帧，拒绝。否则 immutable 只读，backup 到私有 dest。"""
    path = (FROZEN / f"{name}.sqlite").resolve()
    present = [s for s in ("-wal", "-shm", "-journal") if Path(f"{path}{s}").exists()]
    if present:
        raise RuntimeError(f"frozen 原件带侧件 {present}，拒绝 immutable 读取：{path}")
    with closing(sqlite3.connect(path.as_uri() + "?immutable=1", uri=True)) as source, closing(sqlite3.connect(dest)) as target:
        source.execute("PRAGMA query_only=ON")
        source.backup(target)

from r4_anchor_fixture import TestAnchor


def resources():
    return dict(max_frame_bytes=1024*1024, max_pending_connections=8,
                socket_read_timeout_ms=2000, socket_write_timeout_ms=2000,
                max_capture_bytes=64*1024*1024, capture_deadline_ms=5000, verify_deadline_ms=5000,
                max_query_workers=1, max_query_queue=4, max_reply_bytes=2*1024*1024,
                max_atomic_batch_bytes=1024*1024, max_cached_images=1, max_cache_bytes=128*1024*1024,
                poll_interval_ms=100, max_client_leases=8, client_lease_ms=60000,
                max_client_pending_batches=2, max_page_size=8, max_watch_batches=2)


class QueryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix="tb01c-query-")
        cls.base = Path(cls.directory.name)
        cls.seed = cls.base / "seed.sqlite"
        load_frozen("g4", cls.seed)

    @classmethod
    def tearDownClass(cls):
        cls.directory.cleanup()

    @classmethod
    def call(cls, db, operation, *args):
        admission_args = []
        if operation in ("accept", "advance", "recover"):
            state = cls.base / "admission" / db.name
            if not Path(str(db.resolve()) + ".control-owner.json").exists():
                state.mkdir(parents=True, exist_ok=True)
                TestAnchor(db, state)
            admission_args = ["--admission-job-limits", str(state / "s-admission" / "limits.json")]
        result = subprocess.run([str(BINARY), operation, "--db", str(db), *admission_args, *map(str, args)],
            capture_output=True, text=True, timeout=60,
            env=dict(os.environ, S_SESSION_PRAGMA_SIDECAR_PATH=str(cls.base / "pragma.txt")))
        if result.returncode != 0:
            raise AssertionError(result.stderr)
        return result

    @classmethod
    def input(cls, seq, price):
        value = dict(schema_revision="1", session_id="tb01c-query-unit", source_namespace="testonly.tick.ohlc",
                     source_epoch="1", instrument="TEST.TICK", profile="testonly_tick_1_1_ohlc",
                     events=[dict(event_id=f"e{seq}", revision="1", seq=str(seq), price=str(price), timestamp=str(seq),
                                  volume="1", raw_text=str(price), received_at="2000-01-01T00:00:00Z")])
        path = cls.base / f"event-{seq}.json"
        path.write_text(json.dumps(value), encoding="utf-8")
        return path

    def setUp(self):
        self.db = self.base / (self.id().rsplit(".", 1)[-1] + ".sqlite")
        with closing(reader.open_readonly(self.seed)) as source, closing(sqlite3.connect(self.db)) as dest:
            source.backup(dest)
        self.reader = query.AuditReader(self.db, resources(), vars(reader))
        self.addCleanup(self.reader.close)

    def mutate(self, sql, values=()):
        with closing(sqlite3.connect(self.db)) as conn, conn:
            conn.execute(sql, values)

    def graph_root_bytes(self, root_key):
        """经审计读者自身的图还原（内容寻址+闭包已核）取某根完整展开字节。"""
        with closing(reader.open_readonly(self.db)) as conn:
            conn.execute("BEGIN")
            tables = audit._typed_rows(audit.capture(conn))
        return audit.evidence_sources(tables, audit.evidence_mode(tables))[root_key]

    def replace_graph_root_bytes(self, root_key, data):
        """图域私有副本内把某根展开字节换成精确 data；行内物理列（delta_json 等）不动。

        新叶节点内容寻址自洽，根 node_id/expanded_length/content_sha256 同步、slot_ids 清空；
        仅删除失去全部引用的节点（共享节点保留），使图闭包仍成立，负控只落在语义门上。
        """
        if type(data) is str:
            data = data.encode("utf-8")
        leaf = audit.EVIDENCE_NODE_LEAF
        node_id = hashlib.sha256(bytes([leaf]) + data).digest()
        with closing(sqlite3.connect(self.db)) as conn, conn:
            conn.execute("INSERT OR IGNORE INTO s_evidence_nodes VALUES(?,?,?,?)", (node_id, leaf, data, len(data)))
            updated = conn.execute(
                "UPDATE s_evidence_roots SET node_id=?,slot_ids=?,expanded_length=?,content_sha256=? WHERE root_key=?",
                (node_id, b"", len(data), hashlib.sha256(data).hexdigest(), root_key)).rowcount
            if updated != 1:
                raise AssertionError(f"图根不存在：{root_key}")
            nodes = dict((row[0], (row[1], row[2])) for row in conn.execute(
                "SELECT node_id,node_kind,payload FROM s_evidence_nodes"))
            reachable, stack = set(), []
            for root, slots in conn.execute("SELECT node_id,slot_ids FROM s_evidence_roots"):
                stack.append(root)
                stack.extend(slots[i:i+32] for i in range(0, len(slots), 32))
            while stack:
                current = stack.pop()
                if current in reachable:
                    continue
                reachable.add(current)
                kind, payload = nodes[current]
                index = 0
                while kind == audit.EVIDENCE_NODE_SEQ and index < len(payload):
                    if payload[index] == 0:
                        index += 5 + int.from_bytes(payload[index+1:index+5], "big")
                    elif payload[index] == 1:
                        stack.append(payload[index+1:index+33])
                        index += 33
                    else:
                        index += 3
            for orphan in nodes.keys() - reachable:
                conn.execute("DELETE FROM s_evidence_nodes WHERE node_id=?", (orphan,))

    def test_full_projection_matches_existing_six_family_projection(self):
        proof = self.reader.capture_verified()
        with closing(reader.open_readonly(self.db)) as conn:
            conn.execute("BEGIN")
            for cut in (None, 0, 1, 2, 3, 4):
                expected = reader._project_wire_integers(reader._read_snapshot_in_tx(conn, cut))
                self.assertEqual(audit.project_state(proof, cut, vars(reader))["snapshot"], expected)

    def test_unchanged_cache_and_new_full_typed_capture(self):
        first = self.reader.capture_verified()
        self.assertIs(self.reader.capture_verified(), first)
        self.mutate("INSERT INTO meta VALUES('query_test_change','accepted')")
        later = self.reader.capture_verified()
        self.assertNotEqual(first["capture_digest"], later["capture_digest"])
        self.assertEqual(self.reader.stats, {"captures": 2, "cache_hits": 1})

    def test_same_generation_independent_index_corruption_invalidates_cache(self):
        self.reader.capture_verified()
        self.mutate("UPDATE witnesses SET merged_high='corrupt' WHERE witness_id=(SELECT witness_id FROM witnesses LIMIT 1)")
        with self.assertRaises(ValueError):
            self.reader.capture_verified()
        self.assertIsNone(self.reader.cached)

    def test_unpublished_raw_is_checked(self):
        load_frozen("g4-pending", self.db)
        good = self.reader.capture_verified()
        self.assertEqual(len(good["raw"]), 5)
        self.mutate("UPDATE raw_events SET source_coord='bad' WHERE seq=4")
        with self.assertRaises(ValueError):
            self.reader.capture_verified()

    def test_unpublished_content_hash_is_checked(self):
        load_frozen("g4-pending", self.db)
        self.mutate("UPDATE raw_events SET price='11501' WHERE seq=4")
        with self.assertRaises(ValueError):
            self.reader.capture_verified()

    def test_pending_numeric_domains_despite_consistent_hash_at_g0_and_g4(self):
        """正式 writer 产生两份 pending 原行；只在私有库改变数值并重算摘要。"""
        empty = self.base / (self.id().rsplit(".",1)[-1]+"-g0.sqlite")
        for db, seq, frozen in ((empty,0,"g0-pending"), (self.db,4,"g4-pending")):
            load_frozen(frozen, db)
            check = query.AuditReader(db,resources(),vars(reader))
            try:
                proof = check.capture_verified()
                self.assertEqual(proof["generation"],seq)
                with closing(sqlite3.connect(db)) as conn:
                    columns = [r[1] for r in conn.execute("PRAGMA table_info(raw_events)")]
                    original = dict(zip(columns,conn.execute("SELECT * FROM raw_events WHERE seq=?",(seq,)).fetchone()))
                for field in ("price","ts","volume"):
                    for value, valid in [(v,True) for v in ("0","-1","-9223372036854775808","9223372036854775807")] + [
                            (v,False) for v in ("+1","01","-0","-01"," 1","1.0","１","9223372036854775808","-9223372036854775809")]:
                        with self.subTest(generation=seq,field=field,value=value):
                            changed = dict(original)
                            changed[field] = value
                            content = {k:changed[k] for k in ("event_id","price","raw_text","received_at","volume")}
                            content.update(revision=changed["input_revision"],seq=changed["source_coord"],timestamp=changed["ts"])
                            changed["payload_hash"] = hashlib.sha256(audit.canonical(content)).hexdigest()
                            changed["receipt_id"] = "rcpt-"+hashlib.sha256((changed["identity_key"]+"|"+changed["payload_hash"]).encode()).hexdigest()[:16]
                            with closing(sqlite3.connect(db)) as conn,conn:
                                conn.execute("DELETE FROM raw_events WHERE seq=?",(seq,))
                                conn.execute("INSERT INTO raw_events VALUES("+",".join("?" for _ in columns)+")",[changed[k] for k in columns])
                            if valid:
                                actual = check.capture_verified()
                                self.assertEqual(actual["raw"][seq][field],value)
                            else:
                                with self.assertRaises(ValueError):
                                    check.capture_verified()
                                self.assertIsNone(check.cached)
                                self.assertFalse(check.connection.in_transaction)
            finally:
                check.close()

    def test_hidden_future_indexes_reject_all_public_reads(self):
        for table, field in (("objects", "published_generation"), ("witnesses", "published_generation"),
                             ("relations", "published_generation"), ("observations", "published_generation")):
            with self.subTest(table=table):
                with closing(sqlite3.connect(self.db)) as conn:
                    conn.execute("BEGIN")
                    conn.execute(f"UPDATE {table} SET {field}=5")
                    for method, arg in ((reader.read_state, None), (reader.read_snapshot, 0),
                                        (reader.read_catalog, 4), (reader.read_delta, 0)):
                        with self.assertRaises(ValueError):
                            method(conn, arg)
                    conn.rollback()

    def test_schema_constraint_or_trigger_change_rejected(self):
        self.mutate("CREATE TRIGGER bad_meta AFTER UPDATE ON meta BEGIN SELECT 1; END")
        with self.assertRaises(ValueError):
            self.reader.capture_verified()

    def test_capture_budget_never_caches_partial_image(self):
        self.reader.resources["max_capture_bytes"] = 1
        with self.assertRaises(audit.QueryBudgetExceeded):
            self.reader.capture_verified()
        self.assertIsNone(self.reader.cached)
        self.assertFalse(self.reader.connection.in_transaction)

    def test_three_data_version_race_windows(self):
        for seam_name in ("after_v0", "after_first_read", "before_cache_registration"):
            with self.subTest(seam=seam_name):
                self.reader.cached = self.reader.cached_version = None
                if seam_name != "before_cache_registration":
                    self.reader.capture_verified()
                fired = []
                def seam(name):
                    if name == seam_name and not fired:
                        fired.append(name)
                        self.mutate("INSERT OR REPLACE INTO meta VALUES('query_race',?)", (seam_name,))
                self.reader.seam = seam
                proof = self.reader.capture_verified()
                self.assertEqual(fired, [seam_name])
                if seam_name == "before_cache_registration":
                    self.assertIsNone(self.reader.cached)
                else:
                    self.assertEqual(proof["meta"]["query_race"], seam_name)
                self.reader.seam = lambda _name: None
                self.assertEqual(self.reader.capture_verified()["meta"]["query_race"], seam_name)

    def test_q_connection_cannot_write(self):
        self.reader.capture_verified()
        with self.assertRaises(sqlite3.OperationalError):
            self.reader.connection.execute("DELETE FROM raw_events")

    def test_legacy_writer_epoch_retains_nonnegative_i64_domain(self):
        self.mutate("UPDATE meta SET value='0' WHERE key='writer_epoch'")
        self.assertEqual(self.reader.capture_verified()["meta"]["writer_epoch"], "0")
        for invalid in ("-1", "01", "9223372036854775808", "x"):
            self.mutate("UPDATE meta SET value=? WHERE key='writer_epoch'", (invalid,))
            with self.assertRaises(ValueError):
                self.reader.capture_verified()

    def test_actual_retained_object_measurement_and_cache_limit(self):
        proof = self.reader.capture_verified()
        amount = audit.object_bytes(proof)
        self.assertNotIn("image", proof)
        self.assertEqual(set(proof["tables"]), {"catalog"})
        self.assertTrue(all(set(batch) == {"profile_id", "profile_hash"}
                            for batch in proof["batches"].values()))
        self.assertEqual(audit.object_bytes([proof, proof]), amount+sys.getsizeof([proof, proof]))
        self.reader.close()
        self.reader.resources["max_cache_bytes"] = 1
        self.reader.capture_verified()
        self.assertIsNone(self.reader.cached)
        with self.assertRaises(audit.QueryBudgetExceeded):
            audit.object_bytes(proof, amount-1)

    def test_unretained_batch_bytes_are_freshly_rechecked_after_external_change(self):
        proof = self.reader.capture_verified()
        self.assertIs(self.reader.cached, proof)
        # 图域下替换目标 batch 根的展开字节为 b"{}"，内容地址/长度/摘要自洽，但保留公开 batch_id
        with closing(sqlite3.connect(self.db)) as conn:
            batch_id = conn.execute("SELECT batch_id FROM batches LIMIT 1").fetchone()[0]
            root_key = "batch/" + batch_id
        self.replace_graph_root_bytes(root_key, b"{}")
        # 应到达 `:541-552` 的身份门：证据图根展开字节与公开批次身份不符
        with self.assertRaisesRegex(ValueError, "证据图根展开字节与公开批次身份不符"):
            self.reader.capture_verified()
        self.assertIsNone(self.reader.cached)
        self.assertFalse(self.reader.connection.in_transaction)
        self.assertEqual(self.reader.stats["captures"], 2)

    def test_source_memo_still_rechecks_current_rows_and_matches_disabled(self):
        first = self.reader.capture_verified()
        self.assertEqual(first["_audit_memo"]["misses"], 4)
        self.mutate("INSERT INTO meta VALUES('memo_probe','new-current-value')")
        later = self.reader.capture_verified()
        self.assertEqual(later["_audit_memo"]["hits"], 4)
        self.assertNotEqual(first["capture_digest"], later["capture_digest"])
        disabled = query.AuditReader(self.db, resources(), vars(reader), use_memo=False)
        self.addCleanup(disabled.close)
        full = disabled.capture_verified()
        self.assertNotIn("_audit_memo", full)
        self.assertEqual(later["capture_digest"], full["capture_digest"])
        for cut in (None, 0, 1, 2, 3, 4):
            self.assertEqual(audit.project_state(later, cut, vars(reader)),
                             audit.project_state(full, cut, vars(reader)))
        for after in range(5):
            self.assertEqual(reader._delta_from_proof(later, after), reader._delta_from_proof(full, after))
        self.mutate("UPDATE observations SET detail_json='{}'")
        for check in (self.reader, disabled):
            with self.assertRaises(ValueError):
                check.capture_verified()
            self.assertIsNone(check.cached)
            self.assertFalse(check.connection.in_transaction)

    def test_memo_budget_counts_sources_certificates_and_public_history_together(self):
        proof = self.reader.capture_verified()
        self.assertTrue(all(type(entry["delta_digest_json"]) is bytes
                            for entry in proof["_audit_memo"]["entries"]))
        amount = audit.object_bytes(proof)
        self.assertIs(self.reader.memo, proof["_audit_memo"])
        self.assertEqual(audit.object_bytes((proof, self.reader.memo)), amount+sys.getsizeof((proof, self.reader.memo)))
        self.reader.close()
        self.reader.resources["max_cache_bytes"] = amount-1
        smaller = self.reader.capture_verified()
        self.assertNotIn("_audit_memo", smaller)
        self.assertEqual(self.reader.memo, {})
        self.assertEqual(reader._delta_from_proof(proof, 0), reader._delta_from_proof(smaller, 0))

    def test_memo_does_not_hide_changed_first_known(self):
        self.reader.capture_verified()
        self.mutate("UPDATE objects SET first_known_cut='cut-0' WHERE object_id=(SELECT object_id FROM objects LIMIT 1)")
        with self.assertRaises(ValueError):
            self.reader.capture_verified()
        self.assertEqual(self.reader.memo, {})

    def test_changed_sealed_source_rebuilds_suffix_and_preserves_full_delta_order(self):
        self.reader.capture_verified()
        # 图域下从私有根读第 3 代 delta，反转 witnesses，加 source_extension
        delta = audit.parse_json(self.graph_root_bytes("delta/3").decode("utf-8"))
        self.assertGreater(len(delta["witnesses"]), 1)
        delta["witnesses"].reverse()
        delta["source_extension"] = {"preserved": ["all", "fields"]}
        self.replace_graph_root_bytes("delta/3", audit.canonical(delta))
        changed = self.reader.capture_verified()
        self.assertEqual((changed["_audit_memo"]["hits"], changed["_audit_memo"]["misses"]), (2, 2))
        full_reader = query.AuditReader(self.db, resources(), vars(reader), use_memo=False)
        self.addCleanup(full_reader.close)
        full = full_reader.capture_verified()
        self.assertEqual(changed["capture_digest"], full["capture_digest"])
        self.assertEqual(audit.project_delta(changed, 2)["delta"], delta)
        for after in range(5):
            self.assertEqual(reader._delta_from_proof(changed, after), reader._delta_from_proof(full, after))

    def test_memo_rejects_duplicate_json_key_after_source_change(self):
        self.reader.capture_verified()
        # 图内写入带重复键的文本，应到达重复键门
        self.replace_graph_root_bytes("delta/3", '{"generation":"3","generation":"3"}')
        with self.assertRaisesRegex(ValueError, "持久 JSON 含重复键"):
            self.reader.capture_verified()
        self.assertEqual(self.reader.memo, {})

    def test_encoded_delta_certificate_preserves_exact_digest_after_text_change(self):
        initial = self.reader.capture_verified()
        original_digest = initial["capture_digest"]
        # 图域下从私有根读第 3 代原件；写回精确文本（含空白/转义），不重新 canonical 化。
        delta = audit.parse_json(self.graph_root_bytes("delta/3").decode("utf-8"))
        delta["source_extension"] = {"escaped": "引号\"反斜线\\换行\n制表\t控制\x00"}
        changed_source = " \n" + audit.canonical(delta).decode() + "\n "
        self.replace_graph_root_bytes("delta/3", changed_source)
        later = self.reader.capture_verified()
        self.assertEqual((later["_audit_memo"]["hits"], later["_audit_memo"]["misses"]), (2, 2))
        self.assertNotEqual(later["capture_digest"], original_digest)
        with closing(reader.open_readonly(self.db)) as conn:
            conn.execute("BEGIN")
            image = audit.capture(conn)
        # 不带编码证书的原始逐值算法独立对拍；连同空白/转义字节一并约束。
        self.assertEqual(later["capture_digest"], audit.capture_digest(image))
        self.assertEqual(audit.project_delta(later, 2)["delta"], delta)
        self.reader.cached = None  # 同镜像重新捕获，不冒称另一次实际 Commit。
        repeated = self.reader.capture_verified()
        self.assertEqual(repeated["_audit_memo"]["hits"], 4)
        self.assertEqual(repeated["capture_digest"], later["capture_digest"])

    def test_header_byte_and_total_time_limits_are_not_per_read_timeouts(self):
        left, right = socket.socketpair()
        self.addCleanup(left.close)
        self.addCleanup(right.close)
        source = left.makefile("rb")
        self.addCleanup(source.close)
        bounded = reader._DeadlineInput(source, left, 0.06, 8)
        right.sendall(b"123456789")
        with self.assertRaises(audit.QueryBudgetExceeded):
            bounded.read1(9)
        bounded = reader._DeadlineInput(source, left, 0.01, 8)
        time.sleep(0.02)
        with self.assertRaises(audit.QueryBudgetExceeded):
            bounded.readline()

    def server(self):
        class Quiet(reader.Handler):
            db_path = str(self.db)
            browser_path = str(ROOT/"s_session/browser/index.html")
            def log_message(self, *args):
                pass
        server = reader.QueryHTTPServer(("127.0.0.1", 0), Quiet, self.db, resources(), 7)  # DevSkim: ignore DS162092 有意使用本机回环（本地服务/测试），非调试残留
        server.control_instance_id = "test-only-start-nonce"
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        def cleanup():
            server.shutdown()
            thread.join(timeout=5)
            server.server_close()
        self.addCleanup(cleanup)
        return server

    def envelope(self, payload):
        return dict(schema_revision="s-session/2", session_id="tb01c-query-unit", session_generation="1",
                    source_namespace="query-tests", source_epoch="1", message_id="query-message",
                    producer_id="query-tests", producer_epoch="1", causal_refs=[], payload=payload,
                    payload_hash=hashlib.sha256(audit.canonical(payload)).hexdigest())

    def test_real_http_legacy_health_and_corruption_without_s_socket(self):
        server = self.server()
        def get(path="/api/state"):
            conn = http.client.HTTPConnection(*server.server_address, timeout=3)
            try:
                conn.request("GET", path)
                response = conn.getresponse()
                return response.status, json.loads(response.read())
            finally:
                conn.close()
        status, result = get()
        self.assertEqual(status, 200)
        self.assertTrue(result["ok"])
        self.assertEqual(result["producer_epoch"], "7")
        self.assertEqual(result["control_instance_id"], "test-only-start-nonce")
        ready_status, ready = get("/api/ready")
        self.assertEqual(ready_status, 200)
        self.assertEqual(ready, {key: result[key] for key in ("ok", "cut", "producer_epoch", "control_instance_id")})
        for path in ("/api/ready?as_of=0", "/api/ready?ignored=1"):
            invalid_status, invalid = get(path)
            self.assertEqual((invalid_status, invalid["error"]), (400, "InvalidQuery"))
        self.mutate("UPDATE raw_events SET price='changed' WHERE seq=0")
        status, result = get()
        self.assertEqual((status, result["error"]), (503, "StorageUnavailable"))
        status, result = get("/api/ready")
        self.assertEqual((status, result["error"]), (503, "StorageUnavailable"))

    def test_fixed_static_script_route_and_no_path_traversal(self):
        static = self.base / "query-static"
        static.mkdir(exist_ok=True)
        (static/"index.html").write_text("<!doctype html>", encoding="utf-8")
        (static/"tb01c-client.js").write_bytes(b"export const testOnly = true;\n")
        server = self.server()
        server.RequestHandlerClass.browser_path = str(static/"index.html")
        for path, expected in (("/tb01c-client.js",200), ("/../tb01c-client.js",404)):
            conn = http.client.HTTPConnection(*server.server_address, timeout=3)
            try:
                conn.request("GET",path)
                response = conn.getresponse()
                body = response.read()
                self.assertEqual(response.status,expected)
                if expected == 200:
                    self.assertEqual(response.getheader("Content-Type"), "application/javascript; charset=utf-8")
                    self.assertEqual(body,(static/"tb01c-client.js").read_bytes())
            finally:
                conn.close()

    def test_valid_request_bad_storage_gets_correlated_unverified_error_head(self):
        server = self.server()
        self.mutate("UPDATE meta SET value='broken' WHERE key='generation'")
        request = self.envelope(self.request())
        conn = http.client.HTTPConnection(*server.server_address, timeout=3)
        try:
            conn.request("POST", "/api/v2/snapshot", audit.canonical(request), {"Content-Type":"application/json"})
            response = conn.getresponse()
            result = json.loads(response.read())
        finally:
            conn.close()
        self.assertEqual(response.status, 503)
        self.assertEqual(result["schema_revision"], "s-session/2")
        self.assertEqual(result["payload"]["identity_binding"], "request_echo_unverified")
        self.assertNotIn("fixed_cut", result["payload"])
        self.assertNotIn("validated_capture_digest", result["payload"])
        self.assertEqual(result["payload_hash"], hashlib.sha256(audit.canonical(result["payload"])).hexdigest())

    def protocol_model(self, proof=None, retain=8):
        """协议层单元模型，不写入 S 或冒充 v2 正式 init。"""
        proof = dict(proof or self.reader.capture_verified())
        proof["protocol"] = {"session_generation": "1", "delivery_policy": {
            "policy_revision": "generation-window/1", "retain_generations": retain,
            "first_available_generation": max(1, proof["generation"]-retain+1),
            "head_generation": proof["generation"]}}
        return proof

    def request(self, mode="AsKnown", generation="4"):
        return dict(op="snapshot", session_id="tb01c-query-unit", session_generation="1",
                    scope={"structure": "CompleteCut", "economic": "not_started"}, history_mode=mode,
                    as_of_generation=generation, page_size="2", order_version=query.ORDER_VERSION)

    def test_complete_pages_preserve_all_families_and_order(self):
        proof = self.protocol_model()
        first = query.snapshot_page(proof, self.request(), resources(), vars(reader))
        rows, page = [], first
        while True:
            rows.extend(page["rows"])
            if page["done"]:
                break
            page = query.snapshot_page(proof, {"op": "snapshot", "snapshot_token": page["next_token"]}, resources(), vars(reader))
        projection, digest = query.project_fixed_cut(proof, 4, "AsKnown", vars(reader))
        self.assertEqual(rows, projection["rows"])
        self.assertEqual(first["cut_projection_digest"], digest)
        self.assertEqual(int(first["counts"]["total"]), len(rows))

    def test_old_token_survives_fresh_capture_and_new_commit(self):
        first = query.snapshot_page(self.protocol_model(), self.request(), resources(), vars(reader))
        token = first["next_token"]
        self.call(self.db, "accept", "--input", self.input(4, 11500), "--profile", ROOT/"s_session/profiles/testonly_tick_1_1_ohlc.json")
        self.call(self.db, "advance")
        proof = self.protocol_model()
        page = query.snapshot_page(proof, {"op": "snapshot", "snapshot_token": token}, resources(), vars(reader))
        self.assertEqual(page["fixed_cut"]["cut_generation"], "4")
        self.assertEqual(page["cut_projection_digest"], first["cut_projection_digest"])
        self.assertNotEqual(page["validated_capture_digest"], token["issued_capture_digest"])
        self.reader.close()
        restarted = self.protocol_model()
        self.assertEqual(query.snapshot_page(restarted, {"op": "snapshot", "snapshot_token": token}, resources(), vars(reader)), page)

    def test_token_rejects_inconsistent_cut_scope_digest_or_order(self):
        proof = self.protocol_model()
        page = query.snapshot_page(proof, self.request(), resources(), vars(reader))
        for key, value in (("cut_generation", "3"), ("session_generation", "2"), ("order_version", "bogus"),
                           ("scope", {}), ("profile_hash", ""), ("cut_projection_digest", "bogus")):
            token = copy.deepcopy(page["next_token"])
            token[key] = value
            token["token_checksum"] = hashlib.sha256(audit.canonical({k:v for k,v in token.items() if k!="token_checksum"})).hexdigest()
            with self.subTest(key=key), self.assertRaises((ValueError, reader.InvalidQuery)):
                query.snapshot_page(proof, {"op": "snapshot", "snapshot_token": token}, resources(), vars(reader))

    def test_watch_cursor_tracks_delivered_batch_not_observed_head(self):
        proof = self.protocol_model()
        leases = query.ClientLeases(resources())
        request = dict(op="watch", cursor=query.cursor_at(proof, 0, vars(reader)), max_batches="2", client_id="normal")
        result = query.watch_page(proof, request, resources(), vars(reader), leases)
        self.assertIsNone(result["gap"])
        self.assertEqual(result["next_cursor"]["after_generation"], "2")
        self.assertEqual(result["observed_head"]["cut_generation"], "4")
        self.assertTrue(result["has_more"])
        request["cursor"] = query.cursor_at(proof, 4, vars(reader))
        empty = query.watch_page(proof, request, resources(), vars(reader), leases)
        self.assertEqual(empty["next_cursor"], request["cursor"])
        self.assertEqual(empty["deltas"], [])

    def test_retention_gap_and_cross_identity_do_not_share_numeric_range(self):
        proof = self.protocol_model(retain=2)
        request = dict(op="watch", cursor=query.cursor_at(proof, 0, vars(reader)), max_batches="2", client_id="old")
        result = query.watch_page(proof, request, resources(), vars(reader), query.ClientLeases(resources()))
        self.assertEqual(result["gap"]["missing_range"], {"from_generation":"1", "to_generation":"2"})
        request["cursor"]["session_generation"] = "2"
        result = query.watch_page(proof, request, resources(), vars(reader), query.ClientLeases(resources()))
        self.assertIsNone(result["gap"]["missing_range"])
        self.assertEqual(result["gap"]["snapshot_token"]["session_generation"], "1")

    def test_atomic_batch_budget_returns_no_partial_success(self):
        proof = self.protocol_model()
        limits = resources()
        limits["max_atomic_batch_bytes"] = 1
        request = dict(op="watch", cursor=query.cursor_at(proof, 0, vars(reader)), max_batches="2", client_id="small")
        with self.assertRaises(audit.QueryBudgetExceeded):
            query.watch_page(proof, request, limits, vars(reader), query.ClientLeases(limits))

    def test_slow_client_gap_is_local_and_new_valid_cursor_can_catch_up(self):
        limits = resources()
        leases = query.ClientLeases(limits)
        proof = self.protocol_model()
        stalled = dict(op="watch", cursor=query.cursor_at(proof, 4, vars(reader)), max_batches="2", client_id="stalled")
        self.assertIsNone(query.watch_page(proof, stalled, limits, vars(reader), leases)["gap"])
        for seq, price in enumerate((11500, 13000, 12000), 4):
            self.call(self.db, "accept", "--input", self.input(seq, price), "--profile", ROOT/"s_session/profiles/testonly_tick_1_1_ohlc.json")
            self.call(self.db, "advance")
        later = self.protocol_model()
        fresh = dict(op="watch", cursor=query.cursor_at(later, 0, vars(reader)), max_batches="2", client_id="fresh")
        self.assertIsNone(query.watch_page(later, fresh, limits, vars(reader), leases)["gap"])
        result = query.watch_page(later, stalled, limits, vars(reader), leases)
        self.assertEqual(result["gap"]["reason"], "client_backlog_overflow")
        self.assertEqual(result["gap"]["missing_range"], {"from_generation":"5", "to_generation":"7"})

    def test_memo_certificate_binds_evidence_version_bidirectional_hot_cache(self):
        """同一活 reader 跨证据表示重核，摘要仍对应当前全部 typed 行。"""
        def capture_image():
            with closing(reader.open_readonly(self.db)) as conn:
                conn.execute("BEGIN")
                return audit.capture(conn)

        original = capture_image()
        tables = audit._typed_rows(original)
        self.assertEqual(audit.evidence_mode(tables), audit.EVIDENCE_GRAPH_FORMAT)
        sources = audit.evidence_sources(tables, audit.EVIDENCE_GRAPH_FORMAT)

        def write_representation(version):
            # 只改本测试私有库的物理表示，逻辑原文取自冻结夹具的已核图根。
            graph3 = version == audit.EVIDENCE_GRAPH_FORMAT
            with closing(sqlite3.connect(self.db)) as conn, conn:
                for row in tables["structure_deltas"]:
                    text = "" if graph3 else sources[f"delta/{row['generation']}"].decode("utf-8")
                    conn.execute("UPDATE structure_deltas SET delta_json=? WHERE generation=?",
                                 (text, row["generation"]))
                for row in tables.get("structure_facts", []):
                    values = ("", "", "") if graph3 else json.loads(sources[f"fact/{row['object_id']}"])
                    conn.execute(
                        "UPDATE structure_facts SET payload_json=?,input_refs_json=?,source_coords_json=? WHERE object_id=?",
                        (*values, row["object_id"]))
                conn.execute("UPDATE s_evidence_format SET graph_format=?", (version,))
                conn.execute("DELETE FROM s_evidence_roots")
                conn.execute("DELETE FROM s_evidence_nodes")
                for key, data in sources.items():
                    if graph3 or key.startswith("batch/"):
                        leaf = audit.EVIDENCE_NODE_LEAF
                        node_id = hashlib.sha256(bytes([leaf]) + data).digest()
                        conn.execute("INSERT OR IGNORE INTO s_evidence_nodes VALUES(?,?,?,?)",
                                     (node_id, leaf, data, len(data)))
                        conn.execute("INSERT INTO s_evidence_roots VALUES(?,?,?,?,?)",
                                     (key, node_id, b"", len(data), hashlib.sha256(data).hexdigest()))

        versions = (audit.EVIDENCE_GRAPH_FORMAT, audit.EVIDENCE_GRAPH_FORMAT_V2)
        for source, destination in (versions, versions[::-1]):
            with self.subTest(source=source, destination=destination):
                write_representation(source)
                with closing(query.AuditReader(self.db, resources(), vars(reader))) as live:
                    first = live.capture_verified()
                    self.assertEqual((first["_audit_memo"]["hits"], first["_audit_memo"]["misses"]), (0, 4))
                    self.mutate("INSERT OR REPLACE INTO meta VALUES('memo_domain_probe',?)", (source,))
                    hot = live.capture_verified()
                    self.assertEqual((hot["_audit_memo"]["hits"], hot["_audit_memo"]["misses"]), (4, 0))
                    self.assertEqual(len(live.memo["entries"]), 4)
                    self.assertIs(live.memo, hot["_audit_memo"])
                    connection, identity = live.connection, live.identity
                    previous_memo = live.memo
                    before = capture_image()
                    self.assertEqual(hot["capture_digest"], audit.capture_digest(before))

                    write_representation(destination)
                    after = capture_image()
                    self.assertEqual(before["schema"], after["schema"])
                    self.assertEqual(before["schema_rows"], after["schema_rows"])
                    current_tables = audit._typed_rows(after)
                    self.assertEqual(audit.evidence_mode(current_tables), destination)
                    self.assertTrue(all(bool(row["delta_json"]) == (destination == audit.EVIDENCE_GRAPH_FORMAT_V2)
                                        for row in current_tables["structure_deltas"]))
                    self.assertIs(live.memo, previous_memo)
                    warm = live.capture_verified()
                    self.assertIs(live.connection, connection)
                    self.assertEqual(live.identity, identity)
                    self.assertEqual(live.stats["captures"], 3)
                    with closing(query.AuditReader(self.db, resources(), vars(reader), use_memo=False)) as disabled:
                        full = disabled.capture_verified()
                    raw = audit.capture_digest(after)
                    self.assertEqual(full["capture_digest"], raw)
                    self.assertEqual(warm["capture_digest"], raw)
                    self.assertEqual((warm["_audit_memo"]["hits"], warm["_audit_memo"]["misses"]), (0, 4))
                    self.assertNotEqual(previous_memo["context"], live.memo["context"])
                    for cut in (None, 0, 1, 2, 3, 4):
                        self.assertEqual(audit.project_state(hot, cut, vars(reader)),
                                         audit.project_state(warm, cut, vars(reader)))
                    for after_generation in range(5):
                        self.assertEqual(reader._delta_from_proof(hot, after_generation),
                                         reader._delta_from_proof(warm, after_generation))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    args, rest = parser.parse_known_args()
    BINARY = args.binary.resolve()
    unittest.main(argv=[sys.argv[0], *rest])
