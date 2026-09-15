"""#1394 私有图完整性；具名格式夹具，不代替真实 S/Q 验收。"""
import cProfile
import copy
import hashlib
import itertools
import sqlite3
import sys
import unittest
from unittest import mock

import s_query_integrity as graph


class EvidenceSchemaCaptureTests(unittest.TestCase):
    def test_bounded_capture_accepts_every_declared_schema(self):
        for parts in itertools.product(graph._CORE_SCHEMAS, graph._FACT_SCHEMAS,
                                       graph._FORMAT_SCHEMAS, graph._GRAPH_SCHEMAS):
            with self.subTest(parts=tuple(bool(part) for part in parts)):
                conn = sqlite3.connect(":memory:")
                try:
                    conn.executescript("".join(parts))
                    conn.execute("BEGIN")
                    captured = graph.capture(conn, max_bytes=268435456)
                    self.assertIn(captured["schema"], graph.EXPECTED_SCHEMAS)
                finally:
                    conn.close()

    def test_bounded_capture_still_rejects_extra_schema_objects(self):
        conn = sqlite3.connect(":memory:")
        try:
            conn.executescript(graph.LEGACY_SCHEMA + graph.CONTROL_SCHEMA + graph.tb02.SCHEMA
                               + graph.STORAGE_FORMAT_SCHEMA + graph.EVIDENCE_GRAPH_SCHEMA)
            conn.execute("CREATE TABLE extra_untrusted(value TEXT)")
            conn.execute("BEGIN")
            with self.assertRaises(graph.QueryBudgetExceeded):
                graph.capture(conn, max_bytes=268435456)
        finally:
            conn.close()


def node(kind, payload, length):
    return {"node_id": hashlib.sha256(bytes([kind]) + payload).digest(),
            "node_kind": kind, "payload": payload, "expanded_length": length}


def fixture(value=b"9"):
    prefix, suffix = b'{"v":', b'}'
    payload = (b'\x00' + len(prefix).to_bytes(4, 'big') + prefix + b'\x02\x00\x00'
               + b'\x00' + len(suffix).to_bytes(4, 'big') + suffix)
    seq, param = node(1, payload, len(prefix) + len(suffix)), node(0, value, len(value))
    expected = prefix + value + suffix
    digest = hashlib.sha256(expected).hexdigest()
    tables = {"s_evidence_format": [{"singleton": 1, "graph_format": graph.EVIDENCE_GRAPH_FORMAT}],
              "s_evidence_nodes": [seq, param],
              "s_evidence_roots": [{"root_key": "batch/batch-" + digest,
                  "node_id": seq["node_id"], "slot_ids": param["node_id"],
                  "expanded_length": len(expected), "content_sha256": digest}]}
    return tables, "batch-" + digest, expected


class EvidenceGraphIntegrityTests(unittest.TestCase):
    def test_parameter_width_preserves_same_sequence_length(self):
        for value in (b'9', b'1000'):
            tables, key, expected = fixture(value)
            self.assertEqual(graph.evidence_batch_sources(tables), {key: expected})

    def test_sequence_length_tampering_is_rejected(self):
        tables, _, _ = fixture()
        tables['s_evidence_nodes'][0]['expanded_length'] += 1
        with self.assertRaisesRegex(ValueError, '节点声明长度'):
            graph.evidence_batch_sources(tables)

    def test_well_formed_orphan_is_rejected(self):
        tables, _, _ = fixture()
        tables['s_evidence_nodes'].append(node(0, b'orphan', 6))
        with self.assertRaisesRegex(ValueError, '不可达节点'):
            graph.evidence_batch_sources(tables)

    def test_truncated_component_is_rejected_even_with_unchanged_expanded_bytes(self):
        tables, _, _ = fixture()
        broken = copy.deepcopy(tables)
        seq = broken['s_evidence_nodes'][0]
        # 超长字面声明可在旧切片解码器中静默截到末尾，最终展开字节依然相同。
        payload = bytearray(seq['payload'])
        payload[-5:-1] = (100).to_bytes(4, 'big')
        seq.update(node(1, bytes(payload), seq['expanded_length']))
        broken['s_evidence_roots'][0]['node_id'] = seq['node_id']
        with self.assertRaisesRegex(ValueError, '内容截断'):
            graph.evidence_batch_sources(broken)

    def test_superseded_graph_format_is_rejected(self):
        tables, _, _ = fixture()
        tables['s_evidence_format'][0]['graph_format'] = 'evidence-graph/1'
        with self.assertRaisesRegex(ValueError, '未知/损坏'):
            graph.evidence_batch_sources(tables)

    def test_deadline_is_checked_during_expansion(self):
        tables, _, _ = fixture()
        with self.assertRaises(graph.QueryBudgetExceeded):
            graph.evidence_batch_sources(tables, deadline=0)


