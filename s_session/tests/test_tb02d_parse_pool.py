"""#1414 同次严格解析池：有限内存真实核验；不运行S/Q或真实历史库。"""
import copy
from contextlib import closing
import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import sys
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import s_readonly_server as server
from test_tb02d_validate_once import material

audit = server._query_integrity()


def fixture():
    """三个真实typed发布代：创建、同串累计batch、撤回；仅内存SQLite。"""
    with closing(sqlite3.connect(':memory:')) as conn:
        return _fixture(conn)


def _fixture(conn):
    conn.executescript(audit.LEGACY_SCHEMA + audit.tb02.SCHEMA)
    def insert(table, row):
        names = list(row)
        conn.execute('INSERT INTO '+table+' ('+','.join(names)+') VALUES ('+','.join('?' for _ in names)+')', tuple(row.values()))
    scope = dict(structure='CompleteCut', economic='not_started')
    profile = json.loads((ROOT/'profiles/ohlc_integer_tb02a_v1.json').read_text())
    phash = hashlib.sha256(audit.canonical(profile)).hexdigest()
    meta = dict(session_id='pool-memory', catalog_revision='pool-catalog', rule_revision='s2-axis-quantifiers',
                writer_epoch='1', generation='3', structure_cut='cut-3', index_frontier='pending', last_advance_frontier='0',
                scope=audit.canonical(scope).decode(), profile_id=audit.tb02.PROFILE, profile_hash=phash,
                profile_definition=audit.canonical(profile).decode(), input_profile=audit.tb02.PROFILE)
    raw = dict(identity_key='ns|1|TEST|e0', revision=1, input_revision='1', payload_hash='', receipt_id='', seq=0,
               source_namespace='ns', source_epoch='1', instrument='TEST', event_id='e0', received_at='t', raw_text='memory',
               price='1', ts='0', volume='1', source_coord='0', supersedes_revision=None,
               schema_revision=audit.tb02.RAW_SCHEMA, open='1', high='2', low='0', close='1')
    raw['payload_hash'] = hashlib.sha256(audit.canonical(audit._raw_event_content(raw))).hexdigest()
    raw['receipt_id'] = 'rcpt-'+hashlib.sha256((raw['identity_key']+'|'+raw['payload_hash']).encode()).hexdigest()[:16]
    insert('raw_events',{k:v for k,v in raw.items() if k not in ('schema_revision','open','high','low','close')})
    insert('raw_ohlc',{k:raw[k] for k in ('identity_key','revision','schema_revision','open','high','low','close')})
    wire = audit._raw_wire(raw)
    row, _, _ = material()
    refs = [{k:wire[k] for k in audit.tb02.REF_FIELDS}]
    row['input_refs_json'] = audit.canonical(refs).decode()
    identity = dict(kind=row['kind'], fact_key=row['fact_key'], payload=json.loads(row['payload_json']),input_refs=refs,
                    profile_id=meta['profile_id'],rule_revision=meta['rule_revision'])
    row['object_id'] = 'obj-'+hashlib.sha256(audit.canonical(identity)).hexdigest()
    row.update(withdrawn_generation=3, withdrawal_reason='fact_removed')
    incoming={k:wire[k] for k in ('source_coord','open','high','low','close')}
    payload=dict(action='seed',incoming=incoming,acc_before=None,acc_after=incoming,direction=None,direction_evidence=None,comparisons=[],endpoint_order=[],contains=None,high_sources=['0'],low_sources=['0'],waiting_reasons=[])
    extra=dict(row,kind='CC-004.inclusion_step',fact_key=audit.canonical(['CC-004.inclusion_step','0']).decode(),payload_json=audit.canonical(payload).decode())
    identity=dict(kind=extra['kind'],fact_key=extra['fact_key'],payload=payload,input_refs=refs,profile_id=meta['profile_id'],rule_revision=meta['rule_revision'])
    extra['object_id']='obj-'+hashlib.sha256(audit.canonical(identity)).hexdigest()
    facts=[row,extra]
    first_batch = None
    for generation in (1,2,3):
        active = generation < 3
        withdrawals = [dict(object_id=f['object_id'],fact_key=f['fact_key'],reason='fact_removed',superseded_by=None) for f in facts]
        axes = {}
        for axis in audit.tb02.LEGACY_AXES:
            evidence = dict(scope='TB-02-A',object_ids=[f['object_id'] for f in facts if active and f['kind'].startswith(axis+'.')],waiting_reasons=[])
            if axis=='CC-056': evidence.update(raw_revisions=[],withdrawals=[] if active else withdrawals,replaces=[])
            axes[axis] = dict(impl_status='implemented',proof_status='not_proved',run_status='run' if axis=='CC-056' and not active else 'not_run',evidence=evidence)
        internal = [{k:v for k,v in f.items() if k not in ('batch_id','withdrawn_generation','withdrawal_reason','superseded_by')} for f in facts]
        batch = dict(generation=generation,structure_cut=f'cut-{generation}',input_frontier=0,catalog_revision=meta['catalog_revision'],
                     rule_revision=meta['rule_revision'],profile_id=meta['profile_id'],profile_hash=phash,scope=scope,
                     catalog_axes=axes,objects=internal if active else [],witnesses=[],relations=[],observations=[],raw_events=[raw])
        content = audit.canonical(batch)
        batch_id = 'batch-'+hashlib.sha256(content).hexdigest()
        first_batch = first_batch or batch_id
        insert('batches',dict(batch_id=batch_id,canonical_bytes=content,byte_len=len(content)))
        upserts = [audit.tb02.project_fact(dict(f,batch_id=first_batch,withdrawn_generation=None,withdrawal_reason=None),audit.parse_json,meta=meta,raw=[wire]) for f in facts]
        evidence = dict(scope=scope,profile_id=meta['profile_id'],profile_hash=phash,rule_revision=meta['rule_revision'],
                        batch_id=batch_id,structure_cut=f'cut-{generation}',axes=axes)
        for key in ('classified_objects','windows_total','merged_bars','effective_source_positions','withdrawals','replaces','insufficient_knowledge','domain_not_satisfied'):
            evidence[key]='0'
        seq_range = dict(**{'from':'0' if generation==1 else '1','to':'0'})
        header = dict(session_id=meta['session_id'],catalog_revision=meta['catalog_revision'],base_cut=f'cut-{generation-1}',next_cut=f'cut-{generation}',
                      index_frontier=batch_id,catalog_run_status='run' if active else 'not_run',generation=str(generation),input_frontier='0',seq_range=seq_range,catalog_evidence=evidence)
        delta = dict(header,upserts=upserts if active else [],withdrawals=[] if active else withdrawals,replaces=[],witnesses=[],relations=[],observations=[],raw_history_added=[wire] if generation==1 else [])
        insert('structure_deltas',dict(generation=generation,session_id=meta['session_id'],catalog_revision=meta['catalog_revision'],base_cut=header['base_cut'],next_cut=header['next_cut'],index_frontier=batch_id,input_frontier=0,catalog_run_status=header['catalog_run_status'],seq_range_json=audit.canonical(seq_range).decode(),catalog_evidence_json=audit.canonical(evidence).decode(),delta_json=audit.canonical(delta).decode()))
        meta['index_frontier']=batch_id
    for fact in facts:
        fact['batch_id']=first_batch
        insert('structure_facts',fact)
    for axis,item in axes.items():
        insert('catalog',dict(catalog_id=axis,kind='memory',title=axis,domain='memory',branches_json='[]',impl_status=item['impl_status'],proof_status=item['proof_status'],run_status=item['run_status'],evidence_json=audit.canonical(item['evidence']).decode()))
    for key,value in meta.items():insert('meta',dict(key=key,value=value))
    conn.commit()
    return audit.capture(conn)


