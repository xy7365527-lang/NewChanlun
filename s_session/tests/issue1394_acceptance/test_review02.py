"""#1394 独审02九项锁；内存/fake clock/临时日记，不是产品验收。"""
import copy
from dataclasses import asdict, replace
from pathlib import Path
import signal
import tempfile
import time
import unittest
from unittest.mock import patch

import captured_cut as x
import closure as c
import driver as d
import journal as j
import scheduler as s
import supervisor as sup
import test_capture_bridge as cap
import test_review01 as prior
import test_scheduler as st


class Review02Tests(unittest.TestCase):
    def test_R_F3_fixed_early_B_and_each_of_three_times(self):
        old = prior.Review01Tests()
        for times in [('11','11','11'), ('10','10','10'), ('9','9','11'),
                      ('9','11','11'), ('11','9','11'), ('11','11','9'),
                      ('9','9','9'), ('12','11','11'), ('11','12','11'), ('11','11','12')]:
            data, args, decision = old.committed()
            original, pin = args['e_original_raw'], args['e_original_sha']
            args['histories']['B:beta'][-1]['commit_ns'] = times[0]
            args['operations'][-1]['commit_ns'] = times[1]
            decision['local_commit_receipt']['commit_ns'] = times[2]
            with self.subTest(times=times):
                if times in [('11','11','11'), ('10','10','10')]:
                    self.assertEqual(len(old.audit(data,args)['checked_intermediates']),6)
                else:
                    with self.assertRaises(d.Rejected):old.audit(data,args)
                self.assertEqual((original,pin),(args['e_original_raw'],args['e_original_sha']))

    def test_R_F3_missing_times_refused(self):
        old=prior.Review01Tests()
        for where in ('row','operation','receipt'):
            data,args,decision=old.committed()
            target={'row':args['histories']['B:beta'][-1], 'operation':args['operations'][-1],
                    'receipt':decision['local_commit_receipt']}[where]
            target.pop('commit_ns')
            with self.subTest(where=where),self.assertRaises((d.Rejected,KeyError)):old.audit(data,args)

    def test_R_F4_unlock_late_exception_reopen_and_no_truncate(self):
        for mode in ('late','exception_before','exception_after','concurrent_append'):
            now=[1.];plan=st.fixture();step=plan.steps[0]
            with self.subTest(mode=mode), tempfile.TemporaryDirectory(dir='/tmp') as tmp, \
                 patch.object(d.time,'monotonic',side_effect=lambda:now[0]):
                path=Path(tmp)/'journal'
                with j.Journal.create(path,plan,max_bytes=1000000,max_records=1100) as journal:
                    flock=j.fcntl.flock;injected=[False];other_bytes=[b'']
                    def unlock(fd,op):
                        if op==j.fcntl.LOCK_UN and not injected[0] and b'"kind":"candidate"' in path.read_bytes():
                            injected[0]=True
                            if mode=='exception_before':raise OSError('toy unlock before release')
                            flock(fd,op)
                            if mode=='concurrent_append':
                                # 已释放锁，另一持锁写者追加原件。失败路径不能撤其尾。
                                with path.open('ab') as out:
                                    flock(out.fileno(),j.fcntl.LOCK_EX)
                                    out.write(b'foreign-appended-evidence\n');out.flush()
                                    flock(out.fileno(),j.fcntl.LOCK_UN)
                                other_bytes[0]=path.read_bytes()
                            if mode=='exception_after':raise OSError('toy unlock after release')
                            now[0]=51.
                            return
                        return flock(fd,op)
                    with patch.object(j.fcntl,'flock',side_effect=unlock), \
                         patch.object(j.os,'ftruncate',side_effect=AssertionError('不得在解锁后截断')):
                        result=s.Scheduler(plan,st.FakeAdapter(),lambda:now[0],journal=journal).run()
                    self.assertEqual(result.outcome,'FAILED');self.assertFalse(result.stage_receipts)
                    if mode=='concurrent_append':
                        self.assertEqual(path.read_bytes(),other_bytes[0])
                    else:
                        entries,_,_,index,pending=journal.read()
                        self.assertEqual([r['kind'] for r in entries],['consume','candidate'])
                        self.assertEqual((index,pending),(0,step.sha()))
                    identity,header=journal.identity,journal.header
                if mode=='concurrent_append':
                    with self.assertRaises((d.Rejected,ValueError)):
                        j.Journal(path,identity,header,1000000,1100)
                else:
                    with j.Journal(path,identity,header,1000000,1100) as reopened:
                        self.assertEqual(reopened.read()[3:],(0,step.sha()))
                        with self.assertRaises(d.Rejected):reopened.consume(step,plan.seed_sha)

    def test_R_F4_normal_acceptance_and_reopen_quarantine(self):
        plan=st.fixture();now=[1.]
        with tempfile.TemporaryDirectory(dir='/tmp') as tmp, \
             patch.object(d.time,'monotonic',side_effect=lambda:now[0]):
            path=Path(tmp)/'journal'
            with j.Journal.create(path,plan,max_bytes=1000000,max_records=1100) as journal:
                adapter=st.FakeAdapter();adapter.fail_stage=plan.steps[1].stage
                result=s.Scheduler(plan,adapter,lambda:now[0],journal=journal).run()
                self.assertEqual(len(result.stage_receipts),1)
                self.assertEqual(journal.read()[3:],(1,plan.steps[1].sha()))
                identity,header=journal.identity,journal.header;raw=path.read_bytes()
            with j.Journal(path,identity,header,1000000,1100) as reopened:
                self.assertEqual(reopened.read()[3:],(0,plan.steps[0].sha()))
                with self.assertRaises(d.Rejected):reopened.consume(plan.steps[0],plan.seed_sha)
                self.assertEqual(raw,path.read_bytes())

    def test_R_F4_candidate_cannot_be_accepted_without_finish_or_twice(self):
        p=st.fixture();step=p.steps[0]
        with tempfile.TemporaryDirectory(dir='/tmp') as tmp,patch.object(d.time,'monotonic',return_value=1.):
            with j.Journal.create(Path(tmp)/'journal',p,max_bytes=1000000,max_records=1100) as journal:
                r=s.StageReceipt(0,step.stage,step.scenario,step.sha(),p.seed_sha,'a'*64,'SIMULATED_VERIFIED')
                with self.assertRaises(d.Rejected):journal.accept(step,r)
                journal.consume(step,p.seed_sha);journal.finish(step,r)
                self.assertEqual(journal.read()[3:],(0,step.sha()))
                journal.accept(step,r)
                self.assertEqual(journal.read()[3:],(1,None))
                with self.assertRaises(d.Rejected):journal.accept(step,r)

    def test_R_F4_fsync_failure_and_late_accept_keep_pending(self):
        for mode in ('fsync','accept_serialization'):
            p=st.fixture();step=p.steps[0];now=[1.]
            with self.subTest(mode=mode),tempfile.TemporaryDirectory(dir='/tmp') as tmp, \
                 patch.object(d.time,'monotonic',side_effect=lambda:now[0]):
                with j.Journal.create(Path(tmp)/'journal',p,max_bytes=1000000,max_records=1100) as journal:
                    r=s.StageReceipt(0,step.stage,step.scenario,step.sha(),p.seed_sha,'a'*64,'SIMULATED_VERIFIED')
                    journal.consume(step,p.seed_sha)
                    if mode=='fsync':
                        fsync=j.os.fsync;calls=[]
                        def fail_once(fd):
                            calls.append(fd)
                            if len(calls)==1:raise OSError('toy candidate fsync')
                            return fsync(fd)
                        with patch.object(j.os,'fsync',side_effect=fail_once),self.assertRaises(OSError):journal.finish(step,r)
                        self.assertEqual([v['kind'] for v in journal.read()[0]],['consume'])
                    else:
                        journal.finish(step,r);digest=s.StageReceipt.sha
                        def late(receipt):
                            value=digest(receipt);now[0]=51.;return value
                        with patch.object(s.StageReceipt,'sha',late),self.assertRaises(d.Rejected):journal.accept(step,r)
                        self.assertEqual([v['kind'] for v in journal.read()[0]],['consume','candidate'])
                    self.assertEqual(journal.read()[3:],(0,step.sha()))

    def test_R_F5_already_queued_ACK_and_descendant(self):
        for descendant in (False,True):
            now,worker,child,probe,lim,core,binding=prior.Review01Tests().setup_core()
            if descendant:
                core.ack(binding);identity=sup.Identity(102,'grandchild',101,101)
                probe.identities[102]=identity;probe.desc.add(102)
                message={'kind':'REGISTER_DESCENDANT','worker':asdict(worker),'contract_sha256':'a'*64,
                         'identity':asdict(identity),'parent':101}
            else:
                probe.desc.add(101);message={'kind':'ACK_REQUEST','binding':binding}
            now[0]+=.25;calls=[];outputs=[]
            def receive(timeout):
                calls.append(timeout)
                return message if len(calls)==1 else {'kind':'COMPLETE','worker':asdict(worker)}
            sup.monitor(core,receive,outputs.append)
            self.assertEqual(len(calls),2);self.assertEqual(outputs[-1]['status'],'CONTROL_REAPED')
            self.assertTrue(all(r['reaped'] for r in outputs[-1]['owners']))
            self.assertEqual(set(probe.waits),{101,102} if descendant else {101})

    def test_R_F5_queued_forgery_unknown_and_expired_not_adopted(self):
        for mode in ('forged','unknown','heartbeat','work','slow_receive'):
            now,worker,child,probe,lim,core,binding=prior.Review01Tests().setup_core()
            probe.desc.add(101);now[0]+=.25;calls=[]
            if mode=='forged':binding['owner']['start_identity']='forged'
            if mode=='unknown':probe.desc.add(999)
            if mode=='heartbeat':now[0]=core.last_heartbeat+lim.heartbeat_timeout
            if mode=='work':now[0]=lim.work_deadline
            def receive(timeout):
                calls.append(timeout)
                if mode=='slow_receive':now[0]=core.last_heartbeat+lim.heartbeat_timeout
                return {'kind':'ACK_REQUEST','binding':binding}
            with self.subTest(mode=mode),self.assertRaises(d.Rejected):sup.monitor(core,receive,lambda v:None)
            self.assertNotIn(999,core.owners)
            if mode!='unknown':self.assertFalse(core.owners);self.assertFalse(probe.signals)

    def test_R_F6_late_any_position_live_dead_kill_all_touched(self):
        for late_pid in (101,102):
            for state in ('live','dead','kill','unreaped'):
                now,worker,child,probe,lim,core,_=prior.Review01Tests().setup_core()
                core.register(child,parent=100)
                other=sup.Identity(102,'other',102,100);probe.identities[102]=other;core.register(other,parent=100)
                if state=='dead':probe.dead.update((101,102))
                if state in ('kill','unreaped'):probe.term_exits=False
                deadlines=[]
                def wait(identity,deadline):
                    probe.waits.append(identity.pid);deadlines.append(deadline)
                    if identity.pid==late_pid:now[0]=lim.cleanup_deadline+1
                    return state!='unreaped' and identity.pid in probe.dead
                probe.wait=wait
                result=core.stop('COMPLETE')
                with self.subTest(late_pid=late_pid,state=state):
                    self.assertEqual(result['status'],'INCOMPLETE')
                    self.assertEqual(set(probe.waits),{101,102})
                    self.assertTrue(all(deadline<=lim.cleanup_deadline for deadline in deadlines))
                    self.assertTrue(any(r['errors'] for r in result['owners']))
                    if state!='dead':self.assertEqual({p for p,sig in probe.signals if sig==signal.SIGTERM},{101,102})
                    if state in ('kill','unreaped'):self.assertEqual({p for p,sig in probe.signals if sig==signal.SIGKILL},{101,102})
                    if state!='unreaped':self.assertTrue(all(r['reaped'] for r in result['owners']))
                    else:self.assertTrue(all(not r['reaped'] for r in result['owners']))

    def test_R_F6_expired_entry_still_nonblocking_cleans_known_only(self):
        now,worker,child,probe,lim,core,_=prior.Review01Tests().setup_core()
        core.register(child,parent=100);probe.desc.add(999);probe.term_exits=False
        now[0]=lim.cleanup_deadline+1;seen=[]
        def wait(identity,deadline):
            seen.append((identity.pid,deadline));self.assertLess(deadline,now[0])
            return identity.pid in probe.dead
        probe.wait=wait
        result=core.stop('COMPLETE')
        self.assertEqual(result['status'],'INCOMPLETE');self.assertTrue(result['owners'][0]['reaped'])
        self.assertEqual(probe.signals,[(101,signal.SIGTERM),(101,signal.SIGKILL)])
        self.assertEqual(len(seen),2)

    def test_N1_empty_cursor_legal_null_and_broken_order_refused(self):
        self.assertEqual(cap.export(*cap.fixture())['status'],'UNAUTHENTICATED_FACTS')
        for mode in ('null_request','null_reply','next','later_after'):
            contract,captures,nodes,raw=cap.fixture()
            indices=[i for i,z in enumerate(captures) if z.route=='financial-closure']
            i=indices[1] if mode=='later_after' else indices[0];old=captures[i]
            req,reply=d.strict(old.request),d.strict(old.reply)
            if mode=='null_request':req['after']=None
            if mode=='null_reply':reply['after']=None
            if mode=='next':reply['next']='sc1-'+'0'*64
            if mode=='later_after':req['after']=''
            captures[i]=replace(old,request=d.canonical(req),reply=d.canonical(reply))
            contract['captures']=[z.identity() for z in captures]
            with self.subTest(mode=mode),self.assertRaises(d.Rejected):cap.export(contract,captures,nodes,raw)

    def test_N2_fixed_session_request_frame_and_Q_page_caps(self):
        for role,field in [('session','max_requests'),('session','max_request_bytes'),
                           ('session','max_reply_bytes'),('reader','max_request_bytes'),
                           ('reader','max_reply_bytes'),('q','max_frame_bytes'),
                           ('q','max_reply_bytes'),('q','max_page_size'),('session','max_closure_nodes')]:
            contract,captures,nodes,raw=cap.fixture();r=contract['resources']
            target=r['q']['limits'] if role=='q' else r[role]
            selected=[z for z in captures if (z.route=='snapshot')==(role=='q')]
            if field=='max_requests':limit=len(selected)
            elif field=='max_page_size':limit=2
            elif field=='max_closure_nodes':limit=len(nodes)
            elif field=='max_reply_bytes':limit=max(len(z.reply) for z in selected)
            else:limit=max(len(z.request) for z in selected)
            target[field]=str(limit)
            if role=='session' and field=='max_reply_bytes':r['reader']['max_reply_bytes']=str(limit)
            with self.subTest(role=role,field=field):
                self.assertEqual(cap.export(contract,captures,nodes,raw)['status'],'UNAUTHENTICATED_FACTS')
                for bad in (limit-1,1):
                    target[field]=str(bad)
                    with self.assertRaises(d.Rejected):cap.export(contract,captures,nodes,raw)

    def test_N3_separate_Q_and_reader_jobs_exact_boundaries(self):
        contract,captures,nodes,raw=cap.fixture();r=contract['resources']
        for name,route in [('q-job','snapshot'),('financial-job','financial')]:
            selected=[z for z in captures if (z.route=='snapshot')==(route=='snapshot')]
            limits=r['jobs'][name]['limits']
            limits['max_total_requests']=str(len(selected))
            limits['max_total_wire_bytes']=str(sum(len(z.request)+len(z.reply) for z in selected))
        result=cap.export(contract,captures,nodes,raw)
        self.assertEqual(result['observed_job_usage']['q-job']['requests'],4)
        self.assertEqual(result['observed_job_usage']['financial-job']['requests'],17)
        for name in ('q-job','financial-job'):
            for field in ('max_total_requests','max_total_wire_bytes'):
                trial=copy.deepcopy(contract);trial['resources']['jobs'][name]['limits'][field]=str(int(r['jobs'][name]['limits'][field])-1)
                with self.subTest(name=name,field=field),self.assertRaises(d.Rejected):cap.export(trial,captures,nodes,raw)

    def test_N3_binding_old_ambiguity_and_cross_object_rejected(self):
        for mode in ('old_schema','missing','wrong_job','wrong_session','shared_job','reader_limits'):
            contract,captures,nodes,raw=cap.fixture();r=contract['resources']
            if mode=='old_schema':contract['schema_revision']='issue1394-captured-cut/1'
            elif mode=='shared_job':r['jobs']['q-job']['owner']=r['object_ids']['reader']
            elif mode=='reader_limits':r['jobs']['financial-job']['limits']=dict(r['reader']['job_limits'],max_total_requests='17')
            else:
                i=4;binding=copy.deepcopy(captures[i].binding)
                if mode=='missing':binding=None
                elif mode=='wrong_job':binding['job']='q-job'
                else:binding['session']='outsider'
                captures[i]=replace(captures[i],binding=binding);contract['captures']=[z.identity() for z in captures]
            with self.subTest(mode=mode),self.assertRaises(d.Rejected):cap.export(contract,captures,nodes,raw)

    def test_N3_node_and_record_caps_stay_with_their_object(self):
        for job,field,limit in [('financial-job','max_total_nodes',None),
                               ('financial-job','max_total_node_bytes',None),
                               ('financial-job','max_total_records',7),('q-job','max_total_records',7)]:
            contract,captures,nodes,raw=cap.fixture()
            if field=='max_total_nodes':limit=len(nodes)
            if field=='max_total_node_bytes':limit=sum(len(n.body)+len(c.DOMAIN)+9 for n in nodes.values())
            limits=contract['resources']['jobs'][job]['limits'];limits[field]=str(limit)
            self.assertEqual(cap.export(contract,captures,nodes,raw)['status'],'UNAUTHENTICATED_FACTS')
            limits[field]=str(limit-1)
            with self.subTest(job=job,field=field),self.assertRaises(d.Rejected):cap.export(contract,captures,nodes,raw)

    def test_N4_size_observation_not_Voice_64_128_caps(self):
        # 仅len观察对象；无65MiB分配。Voice自身64/128锁仍在test_capture_bridge。
        class SizeOnly:
            def __len__(self):return 65*x.MIB
        contract,captures,_,_=cap.fixture();r=contract['resources'];used={};counts={}
        for role in ('session','reader'):
            r[role]['max_request_bytes']=r[role]['max_reply_bytes']=str(66*x.MIB)
        r['reader']['job_limits']['max_total_wire_bytes']=str(260*x.MIB)
        for _ in range(2):x.charge_capture(replace(captures[4],request=SizeOnly(),reply=SizeOnly()),r,used,counts)
        self.assertEqual(used['financial-job']['wire'],260*x.MIB)
        with self.assertRaises(d.Rejected):x.charge_capture(replace(captures[4],request=SizeOnly(),reply=SizeOnly()),r,used,counts)

    def test_N5_historical_stage_expired_current_valid(self):
        contract,captures,nodes,raw=cap.fixture()
        contract['resources']['deadlines']['stages']['already_finished']=time.monotonic()-1
        self.assertEqual(cap.export(contract,captures,nodes,raw)['status'],'UNAUTHENTICATED_FACTS')
        for mode in ('current','work','q_job','reader_job','elapsed','missing_stage'):
            trial=copy.deepcopy(contract);r=trial['resources'];now=time.monotonic()
            if mode=='current':r['deadlines']['stages']['export']=now-1
            elif mode=='work':r['deadlines']['work']=now-1
            elif mode=='missing_stage':trial['stage']='absent'
            else:
                job=r['jobs']['q-job' if mode=='q_job' else 'financial-job']
                job['started_at']=now-3
                if mode=='elapsed':job['limits']['max_elapsed_ms']='1'
                else:job['deadline']=now-1
            with self.subTest(mode=mode),self.assertRaises(d.Rejected):cap.export(trial,captures,nodes,raw)