def graph3_fixture():
    """#1394 第二段：批次 + Delta 原件 + 结构事实三列原件同图（evidence-graph/3）。"""
    parts = [b'[', b'"9"', b',', b'"9"', b',', b'"9"', b']']
    payload = b''.join(b'\x00' + len(part).to_bytes(4, 'big') + part for part in parts)
    expanded = b'["9","9","9"]'
    seq = node(1, payload, len(expanded))
    digest = hashlib.sha256(expanded).hexdigest()
    tables = {
        "s_evidence_format": [{"singleton": 1, "graph_format": graph.EVIDENCE_GRAPH_FORMAT}],
        "s_evidence_nodes": [seq],
        "s_evidence_roots": [
            {"root_key": "batch/batch-" + digest, "node_id": seq["node_id"],
             "slot_ids": b"", "expanded_length": len(expanded),
             "content_sha256": digest},
            {"root_key": "delta/1", "node_id": seq["node_id"],
             "slot_ids": b"", "expanded_length": len(expanded),
             "content_sha256": digest},
            {"root_key": "fact/obj-1", "node_id": seq["node_id"],
             "slot_ids": b"", "expanded_length": len(expanded),
             "content_sha256": digest},
        ],
        "structure_deltas": [{"generation": 1, "delta_json": ""}],
        "structure_facts": [{"object_id": "obj-1", "payload_json": "",
                             "input_refs_json": "", "source_coords_json": ""}],
    }
    return tables, "batch-" + digest, expanded


def shared_sequence_fixture(mode):
    """多根引用同一层嵌套 SEQ，各根槽值及宽度不同。"""
    tables, key, expanded = fixture()
    tables["s_evidence_format"][0]["graph_format"] = mode
    expected = {"batch/" + key: expanded}
    for value in (b"1000", b"12345"):
        other, key, expanded = fixture(value)
        tables["s_evidence_nodes"].append(other["s_evidence_nodes"][1])
        tables["s_evidence_roots"].extend(other["s_evidence_roots"])
        expected["batch/" + key] = expanded
    shared = tables["s_evidence_nodes"][0]
    wrapper = node(1, b"\x01" + shared["node_id"], shared["expanded_length"])
    tables["s_evidence_nodes"].append(wrapper)
    for root in tables["s_evidence_roots"]:
        root["node_id"] = wrapper["node_id"]
    if mode == graph.EVIDENCE_GRAPH_FORMAT:
        tables["structure_deltas"] = [{"generation": 1}]
        tables["structure_facts"] = [{"object_id": "obj-1"}]
        source = tables["s_evidence_roots"][1]
        for key in ("delta/1", "fact/obj-1"):
            tables["s_evidence_roots"].append(dict(source, root_key=key))
            expected[key] = expected[source["root_key"]]
    return tables, expected


