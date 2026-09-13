#!/usr/bin/env python3
"""#1373：正式 v2 S 自建库 → Q 严格查询扰动；不是结构 oracle 或本叶验收。

--output 必须是不存在的新目录；保留全部 CLI 原件、坏库副本和结果，不碰其他运行库。
"""
import argparse
from contextlib import closing
import copy
import hashlib
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "s_session"))
import s_readonly_server as reader

audit = reader._query_integrity()
query = reader._query_module("s_query")
BINARY = OUTPUT = None


def resources():
    return query.load_resources(ROOT / "s_session/tests/fixtures/tb01c/query-resources.json", audit)


def captured(db, memo=None):
    with closing(reader.open_readonly(db)) as conn:
        with conn:
            conn.execute("BEGIN")
            image = audit.capture(conn)
    return audit.verify(image, vars(reader), memo=memo)


def backup(source, target):
    with closing(reader.open_readonly(source)) as src, closing(sqlite3.connect(target)) as dst:
        src.backup(dst)


class TB02QueryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.calls = 0
        cls.db = OUTPUT / "session.sqlite"
        cls.profile = ROOT / "s_session/profiles/ohlc_integer_tb02a_v1.json"
        cls.env = dict(os.environ, S_SESSION_PRAGMA_SIDECAR_PATH=str(OUTPUT / "pragma.jsonl"))
        clock = {"schema_revision": "s-clock-plan/1", "clock_plan_id": "tb02-q-tests", "origin_utc": "2000-01-01T00:00:00Z", "unit": "ns", "events": {
            f"op-{i}": {"accept_ns": str(i * 10 + 1), "attempts": [{"begin_ns": str(i * 10 + 2), "commit_ns": str(i * 10 + 3)}]} for i in range(6)}}
        cls.clock = OUTPUT / "clock.json"
        cls.clock.write_bytes(audit.canonical(clock))
        cls.call("init", "--session", "tb02-q-tests", "--catalog", ROOT / "s_session/catalog/signed-catalog.json", "--profile", cls.profile,
                 "--session-generation", "1", "--clock-plan", cls.clock, "--delivery-retain-generations", "32")
        cls.zero = captured(cls.db)
        cls.zero_db = OUTPUT / "healthy-g0.sqlite"
        backup(cls.db, cls.zero_db)
        ledger = audit.parse_json((ROOT / "s_session/tests/fixtures/tb02a/raw-ledger.json").read_bytes())
        cases = {case["case_id"]: case["raw_bars"] for case in ledger["cases"]}
        before, after = cases["high_only_before_revision"], cases["high_only_after_revision"]
        revised = [new for old, new in zip(before, after) if old != new]
        if len(before) != 5 or len(revised) != 1:
            raise ValueError("具名原始账簿变更；必须重审测试输入封装")
        cls.proofs = [cls.zero]
        with tempfile.TemporaryDirectory(prefix="t02q-", dir="/tmp") as short:
            endpoint = Path(short) / "s.sock"
            argv = [str(BINARY), "serve", "--db", str(cls.db), "--socket", str(endpoint), "--writer-epoch", "1", "--profile", str(cls.profile),
                    "--clock-plan", str(cls.clock), "--max-frame-bytes", "1048576", "--read-timeout-ms", "2000", "--write-timeout-ms", "2000",
                    "--response-timeout-ms", "10000", "--audit-cache-source-bytes", "268435456", "--queue-capacity", "8", "--max-connections", "8"]
            (OUTPUT / "serve-argv.json").write_bytes(audit.canonical(argv))
            with (OUTPUT / "serve.log").open("xb") as log:
                process = subprocess.Popen(argv, env=cls.env, stdout=log, stderr=log, start_new_session=True)
                try:
                    deadline = time.monotonic() + 10
                    while not endpoint.exists():
                        if process.poll() is not None or time.monotonic() >= deadline:
                            raise RuntimeError("自建 S 未在期限内启动，见 serve.log")
                        time.sleep(0.02)
                    for index, bar in enumerate(before + revised):
                        event = dict(event_id=bar["raw_id"], revision="2" if index == 5 else "1", seq=str(bar["raw_index"]),
                                     received_at="2000-01-01T00:00:00Z", raw_text=audit.canonical(bar).decode(), timestamp=str(bar["raw_index"]), volume="1",
                                     **{key: str(bar[key]) for key in ("open", "high", "low", "close")})
                        source = dict(schema_revision="s-ohlc/1", session_id="tb02-q-tests", source_namespace="tb02-query-test", source_epoch="1", instrument="TEST-OHLC",
                                      profile=audit.tb02.PROFILE, events=[event])
                        payload = dict(op="ingest", target_session_id="tb02-q-tests", target_session_generation="1", writer_epoch="1", clock_event_id=f"op-{index}", raw_input=source)
                        envelope = dict(schema_revision="s-session/2", session_id="tb02-q-tests", session_generation="1", source_namespace="tb02-query-test", source_epoch="1",
                                        message_id=f"input-{index}", producer_id="tb02-query-tests", producer_epoch="1", payload_hash=hashlib.sha256(audit.canonical(payload)).hexdigest(), causal_refs=[], payload=payload)
                        (OUTPUT / f"request-{index}.json").write_bytes(audit.canonical(envelope))
                        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
                            sock.settimeout(15)
                            sock.connect(str(endpoint))
                            sock.sendall(audit.canonical(envelope) + b"\n")
                            sock.shutdown(socket.SHUT_WR)
                            result = bytearray()
                            while not result.endswith(b"\n"):
                                chunk = sock.recv(65536)
                                if not chunk or len(result) + len(chunk) > 1048576:
                                    raise ValueError("S 回执截断或超界")
                                result.extend(chunk)
                        (OUTPUT / f"reply-{index}.json").write_bytes(result)
                        response = audit.parse_json(bytes(result))
                        if response["payload_hash"] != hashlib.sha256(audit.canonical(response["payload"])).hexdigest() or response["payload"].get("ok") is False:
                            raise ValueError("S 正式接纳失败，见回执")
                        proof = captured(cls.db)
                        if proof["generation"] != index + 1:
                            raise ValueError("S 回执后发布代不符合单事件轨迹")
                        cls.proofs.append(proof)
                finally:
                    if process.poll() is None:
                        process.terminate()
                        try:
                            process.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            process.kill()
                            process.wait(timeout=5)
        cls.seed = OUTPUT / "healthy.sqlite"
        backup(cls.db, cls.seed)
        cls.healthy = captured(cls.seed, {})
        (OUTPUT / "healthy-state.json").write_bytes(audit.canonical(audit.project_state(cls.healthy, None, vars(reader))))

    @classmethod
    def call(cls, op, *args):
        cls.calls += 1
        argv = [str(BINARY), op, "--db", str(cls.db), *map(str, args)]
        result = subprocess.run(argv, env=cls.env, capture_output=True, timeout=20)
        (OUTPUT / f"cli-{cls.calls}-argv.json").write_bytes(audit.canonical(argv))
        (OUTPUT / f"cli-{cls.calls}-stdout.txt").write_bytes(result.stdout)
        (OUTPUT / f"cli-{cls.calls}-stderr.txt").write_bytes(result.stderr)
        if result.returncode:
            raise AssertionError(f"正式 S {op} exit={result.returncode}，见原件")

    def bad(self, mutate, source=None):
        target = OUTPUT / (self.id().split(".")[-1] + ".sqlite")
        backup(self.seed if source is None else source, target)
        with closing(sqlite3.connect(target)) as conn:
            with conn:
                mutate(conn)
        before = target.read_bytes()
        with self.assertRaises((ValueError, TypeError, KeyError, sqlite3.Error)) as error:
            captured(target, self.healthy["_audit_memo"])
        self.assertEqual(target.read_bytes(), before)
        (OUTPUT / (target.stem + ".txt")).write_text(str(error.exception))

    def test_g0_profile_order_and_wrong_version(self):
        p, _ = query.project_fixed_cut(self.zero, 0, "AsKnown", vars(reader))
        self.assertEqual(p["order_version"], "s-record-order/2")
        self.assertEqual(p["fixed_cut"]["profile_id"], audit.tb02.PROFILE)
        request = dict(op="snapshot", session_id="tb02-q-tests", session_generation="1", scope=reader._scope(self.zero["meta"]), history_mode="AsKnown", as_of_generation="0", page_size="3", order_version="s-record-order/1")
        with self.assertRaises(reader.InvalidQuery):
            query.snapshot_page(self.zero, request, resources(), vars(reader))

    def test_old_cut_token_survives_revision_and_complete_delta(self):
        before = self.proofs[5]
        request = dict(op="snapshot", session_id="tb02-q-tests", session_generation="1", scope=reader._scope(before["meta"]), history_mode="AsKnown", as_of_generation="5", page_size="3", order_version="s-record-order/2")
        page = query.snapshot_page(before, request, resources(), vars(reader))
        rows = list(page["rows"])
        digest = page["cut_projection_digest"]
        while not page["done"]:
            page = query.snapshot_page(self.healthy, {"op": "snapshot", "snapshot_token": page["next_token"]}, resources(), vars(reader))
            self.assertEqual(page["cut_projection_digest"], digest)
            rows.extend(page["rows"])
        complete, expected = query.project_fixed_cut(self.healthy, 5, "AsKnown", vars(reader))
        self.assertEqual(rows, complete["rows"])
        self.assertEqual(digest, expected)
        self.assertEqual(query.cursor_at(self.healthy, 5, vars(reader))["order_version"], "s-record-order/2")
        final = audit.project_delta(self.healthy, 5)["delta"]
        self.assertTrue(final["withdrawals"])
        self.assertEqual(final["catalog_evidence"]["axes"]["CC-056"]["evidence"]["withdrawals"], final["withdrawals"])
        self.assertEqual(len(self.healthy["delta_json"]), 6)

    def test_raw_ohlc_missing(self):
        self.bad(lambda c: c.execute("DELETE FROM raw_ohlc WHERE rowid=(SELECT MIN(rowid) FROM raw_ohlc)"))

    def test_raw_ohlc_extra(self):
        self.bad(lambda c: c.execute("INSERT INTO raw_ohlc SELECT identity_key||'-extra',revision,schema_revision,open,high,low,close FROM raw_ohlc LIMIT 1"))

    def test_raw_close_alias(self):
        self.bad(lambda c: c.execute("UPDATE raw_ohlc SET close='0' WHERE rowid=(SELECT MIN(rowid) FROM raw_ohlc)"))

    def test_raw_price_domain(self):
        self.bad(lambda c: c.execute("UPDATE raw_ohlc SET high='+1' WHERE rowid=(SELECT MIN(rowid) FROM raw_ohlc)"))

    def test_missing_required_table(self):
        self.bad(lambda c: c.execute("DROP TABLE raw_ohlc"))

    def test_fact_deep_missing_key(self):
        def mutate(c):
            oid, payload = c.execute("SELECT object_id,payload_json FROM structure_facts WHERE kind='CC-004.inclusion_step' LIMIT 1").fetchone()
            p = json.loads(payload)
            del p["incoming"]["high"]
            c.execute("UPDATE structure_facts SET payload_json=? WHERE object_id=?", (audit.canonical(p).decode(), oid))
        self.bad(mutate)

    def test_fact_deep_wrong_type(self):
        def mutate(c):
            oid, payload = c.execute("SELECT object_id,payload_json FROM structure_facts WHERE kind='CC-005.inclusion_group' LIMIT 1").fetchone()
            p = json.loads(payload)
            p["confirmed"] = "true"
            c.execute("UPDATE structure_facts SET payload_json=? WHERE object_id=?", (audit.canonical(p).decode(), oid))
        self.bad(mutate)

    def test_fact_missing_independent_index(self):
        self.bad(lambda c: c.execute("DELETE FROM structure_facts WHERE object_id=(SELECT object_id FROM structure_facts LIMIT 1)"))

    def test_fact_unpublished_future(self):
        self.bad(lambda c: c.execute("UPDATE structure_facts SET published_generation=7 WHERE object_id=(SELECT object_id FROM structure_facts LIMIT 1)"))

    def test_ohcl_schema_constraint_changed(self):
        self.bad(lambda c: c.execute("CREATE TRIGGER forbidden_trigger AFTER UPDATE ON raw_ohlc BEGIN SELECT 1; END"))

    def test_catalog_axes_wrong_cut(self):
        def mutate(c):
            newer = c.execute("SELECT catalog_evidence_json FROM structure_deltas WHERE generation=6").fetchone()[0]
            old = json.loads(c.execute("SELECT catalog_evidence_json FROM structure_deltas WHERE generation=5").fetchone()[0])
            old["axes"] = json.loads(newer)["axes"]
            c.execute("UPDATE structure_deltas SET catalog_evidence_json=? WHERE generation=5", (audit.canonical(old).decode(),))
        self.bad(mutate)

    def test_catalog_current_not_sealed(self):
        self.bad(lambda c: c.execute("UPDATE catalog SET evidence_json='{}' WHERE catalog_id='CC-005'"))

    def test_legacy_profile_cannot_hide_new_tables(self):
        self.bad(lambda c: c.execute("UPDATE meta SET value='testonly_tick_1_1_ohlc' WHERE key='profile_id'"))

    def test_catalog_g0_cannot_hide_unpublished_fact(self):
        self.bad(lambda c: c.execute("UPDATE catalog SET run_status='run' WHERE catalog_id='CC-005'"), self.zero_db)

    def test_catalog_unimplemented_axis_cannot_hide_fact(self):
        self.bad(lambda c: c.execute("UPDATE catalog SET impl_status='implemented' WHERE catalog_id='CC-008'"))

    def test_shape_dependency_field_required(self):
        def mutate(c):
            oid, text = c.execute("SELECT object_id,input_refs_json FROM objects LIMIT 1").fetchone()
            refs = json.loads(text)
            del refs[0]["dependency_refs"]
            c.execute("UPDATE objects SET input_refs_json=? WHERE object_id=?", (audit.canonical(refs).decode(), oid))
        self.bad(mutate)

    def test_shape_dependency_cannot_be_faked_as_member(self):
        state = audit.project_state(self.healthy, None, vars(reader))
        shape = copy.deepcopy(next(o for o in state["snapshot"]["objects"] if o["kind"] == "CC-006.local_shape"))
        group = shape["input_refs"][0]
        group["dependency_refs"].append(copy.deepcopy(group["raw_refs"][0]))
        with self.assertRaises(ValueError):
            audit.tb02.validate_shape_refs(shape)

    def test_shape_has_three_complete_bound_source_groups(self):
        state = audit.project_state(self.healthy, None, vars(reader))
        shapes = [o for o in state["snapshot"]["objects"] if o["kind"] == "CC-006.local_shape"]
        self.assertTrue(shapes, "实际正式S输入未产生本项需要的形态")
        for shape in shapes:
            audit.tb02.validate_shape_refs(shape)
            self.assertEqual(len(shape["input_refs"]), 3)
            for group in shape["input_refs"]:
                self.assertIn("dependency_refs", group)
                self.assertFalse(set(r["source_coord"] for r in group["raw_refs"]) & set(r["source_coord"] for r in group["dependency_refs"]))

    def test_memo_and_full_proof_all_public_values(self):
        no_memo = captured(self.seed)
        memo = captured(self.seed, self.healthy["_audit_memo"])
        self.assertEqual(memo["_audit_memo"]["hits"], 6)
        self.assertEqual(no_memo["capture_digest"], memo["capture_digest"])
        for generation in range(7):
            self.assertEqual(audit.project_state(no_memo, generation, vars(reader)), audit.project_state(memo, generation, vars(reader)))
        for index in range(6):
            self.assertEqual(audit.project_delta(no_memo, index), audit.project_delta(memo, index))

    def test_merged_ordinal_is_not_raw_anchor_and_binds_group(self):
        state = audit.project_state(self.healthy, 5, vars(reader))["snapshot"]
        shape = next(o for o in state["objects"] if o["kind"] == "CC-006.local_shape")
        anchors = [shape[k] for k in ("window_start", "window_mid", "window_end")]
        self.assertEqual(anchors, ["0", "1", "4"])
        self.assertEqual([g["merged_index"] for g in shape["input_refs"]], ["0", "1", "2"])
        groups = {o["payload"]["group_anchor"]: o["payload"] for o in state["objects"] if o["kind"] == audit.tb02.KINDS[1]}
        audit.tb02.validate_shape_refs(shape, groups=groups)
        bad = copy.deepcopy(shape)
        for group in bad["input_refs"]:
            group["merged_index"] = str(int(group["merged_index"]) + 1)
        with self.assertRaises(ValueError):
            audit.tb02.validate_shape_refs(bad, groups=groups)

    def test_raw_anchor_and_ordinal_each_reject_corruption(self):
        state = audit.project_state(self.healthy, 5, vars(reader))["snapshot"]
        shape = next(o for o in state["objects"] if o["kind"] == "CC-006.local_shape")
        for name in ("anchor", "ordinal"):
            bad = copy.deepcopy(shape)
            if name == "anchor":
                bad["window_end"] = "2"
            else:
                bad["input_refs"][2]["merged_index"] = "4"
            with self.subTest(field=name), self.assertRaises(ValueError):
                audit.tb02.validate_shape_refs(bad)


