"""#1414：只运行三 fresh；首 composite 实际独审成功及根单次授权为前门。"""
from pathlib import Path
import errno
import hashlib
import importlib.util
import json
import os
import shutil
import signal
import socket
import stat
import subprocess
import sys
import time

FREEZE = Path(__file__).resolve().parent
WANTED = [('second_open_up','b'),('second_open_down','a'),('second_open_down','b')]

def require(ok, message):
    if not ok:
        raise ValueError(message)

def read(path):
    return json.loads(Path(path).read_text())

def digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for block in iter(lambda:f.read(1048576),b''):
            h.update(block)
    return h.hexdigest()

def dump(path, value):
    with Path(path).open('x') as f:
        json.dump(value, f, ensure_ascii=False, indent=2)
        f.write('\n')

def binding_mismatches(bindings):
    changed = []
    for path, hashed in bindings.items():
        try:
            if digest(path) != hashed:
                changed.append(path)
        except OSError:
            changed.append(path)
    return changed

def verify_snapshots(directory, expected):
    seen = {}
    for entry in read(directory/'MANIFEST.json'):
        require(Path(entry['snapshot']).name == entry['snapshot'] and entry['source'] not in seen, '快照身份不唯一')
        p = directory/entry['snapshot']
        require(p.stat().st_size == entry['bytes'] and digest(p) == entry['sha256'], '快照字节改变')
        seen[entry['source']] = entry['sha256']
    require(seen == expected, '快照未绑定完整源码/二进制')

def gate_evidence(evidence, case, capture_status):
    capture = read(evidence/'RESULT.json')
    oracle = read(evidence/'query-oracle.json')
    browser = read(evidence/'browser/RESULT.json')
    recovery = read(evidence/'normal-recovery.json')
    require(capture['status'] == capture_status and capture['input_count'] == 40
            and capture['core_records'] == 221 and not capture['cleanup']['errors'], '采集未完整')
    require(oracle['status'] == 'passed' and oracle['case'] == case and oracle['cuts'] == 40
            and set(oracle['oracle_cuts']) == {'15','26','27','31','38','39','40'}, '完整DB/proof七cut oracle未过')
    require(browser['status'] == 'passed' and browser['cuts'] == ['27','40'] and not browser['page_errors'], '真实browser未过')
    require(recovery['equal'] is True and recovery['cuts'] == [27,40], '正常恢复未过')

def gate_first(plan, sources):
    # 事前选定的新schema；旧r2 FAIL不得经兜底门改写。
    from explicit_composite import verify_explicit_first
    return verify_explicit_first(plan,sources)

def cleanup_run(plan, run):
    directory = Path(run['directory'])
    config = read(directory/'config.json')
    e = directory/'evidence'
    actions = []
    # 单次runtime的原execute已回收；wrapper仍独立核所属S/Q，任一错误不跳另一项。
    if e.exists():
        for service in ('q','s'):
            try:
                stopped = subprocess.run([plan['dependencies']['python'],str(Path(plan['repo'])/'s_session/s_service_control.py'),
                    'stop-'+service,'--config',str(directory/'config.json'),'--state-dir',str(e/'control')],
                    capture_output=True, timeout=30)
                (directory/('wrapper-stop-'+service+'.stdout')).write_bytes(stopped.stdout)
                (directory/('wrapper-stop-'+service+'.stderr')).write_bytes(stopped.stderr)
                actions.append({'service':service,'exit':stopped.returncode})
            except BaseException as exc:
                actions.append({'service':service,'error':repr(exc)})
    services = []
    for service in ('s','q'):
        handle = e/'control'/(service+'.process.json')
        if handle.exists():
            rec = read(handle)
            ps = subprocess.run(['ps','-p',str(rec['pid']),'-o','pid=,ppid=,stat=,comm='],capture_output=True,text=True)
            services.append({'service':service,'pid':rec['pid'],'ps_exit':ps.returncode,'rows':ps.stdout.strip(),'error':ps.stderr.strip()})
    absent = all(x['ps_exit'] == 1 and not x['rows'] and not x['error'] for x in services)
    with socket.socket() as s:
        s.settimeout(.5)
        port_errno = s.connect_ex(('127.0.0.1',int(config['port'])))
    path = Path(config['socket'])
    if path.exists():
        expected = read(e/'control/s.process.json')['socket_identity']
        def identity():
            st = path.lstat()
            require(stat.S_ISSOCK(st.st_mode) and st.st_uid == os.getuid(), 'socket身份/用户不符')
            return {'path':str(path),'device':st.st_dev,'inode':st.st_ino,'uid':st.st_uid}
        require(absent and identity() == expected, '所属服务未退/socket身份不符')
        with socket.socket(socket.AF_UNIX) as s:
            s.settimeout(.5)
            try:
                s.connect(str(path))
                raise ValueError('所属socket仍服务')
            except OSError as exc:
                require(exc.errno == errno.ECONNREFUSED, 'socket未证明拒连')
        require(identity() == expected, 'socket身份漂移')
        path.unlink()
    result = {'services':services,'actions':actions,'port_connect_errno':port_errno,'socket_absent':not path.exists()}
    result['passed'] = absent and port_errno == errno.ECONNREFUSED and not path.exists() and all(x.get('exit') == 0 for x in actions)
    dump(directory/'WRAPPER-CLEANUP.json', result)
    require(result['passed'], '所属资源清理未过')

