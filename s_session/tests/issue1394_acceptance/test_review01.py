"""独审01固定反例及成对成功锁。全部fake/toy，不是产品验收。"""
import copy
from dataclasses import asdict, replace
import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

import driver as d
import evidence as e
import financial_oracle as f
import journal as j
import scheduler as s
import supervisor as sup
import test_scheduler as st
import test_financial_oracle as financial_tests
from test_supervisor import Probe


class Review01Tests(unittest.TestCase):
    def test_F1_requirements_and_steps_both_removed_fixed_probe(self):
        p = st.fixture()
        for remove_proofs in (False, True):
            r = replace(p.requirements, scenarios=p.requirements.scenarios[:1], negative_domains=(), comparisons=())
            steps = tuple(replace(x, requests=tuple(q for q in x.requests if not q.case),
                          expected_proofs=tuple(k for k in x.expected_proofs
                                               if not remove_proofs or not k.startswith('compare/')))
                          for x in p.steps if x.scenario == r.scenarios[0].name)
            result = s.Scheduler(replace(p, requirements=r, steps=steps), st.FakeAdapter(), lambda: 1.).run()
            self.assertEqual(result.outcome, 'FAILED')
            self.assertFalse(result.attempted)
            self.assertIn('Requirements', result.error)
        result = s.Scheduler(p, st.FakeAdapter(), lambda: 1.).run()
        self.assertEqual(len(result.stage_receipts), 510)
        self.assertEqual(result.outcome, 'SIMULATED_COMPLETE_NOT_PRODUCT_ACCEPTANCE')
        for sha in ('f'*64, '0'*64):
            with self.assertRaises(s.Rejected):
                s.validate_plan(replace(p, requirements=replace(p.requirements, preparation_sha=sha)))

    def test_F2_second_owner_original_probe_and_ordered_full_set(self):
        p = st.fixture(); steps = list(p.steps)
        i = next(i for i,x in enumerate(steps) if x.kind == 'resend')
        req = steps[i].requests
        bad = replace(req[1], binding=replace(req[1].binding, epoch=99,
                      request_sha='f'*64, receipt_key='UNRELATED_NEW_REQUEST'))
        variants = ((req[0],bad), req[:1], req[::-1], req+req[:1],
                    (req[0],replace(req[1],ordinal=1)),
                    (req[0],replace(req[1],binding=replace(req[1].binding,epoch=2))),
                    (req[0],replace(req[1],binding=replace(req[1].binding,request_sha='e'*64))))
        for requests in variants:
            altered=steps[:];altered[i]=replace(steps[i],requests=requests)
            result=s.Scheduler(replace(p,steps=tuple(altered)),st.FakeAdapter(),lambda:1.).run()
            self.assertEqual(result.outcome,'FAILED')
            self.assertFalse(result.attempted)
        self.assertEqual([r.binding.epoch for r in req],[2,1])
        s.validate_plan(p)

    def committed(self):
        data,args,grants=financial_tests.FinancialTests().reserved_fixture()
        final=copy.deepcopy(args['histories']['B:beta'][-1]);final['seq']=2
        cut=f.strict(args['e_original_raw'])['receipts']['Reserve/slot']['cut']
        final['image']['finance']['decisions'][0].update(state='Committed',
            grant={'original':grants[2],'e_cut':copy.deepcopy(cut)},
            local_commit_receipt={'e_cut':copy.deepcopy(cut),'commit_ns':'11'})
        args['histories']['B:beta'].append(final)
        args['operations'].append({'domain':'B:beta','op':'CommitDecision','id':'b1',
            'receipt_key':'commit/b1','outcome':'LocalCommitReceipt','seq':2,'ordinal':0})
        return data,args,final['image']['finance']['decisions'][0]

    def audit(self,data,args):
        raw=d.canonical(data)
        return f.audit(raw,d.sha(raw),**args,budget=d.ReadBudget(time.monotonic()+5,1000,1000000))

    def test_F3_self_consistent_fake_cut_fixed_probe_and_legal_E(self):
        data,args,decision=self.committed()
        self.assertEqual(len(self.audit(data,args)['checked_intermediates']),6)
        cut={'commit_seq':'2','domain_id':'B:outsider','root':'NOT_IN_E_HISTORY','session_id':'OTHER_SESSION'}
        decision['grant']['e_cut']=cut
        decision['local_commit_receipt']['e_cut']=copy.deepcopy(cut)
        with self.assertRaises(d.Rejected):self.audit(data,args)

    def test_F3_missing_original_and_wrong_E_session_generation_root_receipt(self):
        for key in ('e_original_raw','e_original_sha'):
            data,args,_=self.committed();args.pop(key)
            with self.assertRaises(d.Rejected):self.audit(data,args)
        for field in ('session_id','session_generation','domain_id','root','receipt','commit_ns','missing_field'):
            data,args,_=self.committed();original=d.strict(args['e_original_raw'])
            if field in original['meta']:original['meta'][field]='OTHER'
            elif field=='root':original['commits'][2]['root']='f'*64
            elif field=='receipt':original['receipts']['Reserve/slot']['receipt_key']='OTHER'
            elif field=='commit_ns':original['commits'][2]['commit_ns']='12'
            else:original['commits'][2].pop('root')
            args['e_original_raw']=d.canonical(original);args['e_original_sha']=d.sha(args['e_original_raw'])
            with self.assertRaises((d.Rejected,KeyError)):self.audit(data,args)

    def test_F4_last_evidence_crosses_work_deadline_fixed_probe(self):
        p=st.fixture();clock=st.Clock();runner=s.Scheduler(p,st.FakeAdapter(clock),clock)
        original=runner._evidence
        def delayed(step,previous,ev):
            original(step,previous,ev)
            if step==p.steps[-1]:clock.value=61.
        runner._evidence=delayed
        result=runner.run()
        self.assertEqual(result.outcome,'FAILED')
        self.assertEqual(len(result.stage_receipts),len(p.steps)-1)
        self.assertEqual(len(result.cleanup),3)
        self.assertEqual(result.seal.final_outcome,'FAILED')

    def test_F4_serialization_and_journal_return_cannot_publish_success(self):
        for seam in ('serialize','persist'):
            p=st.fixture();clock=st.Clock()
            class Journal:
                def consume(self,*args):pass
                def finish(self,*args):clock.value=51.
            runner=s.Scheduler(p,st.FakeAdapter(clock),clock,journal=Journal() if seam=='persist' else None)
            original=s.digest
            def delayed(value):
                result=original(value)
                if isinstance(value,dict) and value.get('business_status')=='INDEPENDENTLY_VERIFIED':clock.value=51.
                return result
            with patch.object(s,'digest',side_effect=delayed if seam=='serialize' else original):
                result=runner.run()
            self.assertEqual(result.outcome,'FAILED')
            self.assertFalse(result.stage_receipts)

    def test_F4_same_cut_second_parse_and_serialization_fixed_clock(self):
        for seam in ('parse','serialize'):
            now=[1.];budget=d.ReadBudget(2.,10,1000)
            raw=d.canonical({'cut':{'seq':'1'},'image':{'q':'3'}})
            class MemoryOriginal:
                def read(self,b):b.charge((raw,));return raw
            parse,canonical=d.strict,d.canonical;calls=[0]
            def late_parse(value):
                result=parse(value);calls[0]+=1
                if calls[0]==2:now[0]=3.
                return result
            def late_canonical(value):
                result=canonical(value);now[0]=3.;return result
            with patch.object(d.time,'monotonic',side_effect=lambda:now[0]), \
                 patch.object(d,'strict',side_effect=late_parse if seam=='parse' else parse), \
                 patch.object(d,'canonical',side_effect=late_canonical if seam=='serialize' else canonical):
                with self.assertRaises(d.Rejected):
                    e.same_cut(MemoryOriginal(),MemoryOriginal(),{'seq':'1'},budget)

    def test_F4_verify_exchanges_final_empty_set_check_is_budgeted(self):
        now=[1.];budget=d.ReadBudget(2.,10,1000)
        class Snapshots(dict):
            def items(self):now[0]=3.;return super().items()
        with patch.object(d.time,'monotonic',side_effect=lambda:now[0]):
            with self.assertRaises(d.Rejected):
                e.verify_exchanges((),(),budget,session_id='toy',session_generation='1',domain_snapshots=Snapshots())

    def test_F4_late_fsync_keeps_consumed_journal_without_success_receipt(self):
        now=[time.monotonic()];p=st.fixture()
        p=replace(p,steps=(replace(p.steps[0],deadline=now[0]+2),),work_deadline=now[0]+10,
                  cleanup_deadline=now[0]+12,seal_deadline=now[0]+14)
        with tempfile.TemporaryDirectory(dir='/tmp') as tmp:
            with j.Journal.create(Path(tmp)/'journal',p,max_bytes=100000,max_records=20) as journal:
                step=p.steps[0];journal.consume(step,p.seed_sha)
                receipt=s.StageReceipt(0,step.stage,step.scenario,step.sha(),p.seed_sha,'a'*64,'SIMULATED_VERIFIED')
                fsync=j.os.fsync
                def late(fd):fsync(fd);now[0]=step.deadline+1
                with patch.object(d.time,'monotonic',side_effect=lambda:now[0]),patch.object(j.os,'fsync',side_effect=late):
                    with self.assertRaises(d.Rejected):journal.finish(step,receipt)
                entries,_,_,index,pending=journal.read()
                self.assertEqual([x['kind'] for x in entries],['consume'])
                self.assertEqual(index,0);self.assertEqual(pending,step.sha())

    def setup_core(self, interval=.125):
        now=[time.monotonic()];worker=sup.Identity(100,'worker',100,99);child=sup.Identity(101,'child',101,100)
        probe=Probe(worker);probe.identities[101]=child
        lim=sup.Limits(now[0]+10,now[0]+12,now[0]+14,interval,1,.1,.2,4,100,100,100)
        path=Path(sup.__file__).resolve();digest=d.sha(path.read_bytes())
        stages={'toy':{'resource_caps':{'scope':'UNIT_TEST_ONLY'},'deadline':now[0]+9,'domain_id':'E','writer_epoch':'1'}}
        core=sup.SupervisorCore(contract_sha='a'*64,script=path,script_sha=digest,worker=worker,
            worker_script=path,worker_script_sha=digest,limits=lim,stages=stages,probe=probe,clock=lambda:now[0])
        binding={'contract_sha256':'a'*64,'worker_pid':100,'worker_start_identity':'worker','worker_process_group':100,
            'nonce':'n1','stage':'toy','owner':{'pid':101,'start_identity':'child','process_group':101,'domain_id':'E','writer_epoch':'1'},
            'resource_caps':stages['toy']['resource_caps'],'absolute_deadline_monotonic':stages['toy']['deadline']}
        return now,worker,child,probe,lim,core,binding

    def test_F5_monitor_first_ack_receive_race_fixed_probe(self):
        for elapsed in (.125,.2):
            now,worker,child,probe,lim,core,binding=self.setup_core(.125 if elapsed==.125 else .1)
            count=[0];outputs=[]
            def receive(timeout):
                count[0]+=1
                if count[0]==1:
                    now[0]+=elapsed;probe.desc.add(101)
                    return {'kind':'ACK_REQUEST','binding':binding}
                return {'kind':'COMPLETE','worker':asdict(worker)}
            sup.monitor(core,receive,outputs.append)
            self.assertEqual(outputs[0]['kind'],'ACK')
            self.assertEqual(outputs[-1]['status'],'CONTROL_REAPED')
            self.assertTrue(outputs[-1]['owners'][0]['reaped'])

    def test_F5_monitor_descendant_race_and_malicious_binding(self):
        for corrupt in (None,'worker','parent','identity','contract','unknown'):
            now,worker,child,probe,lim,core,binding=self.setup_core()
            core.ack(binding);grandchild=sup.Identity(102,'grandchild',101,101)
            probe.identities[102]=grandchild
            count=[0];outputs=[]
            def receive(timeout):
                count[0]+=1
                if count[0]>1:return {'kind':'COMPLETE','worker':asdict(worker)}
                now[0]+=timeout;probe.desc.add(102)
                message={'kind':'REGISTER_DESCENDANT','worker':asdict(worker),'contract_sha256':'a'*64,
                         'identity':asdict(grandchild),'parent':101}
                if corrupt=='worker':message['worker']['start']='wrong'
                if corrupt=='parent':message['parent']=999
                if corrupt=='identity':message['identity']['start']='wrong'
                if corrupt=='contract':message['contract_sha256']='b'*64
                if corrupt=='unknown':probe.desc.add(999)
                return message
            if corrupt:
                with self.assertRaises(d.Rejected):sup.monitor(core,receive,outputs.append)
                self.assertEqual(outputs[-1]['status'],'INCOMPLETE')
                self.assertNotIn(999,core.owners)
            else:
                sup.monitor(core,receive,outputs.append)
                self.assertEqual(outputs[-1]['status'],'CONTROL_REAPED')
                self.assertEqual(probe.waits,[102,101])

    def test_F5_bad_first_ack_not_adopted(self):
        for field in ('contract_sha256','nonce','owner'):
            now,worker,child,probe,lim,core,binding=self.setup_core()
            if field=='owner':binding[field]['start_identity']='wrong'
            else:binding[field]='' if field=='nonce' else 'b'*64
            def receive(timeout):
                now[0]+=timeout;probe.desc.add(101)
                return {'kind':'ACK_REQUEST','binding':binding}
            with self.assertRaises(d.Rejected):sup.monitor(core,receive,lambda x:None)
            self.assertFalse(core.owners);self.assertFalse(probe.signals)

    def test_F6_wait_late_live_dead_and_kill_branches_keep_reap_fact(self):
        for state in ('live','dead','kill'):
            now,worker,child,probe,lim,core,_=self.setup_core()
            core.register(child,parent=100)
            if state=='dead':probe.dead.add(child.pid)
            if state=='kill':probe.term_exits=False
            calls=[0]
            def wait(identity,deadline):
                calls[0]+=1
                if state=='kill' and calls[0]==1:return False
                now[0]=lim.cleanup_deadline+1;return True
            probe.wait=wait
            result=core.stop('COMPLETE')
            self.assertEqual(result['status'],'INCOMPLETE')
            self.assertTrue(result['owners'][0]['reaped'])
            self.assertTrue(result['owners'][0]['errors'])

    def test_F6_timeout_still_visits_other_registered_owners_and_summary(self):
        now,worker,child,probe,lim,core,_=self.setup_core();core.register(child,parent=100)
        other=sup.Identity(102,'other',102,100);probe.identities[102]=other;core.register(other,parent=100)
        probe.dead.update((101,102));visited=[]
        def wait(identity,deadline):visited.append(identity.pid);now[0]=lim.cleanup_deadline+1;return True
        probe.wait=wait
        result=core.stop('COMPLETE')
        self.assertEqual(visited,[102,101]);self.assertEqual(result['status'],'INCOMPLETE')
        now,_,_,_,lim,core,_=self.setup_core();now[0]=lim.cleanup_deadline+1
        self.assertEqual(core.stop('COMPLETE')['status'],'INCOMPLETE')
