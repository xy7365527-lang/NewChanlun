"""捕获流/桥的toy测试。测试kind和字段映射不是尚未产出的正式G协议。"""
import copy
import time
import unittest
import driver as d
import closure as c
import captured_cut as x
import voice_bridge as v


def budget():return d.ReadBudget(time.monotonic()+10,100000,8*1024*1024)
def fp(value):return d.sha(d.canonical(value))


def fixture():
    b=budget();nodes={};input_raw=b'UNIT_TEST_ONLY complete input bytes'
    resource={'session':{k:'1000000' for k in d.SESSION},'reader':{k:'1000000' for k in d.READER},
              'q':{'schema_revision':'s-query-resources/3','limits':{k:'1000000' for k in x.Q_LIMITS}},
              's_writer':{'schema_revision':'s-structure-resources/1','s_resources':{k:'1000000' for k in x.TRANSPORT|{'audit_cache_source_bytes'}}},
              'economic_domains':{o:{k:'1000000' for k in x.TRANSPORT} for o in ('E','B:alpha','B:beta')},
              'legacy_reader':{'max_reply_bytes':'16777216','timeout_ms':'10000'},
              'deadlines':{'work':b.deadline+10,'cleanup':b.deadline+12,'seal':b.deadline+14,'stages':{'export':b.deadline+9}}}
    resource['session']['schema_revision']='s-financial-session-resources/3'
    resource['reader']['schema_revision']='s-financial-reader-resources/1'
    resource['reader']['job_limits']={k:'1000000' for k in d.JOB}
    source={'commit':'a'*40,'tree':'b'*40,'binary_sha256':'c'*64,'input_sha256':d.sha(input_raw),
            'artifact_manifest_sha256':'d'*64,'profile_hash':'e'*64,'rule_revision':'UNIT_TEST_ONLY_RULE','catalog_revision':'toy-catalog'}
    obj={'object_id':'source','object_revision':'1','kind':'UNIT_TEST_ONLY_SOURCE','fact_key':'source',
         'profile':'toy','start':'0','end':'3'}
    common={'profile':'toy','execution_level':'0','confirmation_level':'1','direction':'Long','anchor':'a',
            'start':'0','end':'3','confirmed_at':'3','labels':['type1','type2'],'source_ids':['source']}
    terminal={'object_id':'terminal','object_revision':'1','kind':'UNIT_TEST_ONLY_TERMINAL','fact_key':'terminal',**common}
    carrier={'object_id':'carrier','object_revision':'1','kind':'UNIT_TEST_ONLY_K','fact_key':'carrier','profile':'toy',
             'level':'0','direction':'Long','anchor':'a','start':'0','end':'3','children':[],'source_ids':['source']}
    snapshot={'session_id':'toy-session','generation':'1','structure_cut':'toy-cut','catalog_revision':'toy-catalog',
              'index_frontier':'batch1','profile_hash':'e'*64,'profile_id':'toy','input_frontier':'3',
              'scope':{'kind':'UNIT_TEST_ONLY'},'seq_range':{'start':'0','end':'3'},
              'catalog_run_status':'toy','history_mode':'AsKnown','as_of_generation':'1',
              'catalog_evidence':{'axes':['UNIT_TEST_ONLY']},**{k:[] for k in x.FAMILIES}}
    snapshot['objects']=[obj,terminal,carrier]
    # 六族记录全在完整basis根内；不能拿空族代替现有来源、关系、见证与历史。
    snapshot['witnesses']=[{'witness_id':'w','object_id':'terminal','slot':'0'}]
    snapshot['relations']=[{'subject':'terminal','relation_type':'source','object':'source'}]
    snapshot['observations']=[{'observation_id':'o','kind':'toy','window_start':None}]
    snapshot['raw_history']=[{'identity_key':'raw','revision':'1','seq':'0'}]
    fixed={k:snapshot[k] for k in ('session_id','structure_cut','catalog_revision','index_frontier','profile_hash','profile_id','input_frontier')}
    fixed.update(session_generation='1',cut_generation='1',seq_range={'start':'0','end':'3'},
                 catalog_run_status='toy',catalog_evidence=x.content_ref(snapshot['catalog_evidence'],b),
                 scope={'kind':'UNIT_TEST_ONLY'},rule_revision=source['rule_revision'],history_mode='AsKnown',as_of_generation='1')
    basis={'kind':'FinancialBasis','session_id':'toy-session','session_generation':'1','writer_epoch':'1',
           'commit_ns':'30','pending_revision':False,'snapshot':snapshot}
    basis_ref=x.content_ref(basis,b,nodes)
    anchor={k:basis[k] for k in ('session_id','session_generation','writer_epoch','commit_ns')}
    anchor.update(structure_cut='toy-cut',index_frontier='batch1',basis_ref=basis_ref)
    rows=[];counts={}
    for family in x.FAMILIES:
        records=sorted(snapshot[family],key=lambda r:x.row_identity(family,r)[1]);counts[family]=str(len(records))
        for record in records:
            rows.append({'family':family,'key':x.row_identity(family,record)[0],'record':x.content_ref(record,b)})
    counts['total']=str(len(rows));catalog=x.content_ref({'toy_catalog':True},b)
    projected_fixed=dict(fixed,catalog_evidence=x.Edge(fixed['catalog_evidence']))
    projection={'fixed_cut':projected_fixed,'catalog':x.Edge(catalog),'rows':[dict(r,record=x.Edge(r['record'])) for r in rows],
                'counts':counts,'scope':fixed['scope'],'omissions':[],'order_version':'s-record-order/2'}
    pr=x.content_ref(projection,b);qdigest='qcut3-'+c.root_bytes(c.node_root(0x22,c.root_bytes(pr['root']))).hex()
    captures=[];next_token=None
    def envelope(payload,ident):
        return {'schema_revision':'s-session/3','session_id':'toy-session','session_generation':'1',
                'source_namespace':'toy','source_epoch':'1','message_id':ident,'producer_id':'toy','producer_epoch':'1',
                'payload_hash':fp(payload),'payload':payload,'causal_refs':[]}
    for offset in range(0,len(rows),2):
        req={'op':'snapshot','snapshot_token':next_token} if next_token else {
            'op':'snapshot','session_id':'toy-session','session_generation':'1','scope':fixed['scope'],
            'history_mode':'AsKnown','as_of_generation':'1','page_size':'2','order_version':'s-record-order/2'}
        end=min(offset+2,len(rows));done=end==len(rows)
        next_token=None
        if not done:
            next_token={k:fixed[k] for k in x.TOKEN_FIXED}
            next_token.update(token_schema='s-snapshot-token/2',order_version='s-record-order/2',page_size='2',
                              offset=str(end),cut_projection_digest=qdigest,issued_capture_digest='f'*64)
            next_token['token_checksum']=fp(next_token)
        page={'fixed_cut':fixed,'catalog':catalog,'rows':rows[offset:end],'counts':counts,'scope':fixed['scope'],
              'omissions':[],'order_version':'s-record-order/2','offset':str(offset),'done':done,'next_token':next_token,
              'cut_projection_digest':qdigest,'validated_capture_digest':'f'*64,'ok':True}
        req=envelope(req,'req'+str(offset));reply=envelope(page,'reply-'+fp([req['source_namespace'],req['source_epoch'],req['message_id'],fp(page)]))
        reply['causal_refs']=[{k:req[k] for k in ('source_namespace','source_epoch','message_id','payload_hash')}]
        captures.append(x.Capture('snapshot',d.canonical(req),d.canonical(reply)))
    query={'schema_revision':'s-financial-query/1',**{k:anchor[k] for k in ('session_id','session_generation','writer_epoch')},
           'object_ids':[r['object_id'] for r in snapshot['objects']]}
    selected=[]
    for index,record in enumerate(snapshot['objects']):
        ref=x.content_ref(record,b)
        selected.append({'object_id':record['object_id'],'snapshot_index':str(index),'object_ref':ref,
                         'object_hash':c.root_bytes(c.node_root(0x23,c.root_bytes(ref['root']))).hex()})
    reply={'schema_revision':'s-financial-basis/3','kind':'FinancialBasisRef','content_format':'s-content/1',
           **{k:anchor[k] for k in ('session_id','session_generation','writer_epoch','commit_ns')},
           'pending_revision':False,'basis_ref':basis_ref,'snapshot_ref':x.content_ref(snapshot,b),
           'selected_objects':selected,'proof_nodes':[]}
    captures.append(x.Capture('financial-basis',d.canonical(query),d.canonical(reply)))
    all_nodes=sorted(nodes.values(),key=lambda n:n.root);after=None
    for offset in range(0,len(all_nodes),30):
        chunk=all_nodes[offset:offset+30];end=offset+len(chunk)
        request={'schema_revision':'s-financial-closure/1','anchor':anchor,'path':[],'after':after,'max_reply_bytes':'65536'}
        reply={'anchor':anchor,'path':[],'after':after,'target_ref':basis_ref,'total':str(len(all_nodes)),
               'nodes':[{'root':n.root,'tag':f'{n.tag:02x}','body_hex':n.body.hex()} for n in chunk],
               'next':chunk[-1].root,'done':end==len(all_nodes),'ok':True}
        captures.append(x.Capture('financial-closure',d.canonical(request),d.canonical(reply)));after=chunk[-1].root
    contract={'schema_revision':'issue1394-captured-cut/1','execution_authorized':False,'source':source,'fixed_cut':fixed,
              'resources':resource,
              'anchor':anchor,'full_scope':fixed['scope'],'page_size':'2','cut_projection_digest':qdigest,
              'captures':[cap.identity() for cap in captures]}
    return contract,captures,nodes,input_raw


