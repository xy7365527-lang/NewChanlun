"""#1404：验收在优化模式下仍拒绝不属于该工作例的合法镜像结果。"""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import unittest

TESTS = Path(__file__).resolve().parent
sys.path.insert(0, str(TESTS))
sys.path.insert(0, str(TESTS.parent))
import s_query_integrity as audit
import s_tb02_contract as contract
from tb02c_verify import check
from test_tb02d_parse_pool import fixture, server


KNOWN_RULE_AXES = {
    's2-axis-quantifiers': contract.LEGACY_AXES,
    's2-axis-quantifiers+new-bi/1': contract.BI_AXES,
    's2-axis-quantifiers+new-bi/1+segment-first/1': contract.FIRST_SEGMENT_AXES,
    's2-axis-quantifiers+new-bi/1+segment-second/1': contract.AXES,
    's2-axis-quantifiers+new-bi/1+segment-second/2': contract.AXES,
}


def rule_image(generation, rule):
    """#1420 独审复现：从真实内存 SQLite capture 构造合法零/一代，不 mock verifier。"""
    image = fixture()

    def rows(table):
        names = [column[1] for column in image['schema'][table][0]]
        return [dict(zip(names, row)) for row in image['tables'][table]]

    def put(table, values):
        names = [column[1] for column in image['schema'][table][0]]
        image['tables'][table] = tuple(tuple(row[key] for key in names) for row in values)

    meta = {row['key']: row['value'] for row in rows('meta')}
    meta.update(generation=str(generation), structure_cut=f'cut-{generation}', rule_revision=rule)
    catalog = rows('catalog')
    if generation == 0:
        for table in image['tables']:
            if table not in ('meta', 'catalog'):
                put(table, [])
        meta.update(index_frontier='', last_advance_frontier='-1')
        for item in catalog:
            item.update(impl_status='not_implemented', proof_status='not_proved', run_status='not_run', evidence_json='{}')
    else:
        delta = rows('structure_deltas')[0]
        stored = next(batch for batch in rows('batches') if batch['batch_id'] == delta['index_frontier'])
        batch = json.loads(stored['canonical_bytes'])
        # 未知规则负控沿用合法 A 轴形状：只能因规则身份而拒绝，不能靠坏目录代替规则门。
        axes = {}
        for cid in KNOWN_RULE_AXES.get(rule, contract.LEGACY_AXES):
            evidence = dict(scope='TB-02-A', object_ids=[], waiting_reasons=[])
            if cid in ('CC-011', 'CC-012', 'CC-013', 'CC-014'):
                evidence['scope'] = 'TB-02-C'
                if 'segment-second/' in rule:
                    continued = rule.endswith('segment-second/2')
                    evidence.update(scope='TB-02-W' if continued else 'TB-02-D',
                                    delivered_subobligations=['finite_complete_prefix_strict_second', 'independent_stroke_and_segment_break_facts'] + (['general_scan_continuation'] if continued else []),
                                    deferred_subobligations=['general_equal_identity'] + ([] if continued else ['general_scan_continuation']) + ['global_G006_bridge'])
            if cid == 'CC-056':
                evidence.update(raw_revisions=[], withdrawals=[], replaces=[])
            axes[cid] = dict(impl_status='not_implemented', proof_status='not_proved', run_status='not_run', evidence=evidence)
        batch.update(rule_revision=rule, objects=[], catalog_axes=axes)
        content = audit.canonical(batch)
        batch_id = 'batch-' + hashlib.sha256(content).hexdigest()
        stored.update(batch_id=batch_id, canonical_bytes=content, byte_len=len(content))
        wire = json.loads(delta['delta_json'])
        evidence = wire['catalog_evidence']
        evidence.update(rule_revision=rule, batch_id=batch_id, axes=axes)
        wire.update(index_frontier=batch_id, upserts=[], catalog_run_status='not_run')
        delta.update(index_frontier=batch_id, catalog_run_status='not_run',
                     catalog_evidence_json=audit.canonical(evidence).decode(), delta_json=audit.canonical(wire).decode())
        meta.update(index_frontier=batch_id, last_advance_frontier='0')
        catalog = [dict(catalog_id=cid, kind='memory', title=cid, domain='memory', branches_json='[]',
                        impl_status=axis['impl_status'], proof_status=axis['proof_status'], run_status=axis['run_status'],
                        evidence_json=audit.canonical(axis['evidence']).decode()) for cid, axis in axes.items()]
        put('batches', [stored])
        put('structure_deltas', [delta])
        put('structure_facts', [])
    put('catalog', catalog)
    put('meta', [dict(key=key, value=value) for key, value in meta.items()])
    return image


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
        self.assertEqual(audit._published_axes('s2-axis-quantifiers+new-bi/1+segment-second/1'), contract.AXES)
        self.assertEqual(audit._published_axes('s2-axis-quantifiers+new-bi/1+segment-second/2'), contract.AXES)
        with self.assertRaisesRegex(ValueError, '未声明的目录规则修订'):
            audit._published_axes('unrecognized-rule')

    def test_public_verifier_known_rules_with_all_memo_modes(self):
        for generation in (0, 1):
            for rule in KNOWN_RULE_AXES:
                image = rule_image(generation, rule)
                for mode in ('none', 'cold', 'previous_known'):
                    with self.subTest(generation=generation, rule=rule, memo=mode):
                        memo = None if mode == 'none' else {}
                        if mode == 'previous_known':
                            memo = audit.verify(copy.deepcopy(image), vars(server), memo={})['_audit_memo']
                        proof = audit.verify(copy.deepcopy(image), vars(server), memo=memo)
                        state = audit.project_state(proof, None, vars(server))
                        self.assertEqual(state['snapshot']['generation'], str(generation))
                        self.assertEqual(proof['meta']['rule_revision'], rule)
                        if mode == 'previous_known':
                            self.assertEqual(proof['_audit_memo']['hits'], generation)

    def test_public_verifier_unknown_rules_with_all_memo_modes(self):
        for generation in (0, 1):
            image = rule_image(generation, 'unknown-rule')
            for mode in ('none', 'cold', 'previous_known'):
                with self.subTest(generation=generation, memo=mode):
                    memo = None if mode == 'none' else {}
                    if mode == 'previous_known':
                        known = rule_image(generation, 's2-axis-quantifiers')
                        memo = audit.verify(known, vars(server), memo={})['_audit_memo']
                    with self.assertRaisesRegex(ValueError, '未声明的目录规则修订'):
                        audit.verify(copy.deepcopy(image), vars(server), memo=memo)


if __name__ == '__main__':
    unittest.main()
