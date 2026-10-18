"""新增逻辑的定向验证：纯数学、假传输、固定Python toy；不是产品验收。"""
import copy
import json
from pathlib import Path
import tempfile
import time
import unittest
from raw_oracle import derive,canonical,digest,public_semantics
from content_closure import Encoder,Edge,JSONText,Graph,frame,root_bytes,close_projection
from driver_v2 import CollectorV2,compare_domain,compare_named_runs,reject_live
from fixed_cut_driver import Budget,FAMILIES
from toy_supervisor import supervise

ROOT=Path(__file__).resolve().parents[1]
LIMITS=dict(max_nodes=100000,max_bytes=16*1024*1024,max_work=1000000)
BASE=json.loads((ROOT/'RAW-INPUT.json').read_bytes())['bars']
LEDGER=json.loads((ROOT/'INDEPENDENT-WITNESS-LEDGER.json').read_bytes())['base']


def signed(value):return {**value,'token_checksum':digest(canonical(value))}


def reply(request,payload):
    ph=digest(canonical(payload))
    return canonical({'schema_revision':'s-session/3','session_id':request['session_id'],
        'session_generation':request['session_generation'],'source_namespace':'s-observe/replies',
        'source_epoch':'1','producer_id':'Q:'+request['session_id'],'producer_epoch':'1',
        'message_id':'reply-'+digest(canonical([request['source_namespace'],request['source_epoch'],request['message_id'],ph])),
        'payload_hash':ph,'causal_refs':[{k:request[k] for k in ('source_namespace','source_epoch','message_id','payload_hash')}],
        'payload':payload})


def fixture(objects=None):
    enc=Encoder();evidence=enc.encode({'toy':'catalog evidence'})
    catalog=enc.encode({'toy':'catalog'})
    fixed={'session_id':'toy','session_generation':'1','structure_cut':'toy-cut','cut_generation':'139',
           'catalog_revision':'toy-catalog','index_frontier':'139','profile_id':'toy-profile','profile_hash':'a'*64,
           'input_frontier':'138','seq_range':{'from':'138','to':'138'},'catalog_run_status':'complete',
           'catalog_evidence':evidence,'scope':{'structure':'CompleteCut','economic':'not_started'},
           'rule_revision':'TOY_ONLY','history_mode':'AsKnown','as_of_generation':'139'}
    rows=[]
    for family in FAMILIES:
        records=objects if family=='objects' and objects is not None else [{'toy_family':family}]
        for i,r in enumerate(records):rows.append({'family':family,'key':[str(i)],'record':enc.encode(r)})
    counts={f:str(sum(r['family']==f for r in rows)) for f in FAMILIES};counts['total']=str(len(rows))
    logical={'fixed_cut':{**fixed,'catalog_evidence':Edge(evidence)},'catalog':Edge(catalog),
             'rows':[{**r,'record':Edge(r['record'])} for r in rows], 'counts':counts,
             'scope':fixed['scope'],'omissions':[],'order_version':'s-record-order/2'}
    root=enc.encode(logical);qcut='qcut3-'+frame(0x22,root_bytes(root['root']))
    snapshot={**logical,'fixed_cut':fixed,'catalog':catalog,'rows':rows,'cut_projection_digest':qcut}
    anchor={'kind':'projection','fixed_cut':fixed,'cut_projection_digest':qcut,'projection_ref':root}
    graph=Graph(list(enc.nodes.values()),**LIMITS)
    page={'anchor':anchor,'path':[],'target_ref':root,'unit':'items','offset':'0','end':'7','total':'7',
          'items':graph.entries(root),'nodes':list(enc.nodes.values()),'done':True,'next_token':None,
          'validated_capture_digest':'b'*64,'ok':True}
    return snapshot,page,graph


