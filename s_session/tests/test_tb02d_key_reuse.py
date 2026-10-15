"""#1414 完整记录键复用：小内存库/真实verify调用，不执行真实DB或S/Q。"""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

from test_tb02d_parse_pool import audit, fixture, outcome, server
ROOT=Path(__file__).resolve().parents[1]
# #1414 自足测试对照臂：只关闭测试模块内的键复用，生产无开关。
# 精确历史candidate对拍由仓外冻结独审证据保留；这里是当前实现的原编码对照。
spec=importlib.util.spec_from_file_location('key_reuse_original_encoding_control',ROOT/'s_query_integrity.py')
baseline=importlib.util.module_from_spec(spec);spec.loader.exec_module(baseline)


def _original_record_set(records, h, name, *, _verified_previous=None):
    # 沿用键复用前的完整原编码/重复成员算法，私有上下文只在测试对照中忽略。
    result = set()
    for record in records:
        if type(record) is not dict:
            raise ValueError(name + " 成员不是对象")
        key = baseline.canonical(h["_legacy_payload_projection"](record))
        if key in result:
            raise ValueError(name + " 含重复成员")
        result.add(key)
    return result


baseline._record_set = _original_record_set
EVIDENCE={}


def rows(image,table):
    names=[c[1] for c in image['schema'][table][0]]
    return names,[dict(zip(names,r)) for r in image['tables'][table]]


def put(image,table,names,values):
    image['tables'][table]=tuple(tuple(row[k] for k in names) for row in values)


def delayed_birth_fixture():
    """原两fact三代图像中第二fact延至gen2出生；gen3均撤回。"""
    image=fixture();fn,facts=rows(image,'structure_facts');bn,batches=rows(image,'batches');dn,deltas=rows(image,'structure_deltas')
    births={f['object_id']:(2 if f['kind']=='CC-004.inclusion_step' else 1) for f in facts}
    for fact in facts:
        first=births[fact['object_id']]
        fact.update(first_known_generation=first,published_generation=first,first_known_cut=f'cut-{first}')
    bygen={json.loads(b['canonical_bytes'])['generation']:(b,json.loads(b['canonical_bytes'])) for b in batches}
    ids={}
    for gen in (1,2,3):
        stored,batch=bygen[gen]
        batch['objects']=[obj for obj in batch['objects'] if births[obj['object_id']]<=gen]
        for obj in batch['objects']:
            first=births[obj['object_id']];obj.update(first_known_generation=first,published_generation=first,first_known_cut=f'cut-{first}')
        present={obj['object_id'] for obj in batch['objects']}
        for axis in batch['catalog_axes'].values():axis['evidence']['object_ids']=[oid for oid in axis['evidence']['object_ids'] if oid in present]
        content=audit.canonical(batch);bid='batch-'+hashlib.sha256(content).hexdigest();ids[gen]=bid
        stored.update(batch_id=bid,canonical_bytes=content,byte_len=len(content))
    for fact in facts:fact['batch_id']=ids[births[fact['object_id']]]
    for row in deltas:
        gen=row['generation'];delta=json.loads(row['delta_json']);evidence=json.loads(row['catalog_evidence_json']);evidence.update(batch_id=ids[gen],axes=bygen[gen][1]['catalog_axes'])
        delta['upserts']=[obj for obj in delta['upserts'] if births[obj['object_id']]<=gen]
        for obj in delta['upserts']:
            first=births[obj['object_id']];obj.update(first_known_generation=str(first),published_generation=str(first),first_known_cut=f'cut-{first}',batch_id=ids[first])
        delta.update(index_frontier=ids[gen],catalog_evidence=evidence)
        row.update(index_frontier=ids[gen],catalog_evidence_json=audit.canonical(evidence).decode(),delta_json=audit.canonical(delta).decode())
    mn,meta=rows(image,'meta')
    for item in meta:
        if item['key']=='index_frontier':item['value']=ids[3]
    put(image,'structure_facts',fn,facts);put(image,'batches',bn,batches);put(image,'structure_deltas',dn,deltas);put(image,'meta',mn,meta)
    return image


