"""#1392 新笔公共合同的结构、版本和分域拒绝测试；不是结构 oracle。"""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('tb02_contract_test', Path(__file__).resolve().parents[1] / 's_tb02_contract.py')
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)


class ContractTest(unittest.TestCase):
    def setUp(self):
        endpoint = dict(kind='BOTTOM', merged_index='1', group_anchor='1', price='2', extreme_roots=['1'],
                        raw_position='1', source_coords=['0', '1', '2'], sealed_at=None, waiting_reasons=[])
        other = dict(endpoint, kind='TOP', merged_index='4', group_anchor='5', price='13', extreme_roots=['5'],
                     raw_position='5', source_coords=['4', '5', '6'])
        conditions = dict(merged_gap='3', raw_between_actual_extrema='3', top_price='13', bottom_price='2',
                          vector=[True, True, True], failed_conditions=[], waiting_reasons=[])
        self.p = dict(schema_revision='s-new-bi/1', semantic_version='new-bi-dual-coordinate/1', policy_version='standard_raw_gap_3',
                      view_role='main', object_generation='1', slot=['1', '1', '5'],
                      data=dict(old=endpoint, new=other, endpoint_kind_pair='BOTTOM/TOP', conditions=conditions,
                                comparison=None, selection=None, retained_anchors=[], waiting_reasons=[]))

    def test_complete_vector_is_preserved(self):
        c.validate_bi_payload('CC-008.new_bi_pair', self.p)
        self.assertEqual(self.p['data']['conditions']['vector'], [True, True, True])

    def test_missing_false_and_unknown_are_distinct(self):
        for vector in ([True, True], [True, 'false', True], [1, False, True]):
            bad = copy.deepcopy(self.p)
            bad['data']['conditions']['vector'] = vector
            with self.assertRaises(ValueError):
                c.validate_bi_payload('CC-008.new_bi_pair', bad)

    def test_same_kind_cannot_borrow_opposite_conditions(self):
        bad = copy.deepcopy(self.p)
        bad['data']['new']['kind'] = 'BOTTOM'
        bad['data']['endpoint_kind_pair'] = 'BOTTOM/BOTTOM'
        with self.assertRaises(ValueError):
            c.validate_bi_payload('CC-008.new_bi_pair', bad)
        with self.assertRaises(ValueError):
            c.validate_bi_payload('CC-009.same_kind', bad)

    def test_unknown_versions_and_missing_roots_reject(self):
        for mutate in (lambda p: p.update(semantic_version='unknown'),
                       lambda p: p['data']['new'].update(source_coords=['4', '6'])):
            bad = copy.deepcopy(self.p)
            mutate(bad)
            with self.assertRaises(ValueError):
                c.validate_bi_payload('CC-008.new_bi_pair', bad)


if __name__ == '__main__':
    unittest.main()