def export(contract,captures,nodes,input_raw):
    b=budget();raw=d.canonical(contract)
    return x.export_captured(raw,d.sha(raw),captures,nodes.__getitem__,b,c.Budget(10000,4000000,100000,b.deadline),input_raw=input_raw)


def bridge_fixture(facts,input_raw):
    objects=facts['snapshot']['objects'];terminal=next(o for o in objects if o['object_id']=='terminal');host=next(o for o in objects if o['object_id']=='carrier')
    gamma={k:terminal[k] for k in v.TERMINAL_FIELDS-{'profile','labels'}}
    gamma.update(gamma_id='gamma',terminal_id='terminal',terminal_sha256=fp(terminal),profile_id='toy',terminal_labels=terminal['labels'],
                 host_id='carrier',host_sha256=fp(host))
    proof={'proof_id':'toy-proof','method':'UNIT_TEST_ONLY_HAND_WRITTEN','author_id':'independent-toy',
           'input_sha256':d.sha(input_raw),'fixed_cut':facts['fixed_cut'],'profile_id':'toy','gamma_ids':['gamma']}
    instrument={'instrument':'TOY','input_sha256':d.sha(input_raw),'profile_id':'toy'}
    clock={'kind':'TestOnlyAffineOrdinalToNs','source_origin':'0','nanosecond_origin':'100','nanoseconds_per_ordinal':'10'}
    raw={'input':input_raw,'controlled_gammas':d.canonical([gamma]),'carrier_objects':d.canonical([host]),
         'terminal_objects':d.canonical([terminal]),'source_objects':d.canonical(objects),
         'instrument_declaration':d.canonical(instrument),'opening_raw':d.canonical({'position':{'quantity':'2','units':'shares'}}),
         'gamma_proof':d.canonical(proof),'clock_mapping':d.canonical(clock)}
    contract={'schema_revision':'issue1394-voice-bridge/1','execution_authorized':False,'facts_sha256':fp(facts),
              'mapping':{'terminal_kinds':['UNIT_TEST_ONLY_TERMINAL'],'carrier_kinds':['UNIT_TEST_ONLY_K'],
                         'terminal':{k:'/'+k for k in v.TERMINAL_FIELDS},'carrier':{k:'/'+k for k in v.CARRIER_FIELDS},
                         'source':{k:'/'+k for k in v.SOURCE_FIELDS}},
              'instrument_declaration':instrument,'clock_mapping':clock,'independent_gamma_proof':proof,'bundle_author_id':'toy-author',
              'opening_source':{'fact_key':'a'*64,'fact_hash':'b'*64,'allocation_id':'toy-allocation',
                                'raw_sha256':d.sha(raw['opening_raw']),'json_pointer':'/position','quantity':'2','units':'shares'}}
    repin_artifacts(contract,raw)
    return contract,raw