def mutate_delta(image,change,gen=2):
    image=copy.deepcopy(image);names,data=rows(image,'structure_deltas');target=data[gen-1];delta=json.loads(target['delta_json']);change(delta)
    target['delta_json']=audit.canonical(delta).decode();put(image,'structure_deltas',names,data);return image


def bad_batch(image):
    """只污染batch typed字段，修复其外层源hash/length/证据指针以触真实validator。"""
    image=copy.deepcopy(image);bn,batches=rows(image,'batches');dn,deltas=rows(image,'structure_deltas')
    target=next(b for b in batches if json.loads(b['canonical_bytes'])['generation']==2)
    batch=json.loads(target['canonical_bytes']);batch['objects'][0]['fact_key']='bad-key'
    content=audit.canonical(batch);bid='batch-'+hashlib.sha256(content).hexdigest();target.update(batch_id=bid,canonical_bytes=content,byte_len=len(content))
    row=deltas[1];delta=json.loads(row['delta_json']);evidence=json.loads(row['catalog_evidence_json']);evidence['batch_id']=bid
    delta.update(index_frontier=bid,catalog_evidence=evidence);row.update(index_frontier=bid,catalog_evidence_json=audit.canonical(evidence).decode(),delta_json=audit.canonical(delta).decode())
    put(image,'batches',bn,batches);put(image,'structure_deltas',dn,deltas);return image


