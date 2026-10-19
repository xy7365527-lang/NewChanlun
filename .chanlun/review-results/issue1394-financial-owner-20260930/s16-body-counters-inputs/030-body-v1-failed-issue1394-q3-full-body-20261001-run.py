#!/usr/bin/env python3
"""Private #1394 S16 Q21 full Content fixture; one owned clone, one Q, no install."""
import ast, contextlib, hashlib, json, os, shutil, signal, socket, sqlite3, stat, struct, subprocess, sys, time, traceback
from pathlib import Path
if not __debug__:
    raise SystemExit('optimized Python disables acceptance asserts')
sys.dont_write_bytecode = True
ROOT = Path('/Users/silencehan/Projects/NewChanlun-1394-content-v3')
HERE = Path(__file__).resolve().parent
SOURCE_DIR = ROOT / '.scratch/issue1394-formal-fullclock-prefix16-20261001/release1/A'
SOURCE = SOURCE_DIR / 's.sqlite'
Q21 = ROOT / '.scratch/issue1394-s3-q3-joint-20261001/revision1/inputs/query-resources.json'
BINARY = Path('/Users/silencehan/Projects/NewChanlun-1394-same-material-20261001/.scratch/issue1394-same-material-20261001/target/release/s_structure_session')
BINARY_BINDING = BINARY.parents[2] / 'SOURCE-BINARY-BINDING.json'
FROZEN = ROOT / '.scratch/issue1394-fullclock-s16-q3-probe-20261001/harness.py'
sys.path.insert(0, str(ROOT/'s_session'))
import s_service_control as control
import s_socket_client as client
canonical = client.canonical
D = b'NewChanlun/SContent/1\0'

def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def write(name, value):
    with (HERE/name).open('x', encoding='utf-8') as f: json.dump(value, f, ensure_ascii=False, sort_keys=True, separators=(',',':')); f.write('\n')
def stamp(p):
    p=Path(p); s=p.stat()
    return {'path':str(p),'size':s.st_size,'st_blocks':s.st_blocks,'allocated_bytes_st_blocks':s.st_blocks*512,
            'inode':s.st_ino,'device':s.st_dev,'mtime_ns':s.st_mtime_ns,'sha256':sha(p)}
def triplet(db): return {s:stamp(str(db)+s) for s in ('','-wal','-shm')}
def owned_blocks(): return sum(p.stat().st_blocks*512 for p in HERE.rglob('*') if p.is_file())
def extract(path,names):
    tree=ast.parse(path.read_text()); nodes=[n for n in tree.body if isinstance(n,(ast.FunctionDef,ast.ClassDef)) and n.name in names]
    assert {n.name for n in nodes}==set(names)
    exec(compile(ast.Module(body=nodes,type_ignores=[]),str(path),'exec'),globals())
extract(FROZEN,{'digest','save','fp','files','envelope_check','http','snapshot','defs'})
OLDQ=ROOT/'.scratch/issue1394-current-q3-formal-20261001'
defs(OLDQ/'run_formal_actual.py',['frame','sequence','Edge','value_digest','projection_root'])

def compare_db(db):
    expected=json.loads((SOURCE_DIR/'final.fullDB.json').read_bytes())
    assert sha(SOURCE_DIR/'final.fullDB.json')=='8a7f8f248d1ffad4081555d2900bfd9bc73fb32971167edb544cb15292ea8174'
    with contextlib.closing(sqlite3.connect(db.as_uri()+'?mode=ro',uri=True)) as c:
        c.execute('PRAGMA query_only=ON');c.execute('BEGIN')
        schema=c.execute('SELECT type,name,tbl_name,rootpage,sql FROM sqlite_schema ORDER BY type,name').fetchall()
        assert canonical(schema)==canonical(expected['schema']), 'schema mismatch'
        props={k:c.execute('PRAGMA '+k).fetchone()[0] for k in ('application_id','auto_vacuum','encoding','page_size','user_version')}
        assert canonical(props)==canonical(expected['db_properties'])
        stats={};native={};blob_cells=0
        for name in sorted(expected['tables']):
            got=expected['tables'][name]
            cols=c.execute('PRAGMA table_xinfo("'+name+'")').fetchall()
            assert canonical(cols)==canonical(got['columns']), ('columns',name)
            rows=[]
            for row in c.execute('SELECT * FROM "'+name+'"'):
                blob_cells+=sum(isinstance(v,bytes) for v in row)
                rows.append([{'blob_hex':v.hex()} if isinstance(v,bytes) else v for v in row])
            rows.sort(key=canonical)
            assert canonical(rows)==canonical(got['rows']), ('full rows or BLOB cell mismatch',name)
            assert len(rows)==got['row_count'] and hashlib.sha256(canonical(rows)).hexdigest()==got['sha256'], ('table digest',name)
            stats[name]={'rows':len(rows),'sha256':hashlib.sha256(canonical(rows)).hexdigest()}
            if name=='s_content_nodes':
                names=[x[1] for x in cols]
                for row in rows:
                    n=dict(zip(names,row));b=bytes.fromhex(n['body']['blob_hex'])
                    assert type(n['tag']) is int and 0<=n['tag']<=255,('native tag SQL type',n['tag'])
                    f=frame(n['tag'],b)
                    assert bytes.fromhex(n['digest']['blob_hex'])==f, ('native node F',n)
                    native['sc1-'+f.hex()]=True
        assert len(stats)==24 and sum(v['rows'] for v in stats.values())==48403 and len(native)==47671 and blob_cells==96397
    return stats,native

