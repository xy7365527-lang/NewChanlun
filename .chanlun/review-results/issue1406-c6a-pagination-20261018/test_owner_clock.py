"""#1406 外部owner时钟协议纯离线正负控；不启动产品或网络。"""
import copy
import importlib.util
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location('c6a_owner_clock_runner', ROOT / '.github/scripts/run-1406-c6a.py')
r = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = r
spec.loader.exec_module(r)


def transcript():
    rows, case_end = [], 600.
    for index, op in enumerate(r.ANCHOR_OPS):
        issued = 1. + index * .1
        request = {'schema': r.ANCHOR_SCHEMA, 'run_id': 'a' * 32, 'seq': index + 1, 'op': op}
        if op == 'work':
            anchor = {'t0': 0., 'work_deadline': 600., 'final_deadline': 630.}
        elif op.endswith('-case'):
            case_end = min(600., issued + 300)
            anchor = {'started_at': issued, 'case_deadline': case_end, 'work_deadline': 600.}
        else:
            anchor = {'started_at': issued, 'deadline': min(issued + 10, case_end, 600.),
                      'case_deadline': case_end, 'work_deadline': 600.}
        rows.append({'request': request, 'response': {**request, 'anchor': anchor},
                     'received': issued - .01, 'issued': issued, 'sent': issued + .01})
    return {'schema': r.ANCHOR_SCHEMA, 'run_id': 'a' * 32, 'driver_pid': 123,
            'started': 0., 'outer_deadline': 30., 'rows': rows,
            'eof': True, 'failure': None, 'complete': True}


class OwnerClockTests(unittest.TestCase):
    def test_complete_exact_case_domain_schedule_and_ten_second_window(self):
        q, owners = r.verify_clock_transcript(transcript(), 0., 20.)
        self.assertEqual(set(q), {'we5', 'we7'})
        self.assertEqual(set(owners['we5']), {'E', 'B:chong-R1'})
        self.assertEqual(set(owners['we7']), {'E', 'B:chong-N1', 'B:chong-N2', 'B:chong-N3'})
        for case in owners.values():
            for anchor in case.values():
                self.assertEqual(anchor['deadline'], anchor['started_at'] + 10)

    def test_missing_wrong_domain_order_replay_and_deadline_mutations(self):
        mutations = [
            lambda t: t['rows'].pop(2),
            lambda t: t['rows'][2]['request'].update(op='we5-owner:WRONG'),
            lambda t: t['rows'].__setitem__(slice(2, 4), t['rows'][2:4][::-1]),
            lambda t: t['rows'].__setitem__(3, copy.deepcopy(t['rows'][2])),
            lambda t: t['rows'][2]['response']['anchor'].update(deadline=2),
            lambda t: t['rows'][2]['response']['anchor'].update(deadline=999),
            lambda t: t['rows'][2]['response']['anchor'].update(started_at=float('nan')),
            lambda t: t.update(schema='c6a-clock/1'),
        ]
        for mutate in mutations:
            value = transcript()
            mutate(value)
            with self.subTest(mutation=mutate), self.assertRaises(RuntimeError):
                r.verify_clock_transcript(value, 0., 20.)

    def test_broker_emits_full_schedule_then_eof_without_socket_io(self):
        parent, child = Mock(), Mock()
        parent.send.side_effect = len
        broker = r.ClockBroker(parent, child, 'a' * 32)
        broker.arm(0., 30.)
        broker.pid = 123
        for index, op in enumerate(r.ANCHOR_OPS):
            issued = 1. + index * .1
            request = {'schema': r.ANCHOR_SCHEMA, 'run_id': 'a' * 32, 'seq': index + 1, 'op': op}
            with patch.object(r, 'clock', return_value=issued), patch.object(r, 'anchor_packet', return_value=r.anchor_encode(request)):
                broker.tick()
            self.assertIsNone(broker.failure)
        self.assertFalse(broker.complete())
        with patch.object(r, 'clock', return_value=3.), patch.object(r, 'anchor_packet', return_value=b''):
            broker.tick()
        self.assertTrue(broker.complete())
        q, owners = r.verify_clock_transcript(broker.transcript(), 0., 20.)
        self.assertEqual(len(owners['we5']) + len(owners['we7']), 6)
        broker.close()

    def test_runner_entry_checks_digest_then_calls_embedded_verifier(self):
        # Exercise the public runner wrapper without loading product evidence.
        sentinel = {'status': 'ENTRY_REACHED'}
        with patch.object(r, 'exec') as execute:
            execute.side_effect = lambda source, namespace: namespace.update(verify_runtime=lambda *args: sentinel)
            self.assertIs(r.verify_runtime('unused', {}, {}), sentinel)
            execute.assert_called_once()
        with patch.object(r, 'SEMANTICS_SHA256', '0' * 64), patch.object(r, 'exec') as execute:
            with self.assertRaisesRegex(RuntimeError, 'embedded semantics bytes changed'):
                r.verify_runtime('unused', {}, {})
            execute.assert_not_called()

    def test_embedded_semantics_exact_and_limits_unchanged(self):
        expected = (Path(__file__).parent / 'semantics.py').read_text()
        self.assertEqual(r.SEMANTICS_SOURCE, expected)
        self.assertEqual(r.ENDPOINTS, {'build': 1200, 'runtime': 1830, 'verify': 1920, 'archive': 2040, 'final': 2070})


if __name__ == '__main__':
    unittest.main()
