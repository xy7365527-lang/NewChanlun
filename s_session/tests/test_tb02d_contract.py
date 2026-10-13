"""#1414 合同反例锁；人工构造仅检合同，不作段语义或正式验收。"""
import copy
import json
import subprocess
import unittest
from pathlib import Path
from test_tb02c_contract import c, seed_payload

ROOT = Path(__file__).resolve().parents[1]

def envelope(data, slot):
    return dict(schema_revision="s-segment/2", semantic_version="segment-second/1",
                policy_version="seed_closed_strict_second", view_role="main",
                object_generation="1", slot=slot, data=data)

def samples():
    seed = seed_payload()["data"]
    refs = seed["stroke_refs"]
    elements = [dict(high="400", low="200", stroke_idx=str(i), stroke_indices=[str(i)]) for i in range(3)]
    check = dict(elements=elements, top_fractal=True, bottom_fractal=False, through_stroke="2")
    second = envelope(dict(entity_id="second:segment:1:1", segment_id="segment:1:1", role="SECOND_SEQUENCE",
        direction="UP", member_direction="UP", stroke_refs=refs,
        scan=dict(from_stroke="0", through_stroke="2", input_stroke_count="3", resource_truncated=False, tail_cache_window="0"),
        facts=dict(raw_elements=elements, standard=elements, steps=[], checks=[check], from_stroke="1", through_stroke="2",
                   input_stroke_count="3", resource_truncated=False, matched=check), second_sequence_status="CONFIRMED"), "second:segment:1:1")
    known = dict(generation="1", input_frontier="13", receipt_id="r", received_at="t", semantic_commit_ns=None)
    segment = envelope(dict(entity_id="segment:1:1", entity_revision="1", seed_id="seed:1:1", sequence_id="first:segment:1:1",
        sequence_object_id="first-object", second_sequence_id="second:segment:1:1", second_sequence_object_id="second-object",
        construction=seed["construction"], result="CASE_TWO", waiting_reasons=[], termination_attempts=[],
        geometric_end=dict(stroke_index="2", group_anchor="13", extreme_roots=["13"], price="528"), trigger_stroke="2",
        formed_known_at=known, terminated_known_at=known, stroke_refs=refs,
        strict=dict(elements=elements, raw_gap=True, gap_closed_by_c=False, effective_case="CASE_TWO", closure_operator="<=", closure_left="200", closure_right="400"),
        fact_vector=dict(stroke_broken=True, new_segment_formed=True, segment_broken=True,
            stroke_break_witness=dict(earlier_stroke="0", later_stroke="2", earlier_price="384", later_price="528", comparison=">="),
            scan_from_stroke="0", scan_through_stroke="2", scan_complete=True)), "segment:1:1")
    segment["data"]["fact_vector"]["termination_witness"] = dict(kind="CASE_TWO", strict=copy.deepcopy(segment["data"]["strict"]), second_match=copy.deepcopy(check), construction=copy.deepcopy(segment["data"]["construction"]), minimum_count_met=True, through_stroke="2")
    return second, segment

