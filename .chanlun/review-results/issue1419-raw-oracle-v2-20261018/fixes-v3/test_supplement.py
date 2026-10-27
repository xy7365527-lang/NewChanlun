"""首轮后收紧的字段/接口定向复验；复用首轮未改路径的输出。"""
import copy
import json
from pathlib import Path
import sys
import unittest

ROOT=Path(__file__).resolve().parents[1]
sys.path[:0]=[str(ROOT/'driver'),str(ROOT/'fixes-v3')]
import test_fixes as base
from driver_v2 import compare_domain,records
from fixed_cut_driver import bind_named_submissions
from content_closure import Encoder,Graph,Edge
from raw_oracle import raw_to_strokes,derive_segments,derive


class Supplement(unittest.TestCase):
    def test_f1_strict_role_encoding_and_null_exclusion(self):
        for value in ('0800',800,True):
            objs=base.build();base.data_of(objs,'CC-017.center_roles')['roles']['OUTER']['DD']['value']=value
            with self.assertRaisesRegex(ValueError,'角色值'):compare_domain(base.closed(objs),base.LEDGER)
        objs=base.build();base.data_of(objs,'CC-016.center_seed_attempt')['exclusion_reason']='invented'
        with self.assertRaisesRegex(ValueError,'exclusion_reason'):compare_domain(base.closed(objs),base.LEDGER)
        result=compare_domain(base.closed(base.build()),base.LEDGER)
        base.note('F1','补验规范角色值/排除原因null',result=result['status'])

    def test_f2_other_families_keep_shared_dag(self):
        enc=Encoder();root=enc.encode('leaf')
        for _ in range(16):root=enc.encode([Edge(root),Edge(root)])
        record=enc.encode({'large':Edge(root)})
        ref=enc.encode({'rows':[{'family':family,'record':Edge(record)} for family in ('raw_history','withdrawn_objects','observations')]})
        g=Graph(list(enc.nodes.values()),max_nodes=1000,max_bytes=200000,max_work=1000)
        self.assertEqual(records({'graph':g,'projection_ref':ref}),[])
        self.assertLess(g.work,1000)
        base.note('F2','未受测三族保留共享结构不plain展开',work=g.work,nodes=len(g.pool))

    def test_f4_missing_null_and_center_members(self):
        for field in ('center_id','left_center_id','right_center_id'):
            objs=base.build();del base.data_of(objs,'CC-022.unit_assignment',10)[field]
            with self.assertRaisesRegex(ValueError,'引用缺字段'):compare_domain(base.closed(objs),base.LEDGER)
        for field in ('members','zn_members'):
            objs=base.build();base.data_of(objs,'CC-016.center')[field][0]='absent'
            with self.assertRaisesRegex(ValueError,'悬空'):compare_domain(base.closed(objs),base.LEDGER)
        base.note('F4','缺null字段/中心成员悬空',result='REJECT')

    def test_f7_third_pending_element_and_fixed_domains(self):
        strokes=[dict(index=i,direction='UP' if i%2==0 else 'DOWN',low=lo,high=hi)
                 for i,(lo,hi) in enumerate([(800,1600),(1200,1600),(1200,1520),(1160,1520),(1160,1480),(1180,1480)])]
        with self.assertRaisesRegex(ValueError,'特征序列有包含'):derive_segments(strokes)
        # 只核新增有限域门在原四份输入不误拒；不重做139前缀独立几何验证。
        for name in ('base','revise-non-zn-tail','withdraw-second-center','restore-original'):
            p=ROOT/'RAW-INPUT.json' if name=='base' else ROOT/'raw'/f'{name}.json'
            bars=json.loads(p.read_text())['bars'];result=derive(bars)
            base.note('F7','新增域门原输入:'+name,result=result['qualification'])

    def test_f8_bind_frozen_named_messages(self):
        requests=[json.loads(s) for s in (ROOT/'raw/messages.recovery-epoch2.jsonl').read_text().splitlines()][-3:]
        self.assertEqual([len(x['payload']['raw_input']['events']) for x in requests],[1,65,65])
        # 这里的cut代际只是接口测试值，不写到产品合同或原件中。
        cuts=[dict(session_id=x['session_id'],session_generation=x['session_generation'],cut_generation=str(800+i),input_frontier=str(f)) for i,(x,f) in enumerate(zip(requests,[139,204,269]))]
        bindings=bind_named_submissions(requests,cuts)
        self.assertEqual([b['accepted_events'] for b in bindings.values()],['1','65','65'])
        bad=copy.deepcopy(requests);bad[0]['payload']['raw_input']['events'].append({})
        with self.assertRaisesRegex(ValueError,'摘要'):bind_named_submissions(bad,cuts)
        with self.assertRaisesRegex(ValueError,'重复'):bind_named_submissions(requests,[cuts[0],cuts[0],cuts[2]])
        base.note('F8','原始具名请求事件数到实际cut绑定接口',counts=[1,65,65],product_binding='UNVERIFIED')

    def test_changed_api_regressions(self):
        base.Fixes('test_changed_api_regressions').debug()
        base.Fixes('test_f1_first_visibility_binding_interface').debug()
        base.Fixes('test_f8_frontier_validation_and_cross_epoch').debug()


if __name__=='__main__':
    result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Supplement))
    (ROOT/'fixes-v3/SUPPLEMENT-OUTPUT.json').write_text(json.dumps({'qualification':'尾差定向验证，非独立评审','observations':base.OBS,'tests_run':result.testsRun,'failures':len(result.failures),'errors':len(result.errors)},ensure_ascii=False,indent=2)+'\n')
    sys.exit(not result.wasSuccessful())
