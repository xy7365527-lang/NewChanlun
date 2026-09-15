"""#1394 私有图完整性；具名格式夹具，不代替真实 S/Q 验收。"""
import copy
import hashlib
import unittest

import s_query_integrity as graph


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