class EvidenceSharedSequenceTests(unittest.TestCase):
    modes = (graph.EVIDENCE_GRAPH_FORMAT_V2, graph.EVIDENCE_GRAPH_FORMAT)
    cache_limits = (0, 1024, 1024 * 1024)

    def test_shared_sequence_keeps_each_roots_slots_with_or_without_cache(self):
        for mode, limit in itertools.product(self.modes, self.cache_limits):
            with self.subTest(mode=mode, limit=limit):
                tables, expected = shared_sequence_fixture(mode)
                with mock.patch.object(graph, "_EVIDENCE_FRAMES_MAX_BYTES", limit):
                    self.assertEqual(graph.evidence_sources(tables, mode), expected)

    def test_shared_sequence_decodes_integer_fields_once_per_capture(self):
        for mode in self.modes:
            tables, expected = shared_sequence_fixture(mode)
            profiler = cProfile.Profile()
            self.assertEqual(profiler.runcall(graph.evidence_sources, tables, mode), expected)
            # 两个字面长度和一个 slot 索引只拆一次；根及槽字节仍分别展开。
            calls = sum(entry.callcount for entry in profiler.getstats()
                        if entry.code == "<built-in method from_bytes>")
            with self.subTest(mode=mode):
                self.assertEqual(calls, 3)

    def test_expanded_digest_is_computed_once_for_each_root(self):
        tables, expected = shared_sequence_fixture(graph.EVIDENCE_GRAPH_FORMAT)
        sha256 = hashlib.sha256
        with mock.patch.object(graph.hashlib, "sha256", wraps=sha256) as counted:
            self.assertEqual(graph.evidence_sources(tables, graph.EVIDENCE_GRAPH_FORMAT), expected)
        for value in set(expected.values()):
            self.assertEqual(sum(call.args == (value,) for call in counted.call_args_list),
                             sum(item == value for item in expected.values()))

    def test_cache_accounts_for_python_objects_and_stops_retaining_at_its_limit(self):
        with mock.patch.object(graph, "_EVIDENCE_FRAMES_MAX_BYTES", 1024):
            cache = graph._EvidenceFrames()
            # 原 payload 只有 200 字节，40 个 Python 分量的对象开销已超过缓存容量。
            payload = b"\x00\x00\x00\x00\x00" * 40
            seq = node(1, payload, 0)
            self.assertEqual(len(cache.decode(seq["node_id"], payload, None)), 40)
            self.assertFalse(cache.frames)
        with mock.patch.object(graph, "_EVIDENCE_FRAMES_MAX_BYTES", 4096):
            cache = graph._EvidenceFrames()
            for index in range(40):
                payload = b"\x00\x00\x00\x00\x01" + bytes([index])
                seq = node(1, payload, 1)
                self.assertEqual(cache.decode(seq["node_id"], payload, None), [("lit", bytes([index]))])
                retained = (sys.getsizeof(cache) + sys.getsizeof(cache.remaining)
                            + graph.object_bytes(cache.frames))
                self.assertLessEqual(retained, 4096)
            self.assertGreater(len(cache.frames), 0)
            self.assertLess(len(cache.frames), 40)

    def test_later_root_errors_are_not_hidden_by_shared_frames(self):
        cases = (
            ("length", "根展开长度"), ("digest", "登记摘要"),
            ("batch_id", "公开批次身份"), ("slots", "参数槽越界"),
            ("slot_width", "32 字节倍数"), ("missing_slot", "必须指向存在的叶"),
            ("missing_root", "根引用了不存在"), ("missing_child", "节点缺失"),
            ("truncated", "内容截断"), ("truncated_length", "长度字段截断"),
            ("truncated_ref", "引用分量截断"), ("truncated_slot", "参数槽分量截断"),
            ("bad_marker", "分量标签未声明"), ("node_length", "节点声明长度"),
            ("orphan", "不可达节点"),
            ("same_id", "同 ID 异内容"), ("namespace", "命名空间"),
            ("duplicate_root", "key 未声明或重复"),
        )
        for mode, limit, (case, message) in itertools.product(
                self.modes, self.cache_limits, cases):
            with self.subTest(mode=mode, limit=limit, case=case):
                tables, _ = shared_sequence_fixture(mode)
                root = tables["s_evidence_roots"][1]
                if case == "length":
                    root["expanded_length"] += 1
                elif case == "digest":
                    root["content_sha256"] = "0" * 64
                elif case == "batch_id":
                    root["root_key"] = "batch/batch-" + "0" * 64
                elif case == "slots":
                    root["slot_ids"] = b""
                elif case == "slot_width":
                    root["slot_ids"] += b"x"
                elif case == "missing_slot":
                    root["slot_ids"] = b"x" * 32
                elif case == "missing_root":
                    root["node_id"] = b"x" * 32
                elif case in ("missing_child", "truncated", "truncated_length", "truncated_ref",
                              "truncated_slot", "bad_marker", "node_length"):
                    tail = {"missing_child": b"\x01" + b"x" * 32,
                            "truncated": b"\x00\x00\x00\x00\x02x", "truncated_length": b"\x00",
                            "truncated_ref": b"\x01x", "truncated_slot": b"\x02x",
                            "bad_marker": b"\x03", "node_length": b"\x00\x00\x00\x00\x00"}[case]
                    bad = node(1, b"\x01" + root["node_id"] + tail, root["expanded_length"])
                    tables["s_evidence_nodes"].append(bad)
                    root["node_id"] = bad["node_id"]
                elif case == "orphan":
                    tables["s_evidence_nodes"].append(node(0, b"orphan", 6))
                elif case == "same_id":
                    tables["s_evidence_nodes"][0]["payload"] += b"x"
                elif case == "namespace":
                    root["root_key"] = "unknown/1"
                elif case == "duplicate_root":
                    root["root_key"] = tables["s_evidence_roots"][0]["root_key"]
                with mock.patch.object(graph, "_EVIDENCE_FRAMES_MAX_BYTES", limit):
                    with self.assertRaisesRegex(ValueError, message):
                        graph.evidence_sources(tables, mode)

    def test_graph3_shared_roots_still_need_corresponding_rows(self):
        for limit, table in itertools.product(self.cache_limits,
                                              ("structure_deltas", "structure_facts")):
            with self.subTest(limit=limit, table=table):
                tables, _ = shared_sequence_fixture(graph.EVIDENCE_GRAPH_FORMAT)
                tables[table] = []
                with mock.patch.object(graph, "_EVIDENCE_FRAMES_MAX_BYTES", limit):
                    with self.assertRaisesRegex(ValueError, "没有对应"):
                        graph.evidence_sources(tables, graph.EVIDENCE_GRAPH_FORMAT)

    def test_new_capture_checks_original_bytes_and_node_lengths_again(self):
        for mode, limit, change in itertools.product(self.modes, self.cache_limits,
                                                     ("payload", "expanded_length")):
            with self.subTest(mode=mode, limit=limit, change=change):
                tables, expected = shared_sequence_fixture(mode)
                with mock.patch.object(graph, "_EVIDENCE_FRAMES_MAX_BYTES", limit):
                    self.assertEqual(graph.evidence_sources(tables, mode), expected)
                    fresh = copy.deepcopy(tables)
                    shared = fresh["s_evidence_nodes"][0]
                    if change == "payload":
                        shared[change] += b"x"
                        message = "同 ID 异内容"
                    else:
                        shared[change] += 1
                        message = "节点声明长度"
                    with self.assertRaisesRegex(ValueError, message):
                        graph.evidence_sources(fresh, mode)

    def test_deadline_still_interrupts_later_roots_with_cached_frames(self):
        for mode, limit in itertools.product(self.modes, self.cache_limits):
            with self.subTest(mode=mode, limit=limit):
                tables, _ = shared_sequence_fixture(mode)
                expand = graph._evidence_expand
                completed = []

                def expand_then_expire(*args, **kwargs):
                    result = expand(*args, **kwargs)
                    completed.append(result)
                    return result

                with mock.patch.object(graph, "_EVIDENCE_FRAMES_MAX_BYTES", limit), \
                        mock.patch.object(graph, "_evidence_expand", side_effect=expand_then_expire), \
                        mock.patch.object(graph.time, "monotonic", side_effect=lambda: len(completed)):
                    with self.assertRaises(graph.QueryBudgetExceeded):
                        graph.evidence_sources(tables, mode, deadline=0.5)
                self.assertEqual(len(completed), 1)


