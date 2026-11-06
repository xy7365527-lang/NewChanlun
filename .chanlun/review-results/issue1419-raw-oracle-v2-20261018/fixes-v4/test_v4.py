"""#1419 四项剩余缺陷：固定内存/fake 反例；不启动服务或真实进程。"""
import copy
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(ROOT/'driver'), str(ROOT/'fixes-v3')]
import content_closure as cc
import driver_v2 as dv
from fixed_cut_driver import Collector, Budget, bind_named_submissions
from independent_fixture import build as old_build
from test_v2 import fixture, LIMITS, LEDGER

OBS = []


def public_objects():
    objects = old_build()
    for r in objects:
        if 'payload_json' in r:
            r['payload'] = r.pop('payload_json')
        r.update(object_revision='1', batch_id='fixture-batch', fact_key=r['object_id'], input_refs=[],
                 source_coords=[], first_known_generation='139', first_known_cut='toy-cut',
                 published_generation='139', withdrawn_generation=None, withdrawal_reason=None,
                 superseded_by=None, lifecycle='active')
    return objects


def closed(objects):
    s, p, _ = fixture(objects)
    return cc.close_projection(s, [p], **LIMITS)


def note(case, **kw):
    OBS.append(dict(case=case, **kw))


class CountingInput:
    def __init__(self, values):
        self.values = values
        self.calls = 0
    def __iter__(self):
        for n in self.values:
            self.calls += 1
            yield n