class SecondContractTest(unittest.TestCase):
    def both(self, kind, p, valid=True):
        if valid:
            c.validate_payload(kind, p)
        else:
            with self.assertRaises(ValueError): c.validate_payload(kind, p)
        program = "const a=require(process.argv[1]);let s='';process.stdin.on('data',x=>s+=x);process.stdin.on('end',()=>{const p=JSON.parse(s);try{a.validateSegmentPayload(process.argv[2],p)}catch(e){process.exit(3)}})"
        result = subprocess.run(["node", "-e", program, str(ROOT / "browser/tb01c-client.js"), kind], input=json.dumps(p), text=True, capture_output=True)
        self.assertEqual(result.returncode, 0 if valid else 3, result.stderr)

    def test_second_and_case_two_accept(self):
        second, segment = samples()
        self.both("CC-012.feature_sequence", second)
        self.both("CC-013.segment", segment)

    def test_mixed_versions_rejected(self):
        second, _ = samples()
        second["semantic_version"] = "segment-seed-first/1"
        self.both("CC-012.feature_sequence", second, False)

    def test_second_role_scope_and_evidence_rejected(self):
        for mutate in [lambda d: d.update(member_direction="DOWN"),
                       lambda d: d.update(entity_id="first:segment:1:1"),
                       lambda d: d["facts"].update(matched=None),
                       lambda d: d["facts"]["checks"][0].update(top_fractal="true"),
                       lambda d: d["facts"]["standard"][0].update(stroke_indices=["99"]),
                       lambda d: d["scan"].update(tail_cache_window="7")]:
            second, _ = samples(); mutate(second["data"])
            self.both("CC-012.feature_sequence", second, False)

    def test_strict_and_break_applicability_rejected(self):
        for mutate in [lambda d: d.update(strict=None),
                       lambda d: d["strict"].update(gap_closed_by_c=None),
                       lambda d: d["strict"].update(closure_left="999"),
                       lambda d: d.update(second_sequence_object_id=None),
                       lambda d: d["fact_vector"].update(stroke_break_witness=None),
                       lambda d: d["fact_vector"].update(segment_broken=False),
                       lambda d: d["fact_vector"].update(termination_witness=None),
                       lambda d: d["fact_vector"]["termination_witness"].update(second_match=None)]:
            _, segment = samples(); mutate(segment["data"])
            self.both("CC-013.segment", segment, False)

    def test_pending_can_have_stroke_break_without_segment_break(self):
        _, p = samples(); d=p["data"]
        d.update(result="PENDING_CASE_TWO", geometric_end=None, trigger_stroke=None, terminated_known_at=None)
        d["fact_vector"].update(new_segment_formed=False, segment_broken=False, termination_witness=None)
        self.both("CC-013.segment", p)

    def test_resource_bound_is_unknown_not_false(self):
        _, p = samples(); d=p["data"]
        d.update(result="PENDING_CASE_TWO", geometric_end=None, trigger_stroke=None, terminated_known_at=None)
        d["fact_vector"].update(new_segment_formed=None, segment_broken=None, scan_complete=False, termination_witness=None)
        self.both("CC-013.segment", p)
        d["fact_vector"].update(new_segment_formed=False, segment_broken=False)
        self.both("CC-013.segment", p, False)

    def test_old_schema_does_not_accept_new_fields(self):
        _, p=samples();p.update(schema_revision="s-segment/1",semantic_version="segment-seed-first/1",policy_version="seed_closed_first_no_gap")
        self.both("CC-013.segment",p,False)

    def test_version_scope_and_old_axes(self):
        version = envelope(dict(input_frontier="-1", waiting_reasons=[],
            known_at=dict(generation="1",input_frontier="-1",receipt_id=None,received_at=None,semantic_commit_ns=None),
            delivered_scope="seed_strict_first_and_finite_second",
            deferred_scope=["general_equal_identity","general_scan_continuation","global_G006_bridge"]),"TB-02-D")
        self.both("CC-056.segment_version", version)
        version["slot"]="TB-02-C"
        self.both("CC-056.segment_version", version, False)
        self.assertNotIn("CC-014",c.FIRST_SEGMENT_AXES)
        self.assertIn("CC-014",c.AXES)

    def test_content_links_and_cc014_binding(self):
        second, segment=samples()
        first=copy.deepcopy(second); d=first["data"]
        d.update(entity_id="first:segment:1:1",role="FIRST_SEQUENCE",member_direction="DOWN")
        d["facts"]=dict(raw_elements=[],standard=[],steps=[],checks=[dict(elements=segment["data"]["strict"]["elements"])])
        records=[dict(object_id=oid,kind=kind,payload=p,source_coords=["1","5","9","13"]) for oid,kind,p in
                 [("first-object","CC-012.feature_sequence",first),("second-object","CC-012.feature_sequence",second),("segment-object","CC-013.segment",segment)]]
        axes={cid:dict(evidence=dict(object_ids=["segment-object"])) for cid in ["CC-013","CC-014"]}
        c.validate_segment_links(records, axes)
        with self.assertRaises(ValueError): c.validate_segment_links(records[1:],axes)
        axes["CC-014"]["evidence"]["object_ids"]=["second-object"]
        with self.assertRaises(ValueError): c.validate_segment_links(records,axes)

if __name__ == "__main__": unittest.main()
