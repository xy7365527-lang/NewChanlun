"""#1404 公共类型边界；结构真值由独立 raw 账簿和 S 测试负责。"""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("tb02c_contract", ROOT / "s_tb02_contract.py")
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)


def seed_payload():
    return dict(schema_revision="s-segment/1", semantic_version="segment-seed-first/1",
                policy_version="seed_closed_first_no_gap", view_role="main", object_generation="1",
                slot="seed:1:1", data=dict(entity_id="seed:1:1", stroke_refs=[
                    dict(stroke_index=str(i), entity_id=f"bi:1:{a}", start_anchor=str(a), end_anchor=str(b),
                         start_price=str(x), end_price=str(y), start_roots=[str(a)], end_roots=[str(b)])
                    for i, a, b, x, y in [(0, 1, 5, 120, 384), (1, 5, 9, 384, 192), (2, 9, 13, 192, 528)]],
                    construction=dict(start_stroke="0", stroke_indices=["0", "1", "2"], length="odd_ge3",
                                      vector=[True] * 4, overlap=["192", "384"], overlap_relation="OVERLAP",
                                      direction="UP", failed_conditions=[])))


class SegmentContractTest(unittest.TestCase):
    def test_two_strokes_are_candidate_not_seed(self):
        p = seed_payload()
        p['data']['stroke_refs'].pop()
        p['data']['construction'].update(stroke_indices=['0', '1'], length='n=2', vector=None,
                                          overlap=None, overlap_relation=None, direction=None,
                                          failed_conditions=['fewer_than_three_strokes'])
        c.validate_payload('CC-011.seed_candidate', p)
        with self.assertRaises(ValueError):
            c.validate_payload('CC-011.segment_seed', p)

    def test_seed_four_conditions_coexist(self):
        c.validate_payload("CC-011.segment_seed", seed_payload())

    def test_reject_missing_condition_and_false_direction_claim(self):
        for vector, direction in [([True] * 3, "UP"), ([True, False, True, True], "UP")]:
            p = seed_payload()
            p["data"]["construction"].update(vector=vector, direction=direction)
            with self.assertRaises(ValueError):
                c.validate_payload("CC-011.segment_seed", p)

    def test_reject_unbound_stroke_and_extra_field(self):
        for mutate in [lambda p: p["data"]["stroke_refs"].pop(),
                       lambda p: p["data"]["construction"].update(fake=True)]:
            p = copy.deepcopy(seed_payload())
            mutate(p)
            with self.assertRaises(ValueError):
                c.validate_payload("CC-011.segment_seed", p)


if __name__ == "__main__":
    unittest.main()
