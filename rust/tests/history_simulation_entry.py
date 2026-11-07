#!/usr/bin/env python3
"""#1381: 真实 X binary + 明确受控 E/S authority；非真实 producer/财务闭环。"""
import copy, hashlib, json, os, socket, sqlite3, subprocess, sys, tempfile, threading, time
from pathlib import Path
BIN=Path(sys.argv[1]).resolve()
ROOT=Path(sys.argv[2]).resolve(); ROOT.mkdir(parents=True,exist_ok=True)
def canonical(x): return json.dumps(x,sort_keys=True,separators=(',',':'),ensure_ascii=False)
def digest(x): return hashlib.sha256(canonical(x).encode()).hexdigest()
def request(path,op,payload):
    with socket.socket(socket.AF_UNIX) as s:
        s.settimeout(5); s.connect(str(path)); s.sendall((canonical({'protocol':'x-exact-simulation/1','op':op,'payload':payload})+'\n').encode())
        s.shutdown(socket.SHUT_WR)
        return json.loads(s.makefile('rb').readline())
class Authority:
    def __init__(self,path,role):
        self.path=path; self.role=role; self.calls=[]; self.bad=None; self.records={}; self.authorizations={}; self.drop_after=None
        self.sock=socket.socket(socket.AF_UNIX); self.sock.bind(str(path)); self.sock.listen(16)
        self.thread=threading.Thread(target=self.run,daemon=True); self.thread.start()
    def run(self):
        while True:
            try: c,_=self.sock.accept()
            except OSError: return
            with c:
                q=json.loads(c.makefile('rb').readline()); self.calls.append(q)
                r={k:q[k] for k in ['protocol','session_id','session_generation','authority_id','authority_epoch','command_id','command_hash','command']}
                r.update(state='ReleasedExact' if self.role=='E' else 'ExecuteCommitted',receipt_id=self.role+':'+q['command_id'],release_receipt_id=q['release_receipt_id'],commit_ns=100)
                if self.role=='S':
                    key=q['command_id']
                    if q['op']=='RequestConditionalExecution':
                        assert key not in self.records, 'X tried to authorize twice'
                        self.records[key]=copy.deepcopy(r); self.authorizations[key]=self.authorizations.get(key,0)+1
                    elif q['op']=='QueryExecuteCommitted':
                        r=copy.deepcopy(self.records.get(key,r)); r['state']='ExecuteCommitted' if key in self.records else 'NoRecord'
                    else: raise AssertionError(q['op'])
                    if key==self.drop_after: continue
                if self.bad=='denied': r['state']='Denied'
                if self.bad=='identity': r['authority_epoch']='wrong'
                if self.bad=='content': r['command']['symbol']='wrong'
                if self.bad=='future': r['commit_ns']=9_000_000_000_000_000_000
                if self.bad=='missing': r['state']='NoRecord'
                c.sendall((canonical(r)+'\n').encode())
def model():
    return dict(model_id='history-sim/bar-batch-known-frontier/1',revision='oracle-1',symbol='TEST',quantity_unit='TEST-UNIT',currency='TEST-CASH',tick_value=1,rd11_policy='controlled-fixture-rd11',rd12_policy='per-leg-source-attribution/1',projection_kind='per_leg',seed='none',fee_profile=dict(reference='controlled-fee/1',role_source='controlled-role-map/1',limit_role='maker',market_role='taker',maker=dict(numerator=1,denominator=100,rounding='ceiling'),taker=dict(numerator=2,denominator=100,rounding='ceiling'),market_slippage_ticks=1,market_slippage_source='controlled-oracle/1'))
def order(name,side='buy',quantity=100,price=100):
    return dict(command_id=name,family_id='f-'+name,chong_id='ch-'+name,revision=1,side=side,quantity=quantity,kind=dict(kind='limit',price=price),eligibility_ref='S:'+name,projection_manifest='controlled-projection-'+name)
