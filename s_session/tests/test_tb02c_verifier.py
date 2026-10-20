"""#1404：验收在优化模式下仍拒绝不属于该工作例的合法镜像结果。"""
from contextlib import closing
import hashlib
import json
from pathlib import Path
import subprocess
import sqlite3
import sys
import unittest

TESTS = Path(__file__).resolve().parent
sys.path.insert(0, str(TESTS))
sys.path.insert(0, str(TESTS.parent))
import s_query_integrity as audit
import s_readonly_server as server
import s_tb02_contract as contract
from tb02c_verify import check


def oracle_projection(case):
    """仅为 oracle 函数准备公共结果的所需字段；不冒充完整 wire 验收。"""
    oracle = json.loads((TESTS / 'fixtures/tb02c/hand-oracle.json').read_text())['cuts']['15']
    sign = 1 if case == 'first_up' else -1
    prices = [p * sign for p in oracle['prices']]
    refs = [{'start_anchor': str(a), 'end_anchor': str(b),
             'start_price': str(x), 'end_price': str(y)}
            for a, b, x, y in zip(oracle['stroke_anchors'], oracle['stroke_anchors'][1:], prices, prices[1:])]
    overlap = oracle['seed_overlap'] if sign == 1 else [-oracle['seed_overlap'][1], -oracle['seed_overlap'][0]]
    return {'objects': [
        {'kind': 'CC-011.segment_seed', 'payload': {'data': {
            'entity_id': 'seed:1:1', 'construction': {'vector': oracle['seed_vector'],
            'overlap': list(map(str, overlap)), 'direction': 'UP' if sign == 1 else 'DOWN'}}}},
        {'kind': 'CC-012.feature_sequence', 'payload': {'data': {
            'entity_id': 'first:segment:1:1', 'stroke_refs': refs,
            'facts': {'raw_elements': [{'stroke_idx': str(i)} for i in oracle['first_members']]}}}},
        {'kind': 'CC-013.segment', 'payload': {'data': {
            'entity_id': 'segment:1:1', 'result': oracle['result'], 'geometric_end': None,
            'terminated_known_at': None, 'waiting_reasons': [oracle['reason']]}}},
    ]}


class VerificationBoundaryTest(unittest.TestCase):
    def test_mirror_is_valid_only_for_its_own_oracle(self):
        snapshot = oracle_projection('first_down')
        check(snapshot, 15, 'first_down')
        with self.assertRaises(ValueError):
            check(snapshot, 15, 'first_up')

    def test_optimized_python_still_checks_the_oracle(self):
        program = '''from test_tb02c_verifier import oracle_projection
from tb02c_verify import check
snapshot = oracle_projection("first_down")
check(snapshot, 15, "first_down")
try:
    check(snapshot, 15, "first_up")
except ValueError:
    print("opposite legal case rejected")
else:
    raise RuntimeError("优化模式跳过了独立账簿比对")
'''
        result = subprocess.run([sys.executable, '-O', '-c', program], cwd=TESTS,
                                capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('opposite legal case rejected', result.stdout)

    def test_old_b_publication_does_not_acquire_future_axes(self):
        self.assertEqual(audit._published_axes('s2-axis-quantifiers'), contract.LEGACY_AXES)
        self.assertEqual(audit._published_axes('s2-axis-quantifiers+new-bi/1'), contract.BI_AXES)
        self.assertNotIn('CC-011', audit._published_axes('s2-axis-quantifiers+new-bi/1'))
        self.assertEqual(audit._published_axes('s2-axis-quantifiers+new-bi/1+segment-first/1'), contract.FIRST_SEGMENT_AXES)
        self.assertEqual(audit._published_axes('s2-axis-quantifiers+new-bi/2+segment-second/1'), contract.AXES)
        with self.assertRaisesRegex(ValueError, 'cut规则修订未声明'):
            audit._published_axes('unrecognized-rule')

    def test_empty_session_rejects_unknown_rule_before_any_publication(self):
        profile = json.loads((TESTS.parent / 'profiles/ohlc_integer_tb02a_v1.json').read_text())
        meta = dict(session_id='empty-rule-contract', catalog_revision='testonly',
                    writer_epoch='1', generation='0', structure_cut='cut-0', index_frontier='',
                    last_advance_frontier='-1', scope='{"economic":"not_started","structure":"CompleteCut"}',
                    profile_id=contract.PROFILE, input_profile=contract.PROFILE,
                    profile_definition=audit.canonical(profile).decode(),
                    profile_hash=hashlib.sha256(audit.canonical(profile)).hexdigest())
        rules = ('s2-axis-quantifiers', 's2-axis-quantifiers+new-bi/1',
                 's2-axis-quantifiers+new-bi/2',
                 's2-axis-quantifiers+new-bi/1+segment-first/1',
                 's2-axis-quantifiers+new-bi/1+segment-second/1',
                 's2-axis-quantifiers+new-bi/2+segment-second/1', 'unrecognized-rule')
        for rule in rules:
            with self.subTest(rule=rule), closing(sqlite3.connect(':memory:')) as conn:
                conn.executescript(audit.LEGACY_SCHEMA + contract.SCHEMA)
                conn.executemany('INSERT INTO meta(key,value) VALUES (?,?)',
                                 dict(meta, rule_revision=rule).items())
                conn.commit()
                conn.execute('BEGIN')
                image = audit.capture(conn)
                if rule == 'unrecognized-rule':
                    with self.assertRaisesRegex(ValueError, 'cut规则修订未声明'):
                        audit.verify(image, vars(server))
                else:
                    self.assertEqual(audit.verify(image, vars(server))['generation'], 0)


if __name__ == '__main__':
    unittest.main()