def repin_artifacts(contract,raw):
    contract['artifacts']=[{'name':k,'path':'/UNIT_TEST_ONLY/'+k,'sha256':d.sha(value),'bytes':str(len(value))} for k,value in raw.items()]


def assemble(facts,contract,raw):
    wire=d.canonical(contract)
    return v.assemble(facts,wire,d.sha(wire),raw,budget())


class CaptureBridgeTests(unittest.TestCase):
    def test_complete_toy_capture_and_unapproved_candidate(self):
        facts=export(*fixture());contract,raw=bridge_fixture(facts,fixture()[3])
        result=assemble(facts,contract,raw)
        self.assertEqual(result['status'],'UNAUTHENTICATED_CANDIDATES')
        self.assertEqual(result['candidates'][0]['terminal_labels'],['type1','type2'])
        self.assertEqual(result['candidates'][0]['observation_ns'],'130')
        self.assertFalse(result['execution_authorized'])
        self.assertNotIn('verdict',result)
        for execute in (x.execute,v.execute_bind_voice):
            with self.assertRaises(d.Rejected):execute()

    def test_capture_digest_inventory_and_source_identity_rejected(self):
        for bad in ('source','raw','inventory','input'):
            contract,captures,nodes,input_raw=fixture()
            if bad=='source':contract['source']['commit']=None
            elif bad=='raw':captures[0]=x.Capture('snapshot',captures[0].request,captures[0].reply+b' ')
            elif bad=='input':input_raw+=b'changed'
            else:contract['captures']=contract['captures'][:-1]
            with self.subTest(bad=bad),self.assertRaises(d.Rejected):export(contract,captures,nodes,input_raw)

    def mutate_reply(self,case):
        contract,captures,nodes,input_raw=fixture()
        i=0 if case not in ('closure_after','closure_suffix','closure_missing','closure_bytes','closure_cut') else next(i for i,a in enumerate(captures) if a.route=='financial-closure')
        cap=captures[i];reply=d.strict(cap.reply);page=reply['payload'] if cap.route=='snapshot' else reply
        if case=='cut':page['fixed_cut']['structure_cut']='other'
        if case=='offset':page['offset']='1'
        if case=='token':page['next_token']['issued_capture_digest']='a'*64
        if case=='done':page['done']=True
        if case=='omit_row':page['rows'].pop()
        if case=='counts':page['counts']['raw_history']='0'
        if case=='closure_after':page['after']='sc1-'+'a'*64
        if case=='closure_suffix':page['next']='sc1-'+'f'*64
        if case=='closure_missing':page['nodes'].pop()
        if case=='closure_bytes':page['nodes'][0]['body_hex']='ff'
        if case=='closure_cut':page['anchor']['writer_epoch']='99'
        if cap.route=='snapshot':
            reply['payload_hash']=fp(page);req=d.strict(cap.request)
            reply['message_id']='reply-'+fp([req['source_namespace'],req['source_epoch'],req['message_id'],fp(page)])
        captures[i]=x.Capture(cap.route,cap.request,d.canonical(reply));contract['captures']=[c.identity() for c in captures]
        return contract,captures,nodes,input_raw

    def test_repin_does_not_allow_mixed_cut_missing_page_or_fake_suffix(self):
        for case in ('cut','offset','token','done','omit_row','counts','closure_after','closure_suffix','closure_missing','closure_bytes','closure_cut'):
            with self.subTest(case=case),self.assertRaises((d.Rejected,KeyError)):export(*self.mutate_reply(case))
        contract,captures,nodes,input_raw=fixture();captures.pop(1);contract['captures']=[c.identity() for c in captures]
        with self.assertRaises(d.Rejected):export(contract,captures,nodes,input_raw)

    def test_missing_terminal_K_gamma_reports_no_candidates(self):
        for kind in ('UNIT_TEST_ONLY_TERMINAL','UNIT_TEST_ONLY_K','gamma'):
            facts=export(*fixture());contract,raw=bridge_fixture(facts,fixture()[3])
            if kind=='gamma':raw.pop('gamma_proof')
            else:facts['snapshot']['objects']=[o for o in facts['snapshot']['objects'] if o['kind']!=kind]
            contract['facts_sha256']=fp(facts)
            result=assemble(facts,contract,raw)
            self.assertEqual(result['status'],'MISSING_DEPENDENCY');self.assertEqual(result['candidates'],[])

    def test_no_raw_history_Active_or_OperationSequence_fallback(self):
        for kind in ('OperationSequence','Active','raw_history'):
            facts=export(*fixture());facts['snapshot']['objects'][1]['kind']=kind
            contract,raw=bridge_fixture(facts,fixture()[3])
            self.assertEqual(assemble(facts,contract,raw)['status'],'MISSING_DEPENDENCY')

    def test_bridge_rejects_hash_profile_prefix_labels_interval_and_mapping(self):
        for case in ('hash','profile','confirmed','labels','interval','mapping','author','input','opening'):
            facts=export(*fixture());contract,raw=bridge_fixture(facts,fixture()[3]);gamma=d.strict(raw['controlled_gammas'])
            if case=='hash':gamma[0]['terminal_sha256']='0'*64
            elif case=='profile':gamma[0]['profile_id']='foreign'
            elif case=='confirmed':gamma[0]['confirmed_at']='4'
            elif case=='labels':gamma[0]['terminal_labels']=['type1']
            elif case=='interval':gamma[0]['start']='1'
            elif case=='mapping':contract['mapping']['terminal'].pop('anchor')
            elif case=='author':contract['independent_gamma_proof']['author_id']='toy-author';raw['gamma_proof']=d.canonical(contract['independent_gamma_proof'])
            elif case=='input':raw['input']=b'OLD_50070'
            else:contract['opening_source']['quantity']='3'
            raw['controlled_gammas']=d.canonical(gamma);repin_artifacts(contract,raw)
            with self.subTest(case=case),self.assertRaises((d.Rejected,KeyError)):assemble(facts,contract,raw)

    def test_multi_host_refused_and_multiple_candidates_retained(self):
        for mode in ('multi_host','candidates'):
            facts=export(*fixture());objects=facts['snapshot']['objects'];extra=copy.deepcopy(objects[2 if mode=='multi_host' else 1])
            extra['object_id']='extra';extra['fact_key']='extra';objects.append(extra)
            contract,raw=bridge_fixture(facts,fixture()[3])
            if mode=='multi_host':raw['carrier_objects']=d.canonical([objects[2],extra])
            else:
                raw['terminal_objects']=d.canonical([objects[1],extra]);gammas=d.strict(raw['controlled_gammas'])
                second=copy.deepcopy(gammas[0]);second.update(gamma_id='second',terminal_id='extra',terminal_sha256=fp(extra));gammas.append(second)
                raw['controlled_gammas']=d.canonical(gammas);contract['independent_gamma_proof']['gamma_ids']=['gamma','second']
                raw['gamma_proof']=d.canonical(contract['independent_gamma_proof'])
            repin_artifacts(contract,raw)
            if mode=='multi_host':
                with self.assertRaisesRegex(d.Rejected,'host'):assemble(facts,contract,raw)
            else:self.assertEqual(len(assemble(facts,contract,raw)['candidates']),2)

    def test_size_caps_not_widened_and_expired_budget(self):
        facts=export(*fixture());contract,raw=bridge_fixture(facts,fixture()[3])
        contract['artifacts'][0]['bytes']=str(64*x.MIB+1)
        with self.assertRaisesRegex(d.Rejected,'64MiB'):assemble(facts,contract,raw)
        contract,raw=bridge_fixture(facts,fixture()[3])
        for entry in contract['artifacts'][:3]:entry['bytes']=str(64*x.MIB)
        with self.assertRaisesRegex(d.Rejected,'128MiB'):assemble(facts,contract,raw)
        expired=d.ReadBudget(time.monotonic()-1,10000,1000000)
        with self.assertRaises(d.Rejected):x.content_ref({'a':1},expired)

    def test_whole_source_interval_gap_and_K_cycle_refused(self):
        for case in ('gap','missing','cycle'):
            facts=export(*fixture())
            if case=='gap':facts['snapshot']['objects'][0]['start']='1'
            elif case=='missing':facts['snapshot']['objects'][1]['source_ids']=['absent']
            else:facts['snapshot']['objects'][2]['children']=['carrier']
            contract,raw=bridge_fixture(facts,fixture()[3])
            with self.subTest(case=case),self.assertRaises(d.Rejected):assemble(facts,contract,raw)

    def test_resources_null_missing_dimensions_old_caps_and_deadlines_refused(self):
        for case in ('null','Q','Job','retained','old_bytes','old_time','deadline','frame'):
            contract,captures,nodes,input_raw=fixture();r=contract['resources']
            if case=='null':r['session']['max_requests']=None
            if case=='Q':r['q']['limits'].pop('max_atomic_delta_records')
            if case=='Job':r['reader']['job_limits'].pop('max_total_nodes')
            if case=='retained':r['session']['retained_bytes']=str(256*x.MIB+1)
            if case=='old_bytes':r['legacy_reader']['max_reply_bytes']=str(16*x.MIB+1)
            if case=='old_time':r['legacy_reader']['timeout_ms']='10001'
            if case=='deadline':r['deadlines']['stages']['export']=1.
            if case=='frame':r['q']['limits']['max_reply_bytes']='1'
            with self.subTest(case=case),self.assertRaises(d.Rejected):export(contract,captures,nodes,input_raw)