def command(change):
    o=change.get('order',change); cid=o['command_id']; family=o['family_id']
    caps=[('released-exact/1','E.ReleaseExact'),('execute-committed/1','S.RequestConditionalExecution'),('simulation-local-transaction/1','X.OwnAttemptAndCall')]
    cert=dict(rd11_policy='controlled-fixture-rd11',responsibility_refs=['controlled-responsibility'],structure_dependencies=['controlled-structure'],market_dependencies=['controlled-market'],rule_revision='controlled-rule',conditions=[dict(condition_id=cap,authority_revision=cap,dependency_set=['controlled-dependency'],invalidation_sources=['controlled-invalidation'],required_check_stages=[stage],evidence_refs=['controlled-evidence'],coverage_proof='controlled-coverage',validator_capability=cap,failure_result='Denied') for cap,stage in caps])
    return dict(command_id=cid,logical_order_family=family,session_id='session',session_generation='g1',x_id='X',x_epoch=1,symbol='TEST',projection_kind='per_leg',change=change,plan_hash=digest(change),certificate_hash=digest(cert),certificate=cert)
def batch(i,changes=[]):
    return dict(request_id='b'+str(i),known_frontier_cut='frontier'+str(i),bar=dict(index=i,open=100,low=90,high=110,close=105),capacity=dict(kind='none'),close_changes=changes,scenario_busts=[])