class EvidenceGraph3IntegrityTests(unittest.TestCase):
    def test_delta_and_fact_roots_are_expanded_into_the_same_rows(self):
        tables, key, expanded = graph3_fixture()
        sources = graph.evidence_sources(tables, graph.EVIDENCE_GRAPH_FORMAT)
        self.assertEqual(set(sources), {"batch/" + key, "delta/1", "fact/obj-1"})
        self.assertEqual(sources["batch/" + key], expanded)
        graph.materialize_graph_originals(tables, graph.EVIDENCE_GRAPH_FORMAT, sources)
        self.assertEqual(tables["structure_deltas"][0]["delta_json"], expanded.decode())
        self.assertEqual(
            [tables["structure_facts"][0][column] for column in
             ("payload_json", "input_refs_json", "source_coords_json")], ["9", "9", "9"])

    def test_row_columns_must_stay_empty_in_graph3(self):
        tables, _, _ = graph3_fixture()
        tables["structure_deltas"][0]["delta_json"] = "{}"
        sources = graph.evidence_sources(tables, graph.EVIDENCE_GRAPH_FORMAT)
        with self.assertRaisesRegex(ValueError, "仍带整块原文"):
            graph.materialize_graph_originals(tables, graph.EVIDENCE_GRAPH_FORMAT, sources)

    def test_delta_root_without_row_is_rejected(self):
        tables, _, _ = graph3_fixture()
        tables["structure_deltas"] = []
        with self.assertRaisesRegex(ValueError, "没有对应 Delta 行"):
            graph.evidence_sources(tables, graph.EVIDENCE_GRAPH_FORMAT)

    def test_extra_delta_root_aliases_are_rejected(self):
        for alias in ("delta/01", "delta/+1", "delta/١"):
            with self.subTest(alias=alias):
                tables, _, _ = graph3_fixture()
                tables["s_evidence_roots"].append(
                    dict(tables["s_evidence_roots"][1], root_key=alias))
                with self.assertRaisesRegex(ValueError, "Delta 私有根"):
                    graph.evidence_sources(tables, graph.EVIDENCE_GRAPH_FORMAT)

    def test_missing_delta_root_is_rejected(self):
        tables, _, _ = graph3_fixture()
        tables["s_evidence_roots"] = [
            row for row in tables["s_evidence_roots"] if not row["root_key"].startswith("delta/")]
        sources = graph.evidence_sources(tables, graph.EVIDENCE_GRAPH_FORMAT)
        with self.assertRaisesRegex(ValueError, "Delta 私有根缺失"):
            graph.materialize_graph_originals(tables, graph.EVIDENCE_GRAPH_FORMAT, sources)

    def test_graph2_domain_rejects_delta_and_fact_roots(self):
        tables, _, _ = graph3_fixture()
        tables["s_evidence_format"][0]["graph_format"] = graph.EVIDENCE_GRAPH_FORMAT_V2
        with self.assertRaisesRegex(ValueError, "未声明该根命名空间"):
            graph.evidence_sources(tables, graph.EVIDENCE_GRAPH_FORMAT_V2)


if __name__ == '__main__':
    unittest.main()