class KeyReuseTests(unittest.TestCase):
    def compare_proofs(self,image):
        old=baseline.verify(image,vars(server),memo={});new=audit.verify(image,vars(server),memo={})
        self.assertEqual(new,old)
        self.assertEqual(new['capture_digest'],audit.capture_digest(image))
        self.assertEqual(audit.object_bytes(new),baseline.object_bytes(old))
        self.assertEqual(audit.object_bytes((new,new['_audit_memo'])),baseline.object_bytes((old,old['_audit_memo'])))
        for gen in (0,1,2,3,None):self.assertEqual(audit.canonical(audit.project_state(new,gen,vars(server))),baseline.canonical(baseline.project_state(old,gen,vars(server))))
        for index in range(3):self.assertEqual(audit.canonical(audit.project_delta(new,index)),baseline.canonical(baseline.project_delta(old,index)))
        return old,new

    def test_real_two_calls_validator_order_bytes_identity_complete_proof(self):
        image=fixture();events=[];details=[];stage=[0];contexts=[];real_record=audit._record_set;real_validate=audit.tb02._validate_fact;delta_c=server._tb02_contract();delta_validate=delta_c.validate_fact
        real_sealed=audit._sealed_generation
        def sealed(*args,**kwargs):
            stage[0]=len(events);self.assertIsNotNone(kwargs.get('_active_keys'))
            contexts.append(kwargs['_verified_facts'])
            return real_sealed(*args,**kwargs)
        def batch_validate(*args,**kwargs):
            result=real_validate(*args,**kwargs)
            if len(args)>1 and args[1] is not None:events.append(('batch-full',args[0]['object_id']))
            return result
        def delta_validator(*args,**kwargs):
            result=delta_validate(*args,**kwargs);events.append(('delta-full',args[0]['object_id']));return result
        def record(records,h,name,**kwargs):
            context=kwargs.get('_verified_previous');available=list(events[stage[0]:]);result=real_record(records,h,name,**kwargs)
            if context is not None:
                active,keys=context;hits=0
                for rec in records:
                    # #1497：本次完整索引已证明的逐字相同事实不重复调用 validator。
                    # 键复用仍须有完整材料证明或该代原 validator 成功证据。
                    if ('delta-full' if name=='upserts' else 'batch-full',rec['object_id']) not in available:
                        self.assertEqual(audit.canonical(rec), contexts[-1].keys[rec['object_id']])
                    if rec['object_id'] in active and rec==active[rec['object_id']]:
                        hits+=1;self.assertTrue(any(k is keys[rec['object_id']] for k in result))
                details.append({'name':name,'records':len(records),'prior':len(active),'hits':hits})
            else:self.assertNotEqual(name,'upserts/batch')
            return result
        with patch.object(audit,'_sealed_generation',side_effect=sealed),patch.object(audit.tb02,'_validate_fact',side_effect=batch_validate),patch.object(delta_c,'validate_fact',side_effect=delta_validator),patch.object(audit,'_record_set',side_effect=record),patch.object(audit,'_verify_control',wraps=audit._verify_control) as control,patch.object(audit,'capture_digest',wraps=audit.capture_digest) as digest:
            proof=audit.verify(image,vars(server),memo={})
        self.assertEqual(control.call_count,1);self.assertEqual(digest.call_count,1)
        self.assertEqual([(x['name'],x['records'],x['prior'],x['hits']) for x in details],[('upserts',2,0,0),('upserts/batch',2,0,0),('upserts',2,2,2),('upserts/batch',2,2,2),('upserts',0,2,0),('upserts/batch',0,2,0)])
        self.assertEqual(sum(e[0]=='batch-full' for e in events),2);self.assertEqual(sum(e[0]=='delta-full' for e in events),0)
        old,new=self.compare_proofs(image);self.assertEqual(proof,new)
        # Existing intern pool shares a stable key across both generations.
        for key in new['_audit_memo']['entries'][0]['sets']['upserts']:
            self.assertTrue(any(key is later for later in new['_audit_memo']['entries'][1]['sets']['upserts']))
        EVIDENCE['complete']={'calls':details,'validators':{'batch':2,'delta':0},'proof_bytes':audit.object_bytes(new),'baseline_proof_bytes':baseline.object_bytes(old),'digest':new['capture_digest'],'protocol':new['protocol']}

    def test_delayed_birth_and_withdrawal_have_original_proof(self):
        image=delayed_birth_fixture();details=[];original=audit._record_set
        def record(records,h,name,**kwargs):
            ctx=kwargs.get('_verified_previous')
            if ctx is not None:details.append((name,len(records),len(ctx[0]),sum(rec.get('object_id') in ctx[0] and rec==ctx[0][rec['object_id']] for rec in records)))
            return original(records,h,name,**kwargs)
        with patch.object(audit,'_record_set',side_effect=record):proof=audit.verify(image,vars(server),memo={})
        self.assertEqual(details,[('upserts',1,0,0),('upserts/batch',1,0,0),('upserts',2,1,1),('upserts/batch',2,1,1),('upserts',0,2,0),('upserts/batch',0,2,0)])
        old,new=self.compare_proofs(image);self.assertEqual(proof,new)
        self.assertEqual(len(audit.project_state(new,1,vars(server))['snapshot']['objects']),1)
        self.assertEqual(len(audit.project_state(new,2,vars(server))['snapshot']['objects']),2)
        self.assertEqual(len(audit.project_state(new,3,vars(server))['snapshot']['withdrawn_objects']),2)
        EVIDENCE['delayed_birth']={'calls':details,'proof_bytes':audit.object_bytes(new)}

    def test_warm_hits_and_current_source_changes_revalidate_tail(self):
        image=fixture();old,new=self.compare_proofs(image)
        for mode in ('same','delta-whitespace','row-whitespace'):
            changed=copy.deepcopy(image)
            if mode!='same':
                names,data=rows(changed,'structure_deltas');column='delta_json' if mode=='delta-whitespace' else 'seq_range_json'
                data[1][column]=' '+data[1][column];put(changed,'structure_deltas',names,data)
            a=baseline.verify(changed,vars(server),memo=old['_audit_memo']);b=audit.verify(changed,vars(server),memo=new['_audit_memo']);self.assertEqual(a,b)
            self.assertEqual(audit.object_bytes(b),baseline.object_bytes(a));self.assertEqual(b['_audit_memo']['hits'],3 if mode=='same' else 1)
            self.assertEqual(b['capture_digest'],audit.capture_digest(changed))
        # Changed batch cannot hide behind the prior memo: all identity fields are checked.
        bad=bad_batch(image)
        self.assertEqual(outcome(lambda:baseline.verify(bad,vars(server),memo=old['_audit_memo'])),outcome(lambda:audit.verify(bad,vars(server),memo=new['_audit_memo'])))
        self.assertNotEqual(outcome(lambda:audit.verify(bad,vars(server),memo=new['_audit_memo']))[0],'ok')
        EVIDENCE['warm']={'same_hits':3,'delta_source_changed_hits':1,'row_columns_changed_hits':1,'bad_batch_rejected':True}

    def test_semantic_first_errors_noncanonical_and_same_id_changed_records(self):
        image=fixture();cases=[]
        cases.append(mutate_delta(image,lambda d:d['upserts'][0]['payload'].update(input_frontier=0)))
        cases.append(mutate_delta(image,lambda d:d['upserts'].append(copy.deepcopy(d['upserts'][0]))))
        # These are individually valid wire records but differ from the previous complete record.
        cases.append(mutate_delta(image,lambda d:d['upserts'][0].update(batch_id='different')))
        cases.append(mutate_delta(image,lambda d:d['upserts'][0]['input_refs'][0].update(receipt_id='different')))
        cases.append(bad_batch(image))
        noncanonical=copy.deepcopy(image);names,data=rows(noncanonical,'structure_facts');data[0]['payload_json']=' '+data[0]['payload_json'];put(noncanonical,'structure_facts',names,data);cases.append(noncanonical)
        results=[]
        for bad in cases:
            a=outcome(lambda:baseline.verify(bad,vars(server),memo={}));b=outcome(lambda:audit.verify(bad,vars(server),memo={}));self.assertEqual(a,b);self.assertNotEqual(a[0],'ok');results.append(a)
        EVIDENCE['semantic_errors']=results

    def test_eq_recursion_miss_is_entered_and_original_encoder_not_swallowed(self):
        # Two exact builtin cyclic lists force native equality RecursionError.
        # They cannot arise from JSON: this is a catch-mechanism test, not a legal fact.
        # No global recursionlimit or arbitrary user-defined __eq__ is involved.
        a=[];b=[];a.append(a);b.append(b)
        left={'kind':audit.tb02.KINDS[0],'object_id':'deep','payload':a};right={'kind':audit.tb02.KINDS[0],'object_id':'deep','payload':b}
        ctx=({'deep':left},{'deep':b'old'});events=[]
        code=audit._record_set.__code__
        def trace(frame,event,arg):
            if frame.f_code is code and event=='exception' and arg[0] is RecursionError:events.append(frame.f_lineno)
            return trace
        oldtrace=sys.gettrace()
        try:
            sys.settrace(trace)
            with patch.object(audit,'canonical',return_value=b'fallback') as encode:
                result=audit._record_set([right],{'_legacy_payload_projection':lambda x:x},'deep',_verified_previous=ctx)
                self.assertEqual(result,{b'fallback'});encode.assert_called_once_with(right)
        finally:sys.settrace(oldtrace)
        self.assertTrue(events)
        for error in (ValueError('encoder semantic'),UnicodeError('encoder utf8'),RecursionError('encoder recursion')):
            with patch.object(audit,'canonical',side_effect=error):
                with self.assertRaises(type(error)) as caught:audit._record_set([right],{'_legacy_payload_projection':lambda x:x},'deep',_verified_previous=ctx)
                self.assertIs(caught.exception,error)
        # Real original deep encoder also still raises after the caught comparison miss.
        with self.assertRaises(RecursionError):audit._record_set([right],vars(server),'deep',_verified_previous=ctx)
        EVIDENCE['recursion']={'comparison_except_entered':True,'trace_exception_lines':events,'cyclic_builtin_lists':True,'original_encoder_exceptions_propagated':True,'scope':'internal mechanism only; cyclic input cannot come from JSON'}

    def test_legacy_domains_default_path_and_deadline_sequence_unchanged(self):
        rec={'kind':'legacy','object_id':'x','detail':{'v':1},'raw_bars':[{'supersedes_revision':2}]};prior=dict(rec,detail={'v':True});key=audit.canonical(prior)
        self.assertEqual(audit._record_set([rec],vars(server),'legacy',_verified_previous=({'x':prior},{'x':key})),baseline._record_set([rec],vars(server),'legacy'))
        image=fixture();counts=[]
        for module in (baseline,audit):
            with patch.object(module,'_deadline',wraps=module._deadline) as check:module.verify(image,vars(server),memo={});counts.append(check.call_count)
        self.assertEqual(counts[0],counts[1]);self.assertGreater(counts[0],0)
        self.assertEqual(outcome(lambda:baseline.verify(image,vars(server),deadline=0))[1],outcome(lambda:audit.verify(image,vars(server),deadline=0))[1])
        EVIDENCE['deadline_calls']=counts


if __name__=='__main__':unittest.main()