def run(name):
    directory=ROOT/name; directory.mkdir()
    # Unix socket path stays below sockaddr_un limit; data remains under task root.
    sockets=Path(tempfile.mkdtemp(prefix='i1381-'))
    e=Authority(sockets/'e','E'); s=Authority(sockets/'s','S'); xp=sockets/'x'
    sim=dict(model=model(),session_id='session',session_generation='g1',x_id='X',source_namespace='x-sim',source_epoch='epoch1',socket=str(xp),e=dict(socket=str(e.path),authority_id='E',authority_epoch='e1'),s=dict(socket=str(s.path),authority_id='S',authority_epoch='s1'),timeout_ms=500,max_frame_bytes=262144)
    profile=dict(profile_revision='x-execution-profile/1',mode='simulation',credential_scope=dict(env='UNUSED_SIMULATION_CREDENTIAL'),history_simulation=sim)
    pp=directory/'profile.json'; pp.write_text(canonical(profile)); db=directory/'x.sqlite'
    log=open(directory/'x.log','ab')
    fixture=len(sys.argv)>3 and sys.argv[3]=='runtime-fixture'
    def argv(epoch=1):
        return [str(BIN),'--exact','history_simulation::tests::runtime_process_fixture','--nocapture'] if fixture else [str(BIN),'serve','--db',str(db),'--profile',str(pp),'--writer-epoch',str(epoch)]
    def env(crash=None,epoch=1):
        e=dict(os.environ,ISSUE1381_FIXTURE_DB=str(db),ISSUE1381_FIXTURE_PROFILE=str(pp),ISSUE1381_FIXTURE_EPOCH=str(epoch))
        if crash: e['ISSUE1381_TEST_CRASH_AFTER_CLAIM']=crash
        return e
    def start(crash=None,epoch=1):
        p=subprocess.Popen(argv(epoch),env=env(crash,epoch),stdout=log,stderr=log)
        deadline=time.monotonic()+8
        while not xp.exists():
            assert p.poll() is None, (directory/'x.log').read_text()
            assert time.monotonic()<deadline, 'X socket timeout'
            time.sleep(.02)
        return p
    def reject_new_epoch(p,crash=None):
        if p.poll() is None: p.kill(); p.wait(5)
        xp.unlink(missing_ok=True)
        with sqlite3.connect(db) as c:
            before=c.execute("select id,command,state,attempt_batch,execute_evidence from x_exact_commands order by id").fetchall()
            pending=c.execute("select k,v from x_sim_meta where k in ('pending_batch','pending_epoch') order by k").fetchall()
        blocked=subprocess.run(argv(2),env=env(epoch=2),capture_output=True,timeout=5)
        assert blocked.returncode!=0 and b'PendingBatch' in blocked.stderr,(blocked.stdout,blocked.stderr)
        with sqlite3.connect(db) as c:
            assert c.execute("select id,command,state,attempt_batch,execute_evidence from x_exact_commands order by id").fetchall()==before
            assert c.execute("select k,v from x_sim_meta where k in ('pending_batch','pending_epoch') order by k").fetchall()==pending
            if not fixture: assert c.execute("select value from x_meta where key='writer_epoch'").fetchone()==('1',)
        return start(crash)
    main_db=db
    if fixture:
        # Test-only process exits immediately before/after the real initialization transaction commit.
        for phase,code in [('before',44),('after',45)]:
            db=directory/('init-'+phase+'.sqlite'); init_env=env(); init_env['ISSUE1381_TEST_INIT_INTERRUPT']=phase
            interrupted=subprocess.run(argv(),env=init_env,capture_output=True,timeout=5)
            assert interrupted.returncode==code,(phase,interrupted.stdout,interrupted.stderr)
            with sqlite3.connect(db) as c:
                tables={row[0] for row in c.execute("select name from sqlite_master where type='table'")}
                if phase=='before': assert not any(t.startswith('x_sim_') or t=='x_exact_commands' for t in tables)
                else:
                    assert {'x_sim_meta','x_sim_batches','x_sim_originals','x_sim_busts','x_exact_commands'}<=tables
                    assert json.loads(c.execute("select v from x_sim_meta where k='authority_profile'").fetchone()[0])==sim
            recovered=start()
            try: assert request(xp,'QueryCommand','never-submitted')['state']=='NoRecord'
            finally: recovered.terminate(); recovered.wait(5); xp.unlink(missing_ok=True)
        db=main_db
    else:
        # Abort the launcher metadata write after simulation initialization; both belong to one transaction.
        with sqlite3.connect(db) as c:
            c.execute('CREATE TABLE x_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL)')
            c.execute("CREATE TRIGGER issue1381_init_abort BEFORE INSERT ON x_meta WHEN NEW.key='writer_epoch' BEGIN SELECT RAISE(ABORT,'controlled init fault'); END")
        interrupted=subprocess.run(argv(),env=env(),capture_output=True,timeout=5)
        assert interrupted.returncode!=0
        with sqlite3.connect(db) as c:
            assert c.execute('select count(*) from x_meta').fetchone()==(0,)
            assert c.execute("select count(*) from sqlite_master where type='table' and (name like 'x_sim_%' or name='x_exact_commands')").fetchone()==(0,)
            c.execute('DROP TRIGGER issue1381_init_abort')
        recovered=start(); recovered.kill(); recovered.wait(5); xp.unlink(missing_ok=True)
        with sqlite3.connect(db) as c:
            assert c.execute("select value from x_meta where key='writer_epoch'").fetchone()==('1',)
            assert json.loads(c.execute("select v from x_sim_meta where k='authority_profile'").fetchone()[0])==sim
    p=start()
    try:
        a=dict(kind='submit',order=order('a')); ca=command(a)
        before=len(s.calls)
        for bad in ['denied','identity','content','future']:
            e.bad=bad
            rejected=request(xp,'AcceptExact',ca)
            assert rejected.get('kind')=='Error',(bad,rejected)
            assert request(xp,'QueryCommand','a')['state']=='NoRecord'
        e.bad=None
        moved=e.path.with_name('e-unavailable'); e.path.rename(moved)
        assert request(xp,'AcceptExact',ca)['kind']=='Error'
        moved.rename(e.path)
        unknown=copy.deepcopy(ca); unknown['certificate']['conditions'][0]['validator_capability']='unknown'; unknown['certificate_hash']=digest(unknown['certificate'])
        assert request(xp,'AcceptExact',unknown)['kind']=='Error'
        assert request(xp,'AcceptExact',ca)['state']=='Accepted'
        assert len(s.calls)==before, 'Accept must never ask S/call adapter'
        conflicting=copy.deepcopy(ca); conflicting['x_epoch']=2
        assert request(xp,'AcceptExact',conflicting)['kind']=='Error'
        assert request(xp,'AcceptExact',ca)['state']=='Accepted'
        # Invalid input cannot consume S eligibility or poison Accepted with MayHaveCalled.
        invalid_ohlc=batch(0,[a]); invalid_ohlc['bar']['low']=111
        sibling=copy.deepcopy(a); sibling['order'].update(command_id='a-conflict',eligibility_ref='S:a-conflict')
        assert request(xp,'AcceptExact',command(sibling))['state']=='Accepted'
        invalid_close=batch(0,[a,sibling])
        invalid_bust=batch(0,[a]); invalid_bust['scenario_busts']=[dict(exec_id='missing',quantity=1,scenario_ref='missing')]
        before=len(s.calls)
        for invalid in [invalid_ohlc,invalid_close,invalid_bust]:
            assert request(xp,'AdvanceBar',invalid)['kind']=='Error'
            assert request(xp,'QueryCommand','a')['state']=='Accepted'
            assert request(xp,'QueryCommand','a-conflict')['state']=='Accepted'
            assert len(s.calls)==before, 'invalid input consumed S eligibility'
            with sqlite3.connect(db) as c:
                assert c.execute('select attempt_batch from x_exact_commands where id=?',('a',)).fetchone()==(None,)
                assert c.execute('select count(*) from x_sim_batches').fetchone()==(0,)
        for bad in ['denied','identity','content','future']:
            s.bad=bad; assert request(xp,'AdvanceBar',batch(0,[a]))['kind']=='Error'
            assert request(xp,'QueryCommand','a')['state']=='ExecutionRequested'
        s.bad=None
        p=reject_new_epoch(p)  # ExecutionRequested remains recoverable by epoch 1.
        r0=request(xp,'AdvanceBar',batch(0,[a])); assert r0['originals']==[]
        assert request(xp,'QueryCommand','a')['state']=='Observed'
        # Replace at bar 1 close: current fill still old price, next bar new price and remaining quantity.
        replacement=copy.deepcopy(a['order']); replacement.update(command_id='r',revision=2,eligibility_ref='S:r',kind=dict(kind='limit',price=95))
        change=dict(kind='replace',order=replacement)
        assert request(xp,'AcceptExact',command(change))['state']=='Accepted'
        b1=batch(1,[change]); b1['capacity']=dict(kind='scenario',reference='controlled-cap-40',buy=40,sell=0)
        r1=request(xp,'AdvanceBar',b1); fill=r1['originals'][0]['event']['fill']; assert (fill['quantity'],fill['price'],fill['amount'],fill['fee'])==(40,100,4000,40),r1
        assert request(xp,'AdvanceBar',b1)==r1
        p.terminate(); p.wait(5); xp.unlink(missing_ok=True); p=start()
        assert request(xp,'AdvanceBar',b1)==r1
        r2=request(xp,'AdvanceBar',batch(2)); fill=r2['originals'][0]['event']['fill']; assert (fill['quantity'],fill['price'])==(60,95)
        pending=dict(kind='submit',order=order('cancel-me',quantity=10))
        assert request(xp,'AcceptExact',command(pending))['state']=='Accepted'
        assert request(xp,'AdvanceBar',batch(3,[pending]))['originals']==[]
        cancel=dict(kind='cancel',command_id='cancel',family_id='f-cancel-me',eligibility_ref='S:cancel')
        market_order=order('market',side='sell',quantity=2); market_order['kind']=dict(kind='market',exec_index=5)
        market=dict(kind='submit',order=market_order)
        for change in [cancel,market]: assert request(xp,'AcceptExact',command(change))['state']=='Accepted'
        b4=batch(4,[cancel,market]); b4['capacity']=dict(kind='scenario',reference='controlled-cap-4',buy=4,sell=0)
        r4=request(xp,'AdvanceBar',b4); assert r4['originals'][0]['event']['fill']['quantity']==4
        r5=request(xp,'AdvanceBar',batch(5)); assert len(r5['originals'])==2
        fill=r5['originals'][0]['event']['fill']; assert (fill['quantity'],fill['price'],fill['fee'],fill['slot'])==(2,104,5,'close')
        for raw in r1['originals']+r2['originals']+r4['originals']+r5['originals']:
            assert request(xp,'QueryOriginal',raw['event']['source']['external_fact_id'])==raw
        # R3: stable (1,2) epoch survives process restart; varying capacity ref cannot reset it.
        cap_orders=[dict(kind='submit',order=order('epoch-a',quantity=1)),dict(kind='submit',order=order('epoch-b',quantity=2))]
        for change in cap_orders: assert request(xp,'AcceptExact',command(change))['state']=='Accepted'
        b6=batch(6,cap_orders)
        wrong=copy.deepcopy(b6); wrong['close_changes'][1]['order']['kind']['price']=99
        before=len(s.calls); assert request(xp,'AdvanceBar',wrong)['kind']=='Error'
        assert len(s.calls)==before, 'late command mismatch consumed earlier S eligibility'
        for command_name in ['epoch-a','epoch-b']: assert request(xp,'QueryCommand',command_name)['state']=='Accepted'
        # First S committed and persisted locally; second S committed but reply was lost.
        s.drop_after='epoch-b'; failed=request(xp,'AdvanceBar',b6); assert failed['kind']=='Error',failed
        assert request(xp,'QueryCommand','epoch-a')['state']=='ExecutionReady'
        assert request(xp,'QueryCommand','epoch-b')['state']=='ExecutionRequested'
        s.drop_after=None
        p=reject_new_epoch(p,'b6' if fixture else None)  # ExecutionReady + ExecutionRequested
        bypass=batch(6); bypass['request_id']='bypass6'
        assert request(xp,'AdvanceBar',bypass)['kind']=='Error'
        s.bad='missing'; assert request(xp,'AdvanceBar',b6)['kind']=='Error'; s.bad=None
        assert request(xp,'QueryCommand','epoch-b')['state']=='ExecutionRequested'
        assert s.authorizations['epoch-b']==1, 'NoRecord caused reauthorization'
        if fixture:
            try: request(xp,'AdvanceBar',b6)
            except (ValueError,ConnectionError): pass
            assert p.wait(5)==43
        else:
            # Fault the model transaction after real X has durably claimed MayHaveCalled.
            with sqlite3.connect(db) as c:
                c.execute("CREATE TRIGGER issue1381_block_publish BEFORE INSERT ON x_sim_batches WHEN NEW.id='b6' BEGIN SELECT RAISE(ABORT,'controlled publish fault'); END")
            assert request(xp,'AdvanceBar',b6)['kind']=='Error'
        with sqlite3.connect(db) as c:
            assert c.execute("select count(*) from x_exact_commands where state='MayHaveCalled'").fetchone()==(2,)
            assert c.execute("select count(*) from x_sim_batches where id='b6'").fetchone()==(0,)
        p=reject_new_epoch(p)  # MayHaveCalled cannot irreversibly advance writer_epoch.
        if not fixture:
            with sqlite3.connect(db) as c: c.execute('DROP TRIGGER issue1381_block_publish')
        assert request(xp,'AdvanceBar',bypass)['kind']=='Error'
        r6=request(xp,'AdvanceBar',b6); assert r6['originals']==[]
        assert s.authorizations['epoch-a']==s.authorizations['epoch-b']==1

        reordered=copy.deepcopy(b6); reordered['close_changes'].reverse(); assert request(xp,'AdvanceBar',reordered)==r6
        b7=batch(7); b7['capacity']=dict(kind='scenario',reference='r3-source-7',buy=1,sell=0)
        half=copy.deepcopy(b7); half['capacity_participants']=['f-epoch-a']; assert request(xp,'AdvanceBar',half)['kind']=='Error'
        r7=request(xp,'AdvanceBar',b7); assert r7['originals'][0]['event']['fill']['family_id']=='f-epoch-b'
        assert request(xp,'QueryBatch','b7')==r7
        p.terminate(); p.wait(5); xp.unlink(missing_ok=True); p=start()
        assert request(xp,'AdvanceBar',b7)==r7
        b8=batch(8); b8['capacity']=dict(kind='scenario',reference='r3-source-8',buy=1,sell=0)
        r8=request(xp,'AdvanceBar',b8); assert r8['originals'][0]['event']['fill']['family_id']=='f-epoch-b'
        assert r7['originals'][0]['event']['fill']['capacity_epoch']==r8['originals'][0]['event']['fill']['capacity_epoch']
        b9=batch(9); b9['capacity']=dict(kind='scenario',reference='r3-source-9',buy=1,sell=0)
        r9=request(xp,'AdvanceBar',b9); assert r9['originals'][0]['event']['fill']['family_id']=='f-epoch-a'
        # Sequential historical busts reach the full original amount: cold replay must use each prior frontier.
        bust_exec=r1['originals'][0]['event']['fill']['exec_id']
        bust_batches=[]
        for index,quantity in [(10,5),(11,35)]:
            bb=batch(index); bb['capacity']=dict(kind='scenario',reference='bust-r5',buy=0,sell=0)
            bb['scenario_busts']=[dict(exec_id=bust_exec,quantity=quantity,scenario_ref='bust-r5')]
            rr=request(xp,'AdvanceBar',bb); assert rr['kind']=='SimulationBatch',rr
            assert len(rr['originals'])==1 and rr['originals'][0]['event']['kind']=='bust'
            bust_batches.append(rr)
        # Raw immutable originals are the double-run comparison, not nondeterministic process clocks.
        with sqlite3.connect(db) as c:
            originals=c.execute('select id,raw from x_sim_originals order by id').fetchall()
            clocks=c.execute('select accept_clock,release_evidence from x_exact_commands').fetchall()
            for clock,release in clocks:
                clock=json.loads(clock); release=json.loads(release)
                assert clock['received_ns']<=clock['first_known_ns']<=clock['commit_ns']
                assert clock['first_known_ns']>=release['commit_ns']
        (directory/'originals.json').write_text(canonical(originals))
        p.terminate(); p.wait(5); xp.unlink(missing_ok=True)
        # Cold recovery refuses missing authority/original evidence; exact restoration resumes.
        original_id=r7['originals'][0]['event']['source']['external_fact_id']
        for table,key in [('x_exact_commands','a'),('x_sim_originals',original_id)]:
            with sqlite3.connect(db) as c:
                row=c.execute('select * from '+table+' where id=?',(key,)).fetchone()
                c.execute('delete from '+table+' where id=?',(key,))
            blocked=subprocess.run(argv(),env=env(),capture_output=True,timeout=5)
            assert blocked.returncode!=0, (table,blocked.stdout,blocked.stderr)
            with sqlite3.connect(db) as c: c.execute('insert into '+table+' values('+','.join('?' for _ in row)+')',row)
        with sqlite3.connect(db) as c:
            receipt=c.execute('select execute_evidence from x_exact_commands where id=?',('a',)).fetchone()[0]
            c.execute('update x_exact_commands set execute_evidence=NULL where id=?',('a',))
        blocked=subprocess.run(argv(),env=env(),capture_output=True,timeout=5)
        assert blocked.returncode!=0
        with sqlite3.connect(db) as c: c.execute('update x_exact_commands set execute_evidence=? where id=?',(receipt,'a'))
        # Joint derived-evidence damage cannot erase obligations still present in input batches.
        for mode in ['fee-delete','fee-modify','bust-delete']:
            bid='b10' if mode=='bust-delete' else 'b1'
            with sqlite3.connect(db) as c:
                saved_result=c.execute('select result from x_sim_batches where id=?',(bid,)).fetchone()[0]
                result=json.loads(saved_result); kind='bust' if mode=='bust-delete' else 'fee'
                target=next(d for d in result['originals'] if d['event']['kind']==kind)
                oid=target['event']['source']['external_fact_id']
                saved_original=c.execute('select raw from x_sim_originals where id=?',(oid,)).fetchone()[0]
                saved_bust=c.execute('select * from x_sim_busts where id=?',(oid,)).fetchone()
                if mode=='fee-modify':
                    target['event']['amount']+=1
                    c.execute('update x_sim_originals set raw=? where id=?',(canonical(target),oid))
                else:
                    result['originals']=[d for d in result['originals'] if d['event']['source']['external_fact_id']!=oid]
                    c.execute('delete from x_sim_originals where id=?',(oid,))
                    if saved_bust: c.execute('delete from x_sim_busts where id=?',(oid,))
                c.execute('update x_sim_batches set result=? where id=?',(canonical(result),bid))
            blocked=subprocess.run(argv(),env=env(),capture_output=True,timeout=5)
            assert blocked.returncode!=0,(mode,blocked.stdout,blocked.stderr)
            with sqlite3.connect(db) as c:
                c.execute('update x_sim_batches set result=? where id=?',(saved_result,bid))
                c.execute('insert or replace into x_sim_originals values(?,?)',(oid,saved_original))
                if saved_bust: c.execute('insert into x_sim_busts values(?,?,?)',saved_bust)
        with sqlite3.connect(db) as c:
            binding=c.execute("select v from x_sim_meta where k='authority_profile'").fetchone()[0]
            c.execute("delete from x_sim_meta where k='authority_profile'")
        changed_profile=copy.deepcopy(profile)
        changed_profile['history_simulation']['e']['socket']=str(sockets/'wrong-e')
        changed_profile['history_simulation']['s']['socket']=str(sockets/'wrong-s')
        for candidate in [profile,changed_profile]:
            pp.write_text(canonical(candidate)); blocked=subprocess.run(argv(),env=env(),capture_output=True,timeout=5)
            assert blocked.returncode!=0 and b'IdentityConflict' in blocked.stderr
            with sqlite3.connect(db) as c: assert c.execute("select v from x_sim_meta where k='authority_profile'").fetchone() is None
        pp.write_text(canonical(profile))
        with sqlite3.connect(db) as c: c.execute("insert into x_sim_meta values('authority_profile',?)",(binding,))
        p=start(); assert request(xp,'QueryBatch','b7')==r7; assert request(xp,'QueryOriginal',original_id)==r7['originals'][0]
        assert request(xp,'QueryBatch','b10')==bust_batches[0] and request(xp,'QueryBatch','b11')==bust_batches[1]
        p.terminate(); p.wait(5); xp.unlink(missing_ok=True)
        # Frozen namespace cannot be re-opened as venue/dry-run profile.
        bad=copy.deepcopy(profile); bad.pop('history_simulation'); bad['mode']='dry-run'; pp.write_text(canonical(bad))
        blocked=subprocess.run(argv(),env=env(),capture_output=True,timeout=5)
        assert blocked.returncode!=0 and (fixture or b'IdentityConflict' in blocked.stderr)
        pp.write_text(canonical(profile))
        # Recovery uses the same clock predicate and refuses a corrupted earlier-than-release record.
        with sqlite3.connect(db) as c:
            clock=json.loads(clocks[0][0]); clock.update(received_ns=98,first_known_ns=99,commit_ns=101)
            c.execute('update x_exact_commands set accept_clock=? where id=?',(canonical(clock),'a'))
        blocked=subprocess.run(argv(),env=env(),capture_output=True,timeout=5)
        assert blocked.returncode!=0 and b'InvalidDomain' in blocked.stderr
        print(name+': PASS '+('test runtime with post-claim exit43' if fixture else 'production X binary')+', invalid input stays Accepted, S reply loss/query-only recovery, unique S authorization, pending epoch2 rejected without writes, full fee/bust replay, frozen binding, atomic first initialization recovery')
        return originals
    finally:
        if p.poll() is None: p.terminate(); p.wait(5)
        log.close(); e.sock.close(); s.sock.close()
        for file in sockets.iterdir(): file.unlink()
        sockets.rmdir()
a=run('run-a'); b=run('run-b'); assert a==b, 'fresh process raw originals differ'
print('PASS two fresh processes: byte-identical raw fill+fee originals')