def main():
    global BINARY, OUTPUT
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    BINARY, OUTPUT = args.binary.resolve(), args.output.resolve()
    OUTPUT.mkdir(parents=True, exist_ok=False)
    sources = [ROOT / "s_session" / name for name in ("s_query_integrity.py", "s_query.py", "s_readonly_server.py", "s_tb02_contract.py")]
    sources += [Path(__file__).resolve(), BINARY]
    manifest = {str(p): {"bytes": p.stat().st_size, "sha256": hashlib.sha256(p.read_bytes()).hexdigest()} for p in sources}
    (OUTPUT / "SOURCE.json").write_bytes(audit.canonical(manifest))
    result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(TB02QueryTests))
    unchanged = all(hashlib.sha256(Path(path).read_bytes()).hexdigest() == item["sha256"] for path, item in manifest.items())
    report = {"scope": "正式 S 自建 v2 库的 Q 类型/完整性扰动；非结构 oracle/GUI/本叶验收", "tests": result.testsRun,
              "failures": result.failures, "errors": result.errors, "sources_unchanged": unchanged, "passed": result.wasSuccessful() and unchanged}
    # unittest对象仅转为可定位的测试身份，异常文本保留完整。
    report["failures"] = [(str(test), message) for test, message in result.failures]
    report["errors"] = [(str(test), message) for test, message in result.errors]
    (OUTPUT / "RESULT.json").write_bytes(audit.canonical(report))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