def domain_records(ledger):
    result=[]
    def add(kind,data):result.append({'kind':kind,'payload_json':JSONText({'data':data})})
    strokes=ledger['low_level']['strokes']
    for s in ledger['segments']+[ledger['pending_segment']]:
        end=s.get('observed_through_stroke',len(strokes)-1)
        refs=[{'stroke_index':str(b['index']),'start_anchor':str(b['start_raw']),'end_anchor':str(b['end_raw']),
               'start_price':str(b['start_price']),'end_price':str(b['end_price']),
               'start_roots':[str(b['start_raw'])],'end_roots':[str(b['end_raw'])]} for b in strokes[s['start_stroke']:end+1]]
        add('CC-013.segment',{'entity_id':s['id'],'construction':{'start_stroke':str(s['start_stroke'])},
                             'result':s.get('result','NO_FIRST'),'geometric_end':{'stroke_index':str(s.get('end_stroke',0))},'stroke_refs':refs})
    for c in ledger['centers']:
        add('CC-016.center',{'entity_id':c['id'],'seed_units':c['seed'],'members':c['members'],'zn_members':c['Zn']})
        roles={}
        for group,keys in [('CORE',['ZD','ZG']),('OUTER',['DD','GG']),('INNER_EXTREMA',['D','G'])]:
            roles[group]={k:{'value':str(c[k] if k in ('ZD','ZG') else c['roles'][k]),'frozen':group=='CORE',
                             'contributors':c['contributors'].get(k,[])} for k in keys}
        add('CC-017.center_roles',{'center_id':c['id'],'roles':roles})
    for a in ledger['assignments']:
        add('CC-022.unit_assignment',{'unit':{'unit_id':a['unit']},'status':a['status'],'center_id':a.get('center'),
                                     'left_center_id':a.get('left_center'),'right_center_id':a.get('right_center')})
    for p in ledger['pairs']:
        matrix={'B.'+y+'?A.'+x:{'LT':'>','EQ':'=','GT':'<'}[v] for xy,v in p['matrix16'].items() for x,y in [xy.split(':')]}
        add('CC-020.center_pair',{'a_center_id':p['left'],'b_center_id':p['right'],'leaf':p['leaf'],'matrix':matrix})
    return result


