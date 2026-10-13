"""#1392/#1405 新笔公共合同的结构、版本和分域拒绝测试；不是结构 oracle。"""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('tb02_contract_test', Path(__file__).resolve().parents[1] / 's_tb02_contract.py')
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)

V2 = dict(schema_revision='s-new-bi/2', semantic_version='new-bi-dual-coordinate/2',
          policy_version='standard_raw_gap_3;same-price/keep-earlier+extreme-region/1')
V1 = dict(schema_revision='s-new-bi/1', semantic_version='new-bi-dual-coordinate/1', policy_version='standard_raw_gap_3')


def endpoint(kind, anchor, price, roots, first, last, sources):
    return dict(kind=kind, merged_index=anchor, group_anchor=anchor, price=price, extreme_roots=roots,
                raw_first=first, raw_last=last, source_coords=sources, sealed_at=None, waiting_reasons=[])


def as_v1(e, position):
    e = {k: v for k, v in e.items() if k not in ('raw_first', 'raw_last')}
    e['raw_position'] = position
    return e


class ContractTest(unittest.TestCase):
    def setUp(self):
        bottom = endpoint('BOTTOM', '1', '2', ['1'], '1', '1', ['0', '1', '2'])
        top = endpoint('TOP', '5', '13', ['5', '6'], '5', '6', ['4', '5', '6', '7'])
        conditions = dict(merged_gap='3', raw_between_actual_extrema='3', top_price='13', bottom_price='2',
                          vector=[True, True, True], failed_conditions=[], waiting_reasons=[])
        self.p = dict(**V2, view_role='main', object_generation='1', slot=['1', '1', '5'],
                      data=dict(old=bottom, new=top, endpoint_kind_pair='BOTTOM/TOP', conditions=conditions,
                                comparison=None, selection=None, retained_anchors=[], evidence=[], waiting_reasons=[]))
        later = endpoint('TOP', '9', '13', ['9'], '9', '9', ['8', '9', '10'])
        self.same = dict(**V2, view_role='main', object_generation='1', slot=['1', '5', '9'],
                         data=dict(old=top, new=later, endpoint_kind_pair='TOP/TOP', conditions=None,
                                   comparison='EQ', selection='KEEP', retained_anchors=['5'],
                                   evidence=['same_kind_equal_price_keep_earlier'], waiting_reasons=[]))

    def reject(self, kind, payload):
        with self.assertRaises(ValueError):
            c.validate_bi_payload(kind, payload)

    def test_complete_vector_is_preserved(self):
        c.validate_bi_payload('CC-008.new_bi_pair', self.p)
        self.assertEqual(self.p['data']['conditions']['vector'], [True, True, True])

    def test_missing_false_and_unknown_are_distinct(self):
        for vector in ([True, True], [True, 'false', True], [1, False, True], [True, None, True]):
            bad = copy.deepcopy(self.p)
            bad['data']['conditions']['vector'] = vector
            self.reject('CC-008.new_bi_pair', bad)

    def test_same_kind_cannot_borrow_opposite_conditions(self):
        bad = copy.deepcopy(self.p)
        bad['data']['new']['kind'] = 'BOTTOM'
        bad['data']['endpoint_kind_pair'] = 'BOTTOM/BOTTOM'
        self.reject('CC-008.new_bi_pair', bad)
        self.reject('CC-009.same_kind', bad)

    def test_unknown_versions_and_missing_roots_reject(self):
        for mutate in (lambda p: p.update(semantic_version='unknown'),
                       lambda p: p.update(policy_version='standard_raw_gap_3'),
                       lambda p: p.update(schema_revision='s-new-bi/1'),
                       lambda p: p['data']['new'].update(source_coords=['4', '6'])):
            bad = copy.deepcopy(self.p)
            mutate(bad)
            self.reject('CC-008.new_bi_pair', bad)

    def test_v2_extreme_region_closes_over_roots(self):
        c.validate_bi_payload('CC-008.endpoint', {**self.p, 'slot': 'fx:1:5', 'data': self.p['data']['new']})
        for mutate in (lambda e: e.update(raw_last='4'),
                       lambda e: e.update(raw_first='6'),
                       lambda e: e.update(raw_first=None),
                       lambda e: e.update(raw_position='5'),
                       lambda e: e.pop('raw_last')):
            bad = copy.deepcopy(self.p)
            mutate(bad['data']['new'])
            self.reject('CC-008.new_bi_pair', bad)

    def test_v2_raw_gap_is_always_determined(self):
        bad = copy.deepcopy(self.p)
        bad['data']['conditions']['raw_between_actual_extrema'] = None
        self.reject('CC-008.new_bi_pair', bad)

    def test_v2_equal_price_keeps_earlier_with_evidence(self):
        c.validate_bi_payload('CC-009.same_kind', self.same)
        for mutate in (lambda d: d.update(selection='UNDETERMINED', retained_anchors=['5', '9']),
                       lambda d: d.update(selection='REPLACE', retained_anchors=['9']),
                       lambda d: d.update(evidence=[]),
                       lambda d: d.update(retained_anchors=['9']),
                       lambda d: d.update(comparison='GT', evidence=[]),
                       lambda d: d.update(waiting_reasons=['same_kind_endpoint_identity_tie'])):
            bad = copy.deepcopy(self.same)
            mutate(bad['data'])
            self.reject('CC-009.same_kind', bad)
        stronger = copy.deepcopy(self.same)
        stronger['data'].update(comparison='GT', selection='REPLACE', retained_anchors=['9'], evidence=[])
        stronger['data']['new']['price'] = '14'
        c.validate_bi_payload('CC-009.same_kind', stronger)

    def test_v2_rejects_retired_same_price_waiting(self):
        for tag in sorted(c.RETIRED_SAME_PRICE_WAITING):
            bad = copy.deepcopy(self.p)
            bad['data']['new']['waiting_reasons'] = [tag]
            self.reject('CC-008.new_bi_pair', bad)

    def test_v1_objects_remain_valid_as_known(self):
        old = copy.deepcopy(self.same)
        old.update(V1)
        old['data'].pop('evidence')
        old['data'].update(selection='UNDETERMINED', retained_anchors=['5', '9'],
                           waiting_reasons=['same_kind_endpoint_identity_tie'])
        old['data']['old'] = as_v1(old['data']['old'], None)
        old['data']['old']['waiting_reasons'] = ['raw_extreme_identity_tie']
        old['data']['new'] = as_v1(old['data']['new'], '9')
        c.validate_bi_payload('CC-009.same_kind', old)
        mixed = copy.deepcopy(old)
        mixed['data']['evidence'] = ['same_kind_equal_price_keep_earlier']
        self.reject('CC-009.same_kind', mixed)

    def test_change_event_before_keeps_its_own_generation_shape(self):
        conditions = self.p['data']['conditions']
        known = dict(generation='1', input_frontier='0', receipt_id='r', received_at='t', semantic_commit_ns=None)
        bi = dict(identity_anchor='1', start=self.p['data']['old'], end=self.p['data']['new'], formation=conditions,
                  confirmation=None, entity_id='bi:1:1', entity_revision='1', state='FORMED_UNCONFIRMED',
                  formed_known_at=known, confirmed_known_at=None, formed_evidence=conditions, version_causes=['formed'])
        before = copy.deepcopy(bi)
        before['start'] = as_v1(before['start'], '1')
        before['end'] = as_v1(before['end'], None)
        event = dict(**V2, view_role='main', object_generation='1', slot=['2', 'bi:1:1', 'extended'],
                     data=dict(entity_id='bi:1:1', change='extended', before=before, after=bi, known_at=known,
                               event_at='0', causes=['new-bi-dual-coordinate/2'], object_id='o'))
        c.validate_bi_payload('CC-055.change_event', event)
        bad = copy.deepcopy(event)
        bad['data']['after'] = before
        self.reject('CC-055.change_event', bad)


