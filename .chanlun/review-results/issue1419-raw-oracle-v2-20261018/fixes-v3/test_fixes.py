"""八项缺陷的定向回归；输出是本修复树证据，不是独立评审。"""
import copy
import io
import json
from pathlib import Path
import signal
import sys
import tempfile
import unittest
from unittest.mock import MagicMock,patch

ROOT=Path(__file__).resolve().parents[1]
sys.path[:0]=[str(ROOT/'driver'),str(ROOT/'fixes-v3')]
from content_closure import Encoder,Edge,Graph,WorkBudget,close_projection
from driver_v2 import compare_domain,records,plain,TESTED_KINDS,compare_first_visibility
from fixed_cut_driver import Collector,Budget,canonical,sha
from raw_oracle import derive,derive_segments,raw_to_strokes
from independent_fixture import build
from test_v2 import fixture,reply,LIMITS,LEDGER,V2Tests
import toy_supervisor as supervisor

OBS=[]


def note(finding,case,**values):
    OBS.append(dict(finding=finding,case=case,**values))


def closed(objects):
    s,p,_=fixture(objects)
    return close_projection(s,[p],**LIMITS)


def data_of(objects,kind,index=0):
    return [r['payload_json']['data'] for r in objects if r['kind']==kind][index]


def leaves(value,path=()):
    if isinstance(value,dict):
        for k,v in value.items():yield from leaves(v,path+(k,))
    elif isinstance(value,list) and value:
        for i,v in enumerate(value):yield from leaves(v,path+(i,))
    else:yield path,value


def mutate(value):
    if type(value) is bool:return not value
    if value is None:return 'forged'
    if isinstance(value,list):return ['forged']
    return 'forged' if not value.isdigit() else str(int(value)+1)