def main():
    assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()=='8fb179091a81e274214056603171b759decfdd08'
    assert subprocess.check_output(['git','rev-parse','HEAD:rust'],cwd=ROOT,text=True).strip()=='7196a77fef56cf87cbe91dc4b135326a142e2395'
    assert sha(BINARY)=='c098bf76bb7870443b7564714ba2f9110f2c5593a3180ca13f9888c01ee16d17'
    binary_binding=json.loads(BINARY_BINDING.read_bytes())
    assert binary_binding['rust_tree']=='7196a77fef56cf87cbe91dc4b135326a142e2395'
    assert binary_binding['binary']['sha256']==sha(BINARY) and binary_binding['binary']['path']==str(BINARY)
    assert binary_binding['patch']['sha256']=='677164b5a0ca4e43ab325502853f3ea72d4ad01313fb43a5fe8152124f19068e'
    assert sha(binary_binding['patch']['path'])==binary_binding['patch']['sha256']
    assert sha(Q21)=='50ea5bf865de8a37d8187069d70bee85ca4caac9c759a60ad2e3c717743d4eb1'
    global limits,cfg,db
    limits=json.loads(Q21.read_bytes())['limits']; assert len(limits)==21
    holder=subprocess.run(['lsof','-nP',str(SOURCE),str(SOURCE)+'-wal',str(SOURCE)+'-shm'],capture_output=True,text=True)
    write('source-lsof.json',{'returncode':holder.returncode,'stdout':holder.stdout,'stderr':holder.stderr})
    assert holder.returncode==1 and not holder.stdout and not holder.stderr,'frozen source has holder or lsof failed'
    first=triplet(SOURCE);time.sleep(.3);second=triplet(SOURCE)
    assert first==second,'source triple unstable'
    write('source-preflight.json',{'first':first,'second':second,'source_sqlite_opened':False})
    clone=HERE/'source';clone.mkdir(exist_ok=False);db=clone/'s.sqlite'
    methods={}
    for suffix in ('','-wal','-shm'):
        src=Path(str(SOURCE)+suffix);dest=Path(str(db)+suffix)
        result=subprocess.run(['cp','-c',str(src),str(dest)],capture_output=True,text=True)
        if result.returncode!=0: shutil.copyfile(src,dest);methods[suffix]='bytecopy'
        else: methods[suffix]='COW cp -c'
    assert triplet(SOURCE)==second,'frozen source changed across copy'
    cloned=triplet(db)
    for suffix in cloned:
        assert cloned[suffix]['sha256']==second[suffix]['sha256'] and cloned[suffix]['inode']!=second[suffix]['inode']
    write('clone-preflight.json',{'source':second,'clone':cloned,'methods':methods,'owned_allocated_bytes_st_blocks':owned_blocks()})
    assert owned_blocks()<=50*1024*1024,'private disk cap before Q'
    stats,native=compare_db(db)
    write('source-fulltable-check.json',{'same_as_frozen_final_fullDB':True,'table_count':len(stats),'rows':sum(v['rows'] for v in stats.values()),
         'blob_cells':96397,'native_nodes_F_verified':len(native),'tables':stats})
    template=json.loads((SOURCE_DIR/'Controller.json').read_bytes())
    inputs=HERE/'inputs';inputs.mkdir(exist_ok=False)
    input_copies={}
    for name,p in [('catalog',Path(template['catalog'])),('profile',Path(template['profile'])),('clock_plan',Path(template['clock_plan'])),('query_resource_config',Q21)]:
        copy=inputs/(name+'.json');shutil.copyfile(p,copy)
        assert sha(copy)==sha(p),('copied input differs',name)
        input_copies[name]={'original':stamp(p),'owned_copy':stamp(copy),'exact_sha256_match':True}
    assert sha(inputs/'query_resource_config.json')=='50ea5bf865de8a37d8187069d70bee85ca4caac9c759a60ad2e3c717743d4eb1'
    port_probe=socket.socket();port_probe.bind(('127.0.0.1',0));port=port_probe.getsockname()[1];port_probe.close()
    cfg=dict(template)
    cfg.update(db=str(db),binary=str(BINARY),port=str(port),socket='/private/tmp/nc1394fb-'+hashlib.sha256(str(HERE).encode()).hexdigest()[:12]+'.sock',
               browser=str(ROOT/'s_session/browser/index.html'),catalog=str(inputs/'catalog.json'),profile=str(inputs/'profile.json'),
               clock_plan=str(inputs/'clock_plan.json'),query_resource_config=str(inputs/'query_resource_config.json'))
    write('Controller.json',cfg);cfg=control.read_config(HERE/'Controller.json')
    write('BINDING.json',{'scope':'formal S16 Content/source private fixture only','root':str(ROOT),'head':'8fb179091a81e274214056603171b759decfdd08',
         'rust_tree':'7196a77fef56cf87cbe91dc4b135326a142e2395','binary_selector':'explicit current release file',
         'binary':stamp(BINARY),'source_repo':str(ROOT),'binary_build_source_repo':str(BINARY.parents[4]),
         'binary_build_head':binary_binding['head'],'binary_binding':stamp(BINARY_BINDING),
         'binary_candidate_patch':stamp(binary_binding['patch']['path']),'binary_candidate_source':binary_binding['source'],
         'original_source':second,'owned_clone':cloned,'four_named_inputs':input_copies,'Q21':stamp(Q21),'Q21_values':limits,
         'controller':stamp(ROOT/'s_session/s_service_control.py'),'collector':stamp(HERE/'collect.js'),'runner':stamp(HERE/'run.py'),
         'fixed_source_fullDB':stamp(SOURCE_DIR/'final.fullDB.json'),'controller_identity':{k:cfg[k] for k in ('session_id','session_generation','source_namespace','source_epoch','query_epoch')}})
    controller=control.Controller(cfg,HERE/'state')
    handle=None;original_popen=subprocess.Popen;node=None;qproc=None
    def capture_popen(argv,*args,**kwargs):
        nonlocal handle
        p=original_popen(argv,*args,**kwargs)
        if argv and argv[0]==str(BINARY) and argv[1]=='query-serve':
            assert handle is None,'single Q only';handle=p
            write('q-process.json',{'pid':p.pid,'argv':argv,'uid':os.getuid(),'pgid':os.getpgid(p.pid),'control_nonce':kwargs['env'].get('S_CONTROL_INSTANCE_ID')})
        return p
    subprocess.Popen=capture_popen
    result={'status':'RUNNING','client_atomic_install':'NOT_ATTEMPTED','client_physical_allocation':'UNMEASURED',
            'client_heap_allocation':'UNMEASURED','full_9011':'NOT_ATTEMPTED','product_frontend_1460':'OUT_OF_SCOPE'}
    try:
        started=time.monotonic_ns();ready=controller.start_q(cfg['query_epoch']);write('q-ready.json',ready)
        result['start_q_elapsed_ns']=time.monotonic_ns()-started
        assert handle is not None and controller.status('q')['pid']==handle.pid
        qproc=json.loads((HERE/'q-process.json').read_bytes())
        assert ready['reply']['control_instance_id']==qproc['control_nonce']
        code,read=http('ready-S16');assert code==200 and read==ready['reply'] and read['cut']['generation']=='16'
        page,rows,root=snapshot('snapshot');assert len(rows)==472
        # The frozen helper supplies the independent projection-root formula,
        # while its compact summary omits page metadata. Recheck all 16 raw
        # HTTP page envelopes before persisting the complete oracle input.
        page_fields=('fixed_cut','catalog','counts','scope','omissions','order_version','cut_projection_digest','validated_capture_digest')
        cursor=0
        for i in range(16):
            wire=json.loads((HERE/f'snapshot-{i}.response.json').read_bytes())
            p=wire['payload']
            assert p['ok'] is True and p['offset']==str(cursor),('snapshot offset',i)
            assert all(canonical(p[k])==canonical(page[k]) for k in page_fields),('snapshot page metadata',i)
            assert p['rows']==rows[cursor:cursor+len(p['rows'])],('snapshot row continuity',i)
            assert 0<len(p['rows'])<=int(limits['max_page_size']),('snapshot page size',i)
            cursor+=len(p['rows'])
            assert p['done']==(i==15) and (p['next_token'] is None)==p['done'],('snapshot terminal',i)
            if not p['done']:
                assert p['next_token']['offset']==str(cursor),('snapshot next offset',i)
                following=json.loads((HERE/f'snapshot-{i+1}.request.http').read_bytes().split(b'\r\n\r\n',1)[1])['payload']
                assert following=={'op':'snapshot','snapshot_token':p['next_token']},('snapshot token handoff',i)
        assert cursor==472 and page['counts']['total']=='472' and root==projection_root(page,rows)
        save('snapshot.complete.json',{'rows':rows,'pages':16,'projection_ref':root,**{k:page[k] for k in page_fields}})
        assert page['fixed_cut']['cut_generation']=='16'
        original_publication=json.loads((SOURCE_DIR/'input-16.exchange.json').read_bytes())['result']['response']['payload']['publication']
        for a,b in [('generation','cut_generation'),('structure_cut','structure_cut'),('index_frontier','index_frontier'),('input_frontier','input_frontier'),('catalog_revision','catalog_revision')]:
            assert original_publication[a]==page['fixed_cut'][b],(a,b)
        # old snapshot() writes snapshot.complete.json directly, consumed by collect.js.
        node_log=(HERE/'collector.stdout').open('xb');node_err=(HERE/'collector.stderr').open('xb')
        try: node=original_popen(['node',str(HERE/'collect.js'),str(HERE),str(port)],stdout=node_log,stderr=node_err,start_new_session=True)
        finally: node_log.close();node_err.close()
        write('collector-process.json',{'pid':node.pid,'argv':['node',str(HERE/'collect.js'),str(HERE),str(port)],'pgid':os.getpgid(node.pid)})
        rss=[];deadline=time.monotonic()+7200
        while node.poll() is None:
            if time.monotonic()>deadline: raise TimeoutError('collector 2h observation bound; no Q restart')
            if owned_blocks()>50*1024*1024: raise RuntimeError('private 50 MiB physical st_blocks budget exceeded')
            p=subprocess.run(['ps','-p',str(handle.pid)+','+str(node.pid),'-o','pid=,rss='],capture_output=True,text=True)
            rss.append({'monotonic_ns':time.monotonic_ns(),'ps':p.stdout.strip()})
            time.sleep(1)
        write('rss-samples.json',rss)
        assert node.returncode==0,('JS collector failed',node.returncode)
        body=json.loads((HERE/'body-summary.json').read_bytes());roots=json.loads((HERE/'content-node-roots.json').read_bytes())
        # Projection metadata and sequence nodes are constructed by Q and need not
        # be rows in persistent s_content_nodes. GraphView proves every proof F;
        # the separate Python oracle checks reachability from the independent root.
        persistent_roots=set(roots)&set(native)
        constructed_roots=set(roots)-set(native)
        assert body['rows']==472 and body['unique_nodes']==len(roots)
        row_lines=(HERE/'row-digests.jsonl').read_text().splitlines();assert len(row_lines)==472
        for i,line in enumerate(row_lines):
            row=json.loads(line);assert row['ordinal']==i and row['record_ref']==rows[i]['record'] and row['family']==rows[i]['family'] and row['row_key']==rows[i]['key']
        result.update(status='PASS_JS_FULL_BODY_PROOF',snapshot_pages=16,snapshot_rows=472,projection_ref=root,
                      content_requests=body['content_requests'],content_unique_nodes=len(roots),content_http_wire_bytes=body['response_wire_bytes'],
                      content_body_bytes=body['response_body_bytes'],projection_body_sha256=body['projection_body_sha256'],
                      proof_nodes_in_frozen_native_db=len(persistent_roots),proof_nodes_constructed_by_Q=len(constructed_roots),
                      six_family_counts=body['families'],independent_python_body_oracle='PENDING_ROOT_AGENT',
                      private_allocated_bytes_st_blocks=owned_blocks())
    except BaseException as exc:
        result.update(status='FAIL',error=repr(exc),traceback=traceback.format_exc())
    finally:
        subprocess.Popen=original_popen
        if node is not None:
            try:
                if node.poll() is None:
                    assert os.getpgid(node.pid)==node.pid,'owned collector process group changed'
                    os.killpg(node.pid,signal.SIGTERM)
                try: result['node_exit_code']=node.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    assert node.poll() is None and os.getpgid(node.pid)==node.pid,'collector kill identity'
                    os.killpg(node.pid,signal.SIGKILL)
                    result['node_exit_code']=node.wait(timeout=5)
                result['same_node_popen_reaped']=True
            except BaseException as exc: result['node_cleanup_error']=repr(exc);result['status']='FAIL_CLEANUP'
        if handle is not None:
            try:
                if handle.poll() is None: result['stop_receipt']=controller.stop('q',signal.SIGTERM)
                result['q_exit_code']=handle.wait(timeout=5);result['same_q_popen_reaped']=True
            except BaseException as exc:
                result['q_term_error']=repr(exc);result['status']='FAIL_CLEANUP'
                try:
                    identity_argv=qproc['argv'] if qproc else json.loads((HERE/'q-process.json').read_bytes())['argv']
                    if handle.poll() is None and os.getpgid(handle.pid)==handle.pid and control.process_identity(handle.pid,identity_argv) is not None:
                        os.killpg(handle.pid,signal.SIGKILL)
                    result['q_exit_code']=handle.wait(timeout=5);result['same_q_popen_reaped']=True
                except BaseException as e:result['q_cleanup_error']=repr(e);result['status']='FAIL_CLEANUP'
        try:
            result['source_semantic_after_Q_equal_frozen']=compare_db(db)[0]==stats
            if not result['source_semantic_after_Q_equal_frozen']: result['status']='FAIL_SOURCE_MUTATION'
        except BaseException as exc: result['source_postcheck_error']=repr(exc);result['status']='FAIL_SOURCE_POSTCHECK'
        try:
            s=socket.socket();s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1);s.bind(('127.0.0.1',port));s.close();result['http_port_rebound']=True
        except BaseException as exc:result['port_cleanup_error']=repr(exc);result['status']='FAIL_CLEANUP'
        try: controller.close();result['controller_closed']=True
        except BaseException as exc:result['controller_close_error']=repr(exc);result['status']='FAIL_CLEANUP'
        try:
            result['original_source_after']=triplet(SOURCE);result['original_source_unchanged']=result['original_source_after']==second
            if not result['original_source_unchanged']:result['status']='FAIL_ORIGINAL_CHANGED'
        except BaseException as exc:result['original_postcheck_error']=repr(exc);result['status']='FAIL_ORIGINAL_POSTCHECK'
        try:
            result['private_allocated_bytes_st_blocks_final']=owned_blocks()
            if result['private_allocated_bytes_st_blocks_final']>50*1024*1024:result['status']='FAIL_PRIVATE_BUDGET'
        except BaseException as exc:result['owned_budget_error']=repr(exc);result['status']='FAIL_PRIVATE_BUDGET'
        try: write('CLEANUP.json',{k:result.get(k) for k in ('same_q_popen_reaped','q_exit_code','same_node_popen_reaped','node_exit_code','stop_receipt','http_port_rebound','original_source_unchanged','source_semantic_after_Q_equal_frozen','q_cleanup_error','node_cleanup_error','controller_close_error')})
        except BaseException as exc: result['cleanup_receipt_error']=repr(exc);result['status']='FAIL_CLEANUP'
        if any(k in result for k in ('node_cleanup_error','q_term_error','q_cleanup_error','controller_close_error','port_cleanup_error','cleanup_receipt_error')):
            result['status']='FAIL_CLEANUP'
        try: write('RESULT.json',result)
        except BaseException as exc:
            result['result_write_error']=repr(exc);result['status']='FAIL_CLEANUP'
            (HERE/'RESULT-FALLBACK.txt').write_text(repr(result)+'\n')
    return result

if __name__=='__main__':
    try: result=main()
    except BaseException as exc:
        result={'status':'FAIL_PREFLIGHT','error':repr(exc),'traceback':traceback.format_exc(),'client_atomic_install':'NOT_ATTEMPTED','client_physical_allocation':'UNMEASURED'}
        if not (HERE/'RESULT.json').exists(): write('RESULT.json',result)
    print(json.dumps({k:result.get(k) for k in ('status','content_requests','snapshot_rows','error','q_cleanup_error')},ensure_ascii=False))
    sys.exit(0 if result['status']=='PASS_JS_FULL_BODY_PROOF' else 1)