def outcome(call):
    try:return ('ok',call())
    except Exception as exc:return (type(exc).__name__,str(exc))


class ParsePoolTests(unittest.TestCase):
    def test_complete_verify_digest_public_projection_and_warm(self):
        image=fixture(); pools=[]
        original=audit._CanonicalParsePool
        with patch.object(audit,'_CanonicalParsePool',side_effect=lambda:(pools.append(original()) or pools[-1])):
            with patch.object(audit,'_verify_control',wraps=audit._verify_control) as control, patch.object(audit,'capture_digest',wraps=audit.capture_digest) as digest:
                proof=audit.verify(image,vars(server),memo={})
                self.assertEqual(control.call_count,1);self.assertEqual(digest.call_count,1)
        with patch.object(audit._CanonicalParsePool,'_layout_matches',return_value=False):
            without=audit.verify(image,vars(server),memo={})
        self.assertEqual(proof,without)
        self.assertEqual(proof['capture_digest'],audit.capture_digest(image))
        # #1497：batch 完整原串已匹配本次严格索引，无需再取三列解析池。
        self.assertEqual((pools[0].seeded,pools[0].hits,pools[0].strict_calls),(4,2,4))
        self.assertEqual(pools[0]._mode,'closed');self.assertEqual(pools[0]._entries,{})
        for gen in (0,1,2,3,None):
            first=audit.project_state(proof,gen,vars(server)); expected=audit.canonical(first)
            self.assertEqual(expected,audit.canonical(audit.project_state(without,gen,vars(server))))
            first['snapshot']['raw_history'].append({'poison':True})
            for obj in first['snapshot']['objects']+first['snapshot']['withdrawn_objects']:
                obj['payload']['waiting_reasons'].append('poison')
            self.assertEqual(expected,audit.canonical(audit.project_state(proof,gen,vars(server))))
        for gen in range(3):
            first=audit.project_delta(proof,gen); expected=audit.canonical(first)
            self.assertEqual(expected,audit.canonical(audit.project_delta(without,gen)))
            first['delta']['upserts'].append({'poison':True})
            self.assertEqual(expected,audit.canonical(audit.project_delta(proof,gen)))
        warm=audit.verify(image,vars(server),memo=proof['_audit_memo'])
        self.assertEqual(warm['_audit_memo']['hits'],3)
        for key in proof:
            if key!='_audit_memo':self.assertEqual(proof[key],warm[key])
        self.assertLess(audit.object_bytes(proof),audit.object_bytes(without))
        wires=[value for _,value in proof['wires']['objects']]
        self.assertIs(wires[0]['input_refs'],wires[1]['input_refs'])
        state=audit.project_state(proof,1,vars(server));objects=state['snapshot']['objects']
        self.assertIsNot(objects[0]['input_refs'],objects[1]['input_refs'])
        objects[0]['input_refs'][0]['receipt_id']='poison'
        self.assertNotEqual(objects[1]['input_refs'][0]['receipt_id'],'poison')
        pending=[proof];seen=set()
        while pending:
            item=pending.pop()
            if id(item) in seen:continue
            seen.add(id(item));self.assertNotIsInstance(item,audit._CanonicalParsePool);self.assertFalse(callable(item))
            if type(item) is dict:pending.extend(item.keys());pending.extend(item.values())
            elif type(item) in (tuple,list,set,frozenset):pending.extend(item)


    def test_key_boundaries_bad_values_and_no_failed_cache(self):
        pool=audit._CanonicalParsePool()
        source='["文字"]'; a=pool.parse(source,list);b=pool.parse(source.encode().decode(),list)
        self.assertIs(a,b);self.assertEqual(pool.hits,1)
        class Text(str):pass
        for raw,expected in [(source,dict),(source.encode(),list),(Text(source),list),([],list),('{"a":1,"a":2}',dict),('{"a":NaN}',dict),('{"a":"\\ud800"}',dict),('{ }',dict),('[1e999]',list)]:
            before=len(pool._entries)
            self.assertEqual(outcome(lambda:pool.parse(raw,expected)),outcome(lambda:audit._parse_canonical_json(raw,expected)))
            self.assertEqual(len(pool._entries),before)
        self.assertIsNot(pool.parse('["changed"]',list),a)
        pool.seal();before=len(pool._entries)
        self.assertEqual(pool.parse('["new-batch"]',list),['new-batch']);self.assertEqual(len(pool._entries),before)
        pool.close();self.assertEqual(pool._entries,{})

    def test_capacity_exact_boundary_preflight_disable_and_layout(self):
        sample=audit._CanonicalParsePool();first=sample._fixed_bytes()+224+80
        for limit,seeded in ((first,1),(first-1,0)):
            pool=audit._CanonicalParsePool(_limit=limit); self.assertEqual(pool.parse('["x"]',list),['x'])
            self.assertEqual(pool.seeded,seeded)
            if not seeded:self.assertEqual(pool.disabled_reason,'capacity')
        pool=audit._CanonicalParsePool()
        for i in range(10000):pool.parse(json.dumps([str(i)],separators=(',',':')),list)
        self.assertEqual(pool.seeded,9414);self.assertEqual(pool.disabled_reason,'capacity')
        self.assertLessEqual(pool.peak_metadata,1048576);self.assertEqual(pool._entries,{})
        with patch.object(audit._CanonicalParsePool,'_layout_matches',return_value=False):
            disabled=audit._CanonicalParsePool();self.assertEqual(disabled.parse('[]',list),[])
            self.assertEqual(disabled.disabled_reason,'layout');self.assertEqual(disabled._entries,{})

    def test_control_before_and_all_exception_exit_cleanup(self):
        image=fixture();original=audit._CanonicalParsePool;captured=[]
        def factory():
            pool=original();captured.append(pool);return pool
        real_control=audit._verify_control
        def control(*args,**kwargs):
            self.assertEqual(captured[-1]._entries,{});self.assertEqual(captured[-1]._mode,'closed')
            return real_control(*args,**kwargs)
        with patch.object(audit,'_CanonicalParsePool',side_effect=factory),patch.object(audit,'_verify_control',side_effect=control):
            audit.verify(image,vars(server))
            with self.assertRaises(audit.QueryBudgetExceeded):audit.verify(image,vars(server),deadline=0)
            with patch.object(audit.tb02,'_validate_fact',side_effect=ValueError('after strict')):
                with self.assertRaisesRegex(ValueError,'after strict'):audit.verify(image,vars(server))
            with patch.object(audit,'capture_digest',side_effect=ValueError('digest sentinel')):
                with self.assertRaisesRegex(ValueError,'digest sentinel'):audit.verify(image,vars(server))
        for pool in captured:self.assertEqual((pool._mode,pool._entries),('closed',{}))
        self.assertEqual(len({id(p) for p in captured}),len(captured))

    def test_complete_bad_source_and_generation_first_errors_with_and_without_pool(self):
        def changed(table, column, change, index=0):
            image=fixture()
            names=[c[1] for c in image['schema'][table][0]]
            rows=list(image['tables'][table]);row=list(rows[index]);slot=names.index(column)
            row[slot]=change(row[slot]);rows[index]=tuple(row);image['tables'][table]=tuple(rows)
            return image
        cases=[
            changed('structure_facts','payload_json',lambda _: '{"x":1,"x":2}'),
            changed('structure_facts','input_refs_json',lambda text: text.replace('rcpt-','bad-')),
            changed('structure_facts','first_known_cut',lambda _: 'cut-999'),
            changed('structure_facts','withdrawn_generation',lambda _: 1),
            changed('structure_deltas','base_cut',lambda _: 'cut-999'),
            changed('structure_deltas','delta_json',lambda text: text.replace('"input_frontier":"0"','"input_frontier":"1"')),
            changed('catalog','evidence_json',lambda _: '{}'),
            changed('raw_events','payload_hash',lambda _: '0'*64),
        ]
        # 只变batch源，同时修复其外层hash/length/Delta源指针，使错误到达完整fact门。
        image=fixture();tables=image['tables']
        def records(table):
            names=[c[1] for c in image['schema'][table][0]]
            return names,[dict(zip(names,r)) for r in tables[table]]
        bnames,batches=records('batches'); dnames,deltas=records('structure_deltas')
        target=next(b for b in batches if json.loads(b['canonical_bytes'])['generation']==2)
        batch=json.loads(target['canonical_bytes']);batch['objects'][0]['payload_json']='{}'
        target['canonical_bytes']=audit.canonical(batch);target['byte_len']=len(target['canonical_bytes'])
        target['batch_id']='batch-'+hashlib.sha256(target['canonical_bytes']).hexdigest()
        d=deltas[1];d['index_frontier']=target['batch_id']
        evidence=json.loads(d['catalog_evidence_json']);evidence['batch_id']=target['batch_id'];d['catalog_evidence_json']=audit.canonical(evidence).decode()
        delta=json.loads(d['delta_json']);delta['index_frontier']=target['batch_id'];delta['catalog_evidence']=evidence;d['delta_json']=audit.canonical(delta).decode()
        tables['batches']=tuple(tuple(b[k] for k in bnames) for b in batches)
        tables['structure_deltas']=tuple(tuple(d[k] for k in dnames) for d in deltas)
        cases.append(image)
        for index,image in enumerate(cases):
            with self.subTest(index=index):
                enabled=outcome(lambda:audit.verify(image,vars(server),memo={}))
                with patch.object(audit._CanonicalParsePool,'_layout_matches',return_value=False):
                    disabled=outcome(lambda:audit.verify(image,vars(server),memo={}))
                self.assertNotEqual(enabled[0],'ok');self.assertEqual(enabled,disabled)

    def test_current_fact_context_rechecked_on_hit_and_alias_precondition(self):
        row,meta,raw=material();pool=audit._CanonicalParsePool()
        first=audit._project_canonical_fact(row,meta,raw,_parse_pool=pool);pool.seal()
        with patch.object(audit.tb02,'_validate_fact',wraps=audit.tb02._validate_fact) as validate:
            second=audit._project_canonical_fact(row,meta,raw,_parse_pool=pool)
            self.assertEqual(validate.call_count,1)
        self.assertIsNot(first,second);self.assertIs(first['payload'],second['payload'])
        bad=copy.deepcopy(raw);bad[0]['receipt_id']='bad'
        self.assertEqual(outcome(lambda:audit._project_canonical_fact(row,meta,bad,_parse_pool=pool)),outcome(lambda:audit._project_canonical_fact(row,meta,bad)))
        # 故意违反私有只读前提：id/完整源键并不检测嵌套mutation。
        first['payload']['waiting_reasons'].append('mutation')
        self.assertEqual(second['payload']['waiting_reasons'],['mutation'])
        pool.close()
        self.assertEqual(audit._project_canonical_fact(row,meta,raw)['payload']['waiting_reasons'],[])


if __name__=='__main__':unittest.main()