class Fixes(unittest.TestCase):
    def test_f1_independent_static_fixture(self):
        result=compare_domain(closed(build()),LEDGER)
        self.assertEqual((result['segments'],result['centers'],result['pairs_compared']),(10,2,1))
        self.assertNotIn('MATCH',result['status'])
        for k in ('first_visibility','known_at','AsKnown','entity_revision','raw_history','withdrawn_objects'):
            self.assertEqual(result['unverified'][k],'UNVERIFIED')
        note('F1','独立静态wire夹具正控',result=result)

    def test_f1_rehashed_field_mutations(self):
        original=build();cases=[]
        for kind,keys in [
            ('CC-013.segment',['geometric_end','construction','strict','fact_vector','trigger_stroke','stroke_refs']),
            ('CC-016.center_seed_attempt',['vector','ZD','ZG','units','center_id','core_order']),
            ('CC-008.endpoint',['kind','price','group_anchor','merged_index','raw_first','raw_last','extreme_roots']),
            ('CC-010.bi',['start','end','formation','formed_evidence']),
            ('CC-016.center',['seed_attempt_id']),
            ('CC-020.center_pair',['connector_units'])]:
            data=data_of(original,kind)
            for key in keys:
                for path,value in leaves(data[key],(key,)):cases.append((kind,path,mutate(value)))
        for kind,path,value in cases:
            with self.subTest(kind=kind,path=path):
                objects=copy.deepcopy(original);data=data_of(objects,kind)
                for k in path[:-1]:data=data[k]
                data[path[-1]]=value
                verified=closed(objects)
                self.assertTrue(verified['closure_verified'])
                with self.assertRaises((ValueError,KeyError)) as e:compare_domain(verified,LEDGER)
                note('F1',kind+'.'+'.'.join(map(str,path)),closure_verified=True,rejection=str(e.exception),exception=type(e.exception).__name__)
        self.assertGreater(len(cases),100)

    def test_f1_lifecycle_is_not_silently_verified(self):
        objects=build();data_of(objects,'CC-013.segment')['terminated_known_at']={'forged':'0'}
        result=compare_domain(closed(objects),LEDGER)
        self.assertEqual(result['unverified']['known_at'],'UNVERIFIED')
        note('F1','篡改未接入known_at仍不得声称获知时点通过',result=result['unverified'])

    def test_f1_first_visibility_binding_interface(self):
        # 两个前缀的接口试验；不替代139前缀产品输出。没有预填产品generation或known_at。
        bars=[{'example':'first'},{'example':'second'}]
        f=dict(anchor=1,first_known_input_count=2)
        ledger=dict(input_count=2,low_level={'fractals':[f],'strokes':[]},segments=[],centers=[])
        traces=[];bindings=[]
        for count in (1,2):
            objects=[] if count==1 else [dict(kind='CC-008.endpoint',payload_json={'data':{'group_anchor':'1'}})]
            s,p,_=fixture(objects)
            # fixture 默认固定139；重新编码固定切面，保持所有hash合法。
            from content_closure import frame,root_bytes
            fixed={**s['fixed_cut'],'cut_generation':str(40+count),'input_frontier':str(count-1)}
            enc=Encoder();logical={k:s[k] for k in ('counts','scope','omissions','order_version')}
            logical.update(fixed_cut={**fixed,'catalog_evidence':Edge(fixed['catalog_evidence'])},catalog=Edge(s['catalog']),rows=[{**r,'record':Edge(r['record'])} for r in s['rows']])
            ref=enc.encode(logical);s['fixed_cut']=fixed;s['cut_projection_digest']='qcut3-'+frame(0x22,root_bytes(ref['root']))
            p['anchor']={'kind':'projection','fixed_cut':fixed,'cut_projection_digest':s['cut_projection_digest'],'projection_ref':ref}
            p['target_ref']=ref;p['nodes']+=list(enc.nodes.values())
            g=Graph(p['nodes'],**LIMITS);p['items']=g.entries(ref)
            traces.append(close_projection(s,[p],**LIMITS))
            bindings.append({k:fixed[k] for k in ('session_id','session_generation','cut_generation','input_frontier')}|{'input_prefix_sha256':sha(canonical(bars[:count]))})
        result=compare_first_visibility(traces,ledger,bars,prefix_bindings=bindings)
        self.assertEqual(result['known_at'],'UNVERIFIED')
        for field,value in [('cut_generation','139'),('input_frontier','0'),('input_prefix_sha256','0'*64)]:
            bad=copy.deepcopy(bindings);bad[1][field]=value
            with self.assertRaisesRegex(ValueError,'绑定|前沿'):compare_first_visibility(traces,ledger,bars,prefix_bindings=bad)
        wrong=copy.deepcopy(ledger);wrong['low_level']['fractals'][0]['first_known_input_count']=1
        with self.assertRaisesRegex(ValueError,'首次可见定义前沿'):compare_first_visibility(traces,wrong,bars,prefix_bindings=bindings)
        note('F1','首次可见接口切面/raw/定义前沿绑定',result=result,product_prefixes='UNVERIFIED')

    def test_f2_shared_dag_budget_boundary(self):
        enc=Encoder();root=enc.encode('leaf')
        for _ in range(16):root=enc.encode([Edge(root),Edge(root)])
        graph=Graph(list(enc.nodes.values()),max_nodes=1000,max_bytes=200000,max_work=1000)
        decoded=graph.decode(root);before=graph.work
        with self.assertRaisesRegex(ValueError,'used=1001, limit=1000') as e:plain(decoded,graph)
        self.assertEqual(graph.work,1001)
        note('F2','16层重复子图',nodes=len(enc.nodes),bytes=graph.bytes,decode_work=before,rejection=str(e.exception),work=graph.work)
        # 截止检查也沿用同一预算实例，不因 decode->plain 重置。
        check=MagicMock();budget=WorkBudget(1000,check)
        graph=Graph(list(enc.nodes.values()),max_nodes=1000,max_bytes=200000,max_work=1000,work_budget=budget)
        decoded=graph.decode(root);check.side_effect=ValueError('总截止已到')
        with self.assertRaisesRegex(ValueError,'总截止'):plain(decoded,graph)

    def test_f2_unselected_families_are_not_plain_expanded(self):
        # 闭包仍核所有六族，领域层只展开受测对象。先测所有目标payload转换受累计帽约束。
        enc=Encoder();root=enc.encode('leaf')
        for _ in range(16):root=enc.encode([Edge(root),Edge(root)])
        payload=enc.encode({'data':{'unused':Edge(root)}})
        obj=enc.encode({'kind':'CC-013.segment','payload_json':Edge(payload)})
        rows=enc.encode({'rows':[{'family':'objects','record':Edge(obj)}]})
        g=Graph(list(enc.nodes.values()),max_nodes=1000,max_bytes=200000,max_work=1000)
        with self.assertRaisesRegex(ValueError,'逻辑工作量'):records({'graph':g,'projection_ref':rows})
        note('F2','records内payload展开',work=g.work)

    def test_f3_all_tested_kinds_reject_bad_payload(self):
        for kind in sorted(TESTED_KINDS):
            for tag,payload in [('missing',None),('null',None),('scalar',7),('no_data',{}),('bad_data',{'data':[]})]:
                extra={'kind':kind}
                if tag!='missing':extra['payload_json']=payload
                verified=closed(build()+[extra])
                with self.assertRaisesRegex(ValueError,'受测kind') as e:compare_domain(verified,LEDGER)
                note('F3',kind+'/'+tag,closure_verified=True,rejection=str(e.exception))

    def test_f4_wire_null_and_dangling_references(self):
        for index in (0,5,10):
            for field in ('center_id','left_center_id','right_center_id'):
                for value in ('nonexistent-center',False,''):
                    objects=build();data_of(objects,'CC-022.unit_assignment',index)[field]=value
                    with self.assertRaisesRegex(ValueError,'wire null|悬空|引用目标') as e:compare_domain(closed(objects),LEDGER)
                    note('F4',f'assignment[{index}].{field}={value!r}',rejection=str(e.exception))
        for field in ('a_center_id','b_center_id'):
            objects=build();data_of(objects,'CC-020.center_pair')[field]='nonexistent-center'
            with self.assertRaisesRegex(ValueError,'悬空'):compare_domain(closed(objects),LEDGER)

    def test_f5_original_snapshot_contentref_metadata(self):
        s,p,_=fixture()
        for target in ('catalog','catalog_evidence','record'):
            for field,value in [('kind','array'),('extent','999'),('format','s-content/9')]:
                changed=copy.deepcopy(s)
                ref=changed['catalog'] if target=='catalog' else changed['fixed_cut']['catalog_evidence'] if target=='catalog_evidence' else changed['rows'][0]['record']
                ref[field]=value
                with self.assertRaisesRegex(ValueError,'引用种类/extent|ContentRef') as e:close_projection(changed,[p],**LIMITS)
                note('F5',target+'.'+field,nodes_changed=False,root_changed=False,rejection=str(e.exception))

    def test_f6_partial_initialization_cleanup(self):
        for failure in ('selector','blocking','register_first','register_second','open_first','open_second'):
            p=MagicMock();p.pid=12345;p.poll.return_value=None
            sel=MagicMock();handle=MagicMock()
            selector_error=OSError('injected '+failure)
            with patch.object(supervisor.Path,'mkdir'),patch.object(supervisor.subprocess,'Popen',return_value=p), \
                 patch.object(supervisor.selectors,'DefaultSelector',side_effect=selector_error if failure=='selector' else None,return_value=sel), \
                 patch.object(supervisor.os,'set_blocking',side_effect=selector_error if failure=='blocking' else None), \
                 patch.object(supervisor.os,'getpgid',return_value=p.pid),patch.object(supervisor.os,'killpg') as kill, \
                 patch.object(supervisor.Path,'open',return_value=handle) as opened:
                if failure.startswith('register'):sel.register.side_effect=[selector_error] if failure.endswith('first') else [None,selector_error]
                if failure.startswith('open'):opened.side_effect=[selector_error] if failure.endswith('first') else [handle,selector_error]
                with self.assertRaisesRegex(OSError,'injected'):supervisor.supervise('wait','unused',seconds=.1,log_bytes=32,term_grace=.01,kill_grace=.1)
                kill.assert_called_once_with(p.pid,signal.SIGKILL);p.wait.assert_called_once()
                p.stdout.close.assert_called_once();p.stderr.close.assert_called_once()
                if failure!='selector':sel.close.assert_called_once()
                if failure in ('open_second','register_second'):handle.close.assert_called_once()
                note('F6',failure,killpg_calls=kill.call_count,wait_calls=p.wait.call_count,stdout_closed=p.stdout.close.call_count,stderr_closed=p.stderr.close.call_count,actual_process_started=False)

    def test_f7_review_19bar_inclusion_rejected(self):
        verts=[804,1596,1204,1516,1284];mids=[820,verts[0]]
        for a,b in zip(verts,verts[1:]):mids.extend(a+(b-a)*j//4 for j in range(1,5))
        mids.append(1300)
        bars=[dict(ordinal=i,open=m,close=m,low=m-4,high=m+4,volume=1) for i,m in enumerate(mids)]
        self.assertEqual(len(bars),19)
        low=raw_to_strokes(bars)
        self.assertEqual([(b['low'],b['high']) for b in low['strokes'][1::2]],[(1200,1600),(1280,1520)])
        with self.assertRaisesRegex(ValueError,'域外：特征序列有包含') as e:derive(bars)
        note('F7','独审19bar原反例',raw_count=19,rejection=str(e.exception))
        # 三元素、无strict候选也必须检查，不仅“两元素”分支。
        strokes=copy.deepcopy(low['strokes'])
        strokes += [dict(index=4,direction='UP',low=1280,high=1500),dict(index=5,direction='DOWN',low=1300,high=1400)]
        with self.assertRaisesRegex(ValueError,'特征序列有包含'):derive_segments(strokes)
        # 非包含两元素可继续pending，未扩大域。
        good=copy.deepcopy(low['strokes']);good[3]['low']=1160;good[3]['high']=1520
        self.assertIsNotNone(derive_segments(good)[1])

    def run_watch(self,frontiers,counts,epochs=None):
        s,_,_=fixture();fixed=s['fixed_cut']
        cursor={k:fixed[k] for k in ('session_id','session_generation','catalog_revision','scope','index_frontier')}
        cursor.update(after_generation='139',base_cut='toy-cut',last_seq='138',order_version='s-record-order/2')
        deltas=[];accepted={};next_cursor=copy.deepcopy(cursor)
        for i,(f,n) in enumerate(zip(frontiers,counts),140):
            delta=dict(generation=str(i),session_id='toy',catalog_revision='toy-catalog',base_cut=next_cursor['base_cut'],next_cut='cut-'+str(i),
                       seq_range={'from':'0','to':'7'},index_frontier=str(i),input_frontier=f,catalog_run_status='complete',catalog_evidence={},delta={})
            next_cursor.update(after_generation=str(i),base_cut=delta['next_cut'],index_frontier=str(i),last_seq=f);deltas.append(delta)
            accepted[str(i)]=dict(session_id='toy',session_generation='1',message_id='named-'+str(i),source_epoch=(epochs or ['1']*len(frontiers))[i-140],accepted_events=n)
        def exchange(route,raw):
            payload=dict(deltas=deltas,next_cursor=next_cursor,observed_head=str(139+len(deltas)),delivery_frontier=str(139+len(deltas)),has_more=False,gap=None,validated_capture_digest='b'*64,ok=True)
            return 200,{},reply(json.loads(raw),payload)
        identity=dict(session_id='toy',session_generation='1',source_namespace='toy.query',source_epoch='1',producer_id='toy-driver',producer_epoch='1')
        with tempfile.TemporaryDirectory() as tmp:
            c=Collector(identity,Budget(1000000,10),exchange,Path(tmp)/'trace','1')
            return c.watch(cursor,139+len(deltas),'client',max_batches=len(deltas),accepted_commits=accepted)

    def test_f8_frontier_validation_and_cross_epoch(self):
        cases=[('100','1','回退'),('-1','1','非规范'),('0139','1','非规范'),('139.0','1','非规范'),(139,'1','非规范'),(True,'1','非规范'),(None,'1','非规范'),('140','1','事件数'),('139','2','事件数')]
        for frontier,count,reason in cases:
            with self.assertRaisesRegex(ValueError,reason) as e:self.run_watch([frontier],[count])
            note('F8',repr(frontier)+'/'+count,rejection=str(e.exception))
        result=self.run_watch(['139','204','269'],['1','65','65'],['1','2','2'])
        self.assertEqual(result['next_cursor']['last_seq'],'269')
        note('F8','具名接纳数+跨epoch正控',frontiers=['138','139','204','269'],affected_seq_to='7',accepted_counts=[1,65,65],result_frontier='269')

    def test_changed_api_regressions(self):
        for name in ('test_domain_adapter_positive_and_zn_mutation_negative','test_domain_withdrawal_rejects_stale_pair','test_driver_collects_fixed_pages_and_full_content'):
            V2Tests(name).debug()
            note('回归',name,result='PASS',fixture_qualification='旧自洽模型夹具；非独立证据')

    def test_changed_supervisor_normal_paths(self):
        import test_v2
        # 保存本轮toy输出，不覆盖旧树报告、PID或时间身份。
        target=ROOT/'fixes-v3'/'toy-regression'
        with patch.object(test_v2,'ROOT',target):V2Tests('test_toy_supervisor_paths_reap').debug()
        note('F6','固定Python六条原正常/取消路径',result='PASS',output='toy-regression/validation',product_run='NOT_RUN')


if __name__=='__main__':
    suite=unittest.defaultTestLoader.loadTestsFromTestCase(Fixes)
    result=unittest.TextTestRunner(verbosity=2).run(suite)
    (ROOT/'fixes-v3'/'CHECK-OUTPUT.json').write_text(json.dumps({'qualification':'修复工蜂定向回归，非独立评审','observations':OBS,'tests_run':result.testsRun,'failures':len(result.failures),'errors':len(result.errors),'product_run':'NOT_RUN'},ensure_ascii=False,indent=2)+'\n')
    sys.exit(not result.wasSuccessful())
