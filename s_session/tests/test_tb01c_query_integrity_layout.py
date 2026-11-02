"""#1491 ADR 0037 第 3 轮：Python 只读面只读 v1/v2 库，不引入布局组件（读 v3 的是 Rust Q 投影）。

`s_query_integrity` 与 `s_readonly_server` 的权威模式只含 v1/v2 表（`objects`/`structure_facts` 等），
没有 `s_content_*`；v2 写者规则串不含 `bounded-layout`。本文件锁三件事：v2 读者对象次序与 Rust v2
读者 SQL（`s_structure_session.rs` 的 `OBJECT_*_SQL`）同口径；`project_state` 的撤回数组按
first_known_generation；任何布局组件在 v2 读面上一律拒收（负控）。具名夹具，不代替真实 S/Q 验收。
文件名取 `test_tb01c_*` 前缀，使 CI 的 `unittest discover -s s_session/tests -p 'test_tb01c_*.py'` 实际执行本锁。
"""
import re
import sqlite3
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import s_query_integrity as graph
import s_readonly_server as readonly

V2_RULE = "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2"
RUST_V1V2_READER = Path(__file__).resolve().parents[2] / "rust/src/bin/s_structure_session.rs"

# (object_id, window_start, first_known_generation, withdrawn_generation)：窗口序、出生序与撤回代序两两不同。
ROWS = (("o-a", 0, 1, 4), ("o-b", 10, 2, 3), ("o-c", 20, 3, None), ("o-d", 5, 4, None))


def _v2_objects_db():
    conn = sqlite3.connect(":memory:")
    conn.executescript(graph.LEGACY_SCHEMA)
    for oid, ws, fkg, wg in ROWS:
        conn.execute(
            "INSERT INTO objects VALUES (?,1,'shape','batch-x','TOP','up','down',?,?,?,'[]','[]','[]',?,?,?,?,?,NULL)",
            (oid, ws, ws + 1, ws + 2, fkg, f"cut-{fkg}", fkg, wg, None if wg is None else "superseded"))
    return conn


def _ids(rows):
    return [r["object_id"] for r in rows]


def _rust_order_by(const):
    text = RUST_V1V2_READER.read_text(encoding="utf-8")
    match = re.search(r'const %s: &str = "([^"]*)";' % const, text)
    if match is None:
        raise AssertionError("Rust v1/v2 读者常量缺失：" + const)
    return match.group(1).rsplit("ORDER BY ", 1)[1]


class V2ReaderHasNoLayoutTests(unittest.TestCase):
    def test_v2_rule_without_layout_component_selects_axes(self):
        self.assertEqual(graph._published_axes(V2_RULE), graph.tb02.AXES)

    def test_v2_reader_rejects_any_layout_component(self):
        # 负控：v2 读面没有出生序读法；bounded-layout 串（含未知版本、位置互换）不得被静默接受。
        for rule in (V2_RULE + "+bounded-layout/1", V2_RULE + "+bounded-layout/2",
                     "s2-axis-quantifiers+bounded-layout/1+new-bi/2"):
            with self.assertRaisesRegex(ValueError, "cut规则修订未声明"):
                graph._published_axes(rule)

    def test_readonly_object_order_is_the_rust_v2_reader_order(self):
        self.assertEqual(_rust_order_by("OBJECT_ACTIVE_SQL"), "window_start")
        self.assertEqual(_rust_order_by("OBJECT_ACTIVE_ASOF_SQL"), "window_start")
        self.assertEqual(_rust_order_by("OBJECT_DEAD_SQL"), "first_known_generation")
        self.assertEqual(_rust_order_by("OBJECT_DEAD_ASOF_SQL"), "first_known_generation")
        conn = _v2_objects_db()
        active, withdrawn = readonly._read_objects_view(conn, None)
        self.assertEqual(_ids(active), ["o-d", "o-c"])
        self.assertEqual(_ids(withdrawn), ["o-a", "o-b"])
        active, withdrawn = readonly._read_objects_view(conn, 3)
        self.assertEqual(_ids(active), ["o-a", "o-c"])
        self.assertEqual(_ids(withdrawn), ["o-b"])

    def test_readonly_object_order_is_not_birth_order(self):
        # 负控：同一夹具的出生序／撤回代序与 v2 次序不同，v2 读面不得给出它们。
        active, withdrawn = readonly._read_objects_view(_v2_objects_db(), None)
        births = [oid for oid, _ws, _f, wg in sorted(ROWS, key=lambda r: r[2]) if wg is None]
        withdrawals = [oid for oid, _ws, _f, wg in sorted(ROWS, key=lambda r: r[3] or 0) if wg is not None]
        self.assertEqual((births, withdrawals), (["o-c", "o-d"], ["o-b", "o-a"]))
        self.assertNotEqual(_ids(active), births)
        self.assertNotEqual(_ids(withdrawn), withdrawals)


class V2ProjectStateOrderTests(unittest.TestCase):
    @staticmethod
    def _project(gen):
        typed = graph.tb02.KINDS[0]
        objects = [{"object_id": oid, "kind": "shape", "fact_key": "", "window_start": str(ws),
                    "first_known_generation": str(fkg), "withdrawn_generation": None if wg is None else str(wg)}
                   for oid, ws, fkg, wg in ROWS]
        objects += [{"object_id": "t-z", "kind": typed, "fact_key": "f-1", "window_start": "0",
                     "first_known_generation": "1", "withdrawn_generation": None},
                    {"object_id": "t-a", "kind": typed, "fact_key": "f-0", "window_start": "0",
                     "first_known_generation": "4", "withdrawn_generation": None}]
        delta = {"input_frontier": "-1", "seq_range": {"from": "0", "to": "-1"}, "catalog_run_status": "not_run",
                 "catalog_evidence": {}, "index_frontier": "batch-x"}
        proof = {"generation": 4, "deltas": [delta] * 4, "batches": {}, "raw": [], "tables": {"catalog": []},
                 "meta": {"session_id": "s", "catalog_revision": "c", "rule_revision": V2_RULE, "profile_id": "p"},
                 "wires": {"objects": [(int(o["first_known_generation"]), o) for o in objects],
                           "witnesses": [], "relations": [], "observations": []}}
        h = {"InvalidQuery": ValueError, "_scope": lambda meta: {}, "_project_wire_integers": lambda v: v}
        return graph.project_state(proof, gen, h)["snapshot"]

    def test_project_state_keeps_v2_content_order(self):
        snapshot = self._project(4)
        self.assertEqual(_ids(snapshot["objects"]), ["o-d", "o-c", "t-a", "t-z"])
        self.assertEqual(_ids(snapshot["withdrawn_objects"]), ["o-a", "o-b"])
        snapshot = self._project(3)
        self.assertEqual(_ids(snapshot["objects"]), ["o-a", "o-c", "t-z"])
        self.assertEqual(_ids(snapshot["withdrawn_objects"]), ["o-b"])


if __name__ == "__main__":
    unittest.main()
