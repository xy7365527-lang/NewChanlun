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


if __name__ == '__main__':
    unittest.main()