def main():
    require(sys.argv[1:] == ['--execute'], '只能由根单次授权执行')
    plan, auth = read(FREEZE/'PLAN.json'), read(FREEZE/'ROOT-REMAINING-AUTHORIZATION.json')
    require(auth['status'] == 'ROOT_APPROVED_REMAINING_MATRIX_ONCE' and auth['output'] == plan['output'], '缺根单次授权')
    require(auth['plan_sha256'] == digest(FREEZE/'PLAN.json'), '计划指纹改变')
    for name in ('remaining-driver.py','remaining-harness.py','CURRENT-SOURCES.json',
                 'explicit_composite.py','dependency_identity.py','reuse_state.py','completion_io.py'):
        require(auth['scripts'][name] == digest(FREEZE/name), '执行物指纹改变')
    require(auth['independent_review'], '缺独立准备审')
    for path, sha in auth['independent_review'].items():
        require(digest(path) == sha, '独立准备审改变')
    require(plan['limits_seconds'] == {'arm':900,'collect_core':120,'frozen_oracle':180,'oracle_node':120,'query':30,'browser':30,'browser_cleanup':5}, '窗口改变')
    require([(x['case'],x['arm']) for x in plan['runs']] == WANTED, '三fresh范围错误')
    sources = read(FREEZE/'CURRENT-SOURCES.json')
    for path, sha in {**sources,**plan['frozen_evidence']}.items():
        require(digest(path) == sha, '来源改变：'+path)
    first_evidence = gate_first(plan, sources)
    dep = plan['dependencies']
    dependency_bindings = {dep['python']:dep['python_sha256'],dep['node']:dep['node_sha256'],
                           dep['browser']:dep['browser_sha256'],str(Path(dep['playwright_module'])/'package.json'):dep['playwright_package_sha256']}
    for path, sha in dependency_bindings.items():
        require(digest(path) == sha, '依赖二进制改变')
    from dependency_identity import verify_dependencies
    from explicit_composite import verify_dependency_plan, verify_comparison_binding
    verify_dependency_plan(plan)
    dependency_pre=verify_dependencies(dependency_bindings,dep['python'],plan['declared_python_dependency'],
                    browser_path=dep['browser'],chrome_declaration=plan['declared_chrome_dependency'])
    require(dependency_pre['status']=='PASS','remaining严格依赖初核失败')
    out = Path(plan['output'])
    require(not out.exists(), '输出必须新且独占')
    for run in plan['runs']:
        with socket.socket() as s:
            s.bind(('127.0.0.1',run['port']))
    sys.path[:0] = [str(Path(plan['repo'])/'s_session/tests'),str(Path(plan['repo'])/'s_session')]
    spec = importlib.util.spec_from_file_location('remaining_harness',FREEZE/'remaining-harness.py')
    harness = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(harness)
    runplan = harness.prepare(out,Path(plan['repo'])/'rust/target/release/s_structure_session',Path(dep['python']),Path(dep['node']),Path(dep['playwright_module']),plan['port_base'])
    require([(x['case'],x['arm']) for x in runplan['runs']] == WANTED, '真实准备范围不符')
    require(runplan['run_deadline_seconds'] == 900 and runplan['operation_deadline_seconds'] == 30
            and runplan['offline_deadlines_seconds'] == {'collect_core':120,'frozen_oracle':180,'oracle_node':120}, '实际helper门不符')
    runtime_root = Path(plan['repo'])/'s_session'
    binary = Path(plan['repo'])/'rust/target/release/s_structure_session'
    expected_runtime = {path:sha for path,sha in sources.items()
                        if (Path(path).is_relative_to(runtime_root) and Path(path).suffix in {'.py','.js','.cjs','.html','.sh','.json','.jsonl'})
                        or Path(path) == binary}
    require(runplan['source_sha256'] == expected_runtime, 'fresh实际原runtime源集合不等于已采纳正式冻结源')
    require(digest(plan['matrix_reference']['path']) == plan['matrix_reference']['sha256'], '矩阵依据改变')
    old_matrix = read(plan['matrix_reference']['path'])
    for run, expected in zip(runplan['runs'],[x for x in old_matrix['arms'] if (x['case'],x['arm']) in WANTED]):
        directory = Path(run['directory'])
        for name, item in expected['historical_generated_inputs'].items():
            require(digest(directory/name) == item['sha256'], '自然冻结输入改变')
        require(not Path(read(directory/'config.json')['socket']).exists(), 'socket已有')
    freeze_copy = out/'freeze'
    freeze_copy.mkdir()
    for name in ('PLAN.json','ROOT-REMAINING-AUTHORIZATION.json','remaining-driver.py','remaining-harness.py','CURRENT-SOURCES.json',
                 'explicit_composite.py','dependency_identity.py','reuse_state.py','completion_io.py'):
        shutil.copyfile(FREEZE/name, freeze_copy/name)
    os.environ.update(plan['environment_overrides'])
    package_hashes = {str(FREEZE/name):digest(FREEZE/name) for name in ('PLAN.json','ROOT-REMAINING-AUTHORIZATION.json','remaining-driver.py','remaining-harness.py','CURRENT-SOURCES.json',
                     'explicit_composite.py','dependency_identity.py','reuse_state.py','completion_io.py')}
    package_hashes.update(auth['independent_review'])
    dump(out/'PACKAGE-BINDING.json', package_hashes)
    dump(out/'DEPENDENCY-PRE.json',dependency_pre)
    def cancel(_signum, _frame):
        raise RuntimeError('根中断；原harness回收本arm，wrapper核资源后保留失败')
    signal.signal(signal.SIGTERM, cancel)
    signal.signal(signal.SIGINT, cancel)
    started = time.time()
    completed = []
    cause = None
    try:
        for run in runplan['runs']:
            try:
                harness.execute({**runplan,'runs':[run]},out)
            finally:
                cleanup_run(plan,run)
            harness.validate_source_binding(runplan,run)
            gate_evidence(Path(run['directory'])/'evidence',run['case'],'CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER')
            require(read(Path(run['directory'])/'EXIT.json') == {'exit':0}, 'fresh EXIT非0')
            e = Path(run['directory'])/'evidence'
            queries = [json.loads(line) for line in (e/'events.jsonl').read_text().splitlines() if json.loads(line)['kind']=='bounded_query']
            commands = next(x for x in old_matrix['arms'] if (x['case'],x['arm']) == (run['case'],run['arm']))
            require([x['command'] for x in queries] == commands['public_commands'] + commands['normal_recovery_commands']
                    and len(queries) == 124 and all(x['deadline_seconds']==30 for x in queries), '124真实Q或单调用30门不符')
            for query in queries:
                receipt = read(e/'http/commands'/query['command']['id']/'RESULT.json')
                require(receipt['result']=='captured_validated_commit' and receipt['full_state_installed'] is True
                        and receipt['sources_unchanged'] is True, '真实Q安装收据未过')
            completed.append(run)
        # 前门在完整双跑比较前再核；旧r4 FAIL从未交给compare_completed。
        first_evidence = gate_first(plan,sources)
        from tb02c_compare import compare_exact_traces
        pairs = [('second_open_up', first_evidence/'semantic-core.jsonl', Path(completed[0]['directory'])/'evidence/semantic-core.jsonl'),
                 ('second_open_down', Path(completed[1]['directory'])/'evidence/semantic-core.jsonl', Path(completed[2]['directory'])/'evidence/semantic-core.jsonl')]
        comparisons = [{'case':case,**compare_exact_traces(a,b,harness.expected_layout(40))} for case,a,b in pairs]
        if comparisons[0]['status']=='PASS':
            verify_comparison_binding(comparisons[0],plan['first_composite'])
        result = {'status':'PASS' if all(x['status']=='PASS' for x in comparisons) else 'NOT_VERIFIED','comparisons':comparisons,
                  'first_composite':plan['first_composite'],'fresh_runs':WANTED,'old_r4_status':'FAIL','scope':'TB-02-D，非整图完成'}
        dump(out/'COMPOSITE-COMPARISON-RESULT.json',result)
        require(result['status']=='PASS','原完整221布局/全部字段/字节未通过')
    except BaseException as exc:
        cause = repr(exc)
        raise
    finally:
        mismatches = binding_mismatches({**sources,**package_hashes})
        evidence_mismatches = binding_mismatches(plan['frozen_evidence'])
        dependency_mismatches = binding_mismatches(dependency_bindings)
        try:
            dependency_post=verify_dependencies(dependency_bindings,dep['python'],plan['declared_python_dependency'],
                dependency_pre['receipts'],browser_path=dep['browser'],chrome_declaration=plan['declared_chrome_dependency'])
        except Exception as exc:
            dependency_post={'status':'FAIL','receipts':{},'mismatches':list(dependency_bindings),
                             'errors':[{'failure_stage':'postcheck','error':repr(exc)}]}
        dump(out/'DEPENDENCY-POST.json',dependency_post)
        dependency_mismatches=sorted(set(dependency_mismatches)|set(dependency_post['mismatches']))
        matrix_mismatches = binding_mismatches({plan['matrix_reference']['path']:plan['matrix_reference']['sha256']})
        dump(out/'SOURCE-POSTCHECK.json',{'count':len(sources),'mismatches':mismatches,
             'evidence_count':len(plan['frozen_evidence']),'evidence_mismatches':evidence_mismatches,
             'dependency_mismatches':dependency_mismatches,'matrix_mismatches':matrix_mismatches})
        dump(out/'EXECUTION.json',{'cause':cause,'completed_fresh':[x['directory'] for x in completed],'elapsed_seconds':time.time()-started})
        require(not mismatches and not evidence_mismatches and not dependency_mismatches
                and not matrix_mismatches,'source/package/证据/依赖/矩阵后核失败')

if __name__ == '__main__':
    main()