class RuleBindingTest(unittest.TestCase):
    def test_session_rule_binds_bi_and_segment_contract_generations(self):
        self.assertEqual(c.RULE_CONTRACTS['s2-axis-quantifiers+new-bi/2+segment-second/1'],
                         ('s-new-bi/2', 's-segment/2', 'established_direction_with_extreme_root_sets'))
        self.assertEqual(c.RULE_CONTRACTS['s2-axis-quantifiers+new-bi/1+segment-second/1'],
                         ('s-new-bi/1', 's-segment/2', 'established_direction_without_extreme_identity_competition'))
        self.assertEqual(c.RULE_CONTRACTS['s2-axis-quantifiers+new-bi/1'][:2], ('s-new-bi/1', None))

    def test_knowledge_domain_accepts_both_generations_and_rejects_others(self):
        payload = dict(scope='TB-02-A', input_frontier='0', known_facts=['raw_ohlc', 'inclusion_trace', 'group_provenance'],
                       waiting_reasons=[], requests=[])
        for domain in (c.KNOWLEDGE_DOMAIN_V1, c.KNOWLEDGE_DOMAIN_V2):
            c.validate_payload('CC-054.knowledge_state', {**payload, 'domain': domain})
        with self.assertRaises(ValueError):
            c.validate_payload('CC-054.knowledge_state', {**payload, 'domain': 'established_direction'})


if __name__ == '__main__':
    unittest.main()