class V2Tests(unittest.TestCase):
    def test_full_raw_proof_and_hand_calculated_anchors(self):
        x=derive(BASE)
        self.assertEqual(x,LEDGER)
        self.assertEqual((len(x['low_level']['groups']),len(x['low_level']['fractals']),len(x['low_level']['strokes']),len(x['segments'])),(139,35,34,10))
        self.assertEqual([s['result'] for s in x['segments']].count('CASE_TWO'),4)
        self.assertEqual([s['first_known_input_count'] for s in x['segments']],[39,39,63,63,87,87,111,111,123,135])
        self.assertEqual([c['members'] for c in x['centers']],[['u0','u1','u2','u3','u4'],['u6','u7','u8','u9']])
        self.assertEqual(len(x['pairs'][0]['matrix16']),16)

    def test_no_early_segment_or_center_from_missing_witness(self):
        for before,after,count in [(38,39,2),(62,63,4),(86,87,6),(110,111,8),(122,123,9),(134,135,10)]:
            self.assertLess(len(derive(BASE[:before])['segments']),count)
            self.assertEqual(len(derive(BASE[:after])['segments']),count)

    def test_bad_raw_domain_rejected_instead_of_relaxed_fallback(self):
        bad=copy.deepcopy(BASE);bad[1].update(high=bad[0]['high'],low=bad[0]['low'],open=bad[0]['open'],close=bad[0]['close'])
        with self.assertRaisesRegex(ValueError,'包含'):derive(bad)

    def test_revision_withdrawal_restore_named_expected(self):
        actual={'base':public_semantics(derive(BASE))}
        for name in ['revise-non-zn-tail','withdraw-second-center','restore-original']:
            bars=json.loads((ROOT/f'raw/{name}.json').read_bytes())['bars']
            actual[name]=public_semantics(derive(bars))
        expected={name:json.loads((ROOT/f'expected/{name}.json').read_bytes()) for name in actual}
        self.assertEqual(compare_named_runs(actual,expected)['product_run'],'NOT_RUN')
        actual['revise-non-zn-tail']['centers'][1]['roles']['DD']=2112
        with self.assertRaisesRegex(ValueError,'expected不符'):compare_named_runs(actual,expected)

    def test_second_sequence_confirm_precedes_same_stroke_crossing(self):
        for s in LEDGER['segments']:
            if s['result']=='CASE_TWO':
                flow=s['proof']['second_sequence']['per_stroke_order']
                self.assertTrue(flow[-1]['fractal_first'])
                self.assertFalse(any(x['strict_crossing_after_fractal_check'] for x in flow[:-1]))
                self.assertEqual(flow[-1]['state'],'CONFIRMED')

    def test_closure_validates_six_families_and_qcut(self):
        snapshot,page,_=fixture()
        result=close_projection(snapshot,[page],**LIMITS)
        self.assertTrue(result['closure_verified'])
        self.assertEqual(len(result['decoded']['rows']),6)
        changed=copy.deepcopy(snapshot);changed['fixed_cut']['input_frontier']='137'
        with self.assertRaisesRegex(ValueError,'qcut3'):close_projection(changed,[page],**LIMITS)

    def test_closure_rejects_missing_node_tamper_extent_and_mixed_anchor(self):
        s,p,_=fixture()
        # 缺外部record实体，重建的七字段不能替它补出来。
        missing=copy.deepcopy(p);target=s['rows'][0]['record']['root'];missing['nodes']=[n for n in p['nodes'] if n['root']!=target]
        with self.assertRaisesRegex(ValueError,'缺失'):close_projection(s,[missing],**LIMITS)
        tampered=copy.deepcopy(p);tampered['nodes'][0]['body_hex']+='00'
        with self.assertRaisesRegex(ValueError,'hash'):close_projection(s,[tampered],**LIMITS)
        mixed=copy.deepcopy(p);mixed['anchor']['projection_ref']['extent']='8'
        with self.assertRaisesRegex(ValueError,'混切面'):close_projection(s,[mixed],**LIMITS)
        changed=copy.deepcopy(p);changed['items'][0]['key']='伪造'
        with self.assertRaisesRegex(ValueError,'内容页'):close_projection(s,[changed],**LIMITS)

    def test_codec_large_bytes_numbers_and_json_text(self):
        from content_closure import Number
        value={'text':'中'*1400,'number':Number('1.2300e+7'),'json':JSONText({'x':'值'})}
        enc=Encoder();ref=enc.encode(value);g=Graph(list(enc.nodes.values()),**LIMITS)
        self.assertEqual(g.decode(ref),value)
        with self.assertRaisesRegex(ValueError,'集合超'):Graph(list(enc.nodes.values()),max_nodes=1,max_bytes=20,max_work=30)

    def test_domain_adapter_positive_and_zn_mutation_negative(self):
        objs=domain_records(LEDGER);s,p,_=fixture(objs)
        self.assertEqual(compare_domain(close_projection(s,[p],**LIMITS),LEDGER)['centers'],2)
        roles=next(r for r in objs if r['kind']=='CC-017.center_roles' and r['payload_json'].value['data']['center_id']=='c6')
        roles['payload_json'].value['data']['roles']['OUTER']['DD']['value']='2080'
        s,p,_=fixture(objs)
        with self.assertRaisesRegex(ValueError,'角色值'):compare_domain(close_projection(s,[p],**LIMITS),LEDGER)

    def test_domain_withdrawal_rejects_stale_pair(self):
        w=json.loads((ROOT/'INDEPENDENT-WITNESS-LEDGER.json').read_bytes())['revision_ledgers']['withdraw-second-center']
        objs=domain_records(w);s,p,_=fixture(objs)
        self.assertEqual(compare_domain(close_projection(s,[p],**LIMITS),w,require_pair=False)['centers'],1)
        old_pair=next(r for r in domain_records(LEDGER) if r['kind']=='CC-020.center_pair')
        s,p,_=fixture(objs+[old_pair])
        with self.assertRaisesRegex(ValueError,'撤回后'):compare_domain(close_projection(s,[p],**LIMITS),w,require_pair=False)

    def test_driver_collects_fixed_pages_and_full_content(self):
        snapshot,root_page,graph=fixture()
        identity={'session_id':'toy','session_generation':'1','source_namespace':'toy.query','source_epoch':'1','producer_id':'toy-driver','producer_epoch':'1'}
        captured=[]
        def exchange(route,raw):
            req=json.loads(raw);p=req['payload'];captured.append((route,p))
            if route.endswith('snapshot'):
                token=p.get('snapshot_token');offset=int(token['offset']) if token else 0;size=2;end=min(offset+size,6)
                nt=None
                if end<6:
                    nt=signed({'token_schema':'s-snapshot-token/2',**{k:snapshot['fixed_cut'][k] for k in ('session_id','session_generation','cut_generation','scope','history_mode','structure_cut','catalog_revision','index_frontier','profile_hash','rule_revision')},'order_version':'s-record-order/2','page_size':'2','offset':str(end),'cut_projection_digest':snapshot['cut_projection_digest'],'issued_capture_digest':'b'*64})
                payload={**snapshot,'rows':snapshot['rows'][offset:end],'offset':str(offset),'done':end==6,'next_token':nt,'validated_capture_digest':'b'*64,'ok':True}
            else:
                q=p.get('content_token',p);path=q['path'];target=graph.resolve(root_page['target_ref'],path)
                offset=int(q['offset']);limit=int(q['limit']);unit=q['unit'];total=1 if unit=='value' else int(target['extent']);end=min(offset+limit,total)
                items=graph.entries(target)[offset:end] if unit=='items' else graph.byte_value(target)[offset:end].hex() if unit=='bytes' else [target]
                nt=None if end==total else signed({'token_schema':'s-content-token/1','anchor':q['anchor'],'path':path,'target_ref':target,'unit':unit,'offset':str(end),'limit':str(limit),'issued_capture_digest':'b'*64})
                payload={**root_page,'path':path,'target_ref':target,'unit':unit,'offset':str(offset),'end':str(end),'total':str(total),'items':items,'done':end==total,'next_token':nt}
            return 200,{},reply(req,payload)
        with tempfile.TemporaryDirectory() as tmp:
            collector=CollectorV2(identity,Budget(16*1024*1024,10),exchange,Path(tmp)/'capture','1')
            _,_,v=collector.closed_snapshot(snapshot['fixed_cut'],2,**LIMITS)
            self.assertTrue(v['closure_verified']);self.assertGreater(len(captured),3)

    def test_live_is_unconditionally_denied(self):
        with self.assertRaisesRegex(RuntimeError,'authorization=false'):reject_live()

    def test_toy_supervisor_paths_reap(self):
        outcomes=[]
        with tempfile.TemporaryDirectory() as tmp:
            for i,(name,opts) in enumerate([
                ('success',{}),('wait',{}),('ignore-term',{}),('flood',{'log_bytes':128}),
                ('wait',{'owner_alive':lambda:False}),('wait',{'cancelled':lambda:True})]):
                kwargs=dict(seconds=0.2,log_bytes=4096,term_grace=0.05,kill_grace=0.5);kwargs.update(opts)
                result=supervise(name,Path(tmp)/str(i),**kwargs);outcomes.append(result)
                logs=ROOT/'validation/toy-logs'/str(i);logs.mkdir(parents=True,exist_ok=True)
                for label in ('stdout','stderr'):
                    (logs/(label+'.raw')).write_bytes((Path(tmp)/str(i)/(label+'.raw')).read_bytes())
                self.assertTrue(result['reaped']);self.assertLessEqual(result['log_bytes_saved'],kwargs['log_bytes'])
            self.assertIn('SIGKILL',outcomes[2]['signals'])
            self.assertFalse(outcomes[3]['logs_complete'])
            self.assertEqual(outcomes[4]['reason'],'owner失联toy')
            self.assertEqual(outcomes[5]['reason'],'人工取消toy')
        (ROOT/'validation/supervisor-toy.json').write_text(json.dumps(outcomes,ensure_ascii=False,indent=2)+'\n')


if __name__=='__main__':unittest.main(verbosity=2)
