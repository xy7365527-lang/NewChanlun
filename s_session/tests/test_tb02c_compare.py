"""#1404：完整性、精确 JSON 和跨块比较回归；只用临时普通文件。"""
import json
from pathlib import Path
import tempfile
import unittest

from tb02c_compare import CHUNK_BYTES, compare_exact_traces, expected_layout
from tb02c_harness import save_comparison


class TraceComparisonTests(unittest.TestCase):
    def test_reverification_preserves_existing_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'COMPARISON-RESULT.json'
            receipt = {'status': 'passed', 'comparisons': [{'case': 'first_up', 'status': 'PASS'}]}
            save_comparison(path, receipt)
            original = path.read_bytes()
            stamp = path.stat().st_mtime_ns
            save_comparison(path, receipt)
            self.assertEqual(path.read_bytes(), original)
            self.assertEqual(path.stat().st_mtime_ns, stamp)
            with self.assertRaisesRegex(ValueError, '原收据保留'):
                save_comparison(path, {'status': 'NOT_VERIFIED'})
            self.assertEqual(path.read_bytes(), original)

    def compare(self, left, right=None, layout=None):
        with tempfile.TemporaryDirectory() as directory:
            a, b = [Path(directory) / name for name in ('a.jsonl', 'b.jsonl')]
            a.write_bytes(left)
            b.write_bytes(left if right is None else right)
            return compare_exact_traces(a, b, layout or [{'kind': 'x'}])

    def test_large_record_is_fully_read_in_chunks(self):
        # 大数组跨过旧16MiB记录上限；每个标量很小。
        with tempfile.TemporaryDirectory() as directory:
            paths = [Path(directory) / name for name in ('a', 'b')]
            for path in paths:
                with path.open('wb') as stream:
                    stream.write(b'{"kind":"x","rows":[')
                    for _ in range(257):
                        stream.write(b'"' + b'x' * (CHUNK_BYTES - 4) + b'",')
                    stream.write(b'null]}\n')
            result = compare_exact_traces(*paths, [{'kind': 'x'}])
            self.assertEqual(result['status'], 'PASS', result)
            self.assertGreater(result['compared_bytes'], 16 * 1024 * 1024)
            self.assertEqual(result['compared_bytes'], paths[0].stat().st_size)
            self.assertEqual(result['sha256'][0], result['sha256'][1])

    def test_precise_json_required_even_when_bytes_match(self):
        invalid = [b'{}\n', b'[]\n', b'\n', b'{"kind":"x","v":1.0}\n',
                   b'{"kind":"x","v":NaN}\n', b'{"kind":"x","v":Infinity}\n',
                   b'{"kind":"x","v":1e0}\n', b'{"kind":"x","v":{"a":1,"\\u0061":2}}\n',
                   b'{"kind":"x","v":"\xff"}\n', b'{"kind":"x",}\n',
                   b'{"kind":"x"} {"kind":"x"}\n', b'{"kind":"x"',
                   b'{"kind":[]}', b'{"kind":"x","v":[\n1]}\n']
        for raw in invalid:
            with self.subTest(raw=raw):
                self.assertEqual(self.compare(raw)['status'], 'NOT_VERIFIED')

    def test_shared_truncation_and_wrong_identity_cannot_pass(self):
        layout = [{'kind': 'x', 'operation': '0'}, {'kind': 'x', 'operation': '1'}]
        first = b'{"kind":"x","operation":"0"}\n'
        self.assertEqual(self.compare(first, layout=layout)['status'], 'NOT_VERIFIED')
        self.assertEqual(self.compare(first * 2, layout=layout)['status'], 'NOT_VERIFIED')
        self.assertEqual(self.compare(first + b'{"kind":"x","operation":"1"}\n', layout=layout)['status'], 'PASS')

    def test_all_bytes_and_extra_tail_are_checked(self):
        a = b'{"kind":"x","padding":"' + b'a' * (CHUNK_BYTES * 2) + b'","tail":[1,2]}\n'
        b = a.replace(b'[1,2]', b'[2,1]')
        self.assertEqual(self.compare(a, b)['status'], 'NOT_VERIFIED')
        self.assertEqual(self.compare(a, a + b'\n')['status'], 'NOT_VERIFIED')
        self.assertEqual(self.compare(a + b'\n')['status'], 'NOT_VERIFIED')

    def test_object_key_order_is_only_unverified_not_semantic_failure(self):
        result = self.compare(b'{"kind":"x","a":1}', b'{"a":1,"kind":"x"}')
        self.assertEqual(result['status'], 'NOT_VERIFIED')
        self.assertIn('尚未证明语义相同', result['detail'])

    def test_layout_binds_complete_query_and_table_sequence(self):
        layout = expected_layout(28)
        self.assertEqual(len(layout), 161)
        self.assertEqual(layout[56]['command.id'], 'live-0')
        self.assertEqual(layout[56]['command.op'], 'load')
        self.assertEqual(layout[58]['command.op'], 'watch')
        self.assertEqual(layout[111]['command.id'], 'known-27')
        self.assertEqual(layout[112]['command.id'], 'revisit-0')
        self.assertEqual(layout[139]['command.id'], 'revisit-27')
        self.assertEqual(layout[140], {'kind': 'authoritative_table', 'table': 'batches'})
        self.assertEqual(layout[-1], {'kind': 'normal_recovery_cut', 'generation': '28'})
        raw = json.dumps({'kind': 'public_candidate', 'command': {'id': 'live-0', 'client': 'live', 'op': 'load'}, 'candidate': {}}).encode()
        self.assertEqual(self.compare(raw, layout=[layout[56]])['status'], 'PASS')
        spoof = b'{"kind":"public_candidate","command.id":"live-0","command.client":"live","command.op":"watch"}'
        self.assertEqual(self.compare(spoof, layout=[layout[56]])['status'], 'NOT_VERIFIED')
        missing_payload = b'{"kind":"public_candidate","command":{"id":"live-0","client":"live","op":"load"}}'
        self.assertEqual(self.compare(missing_payload, layout=[layout[56]])['status'], 'NOT_VERIFIED')


if __name__ == '__main__':
    unittest.main()