class Tests(unittest.TestCase):
    def test_f1_public_positive_and_geometry(self):
        objects = public_objects()
        result = dv.compare_domain(closed(objects), LEDGER)
        self.assertEqual((result['segments'], result['centers'], result['pairs_compared']), (10, 2, 1))
        self.assertNotIn('MATCH', result['status'])
        note('F1公共payload正控', result=result)
        for key in ('price', 'group_anchor'):
            bad = copy.deepcopy(objects)
            row = next(r for r in bad if r['kind'] == 'CC-013.segment')
            row['payload']['data']['geometric_end'][key] = '0'
            verified = closed(bad)
            with self.assertRaisesRegex(ValueError, 'geometric_end') as error:
                dv.compare_domain(verified, LEDGER)
            note('F1公共几何篡改:'+key, closure=verified['closure_verified'], rejection=str(error.exception))

    def test_f1_conflicting_internal_fields(self):
        for public_bad in (True, False):
            objects = public_objects()
            row = next(r for r in objects if r['kind'] == 'CC-013.segment')
            row['payload_json'] = copy.deepcopy(row['payload'])
            row['payload' if public_bad else 'payload_json']['data']['geometric_end']['price'] = '0'
            verified = closed(objects)
            with self.assertRaisesRegex(ValueError, '公共对象禁止内部字段payload_json') as error:
                dv.compare_domain(verified, LEDGER)
            note('F1双字段冲突', public_bad=public_bad, rejection=str(error.exception))

    def test_f3_public_complete_set(self):
        for kind in sorted(dv.TESTED_KINDS):
            for mode in ('missing', 'null', 'string', 'data_null', 'data_missing'):
                row = dict(kind=kind, object_id='extra')
                if mode != 'missing':
                    row['payload'] = {'null': None, 'string': '{}', 'data_null': {'data': None}, 'data_missing': {}}[mode]
                verified = closed(public_objects()+[row])
                with self.assertRaisesRegex(ValueError, 'payload') as error:
                    dv.compare_domain(verified, LEDGER)
                note('F3公共全集:'+kind+'/'+mode, closure=True, rejection=str(error.exception))

    def test_f2_graph_duplicate_and_expired(self):
        enc = cc.Encoder(); enc.encode(None); n = next(iter(enc.nodes.values()))
        for expired in (False, True):
            calls = []
            def check():
                calls.append(1)
                if expired: raise ValueError('总截止已到')
            budget = cc.WorkBudget(1, check)
            nodes = CountingInput([n]*20000)
            with patch.object(cc, 'frame', wraps=cc.frame) as hashing:
                with self.assertRaisesRegex(ValueError, '工作量|截止') as error:
                    cc.Graph(nodes, max_nodes=1, max_bytes=1, max_work=1, work_budget=budget)
            self.assertLessEqual(nodes.calls, 1)
            self.assertEqual(hashing.call_count, 0)
            self.assertLessEqual(budget.used, 1)
            self.assertTrue(calls)
            note('F2重复20000', expired=expired, input_calls=nodes.calls, hash_calls=hashing.call_count,
                 work=budget.used, deadline_calls=len(calls), rejection=str(error.exception))

    def test_f2_empty_and_split(self):
        def expired(): raise ValueError('总截止已到')
        with self.assertRaisesRegex(ValueError, '截止'):
            cc.Graph([], max_nodes=1, max_bytes=1, max_work=10, work_budget=cc.WorkBudget(10, expired))
        budget = cc.WorkBudget(3)
        cc.Graph([], max_nodes=1, max_bytes=1, max_work=3, work_budget=budget)
        with self.assertRaisesRegex(ValueError, '工作量'):
            cc.Graph([], max_nodes=1, max_bytes=1, max_work=3, work_budget=budget)
        note('F2空输入分块', used=budget.used, limit=budget.limit)
        s,p,_=fixture()
        with self.assertRaisesRegex(ValueError,'固定集合'):
            cc.close_projection(s,iter([p]),**LIMITS)
        with self.assertRaisesRegex(ValueError,'缺失闭包节点'):
            cc.close_projection(s,[],**LIMITS)
        note('F2空页与一次性迭代器不能绕闭包',result='REJECT')

    def test_f2_collector_before_projection(self):
        s, _, _ = fixture()
        for expired in (False, True):
            callbacks = []
            def check():
                callbacks.append(1)
                if expired: raise ValueError('总截止已到')
            collector = object.__new__(dv.CollectorV2)
            collector.budget = type('B', (), {'check': staticmethod(check)})()
            with patch.object(collector, 'snapshot', return_value=s) as snap, \
                 patch.object(collector, 'content', side_effect=AssertionError('不应先取内容')) as content, \
                 patch.object(cc, 'frame', wraps=cc.frame) as hashing:
                with self.assertRaisesRegex(ValueError, '工作量|截止') as error:
                    collector.closed_snapshot(s['fixed_cut'], 2, max_nodes=100000, max_bytes=16*1024*1024, max_work=10)
            self.assertEqual(content.call_count, 0)
            self.assertEqual(hashing.call_count, 0)
            if expired: self.assertEqual(snap.call_count, 0)
            note('F2Collector前置拒绝', expired=expired, snapshot_calls=snap.call_count,
                 content_calls=content.call_count, hash_calls=hashing.call_count,
                 work=collector.work_budget.used, deadline_calls=len(callbacks), rejection=str(error.exception))

    def test_f2_real_content_loop_is_cumulative(self):
        # 基类分页循环确实使用同一预算；fake call 不产生文件/请求。
        collector = object.__new__(dv.CollectorV2)
        collector.budget = Budget(10000, 10)
        collector.work_budget = cc.WorkBudget(3)
        with patch.object(collector, 'call', side_effect=AssertionError('预算应在call前耗尽')) as call:
            with self.assertRaisesRegex(ValueError, '工作量'):
                collector.content({'many': list(range(100))}, [], 'items', 1)
        self.assertEqual(call.call_count, 0)
        note('F2内容分配前预算', call_count=call.call_count, work=collector.work_budget.used)

    def test_f2_decode_plain_records_same_budget(self):
        enc=cc.Encoder(); root=enc.encode('leaf')
        for _ in range(16): root=enc.encode([cc.Edge(root),cc.Edge(root)])
        callbacks=[]
        budget=cc.WorkBudget(1000,lambda:callbacks.append(1))
        graph=cc.Graph(list(enc.nodes.values()),max_nodes=1000,max_bytes=200000,max_work=1000,work_budget=budget)
        built=graph.work; decoded=graph.decode(root); decoded_work=graph.work
        with self.assertRaisesRegex(ValueError,'工作量') as error: dv.plain(decoded,graph)
        self.assertIs(graph.budget,budget);self.assertEqual(graph.work,1000)
        note('F2decode到plain累计',build_work=built,decode_work=decoded_work,used=graph.work,
             deadline_calls=len(callbacks),rejection=str(error.exception))
        base_nodes=list(enc.nodes.values())
        obj=enc.encode({'kind':'CC-013.segment','payload':{'data':{'unused':cc.Edge(root)}}})
        ref=enc.encode({'rows':[{'family':'objects','record':cc.Edge(obj)}]})
        budget=cc.WorkBudget(1000)
        with self.assertRaisesRegex(ValueError,'工作量') as error:
            graph=cc.Graph(list(enc.nodes.values()),max_nodes=1000,max_bytes=200000,max_work=1000,work_budget=budget)
            dv.records({'graph':graph,'projection_ref':ref})
        note('F2records路径累计可提前拒绝',used=budget.used,rejection=str(error.exception))
        g=cc.Graph(base_nodes,max_nodes=1000,max_bytes=200000,max_work=1000)
        decoded=g.decode(root)
        g.budget.check=lambda:(_ for _ in ()).throw(ValueError('总截止已到'))
        with self.assertRaisesRegex(ValueError,'截止'):dv.plain(decoded,g)

    def test_f2_graph_byte_work_before_hex_hash(self):
        raw=dict(root='sc1-'+'0'*64,tag='10',body_hex='ab'*100000)
        budget=cc.WorkBudget(10)
        with patch.object(cc,'frame',wraps=cc.frame) as hashing:
            with self.assertRaisesRegex(ValueError,'工作量') as error:
                cc.Graph([raw],max_nodes=1,max_bytes=200000,max_work=10,work_budget=budget)
        self.assertEqual(hashing.call_count,0)
        note('F2大hex先预算',work=budget.used,hash_calls=hashing.call_count,rejection=str(error.exception))

    def test_f2_collector_budget_survives_second_snapshot(self):
        s,_,_=fixture();c=object.__new__(dv.CollectorV2);c.budget=Budget(10000,10)
        with patch.object(c,'snapshot',return_value=s),patch.object(c,'content') as content:
            for _ in range(2):
                with self.assertRaisesRegex(ValueError,'工作量'):
                    c.closed_snapshot(s['fixed_cut'],2,max_nodes=1000,max_bytes=200000,max_work=10)
                budget=c.work_budget
            self.assertIs(c.work_budget,budget);self.assertEqual(content.call_count,0)
            with self.assertRaisesRegex(ValueError,'禁止重置/加帽'):
                c.closed_snapshot(s['fixed_cut'],2,max_nodes=1000,max_bytes=200000,max_work=11)
        note('F2Collector不重置不加帽',used=budget.used,limit=budget.limit)

    def test_f8_frozen_named_messages(self):
        requests=[json.loads(x) for x in (ROOT/'raw/messages.recovery-epoch2.jsonl').read_text().splitlines()][-3:]
        s,_,_=fixture(); cuts=[]
        for i,(r,f) in enumerate(zip(requests,[139,204,269]),800):
            cuts.append(dict(s['fixed_cut'],session_id=r['session_id'],session_generation=r['session_generation'],
                        cut_generation=str(i),structure_cut='cut-'+str(i),as_of_generation=str(i),input_frontier=str(f)))
        bindings=bind_named_submissions(requests,cuts)
        self.assertEqual([b['accepted_events'] for b in bindings.values()],['1','65','65'])
        saved=copy.deepcopy(bindings);cuts[0]['input_frontier']='999'
        self.assertEqual(saved,bindings)
        note('F8冻结具名原请求绑定',counts=[1,65,65],copied_cut=True,product_binding='UNVERIFIED')

    def test_f5_all_headers(self):
        s, p, _ = fixture()
        cases = [(255,b''), (3,b'\0'*39), (0,b'x'), (8,b'x'), (17,b'x'),
                 (7,b''), (18,b''), (16,b''), (16,b'x'*4097), (19,b'\0'*71),
                 (19,cc.u64(1)+b'\0'*64), (20,b'\0'*63), (5,(2**63).to_bytes(8,'big')+b'\0'*32)]
        for tag, body in cases:
            raw = dict(root='sc1-'+cc.frame(tag,body), tag=f'{tag:02x}', body_hex=body.hex())
            bad = copy.deepcopy(p); bad['nodes'].append(raw)
            with self.assertRaisesRegex(ValueError, '节点头|未知tag') as error:
                cc.close_projection(s, [bad], **LIMITS)
            note('F5未引用头负控', tag=raw['tag'], body_bytes=len(body), rejection=str(error.exception))
        # 合法头的未引用节点允许入池；不冒充它们的完整闭包。
        bodies = {0:b'',1:b'',2:b'',3:cc.u64(0)+b'\0'*32,4:cc.u64(0)+b'\0'*32,
                  5:cc.u64(0)+b'\0'*32,6:cc.u64(0)+b'\0'*32,7:b'\0'*32,8:b'',
                  16:b'x',17:b'',18:b'\0'*32,19:cc.u64(2)+b'\0'*64,20:b'\0'*64}
        for tag, body in bodies.items():
            g = cc.Graph([dict(root='sc1-'+cc.frame(tag,body),tag=f'{tag:02x}',body_hex=body.hex())], **LIMITS)
            self.assertEqual(len(g.pool), 1)
        note('F5合法头形状正控', tags=sorted(bodies))

    def watch_case(self, frontiers, counts, *, change_cut=None, change_delta=None):
        s, _, _ = fixture(); fixed = s['fixed_cut']
        cursor = {k:fixed[k] for k in ('session_id','session_generation','catalog_revision','scope','index_frontier')}
        cursor.update(after_generation='139',base_cut='toy-cut',last_seq='138',order_version='s-record-order/2')
        submissions = []; cuts = []; deltas = []; next_cursor = copy.deepcopy(cursor)
        for i,(frontier,count) in enumerate(zip(frontiers,counts),140):
            payload = {'raw_input': {'events':[{}]*count}}
            request = dict(session_id='toy',session_generation='1',message_id='named-'+str(i),
                           source_epoch='1' if i==140 else '2', payload=payload,
                           payload_hash=__import__('fixed_cut_driver').sha(__import__('fixed_cut_driver').canonical(payload)))
            cut = dict(fixed,cut_generation=str(i),structure_cut='cut-'+str(i),input_frontier=frontier,
                       index_frontier=str(i),as_of_generation=str(i),seq_range={'from':'7','to':'7'})
            delta = dict(generation=str(i),session_id='toy',catalog_revision=cut['catalog_revision'],
                         base_cut=next_cursor['base_cut'],next_cut=cut['structure_cut'],index_frontier=cut['index_frontier'],
                         input_frontier=frontier,seq_range=cut['seq_range'],catalog_run_status=cut['catalog_run_status'],
                         catalog_evidence=cut['catalog_evidence'],delta={})
            next_cursor.update(after_generation=str(i),base_cut=delta['next_cut'],index_frontier=str(i),last_seq=frontier)
            submissions.append(request); cuts.append(cut); deltas.append(delta)
        if change_cut: change_cut(cuts)
        bindings = bind_named_submissions(submissions,cuts)
        if change_delta: change_delta(deltas)
        collector = object.__new__(Collector)
        page = dict(deltas=deltas,next_cursor=next_cursor,observed_head=str(139+len(deltas)),delivery_frontier=str(139+len(deltas)),
                    has_more=False,gap=None,validated_capture_digest='b'*64,ok=True)
        with patch.object(collector,'call',return_value=page):
            result = collector.watch(cursor,139+len(deltas),'client',max_batches=len(deltas),accepted_commits=bindings)
        return result, bindings

    def test_f8_cut_frontier_conflict(self):
        with self.assertRaisesRegex(ValueError,'接纳切面.*input_frontier') as error:
            self.watch_case(['139'],[1],change_cut=lambda cuts:cuts[0].update(input_frontier='999'))
        note('F8同代140前沿999对139', rejection=str(error.exception))

    def test_f8_identity_and_cross_epoch(self):
        result, bindings = self.watch_case(['139','204','269'],[1,65,65])
        self.assertEqual(result['next_cursor']['last_seq'],'269')
        self.assertEqual([b['accepted_cut']['input_frontier'] for b in bindings.values()],['139','204','269'])
        note('F8跨epoch正控', counts=[1,65,65], frontiers=['139','204','269'], seq_to='7')
        for key,value in [('structure_cut','alien'),('index_frontier','999'),('catalog_run_status','alien'),
                          ('catalog_evidence',{}),('seq_range',{'from':'8','to':'8'}),('scope',{})]:
            with self.assertRaises(ValueError) as error:
                self.watch_case(['139'],[1],change_cut=lambda cuts:cuts[0].update({key:value}))
            note('F8切面身份:'+key, rejection=str(error.exception))
        for frontier,count in [('100',1),('-1',1),('0139',1),('139.0',1),(139,1),(True,1),(None,1),('140',1),('139',2)]:
            with self.assertRaises(ValueError) as error:self.watch_case([frontier],[count])
            note('F8保留前沿门', frontier=frontier, count=count, rejection=str(error.exception))


if __name__ == '__main__':
    label = sys.argv[1] if len(sys.argv)>1 else 'final'
    result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Tests))
    (Path(__file__).parent/(label+'-observations.json')).write_text(json.dumps(dict(tests=result.testsRun,
        failures=len(result.failures), errors=len(result.errors), observations=OBS, product_run='NOT_RUN'),ensure_ascii=False,indent=2)+'\n')
    sys.exit(not result.wasSuccessful())
